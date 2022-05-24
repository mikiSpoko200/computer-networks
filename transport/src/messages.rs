//! This module defines out custom download communication protocol.
//!
//! It exposes two message types: Response, and Request.

use std::ops::{Range, RangeInclusive};
use std::fmt::{Display, Formatter};
use std::str;


pub type ByteRange = Range<usize>;


pub const MAX_MESSAGE_SIZE: usize = Response::MAX_SIZE;


pub struct Response<'message> {
    message_bytes: &'message [u8],
    header: &'message str,
    data: &'message [u8],
    byte_range: Range<usize>,
}

impl<'message> Response<'message> {
    const BASE_HEADER_SIZE: usize = 7;
    const MAX_START: usize = 8;
    const MIN_START: usize = 1;
    const MAX_LENGTH: usize = 4;
    const MIN_LENGTH: usize = 1;
    pub const MIN_SIZE: usize = Self::DATA_SIZE + Self::MIN_HEADER_SIZE;
    pub const MAX_SIZE: usize = Self::DATA_SIZE + Self::MAX_HEADER_SIZE;
    pub const MAX_HEADER_SIZE: usize = Self::BASE_HEADER_SIZE + Self::MAX_START + Self::MAX_LENGTH;
    pub const MIN_HEADER_SIZE: usize = Self::BASE_HEADER_SIZE + Self::MIN_START + Self::MIN_LENGTH;


    pub const HEADER_SIZE_RANGE: RangeInclusive<usize> = Self::MIN_HEADER_SIZE..=Self::MAX_HEADER_SIZE;
    pub const SIZE_RANGE: RangeInclusive<usize> = Self::MIN_SIZE..=Self::MAX_SIZE;
    pub const DATA_SIZE: usize = 500;

    pub const fn is_message_size_valid(size: usize) -> bool {
        Self::MIN_SIZE <= size && size <= Self::MAX_SIZE
    }

    pub const fn is_header_size_valid(size: usize) -> bool {
        Self::MIN_HEADER_SIZE <= size && size <= Self::MAX_HEADER_SIZE
    }

    /* Note: this should be a TryFrom (return Result, not panic straight away) */

    /// Response Message in our communication protocol.
    ///
    /// # Panics
    ///
    /// This function will panic if message_bytes does not contain valid response.
    /// Response message is as follow:
    pub fn new(message_bytes: &'message [u8]) -> Self {
        let newline_index = message_bytes
            .iter()
            .position(|&byte| byte == '\n' as u8)
            .expect(r#"invalid response format, no Line Feed '\n') found"#);
        let (header_bytes, other ) = message_bytes.split_at(newline_index);
        let data = &other[1..];
        let header= str::from_utf8(header_bytes).expect("invalid response format, header is not valid utf");
        let mut words = header.split_whitespace();
        let start = words.nth(1)
            .expect("invalid response header format, start missing")
            .parse()
            .expect("invalid response header format, start is not a number");
        let length = words.next()
            .expect("invalid response header format, length missing")
            .parse::<usize>()
            .expect("invalid response header format, length is not a number");

        if data.len() != Response::DATA_SIZE {
            panic!("invalid response format, data section has invalid length. Expected: {}, got: {}", Response::DATA_SIZE, data.len());
        }

        Self { message_bytes, header, data, byte_range: start..(start+length) }
    }

    pub fn byte_range(&self) -> &Range<usize> {
        &self.byte_range
    }

    pub fn data(&self) -> &[u8] {
        self.data
    }
}


pub struct Request<'range> {
    byte_range: &'range ByteRange,
}

impl<'range> Request<'range> {
    const BASE_HEADER_SIZE: usize = 6;
    const MAX_START: usize = 8;
    const MIN_START: usize = 1;
    const MAX_LENGTH: usize = 4;
    const MIN_LENGTH: usize = 1;
    pub const MAX_SIZE: usize = Self::BASE_HEADER_SIZE + Self::MAX_START + Self::MAX_LENGTH;
    pub const MIN_SIZE: usize = Self::BASE_HEADER_SIZE + Self::MIN_START + Self::MIN_LENGTH;

    pub fn new(byte_range: &'range Range<usize>) -> Self {
        Self { byte_range }
    }
    
    pub fn header_length(&self) -> usize {
        let start = (self.byte_range.start as f64).log10() as usize;
        let len = (self.byte_range.len() as f64).log10() as usize;
        Self::BASE_HEADER_SIZE + start + len
    }
}

impl Display for Request<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "GET {} {}\n", self.byte_range.start, self.byte_range.len())
    }
}
