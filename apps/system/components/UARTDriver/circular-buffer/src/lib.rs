// Copyright 2022 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! A u8 buffer with a beginning and ending that wrap around a fixed size array.
//!
//! This is a FIFO queue that stops when buffer is full and returns None
//! when buffer is empty.

#![cfg_attr(not(test), no_std)]

const BUFFER_CAPACITY: usize = 512;

#[derive(Debug, PartialEq)]
pub struct Buffer {
    begin: usize,
    end: usize,
    size: usize,
    data: [u8; BUFFER_CAPACITY],
}

impl Buffer {
    pub const fn new() -> Buffer {
        Self {
            begin: 0,
            end: 0,
            size: 0,
            data: [0; BUFFER_CAPACITY],
        }
    }

    /// Resets buffer.
    ///
    /// This does not modify the data.
    pub fn clear(&mut self) {
        self.begin = 0;
        self.end = 0;
    }

    /// Returns true if buffer is empty, false otherwise.
    pub fn is_empty(&self) -> bool { self.size == 0 }

    /// Returns available data slot to be written.
    pub fn available_data(&self) -> usize { BUFFER_CAPACITY - self.size }

    /// Adds an item to the buffer.
    ///
    /// Returns false if buffer is full, otherwise true.
    #[must_use]
    pub fn push(&mut self, item: u8) -> bool {
        if self.available_data() == 0 {
            return false;
        }
        self.data[self.end] = item;
        self.end = Buffer::advance(self.end);
        self.size += 1;
        true
    }

    /// Remove an item at the front of the buffer.
    ///
    /// Returns None if buffer is empty, otherwise the result.
    #[must_use]
    pub fn pop(&mut self) -> Option<u8> {
        if self.is_empty() {
            return None;
        }
        let result = self.data[self.begin];
        self.begin = Buffer::advance(self.begin);
        self.size -= 1;
        Some(result)
    }

    /// Increments the begin or end marker and wrap around if necessary.
    fn advance(position: usize) -> usize { (position + 1) % BUFFER_CAPACITY }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pop will return pushed value.
    #[test]
    fn push_pop() {
        let mut buffer = Buffer::new();
        assert!(buffer.push(1));
        let result = buffer.pop();
        assert_eq!(Some(1), result);
    }

    /// Check that popping an empty buffer will return None.
    #[test]
    fn pop_empty_buffer() {
        let mut buffer = Buffer::new();
        assert_eq!(None, buffer.pop());
        assert!(buffer.push(1));
        assert_eq!(Some(1), buffer.pop());
        assert_eq!(None, buffer.pop());
    }

    /// Pop will return FIFO order.
    #[test]
    fn pop_fifo() {
        let mut buffer = Buffer::new();
        assert!(buffer.push(1));
        assert!(buffer.push(2));
        let result = buffer.pop();
        assert_eq!(Some(1), result);
    }

    /// Pushing to a full buffer will ignore the value and return False.
    #[test]
    fn push_full_buffer() {
        let mut buffer = Buffer::new();
        // Fill buffer to max capacity with u8s.
        for i in 0..BUFFER_CAPACITY {
            buffer.push((i % BUFFER_CAPACITY) as u8);
        }
        assert_eq!(BUFFER_CAPACITY, buffer.size);
        assert!(!buffer.push(1));
        assert_eq!(Some(0), buffer.pop());
    }

    /// Check that popping an empty buffer will return None.
    #[test]
    fn buffer_end_wrap() {
        let mut buffer = Buffer::new();
        assert!(buffer.push(1));
        assert_eq!(Some(1), buffer.pop());
        for i in 0..BUFFER_CAPACITY {
            buffer.push((i % BUFFER_CAPACITY) as u8);
        }
        assert_eq!(BUFFER_CAPACITY, buffer.size);
        assert_eq!(0, buffer.available_data());
        assert_eq!(Some(0), buffer.pop());
    }

    /// Check that push and pop over capacity will wrap and not break.
    #[test]
    fn buffer_begin_wrap() {
        let mut buffer = Buffer::new();
        for i in 0..(BUFFER_CAPACITY * 2) {
            let value = (i % BUFFER_CAPACITY) as u8;
            assert!(buffer.push(value));
            assert_eq!(Some(value), buffer.pop());
        }
        assert!(buffer.is_empty());
    }
}
