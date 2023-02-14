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
use crate::CONTROL_A;
use crate::CONTROL_B;
use crate::CONTROL_D;
use crate::CONTROL_E;
use crate::CONTROL_F;
use crate::CONTROL_K;
use crate::CONTROL_U;
use crate::CONTROL_W;
use crate::DELETE;

const LINE_MAX: usize = 128;

// Borrowed from the reference at
// http://www.braun-home.net/michael/info/misc/VT100_commands.htm
const BELL: u8 = 7u8;
const ESC: u8 = 27u8;
const VT100_SAVE_CURSOR: [u8; 3] = [ESC, b'[', b's'];
const VT100_RESTORE_CURSOR: [u8; 3] = [ESC, b'[', b'u'];
const VT100_ERASE_TO_EOL: [u8; 3] = [ESC, b'[', b'K'];
const VT100_CURSOR_LEFT: [u8; 4] = [ESC, b'[', b'1', b'D'];
const VT100_CURSOR_RIGHT: [u8; 4] = [ESC, b'[', b'1', b'C'];

pub struct LineReader {
    // Owned by LineReader to facilitate static allocation.
    buf: [u8; LINE_MAX],

    // Length of the last valid character in the buffer.
    end: usize,

    // Position of the cursor in the buffer.
    point: usize,
}

impl Default for LineReader {
    fn default() -> Self { Self::new() }
}

impl LineReader {
    pub fn new() -> Self {
        Self {
            buf: [0u8; LINE_MAX],
            end: 0,
            point: 0,
        }
    }

    fn send_bell(&mut self, output: &mut dyn io::Write) -> Result<(), LineReadError> {
        output.write(&[BELL])?;
        Ok(())
    }

    fn update_display(&mut self, output: &mut dyn io::Write) -> Result<(), LineReadError> {
        output.write(&VT100_ERASE_TO_EOL)?;
        output.write(&self.buf[self.point..self.end])?;

        // Go left the number of characters we just wrote to keep the cursor in place.
        for _ in self.point..self.end {
            // Use backspace since we expect backspace to be non-destructive, and it's faster
            output.write(&[BACKSPACE])?;
        }

        Ok(())
    }

    fn delete_backward_char(&mut self, output: &mut dyn io::Write) -> Result<(), LineReadError> {
        // No text to delete
        if self.end == 0 {
            return self.send_bell(output);
        }

        // Point at the beginning -- nothing to delete.
        if self.point == 0 {
            output.write(&[BELL])?;
            return Ok(());
        }

        self.buf.copy_within(self.point..self.end, self.point - 1);
        self.end -= 1;
        self.point -= 1;
        output.write(&[BACKSPACE])?;

        self.update_display(output)
    }

    fn delete_backward_word(&mut self, output: &mut dyn io::Write) -> Result<(), LineReadError> {
        // No text to delete
        if self.end == 0 {
            output.write(&[BELL])?;
            return Ok(());
        }

        // Point at the beginning -- nothing to delete.
        if self.point == 0 {
            output.write(&[BELL])?;
            return Ok(());
        }

        // If prior char is a space, skip it during the search
        let search_start = match self.buf[self.point - 1] {
            b' ' => self.point - 1,
            _ => self.point,
        };
        let word_start = self.buf[0..search_start]
            .iter()
            .rposition(|&c| c == b' ')
            .unwrap_or(0);
        let word_len = self.point - word_start;

        self.buf.copy_within(self.point..self.end, word_start);
        self.end -= word_len;
        self.point -= word_len;

        for _ in 0..word_len {
            output.write(&[BACKSPACE])?;
        }

        self.update_display(output)
    }

    fn delete_forward_char(&mut self, output: &mut dyn io::Write) -> Result<(), LineReadError> {
        // No text to delete
        if self.end == 0 {
            output.write(&[BELL])?;
            return Ok(());
        }

        // At EoL, nothing to delete
        if self.point >= self.end {
            output.write(&[BELL])?;
            return Ok(());
        }

        self.buf.copy_within(self.point + 1..self.end, self.point);
        self.end -= 1;

        self.update_display(output)
    }

    fn insert_char(&mut self, c: u8, output: &mut dyn io::Write) -> Result<(), LineReadError> {
        if self.end >= LINE_MAX {
            output.write(&[BELL])?;
            return Ok(());
        }

        // If we're inserting in the middle or beginning of the string, we need
        // to copy the buffer to the right by one.
        if self.point < self.end {
            self.buf.copy_within(self.point..self.end, self.point + 1);
        }

        self.buf[self.point] = c;
        self.end += 1;
        self.point += 1;

        output.write(&[c])?;

        self.update_display(output)
    }

    fn forward_point(&mut self, output: &mut dyn io::Write) -> Result<(), LineReadError> {
        if self.point >= self.end {
            self.send_bell(output)
        } else {
            self.point += 1;
            output.write(&VT100_CURSOR_RIGHT)?;
            Ok(())
        }
    }

    fn backward_point(&mut self, output: &mut dyn io::Write) -> Result<(), LineReadError> {
        if self.point == 0 {
            self.send_bell(output)
        } else {
            self.point -= 1;
            output.write(&VT100_CURSOR_LEFT)?;
            Ok(())
        }
    }

    fn beginning_of_line(&mut self, output: &mut dyn io::Write) -> Result<(), LineReadError> {
        self.point = 0;
        output.write(&VT100_RESTORE_CURSOR)?;
        Ok(())
    }

    fn end_of_line(&mut self, output: &mut dyn io::Write) -> Result<(), LineReadError> {
        for _ in self.point..self.end {
            output.write(&VT100_CURSOR_RIGHT)?;
        }

        self.point = self.end;
        Ok(())
    }

    fn kill_line_forward(&mut self, output: &mut dyn io::Write) -> Result<(), LineReadError> {
        self.end = self.point;
        output.write(&VT100_ERASE_TO_EOL)?;
        Ok(())
    }

    fn kill_line(&mut self, output: &mut dyn io::Write) -> Result<(), LineReadError> {
        self.end = 0;
        self.point = 0;

        output.write(&VT100_RESTORE_CURSOR)?;
        output.write(&VT100_ERASE_TO_EOL)?;

        Ok(())
    }

    pub fn read_line(
        &mut self,
        output: &mut dyn io::Write,
        input: &mut dyn io::Read,
    ) -> Result<&str, LineReadError> {
        self.end = 0;
        self.point = 0;

        // Save our prompt position first
        output.write(&VT100_SAVE_CURSOR)?;

        loop {
            match get_u8(input)? {
                // Non-destructive editing keys
                CONTROL_A => self.beginning_of_line(output)?,
                CONTROL_E => self.end_of_line(output)?,
                CONTROL_B => self.backward_point(output)?,
                CONTROL_F => self.forward_point(output)?,

                // Destructive editing keys
                BACKSPACE => self.delete_backward_char(output)?,
                DELETE => self.delete_backward_char(output)?,
                CONTROL_D => self.delete_forward_char(output)?,
                CONTROL_W => self.delete_backward_word(output)?,
                CONTROL_K => self.kill_line_forward(output)?,
                CONTROL_U => self.kill_line(output)?,

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
