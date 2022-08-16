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

//! Hardware structs for OpenTitan timers.
// https://docs.opentitan.org/hw/ip/rv_timer/doc/

#![allow(unused)]
use core::ptr;
use modular_bitfield::prelude::*;

// The intent is to update this file with tock-registers instead.
// The tock-registers format is displayed here for layout and future purposes.

// register_structs! {
//     pub TimerRegisters {
//         (0x000 => ctrl: ReadWrite<u32, ctrl::Register>), // XXX: Simulation has this at 0x4.
//         (0x004 => _reserved),
//         (0x100 => config: ReadWrite<u32, config::Register>),
//         (0x104 => value_low: ReadWrite<u32>),
//         (0x108 => value_high: ReadWrite<u32>),
//         (0x10c => compare_low: ReadWrite<u32>),
//         (0x110 => compare_high: ReadWrite<u32>),
//         (0x114 => intr_enable: ReadWrite<u32, intr::Register>),
//         (0x118 => intr_state: ReadWrite<u32, intr::Register>),
//         (0x11c => intr_test: WriteOnly<u32, intr::Register>),
//         (0x120 => @END),
//     }
// }
// register_bitfields![u32,
//     ctrl [
//         enable OFFSET(0) NUMBITS(1) []
//     ],
//     config [
//         prescale OFFSET(0) NUMBITS(12) [],
//         step OFFSET(16) NUMBITS(8) []
//     ],
//     intr [
//         timer0 OFFSET(0) NUMBITS(1) []
//     ]
// ];

#[bitfield]
pub struct Ctrl {
    pub enable: bool,
    #[skip]
    _unused: B31,
}

#[bitfield]
pub struct Config {
    pub prescale: B12,
    #[skip]
    _unused0: B4,
    pub step: B8,
    #[skip]
    _unused1: B8,
}

#[bitfield]
pub struct Intr {
    pub timer0: bool,
    #[skip]
    _unused: B31,
}

extern "C" {
    static csr: *mut u32;
}

fn get_u32(idx: isize) -> u32 { unsafe { ptr::read_volatile(csr.offset(idx)) } }

fn set_u32(idx: isize, val: u32) {
    unsafe {
        ptr::write_volatile(csr.offset(idx), val);
    }
}

fn get_bytes(idx: isize) -> [u8; 4] { get_u32(idx).to_ne_bytes() }

fn set_bytes(idx: isize, bytes: [u8; 4]) { set_u32(idx, u32::from_ne_bytes(bytes)); }

pub fn get_ctrl() -> Ctrl { Ctrl::from_bytes(get_bytes(1)) }

pub fn set_ctrl(ctrl: Ctrl) { set_bytes(1, ctrl.into_bytes()); }

pub fn get_config() -> Config { Config::from_bytes(get_bytes(0x40)) }

pub fn set_config(config: Config) { set_bytes(0x40, config.into_bytes()); }

pub fn get_value_low() -> u32 { get_u32(0x41) }

pub fn set_value_low(val: u32) { set_u32(0x41, val); }

pub fn get_value_high() -> u32 { get_u32(0x42) }

pub fn set_value_high(val: u32) { set_u32(0x42, val); }

pub fn get_compare_low() -> u32 { get_u32(0x43) }

pub fn set_compare_low(val: u32) { set_u32(0x43, val); }

pub fn get_compare_high() -> u32 { get_u32(0x44) }

pub fn set_compare_high(val: u32) { set_u32(0x44, val); }

pub fn get_intr_enable() -> Intr { Intr::from_bytes(get_bytes(0x45)) }

pub fn set_intr_enable(intr_enable: Intr) { set_bytes(0x45, intr_enable.into_bytes()); }

pub fn get_intr_state() -> Intr { Intr::from_bytes(get_bytes(0x46)) }

pub fn set_intr_state(intr_state: Intr) { set_bytes(0x46, intr_state.into_bytes()); }

pub fn get_intr_test() -> Intr { Intr::from_bytes(get_bytes(0x47)) }

pub fn set_intr_test(intr_test: Intr) { set_bytes(0x47, intr_test.into_bytes()); }
