//! This module exposes the downloader struct which allows for asynchronous
//! communication with the server and download of files.

#![allow(dead_code)]

use std::fmt::Debug;
use std::fs::{File, OpenOptions};
use std::io;
use std::io::Write as _;
use std::fmt::Write as _;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, UdpSocket};
use std::os::unix::prelude::*;

use crate::messages::{ByteRange, Request, Response};
use crate::segment::Segment;
use crate::registry::{EventType, Registry};
use crate::window::Window;
use crate::{registry, util};


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
    type Item = ByteRange;

    fn next(&mut self) -> Option<Self::Item> {
        if self.base_byte_offset < self.file_size {
            let diff = self.file_size - self.base_byte_offset;
            let start = self.base_byte_offset;
            let end = start + if diff < self.seg_size { diff } else { self.seg_size };
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
        assert_eq!(Some(0..100), seg_iter.next());
        assert_eq!(None, seg_iter.next());
    }
}


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
    const HOST_IP_ADDRESS: Ipv4Addr = Ipv4Addr::new(127, 0, 0, 1);
    const HOST_PORT: u16 = 54321;

    pub fn new(server_address: SocketAddrV4, file_name: &str, file_size: usize) -> Self {
        let socket = UdpSocket::bind(SocketAddrV4::new(Self::HOST_IP_ADDRESS, Self::HOST_PORT)).map_err(|err| {
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

        registry.add_interest(EventType::Read, socket.as_raw_fd()).map_err(|err|
            util::fail_with_message(format!("could not register interesed for "))
        );

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
        This function works correctly if this assumption holds.
    */
    fn await_socket_read_ready(&mut self) -> Notification {
        match self.registry.await_events() {
            registry::Notification::Timeout => Notification::Timeout,
            registry::Notification::Events(_) => Notification::ReadReady,
        }
    }

    fn byte_ranges(&self) -> SegmentByteRangeIter {
        SegmentByteRangeIter::new(self.file_size, Segment::SIZE)
    }

    fn send_window_with_buf(&mut self, request_buffer: &mut String) -> io::Result<()> {
        for segment in self.window.unacknowledged_segments() {
            request_buffer.clear();
            write!(request_buffer, "{}", segment.request()).unwrap();
            self.socket.send_to(segment.as_mut(), self.server_address)?;
        }
        Ok(())
    }

    fn store_segments(&mut self, message_buffer: &mut [u8]) {
        loop {
            match self.socket.recv_from(message_buffer) {
                Ok((message_size, SocketAddr::V4(sender))) if sender == self.server_address && Response::is_message_size_valid(message_size)  => {
                    let response = Response::new(message_buffer);
                    /* If segment is outside of window we ignore it. */
                    if self.window.contains(response.byte_range()) {
                        /* Otherwise we copy response's data into its buffer. */
                        let segment = &mut self.window[response.byte_range()];
                        segment.write_all(message_buffer);
                    }
                }
                Ok(_) => continue,
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => break,
                Err(err) => util::fail_with_message(format!("error occurred while reading from the socket. {}", err))
            };
        }
    }

    pub fn download(&mut self) -> io::Result<()> {
        let mut byte_ranges = self.byte_ranges();
        let mut request_buffer= String::with_capacity(Request::MAX_SIZE);
        let mut response_buffer = Vec::with_capacity(Response::MAX_SIZE).into_boxed_slice();
        let mut bytes_downloaded = 0;

        while bytes_downloaded < self.file_size {
            self.window.extend(&mut byte_ranges);
            self.socket.set_nonblocking(false)?;
            self.send_window_with_buf(&mut request_buffer)?;
            self.socket.set_nonblocking(true)?;
            match self.await_socket_read_ready() {
                Notification::Timeout   => {
                    let segments = self.window.shrink();
                    for segment in segments {
                        self.file_handle.write_all(segment.as_ref()).map_err(|err| {
                            util::fail_with_message(format!("could not append to file: {err}"));
                        });
                        bytes_downloaded += segment.len();
                    }}
                Notification::ReadReady => self.store_segments(&mut response_buffer),
            };
        }
        Ok(())
    }
}


pub struct DownloaderConfig {
    pub address: SocketAddrV4,
    pub file_name: String,
    pub size: usize,
}
