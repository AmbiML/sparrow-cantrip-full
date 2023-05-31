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
use uart_interface::*;

pub struct Rx {
    dataport: &'static [u8],
}
impl Default for Rx {
    fn default() -> Self { Self::new() }
}
impl Rx {
    pub fn new() -> Self {
        extern "Rust" {
            fn get_rx_dataport() -> &'static [u8];
        }
        Self {
            dataport: unsafe { get_rx_dataport() },
        }
    }
}
impl io::Read for Rx {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = uart_read(buf.len()).or(Err(io::Error))?;
        buf[..n].copy_from_slice(&self.dataport[..n]);
        Ok(n)
    }
}

pub struct Tx {
    dataport: &'static mut [u8],
}
impl Default for Tx {
    fn default() -> Self { Self::new() }
}
impl Tx {
    pub fn new() -> Self {
        extern "Rust" {
            fn get_tx_dataport_mut() -> &'static mut [u8];
        }
        Self {
            dataport: unsafe { get_tx_dataport_mut() },
        }
    }
}
impl io::Write for Tx {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.dataport[..buf.len()].copy_from_slice(buf);
        uart_write(buf.len()).or(Err(io::Error))
    }

    fn flush(&mut self) -> io::Result<()> { uart_flush().or(Err(io::Error)) }
}
