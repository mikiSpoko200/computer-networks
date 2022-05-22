use std::fmt::{Display, Formatter};
use std::ops::Range;

// region Status
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum Status {
    Acknowledged,
    Unacknowledged,
}

impl Default for Status {
    fn default() -> Self {
        Self::Unacknowledged
    }
}
// endregion


pub struct Request<'a> {
    byte_range: &'a Range<usize>,
}

impl Display for Request<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "GET {} {}\n", self.byte_range.start, self.byte_range.len())
    }
}


// region SegmentBuffer
#[derive(Debug, Clone)]
pub struct SegmentBuffer {
    data: [u8; SegmentBuffer::MAX_SIZE]
}

impl SegmentBuffer {
    const CONFIG_MAX_SIZE: usize = 19;
    const DATA_MAX_SIZE: usize = Segment::SIZE;
    const MAX_SIZE: usize = Self::CONFIG_MAX_SIZE + Self::DATA_MAX_SIZE;
}

impl Default for SegmentBuffer {
    fn default() -> Self {
        Self { data: [0; Self::MAX_SIZE] }
    }
}

impl AsRef<[u8]> for SegmentBuffer {
    fn as_ref(&self) -> &[u8] {
        &self.data[..]
    }
}

impl AsMut<[u8]> for SegmentBuffer {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.data[..]
    }
}
// endregion


/* TODO: abstract away the buffer type. 
    Trait bounds: 
        AsRef<[u8]>,
        AsMut<[u8]>,
        Read (?)
        Write (?)
*/
#[derive(Debug, Clone)]
pub struct Segment {
    byte_range: Range<usize>,
    buffer: Box<SegmentBuffer>,
    status: Status,
}

impl Segment {
    pub const SIZE: usize = 500;

    pub fn new(byte_range: Range<usize>) -> Self {
        Self::with_buffer(byte_range, Box::new(Default::default()))
    }

    pub fn with_buffer(byte_range: Range<usize>, buffer: Box<SegmentBuffer>) -> Self {
        Self { byte_range, buffer, status: Status::Unacknowledged }
    }

    pub fn acknowledge(&mut self) {
        self.status = Status::Acknowledged;
    }

    pub fn is_acknowledged(&self) -> bool {
        self.status == Status::Acknowledged
    }

    pub fn byte_range(&self) -> &Range<usize> {
        &self.byte_range
    }

    pub fn request(&self) -> Request<'_> {
        Request { byte_range: &self.byte_range }
    }
}

impl From<Range<usize>> for Segment {
    fn from(byte_range: Range<usize>) -> Self {
        Self::new(byte_range)
    }
}

impl AsRef<[u8]> for Segment {
    fn as_ref(&self) -> &[u8] {
        self.buffer.data.as_ref()
    }
}

impl AsMut<[u8]> for Segment {
    fn as_mut(&mut self) -> &mut [u8] {
        self.buffer.data.as_mut()
    }
}
