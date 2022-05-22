mod registry;
mod segment;
mod util;

use libc;

use std::collections::VecDeque;
use std::env;
use std::fs::{File, OpenOptions};
use std::io;
use std::iter;
use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};
use std::ops::{Index, IndexMut, Range};
use std::os::unix::prelude::*;
use std::path::{Iter, Path};

use crate::segment::Segment;
use crate::registry::{EventType, Registry};

/* Sliding window:

Zaczynamy od czytania konfiguracji
adres ip, port, rozmiar pliku do pobrania, nazwa pliku wyjściowego <- przekazane jako ARGUMENTY DO PROGRAMU.
stworz socket'a udp.

1. resend each packet after approx. 1.5 s (max delay)
2. check if given segment was already received.

1. sleep until there is read on socket
2. read from socket until empty
3. check how many packets can we written to file, write them and move window.
*/

#[derive(Debug)]
struct SegmentByteRangeIter {
    base_byte_offset: usize,
    file_size: usize,
    seg_size: usize,
}

impl SegmentByteRangeIter {
    pub fn new(file_size: usize, seg_size: usize) -> Self {
        Self { base_byte_offset: 0, file_size, seg_size }
    }
}

impl Iterator for SegmentByteRangeIter {
    type Item = Range<usize>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.base_byte_offset < self.file_size {
            let diff = self.file_size - self.base_byte_offset;
            let start = self.base_byte_offset;
            let end = start + if diff < self.seg_size { diff } else { self.seg_size }
            self.base_byte_offset = end;
            Some(start..end)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests_segment_byte_range_iter {
    use super::SegmentByteRangeIter;

    #[test]
    fn test_1() {
        let mut seg_iter = SegmentByteRangeIter::new(1000, 300);
        assert_eq!(Some(0..300), seg_iter.next());
        assert_eq!(Some(300..600), seg_iter.next());
        assert_eq!(Some(600..900), seg_iter.next());
        assert_eq!(Some(900..1000), seg_iter.next());
        assert_eq!(None, seg_iter.next());
    }

    #[test]
    fn test_2() {
        let mut seg_iter = SegmentByteRangeIter::new(100, 1000);
        assert_eq!(None, seg_iter.next());
    }
}


// region Window
#[derive(Debug)]
pub struct Window {
    queue: VecDeque<Segment>,
    acked_buffer: Vec<Segment>,
    read_seg_count: usize,
}

impl Window {
    const SIZE: usize = 1000;

    pub fn new(size: usize) -> Self {
        let queue = VecDeque::with_capacity(size);
        let acked_buffer = Vec::new();
        Self { queue, acked_buffer, read_seg_count: 0 }
    }

    fn slide_len(&self) -> usize {
        self.queue.iter().take_while(|segment| segment.is_acknowledged()).count()
    }

    fn shrink(&mut self) -> &[Segment] {
        debug_assert_eq!(self.acked_buffer.is_empty());
        self.acked_buffer.extend(self.queue.drain(0..self.slide_len()));
        &self.acked_buffer[..]
    }

    fn extend(&mut self, segment_byte_ranges: &mut impl Iterator<Item=Range<usize>>) {
        self.queue.extend(self.acked_buffer.drain(..)
            .zip(segment_byte_ranges)
            .map(|(segment, byte_range)| {
                let Segment { buffer, .. } = segment;
                Segment::with_buffer(byte_range, buffer)
            })
        )
    }

    pub fn unacknowledged_segments(&mut self) -> impl Iterator<Item=&mut Segment> {
        self.queue.iter_mut().filter(|segment| segment.is_acknowledged())
    }

    pub fn acknowledge(&mut self, index: usize) {
        self.queue[index].acknowledge();
    }
}

impl Index<Range<usize>> for Window {
    type Output = Segment;

    fn index(&self, seg_byte_range: Range<usize>) -> &Self::Output {
        let seg_index = seg_byte_range.start / Segment::SIZE;
        &self.queue[seg_index - self.read_seg_count]
    }
}

impl IndexMut<Range<usize>> for Window {
    fn index_mut(&mut self, seg_byte_range: Range<usize>) -> &mut Self::Output {
        let seg_index = seg_byte_range.start / Segment::SIZE;
        &mut self.queue[seg_index - self.read_seg_count]
    }
}

impl Default for Window {
    fn default() -> Self {
        Self { queue: VecDeque::with_capacity(Self::SIZE), acked_buffer: Vec::new(), read_seg_count: 0 }
    }
}

impl IntoIterator for Window {
    type Item = Segment;
    type IntoIter = <VecDeque<Segment> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.queue.into_iter()
    }
}
// endregion


pub struct Downloader {
    socket: UdpSocket,
    registry: Registry,
    window: Window,
    server_address: SocketAddrV4,
    file_size: usize,
    file_handle: File
}

/* TODO: what's the host address? */

enum Notification {
    Timeout,
    ReadReady,
}


impl Downloader {
    const HOST_ADDRESS: SocketAddrV4 = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 54321);

    pub fn new(server_address: SocketAddrV4, file_name: &str, file_size: usize) -> Self {
        let socket = UdpSocket::bind(Self::HOST_ADDRESS).map_err(|err| {
            util::fail_with_message(format!("could bind socket: {err}"));
        }).unwrap();

        let mut registry = Registry::new().map_err(|err| {
            util::fail_with_message(format!("could not create registry: {err}"));
        }).unwrap();

        let file_handle = OpenOptions::new()
            .append(true)
            .create(true)
            .open(file_name)
            .map_err(|err| util::fail_with_message(format!("error occurred while opening the file: {err}")))
            .unwrap();

        registry.add_interest(EventType::Read, socket.as_raw_fd());

        Self {
            socket,
            registry,
            window: Window::default(),
            server_address,
            file_size,
            file_handle,
        }
    }

    /* warning: as of current implementation there is only one item registered.
       This function works correctly if this assumption holds. */
    fn await_socket_read_ready(&mut self) -> Notification {
        match self.registry.await_events() {
            registry::Notification::Timeout => Notification::Timeout,
            registry::Notification::Events(_) => Notification::ReadReady,
        }
    }

    fn byte_ranges(&self) -> SegmentByteRangeIter {
        SegmentByteRangeIter::new(self.file_size, Segment::SIZE)
    }

    fn send_window(&mut self) -> io::Result<()> {
        for segment in self.window.unacknowledged_segments() {
            self.socket.send_to(segment.as_mut(), self.server_address)?;
        }
        Ok(())
    }

    pub fn download(&mut self) -> io::Result<()> {
        let mut byte_ranges = self.byte_ranges().peekable();
        while let Some(byte_range) = byte_ranges.peek() {
            self.window.extend(&mut byte_ranges);
            self.socket.set_nonblocking(false)?;
            self.send_window()?;
            self.socket.set_nonblocking(true)?;
            match self.await_socket_read_ready() {
                Notification::Timeout => { self.send_window(); }
                Notification::ReadReady => {
                    /* TODO: read all datagram currently in the socket and ack them appropriately.
                         This will require parsing of bytes in actual buffer.
                         Idea: Segment can simply have accessors that just return and optionally parse
                         shouldn't the datagrams be read into the correct buffer anyway?
                         since we pass reference to segment's buffer into send_to?
                         then all we would need to do is parse received datagram into appropriate byte range?
                         FIND OUT!
                    */
                }
            }
        }
        Ok(())
    }
}
pub struct DownloaderConfig {
    pub address: SocketAddrV4,
    pub file_name: String,
    pub size: usize,
}

impl FromIterator<String> for DownloaderConfig {
    fn from_iter<T: IntoIterator<Item=String>>(iter: T) -> Self {
        let address
    }
}


fn main() {



    let downloader = Downloader::new()
}
