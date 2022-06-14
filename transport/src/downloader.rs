//! Mikołaj Depta 328690
//!
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
use std::time::Duration;

use crate::messages::{ByteRange, Request, Response};
use crate::segment::Segment;
use crate::registry::{EventType, Registry};
use crate::window::Window;
use crate::{registry, util};
use crate::util::FailWithMessage;


#[derive(Debug)]
pub(crate) struct SegmentByteRangeIter {
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

enum Notification {
    Timeout,
    ReadReady(Duration),
}


pub struct Downloader {
    socket: UdpSocket,
    registry: Registry,
    window: Window,
    server_address: SocketAddrV4,
    segment_byte_ranges: SegmentByteRangeIter,
    file_size: usize,
    file_handle: File
}

impl Downloader {
    const TIMEOUT: Duration = Duration::from_millis(1000);

    pub fn new(server_address: SocketAddrV4, file_name: &str, file_size: usize) -> Self {
        let socket = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0)).map_err(|err| {
            util::fail_with_message(format!("could not bind the socket: {err}").as_ref());
        }).unwrap();

        let mut registry = Registry::new().or_fail_with_message("could not create registry");

        let file_handle = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(file_name).map_err(|err|{
                util::fail_with_message(format!("error occurred while opening the file {err}").as_str());
            }).unwrap();

        registry.add_interest(EventType::Read, socket.as_raw_fd()).map_err(|err|
            util::fail_with_message(format!("could not register interest for {}", err).as_ref())
        ).unwrap();

        let mut segment_byte_ranges = SegmentByteRangeIter::new(file_size, Segment::SIZE);
        let window = Window::new(&mut segment_byte_ranges);

        Self {
            socket,
            registry,
            window,
            segment_byte_ranges,
            server_address,
            file_size,
            file_handle,
        }
    }

    /* warning: as of current implementation there is only one item registered.
        This function works correctly if this assumption holds.
    */
    fn await_socket_read_ready(&mut self, timeout: &Duration) -> Notification {
        match self.registry.await_events(timeout) {
            registry::Notification::Timeout => Notification::Timeout,
            registry::Notification::Events(_, sleep_time) => Notification::ReadReady(sleep_time),
        }
    }

    fn send_window_with_buf(&mut self, request_buffer: &mut String) {
        for segment in self.window.unacknowledged_segments() {
            request_buffer.clear();
            write!(request_buffer, "{}", segment.request()).unwrap();
            self.socket.send_to(request_buffer.as_ref(), self.server_address)
                .or_fail_with_message("cannot send to the server");
        }
    }

    fn store_segments(&mut self, message_buffer: &mut [u8]) {
        loop {
            match self.socket.recv_from(message_buffer) {
                Ok((message_size, SocketAddr::V4(sender))) if sender == self.server_address && Response::is_message_size_valid(message_size)  => {
                    let response = Response::new(message_buffer);
                    /* If segment is outside of window we ignore it. */
                    if self.window.contains(response.byte_range()) {
                        /* If the segment is a duplicate we ignore it. */
                        let segment = &mut self.window[response.byte_range()];
                        if !segment.is_received() {
                            debug_assert_eq!(response.data().len(), response.byte_range().len());
                            segment.write_all(response.data()).unwrap();
                        }
                    }
                }
                Ok(_) => continue,
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => break,
                Err(err) => util::fail_with_message(format!("error occurred while reading from the socket. {}", err).as_ref())
            };
        }
    }

    pub fn download(&mut self) {
        let mut request_buffer = String::with_capacity(Request::MAX_SIZE);
        let mut response_buffer = vec![0; Response::MAX_SIZE].into_boxed_slice();
        let mut bytes_downloaded = 0;
        let mut timeout = Self::TIMEOUT;

        while bytes_downloaded < self.file_size {
            self.socket.set_nonblocking(false).or_fail_with_message("cannot set socket to blocking mode");
            self.send_window_with_buf(&mut request_buffer);
            self.socket.set_nonblocking(true).or_fail_with_message("cannot set socket to nonblocking mode");
            match self.await_socket_read_ready(&timeout) {
                Notification::Timeout   => {
                    timeout = Self::TIMEOUT;
                    let segments = self.window.shrink();
                    for segment in segments {
                        self.file_handle.write_all(segment.as_ref()).map_err(|err| {
                            util::fail_with_message(format!("could not append to file: {err}").as_ref());
                        }).unwrap();
                        bytes_downloaded += segment.len();
                    }}
                Notification::ReadReady(sleep_time) => {
                    timeout = timeout.saturating_sub(sleep_time);
                    self.store_segments(&mut response_buffer)
                },
            };
            self.window.extend(&mut self.segment_byte_ranges);
        }
        debug_assert_eq!(bytes_downloaded, self.file_size);
    }
}

impl From<DownloaderConfig> for Downloader {
    fn from(config: DownloaderConfig) -> Self {
        Self::new(config.address, config.file_name.as_ref(), config.size)
    }
}


pub struct DownloaderConfig {
    pub address: SocketAddrV4,
    pub file_name: String,
    pub size: usize,
}

impl DownloaderConfig {
    pub fn try_from<I>(mut iter: I) -> Self
    where I: Iterator<Item=String>
    {
        let ip_address = iter.nth(1)
            .or_fail_with_message("server ipv4 address missing")
            .parse()
            .or_fail_with_message("invalid format of server ipv4 address");
        let port = iter.next()
            .or_fail_with_message("server port missing")
            .parse()
            .or_fail_with_message("invalid format of server port");
        let file_name = iter.next()
            .or_fail_with_message("file name missing");
        let size = iter.next()
            .or_fail_with_message("file length missing")
            .parse()
            .or_fail_with_message("invalid format of file length");
        Self { address: SocketAddrV4::new(ip_address, port), size, file_name }
    }
}
