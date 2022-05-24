//! This module contains segments in which files are downloaded.

#![allow(dead_code)]

use std::io::Write;
use crate::messages::{ByteRange, Request, Response};


#[derive(Debug, Eq, PartialEq, Clone)]
enum Status {
    Received,
    NotReceived,
}

impl Default for Status {
    fn default() -> Self {
        Self::NotReceived
    }
}


#[derive(Debug, Clone)]
pub struct Segment {
    byte_range: ByteRange,
    status: Status,
    data: Box<[u8]>,
}

impl Segment {
    pub const SIZE: usize = Response::DATA_SIZE;

    pub fn new(byte_range: ByteRange) -> Self {
        Self::with_buffer(byte_range, Vec::with_capacity(Self::SIZE).into_boxed_slice())
    }

    pub fn with_buffer(byte_range: ByteRange, data: Box<[u8]>) -> Self {
        Self { byte_range, status: Default::default(), data }
    }

    pub fn set_data(&mut self, data: &[u8]) {
        self.data.copy_from_slice(data)
    }

    pub fn is_received(&self) -> bool {
        self.status == Status::Received
    }

    pub fn len(&self) -> usize {
        Self::SIZE
    }

    pub fn yield_buffer(self) -> Box<[u8]> {
        self.data
    }

    pub fn byte_range(&self) -> &ByteRange {
        &self.byte_range
    }

    pub fn request(&self) -> Request {
        Request::new(&self.byte_range)
    }
}

impl Write for Segment {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if buf.len() != Self::SIZE {
            panic!("invalid segment size, expected: {}, got {}", Self::SIZE, buf.len());
        }
        self.data.as_mut().write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl From<ByteRange> for Segment {
    fn from(byte_range: ByteRange) -> Self {
        Self::new(byte_range)
    }
}

impl AsRef<[u8]> for Segment {
    fn as_ref(&self) -> &[u8] {
        self.data.as_ref()
    }
}

impl AsMut<[u8]> for Segment {
    fn as_mut(&mut self) -> &mut [u8] {
        self.data.as_mut()
    }
}
