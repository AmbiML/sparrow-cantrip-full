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

//! Helpers to read/write MMIO registers.

use core::ops::Deref;

extern "C" {
    // UART0 base register.
    static mmio_region: *mut u32;
}

pub struct Field {
    mask: u32,
    offset: u32,
    value: Option<u32>,
}

impl Field {
    pub fn new(mask: u32, offset: u32, val: Option<u32>) -> Self {
        if let Some(value) = val {
            Field {
                mask,
                offset,
                value: Some((value & mask) << offset),
            }
        } else {
            Field {
                mask,
                offset,
                value: None,
            }
        }
    }
}

impl Deref for Field {
    type Target = u32;

    fn deref(&self) -> &Self::Target { self.value.as_ref().unwrap() }
}

pub struct Register(u32);

impl Register {
    pub unsafe fn new(offset: u32) -> Self {
        Register(
            mmio_region
                .cast::<u8>()
                .offset(offset as isize)
                .cast::<()>() as u32,
        )
    }

    pub unsafe fn write(&mut self, value: u32) { (self.0 as *mut u32).write_volatile(value); }

    pub unsafe fn read(&self, field: Field) -> u32 { self.get() >> field.offset & field.mask }

    pub unsafe fn get(&self) -> u32 { (self.0 as *const u32).read_volatile() }
}

pub fn bit(x: u32) -> u32 { 1 << x }
