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

const DATAPORT_SIZE: usize = 4096;

pub struct Rx {
    dataport: &'static [u8],
}
impl Default for Rx {
    fn default() -> Self { Self::new() }
}

impl Rx {
    pub fn new() -> Rx {
        extern "C" {
            static rx_dataport: *mut cty::c_uchar;
        }
        Rx {
            dataport: unsafe { core::slice::from_raw_parts(rx_dataport, DATAPORT_SIZE) },
        }
    }
}

impl io::Read for Rx {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        extern "C" {
            fn uart_read_read(limit: cty::size_t) -> cty::c_int;
        }
        let n = unsafe { uart_read_read(buf.len()) };
        if n >= 0 {
            let s = n as usize;
            buf[..s].copy_from_slice(&self.dataport[..s]);
            Ok(s)
        } else {
            Err(io::Error)
        }
    }
}

pub struct Tx {
    dataport: &'static mut [u8],
}
impl Default for Tx {
    fn default() -> Self { Self::new() }
}

impl Tx {
    pub fn new() -> Tx {
        extern "C" {
            static tx_dataport: *mut cty::c_uchar;
        }
        Tx {
            dataport: unsafe { core::slice::from_raw_parts_mut(tx_dataport, DATAPORT_SIZE) },
        }
    }
}

impl io::Write for Tx {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        extern "C" {
            fn uart_write_write(available: cty::size_t) -> cty::c_int;
        }
        self.dataport[..buf.len()].copy_from_slice(buf);
        let n = unsafe { uart_write_write(buf.len()) };
        if n >= 0 {
            Ok(n as usize)
        } else {
            Err(io::Error)
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        extern "C" {
            fn uart_write_flush() -> cty::c_int;
        }
        if unsafe { uart_write_flush() } == 0 {
            Ok(())
        } else {
            Err(io::Error)
        }
    }
}
