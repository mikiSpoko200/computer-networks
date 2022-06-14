//! Mikołaj Depta 328690
//!
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

    pub fn new(segment_byte_ranges: &mut impl Iterator<Item=ByteRange>) -> Self {
        let mut queue = VecDeque::with_capacity(Self::SIZE);
        queue.extend(segment_byte_ranges.map(Segment::new).take(Self::SIZE));
        let received_buffer = Vec::new();
        Self { queue, received_buffer, read_seg_count: 0 }
    }

    fn slide_len(&self) -> usize {
        self.queue.iter().take_while(|segment| segment.is_received()).count()
    }

    pub fn shrink(&mut self) -> &[Segment] {
        self.received_buffer.extend(self.queue.drain(0..self.slide_len()));
        self.read_seg_count += self.received_buffer.len();
        self.received_buffer.as_ref()
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn extend(&mut self, segment_byte_ranges: &mut impl Iterator<Item=ByteRange>) {
        while let Some(free_segment) = self.received_buffer.pop() {
            if let Some(byte_range) = segment_byte_ranges.next() {
                let mut data = free_segment.yield_buffer();
                data.clear();
                self.queue.push_back(Segment::with_buffer(byte_range, data));
            } else {
                break;
            }
        }
    }

    /* TODO: test this */
    pub fn contains(&self, other: &Range<usize>) -> bool {
        self.read_seg_count * Segment::SIZE <= other.start &&
            other.start < (self.read_seg_count + self.queue.len()) * Segment::SIZE
    }

    pub fn unacknowledged_segments(&mut self) -> impl Iterator<Item=&mut Segment> {
        self.queue.iter_mut().filter(|segment| !segment.is_received())
    }
}

#[cfg(test)]
mod tests {
    use super::Window;
    use crate::downloader::SegmentByteRangeIter;

    #[test]
    fn test_contains_edge_1() {
        let mut seg_iter = SegmentByteRangeIter::new(2000000, 500);
        let window = Window::new(&mut seg_iter);

        let mut seg_iter_copy = SegmentByteRangeIter::new(2000000, 2000000);

        assert!(window.contains(dbg!(&seg_iter_copy.next().unwrap())));
    }

    #[test]
    fn test_contains_edge_2() {
        let mut seg_iter = SegmentByteRangeIter::new(2000000, 500);
        let window = Window::new(&mut seg_iter);

        let mut seg_iter_copy = SegmentByteRangeIter::new(2000000, 500);

        assert!(window.contains(dbg!(&seg_iter_copy.nth(999).unwrap())));
    }

    #[test]
    fn test_contains_edge_3() {
        let mut seg_iter = SegmentByteRangeIter::new(2000000, 500);
        let window = Window::new(&mut seg_iter);

        let mut seg_iter_copy = SegmentByteRangeIter::new(2000000, 500);
        assert!(!window.contains(dbg!(&seg_iter_copy.nth(1000).unwrap())));
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

impl IntoIterator for Window {
    type Item = Segment;
    type IntoIter = <VecDeque<Segment> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.queue.into_iter()
    }
}
