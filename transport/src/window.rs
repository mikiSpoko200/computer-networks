//! This module exposes the sliding window.
//! It helps manage simultaneous segment downloads and reliable assembly into final file.

#![allow(dead_code)]

use std::collections::VecDeque;
use std::ops::{Index, IndexMut, Range};
use crate::messages::ByteRange;

use crate::segment::Segment;


#[derive(Debug)]
pub struct Window {
    queue: VecDeque<Segment>,
    received_buffer: Vec<Segment>,
    read_seg_count: usize,
}

impl Window {
    const SIZE: usize = 1000;

    pub fn new(size: usize) -> Self {
        let queue = VecDeque::with_capacity(size);
        let received_buffer = Vec::new();
        Self { queue, received_buffer, read_seg_count: 0 }
    }

    fn slide_len(&self) -> usize {
        self.queue.iter().take_while(|segment| segment.is_received()).count()
    }

    pub fn shrink(&mut self) -> &[Segment] {
        debug_assert!(self.received_buffer.is_empty());
        self.received_buffer.extend(self.queue.drain(0..self.slide_len()));
        &self.received_buffer[..]
    }

    pub fn extend(&mut self, segment_byte_ranges: &mut impl Iterator<Item=Range<usize>>) {
        self.queue.extend(self.received_buffer.drain(..)
            .zip(segment_byte_ranges)
            .map(|(segment, byte_range)| {
                let data = segment.yield_buffer();
                Segment::with_buffer(byte_range, data)
            })
        )
    }

    /* TODO: test this */
    pub fn contains(&self, other: &Range<usize>) -> bool {
        self.read_seg_count * Segment::SIZE < other.start &&
            other.start <= (self.read_seg_count + self.queue.len()) * Segment::SIZE
    }

    pub fn unacknowledged_segments(&mut self) -> impl Iterator<Item=&mut Segment> {
        self.queue.iter_mut().filter(|segment| segment.is_received())
    }
}

impl Index<&ByteRange> for Window {
    type Output = Segment;

    fn index(&self, seg_byte_range: &ByteRange) -> &Self::Output {
        let seg_index = seg_byte_range.start / Segment::SIZE;
        &self.queue[seg_index - self.read_seg_count]
    }
}

impl IndexMut<&ByteRange> for Window {
    fn index_mut(&mut self, seg_byte_range: &ByteRange) -> &mut Self::Output {
        let seg_index = seg_byte_range.start / Segment::SIZE;
        &mut self.queue[seg_index - self.read_seg_count]
    }
}

impl Default for Window {
    fn default() -> Self {
        Self { queue: VecDeque::with_capacity(Self::SIZE), received_buffer: Vec::new(), read_seg_count: 0 }
    }
}

impl IntoIterator for Window {
    type Item = Segment;
    type IntoIter = <VecDeque<Segment> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.queue.into_iter()
    }
}
