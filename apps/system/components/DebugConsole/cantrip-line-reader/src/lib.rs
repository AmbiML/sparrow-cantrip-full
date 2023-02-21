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

use cfg_if::cfg_if;
use core::fmt;

use cantrip_io as io;

pub enum LineReadError {
    IO(io::Error),
    Encoding(core::str::Utf8Error),
}

impl From<io::Error> for LineReadError {
    fn from(err: io::Error) -> LineReadError { LineReadError::IO(err) }
}

impl From<core::str::Utf8Error> for LineReadError {
    fn from(err: core::str::Utf8Error) -> LineReadError { LineReadError::Encoding(err) }
}

impl fmt::Display for LineReadError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LineReadError::IO(_) => write!(f, "IO error"),
            LineReadError::Encoding(_) => write!(f, "bad character encoding"),
        }
    }
}

fn get_u8(reader: &mut dyn io::Read) -> io::Result<u8> {
    let mut buf: [u8; 1] = [0u8];
    reader.read_exact(&mut buf)?;
    Ok(buf[0])
}

const CONTROL_A: u8 = 1u8; // Beginning of line
const CONTROL_B: u8 = 2u8; // Move backward one char
const CONTROL_D: u8 = 4u8; // Delete one char forward
const CONTROL_E: u8 = 5u8; // End of line
const CONTROL_F: u8 = 6u8; // Move forward one char
const BACKSPACE: u8 = 8u8; // Delete previous character at point
const CONTROL_K: u8 = 11u8; // Kill line from cursor forward
const CONTROL_U: u8 = 21u8; // Delete entire command line
const CONTROL_W: u8 = 23u8; // Delete previous word
const DELETE: u8 = 127u8; // Doubles for backspace

cfg_if! {
    if #[cfg(feature = "simple_support")] {
        mod simple;
        pub use simple::LineReader;
    } else {
        mod edit;
        pub use edit::LineReader;
    }
}
