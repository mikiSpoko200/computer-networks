//! Mikołaj Depta 328690
//!
//! This module contains segments in which files are downloaded.

#![allow(dead_code)]

use std::io::{Write};
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
    data: Vec<u8>,
}

impl Segment {
    pub const SIZE: usize = Response::DATA_SIZE;

    pub fn new(byte_range: ByteRange) -> Self {
        Self::with_buffer(byte_range, Vec::with_capacity(Self::SIZE))
    }

    pub fn with_buffer(byte_range: ByteRange, mut data: Vec<u8>) -> Self {
        data.clear();
        Self { byte_range, status: Default::default(), data }
    }

    pub fn set_data(&mut self, data: &[u8]) {
        self.data.clear();
        self.status = Status::Received;
        self.data.extend(data.iter().take(Self::SIZE));
    }

    pub fn is_received(&self) -> bool {
        self.status == Status::Received
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn yield_buffer(mut self) -> Vec<u8> {
        self.data.clear();
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
        self.set_data(buf);
        Ok(self.data.len())
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
