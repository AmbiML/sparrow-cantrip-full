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

#![no_std]

use cantrip_io as io;
use core2::io::{Cursor, Read};

/// Rx io trait that returns data from a byte string.
pub struct Rx<'a> {
    data: Cursor<&'a [u8]>,
}
impl<'a> Rx<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data: Cursor::new(data),
        }
    }
}
impl<'a> io::Read for Rx<'a> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.data.read(buf).or(Err(io::Error))
    }
}

/// Tx io trait that uses the kernel if console output is
/// is supported, otherwise discards all writes.
pub struct Tx {}
impl Tx {
    pub fn new() -> Self { Self {} }
}
impl Default for Tx {
    fn default() -> Self { Self::new() }
}

impl io::Write for Tx {
    #[cfg(not(feature = "CONFIG_PRINTING"))]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> { Ok(buf.len() as usize) }
    #[cfg(feature = "CONFIG_PRINTING")]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        for &b in buf {
            unsafe {
                sel4_sys::seL4_DebugPutChar(b);
            }
        }
        Ok(buf.len() as usize)
    }

    fn flush(&mut self) -> io::Result<()> { Ok(()) }
}
