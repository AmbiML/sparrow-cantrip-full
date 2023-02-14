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

use cantrip_io as io;

use crate::get_u8;
use crate::LineReadError;
use crate::BACKSPACE;
use crate::DELETE;

const LINE_MAX: usize = 128;

pub struct LineReader {
    // Owned by LineReader to facilitate static allocation.
    buf: [u8; LINE_MAX],

    // Length of the last valid character in the buffer.
    end: usize,
}

impl Default for LineReader {
    fn default() -> Self { Self::new() }
}

impl LineReader {
    pub fn new() -> Self {
        Self {
            buf: [0u8; LINE_MAX],
            end: 0,
        }
    }

    fn delete_backward_char(&mut self, output: &mut dyn io::Write) -> Result<(), LineReadError> {
        // No text to delete
        if self.end == 0 {
            return Ok(());
        }

        self.end -= 1;
        output.write(&[BACKSPACE, b' ', BACKSPACE])?;
        Ok(())
    }

    fn insert_char(&mut self, c: u8, output: &mut dyn io::Write) -> Result<(), LineReadError> {
        if self.end >= LINE_MAX {
            return Ok(());
        }

        self.buf[self.end] = c;
        self.end += 1;

        output.write(&[c])?;
        Ok(())
    }

    pub fn read_line(
        &mut self,
        output: &mut dyn io::Write,
        input: &mut dyn io::Read,
    ) -> Result<&str, LineReadError> {
        self.end = 0;

        loop {
            match get_u8(input)? {
                BACKSPACE => self.delete_backward_char(output)?,
                DELETE => self.delete_backward_char(output)?,

                // Normal printable ASCII case
                // 32 is space, 126 is ~
                c @ 32u8..=126u8 => self.insert_char(c, output)?,

                // Newline -- finish the loop.
                b'\r' | b'\n' => break,

                // Unprintable character, non-ASCII or edit keys
                _ => (),
            };
        }

        if self.end > 0 {
            output.write(&[b'\n'])?;
        }

        Ok(core::str::from_utf8(&self.buf[0..self.end])?)
    }
}
