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
use reg_constants::timer::*;

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

fn get_u32(idx: usize) -> u32 { unsafe { ptr::read_volatile(csr.add(idx)) } }

fn set_u32(idx: usize, val: u32) {
    unsafe {
        ptr::write_volatile(csr.add(idx), val);
    }
}

fn get_bytes(idx: usize) -> [u8; 4] { get_u32(idx).to_ne_bytes() }

fn set_bytes(idx: usize, bytes: [u8; 4]) { set_u32(idx, u32::from_ne_bytes(bytes)); }

const fn u8_to_u32_offset(offset: usize) -> usize {
    assert!(offset % 4 == 0);
    offset >> 2
}

const CTRL_OFFSET: usize = u8_to_u32_offset(RV_TIMER_CTRL_REG_OFFSET);

pub fn get_ctrl() -> Ctrl { Ctrl::from_bytes(get_bytes(CTRL_OFFSET)) }

pub fn set_ctrl(ctrl: Ctrl) { set_bytes(CTRL_OFFSET, ctrl.into_bytes()); }

const CONFIG_OFFSET: usize = u8_to_u32_offset(RV_TIMER_CFG0_REG_OFFSET);

pub fn get_config() -> Config { Config::from_bytes(get_bytes(CONFIG_OFFSET)) }

pub fn set_config(config: Config) { set_bytes(CONFIG_OFFSET, config.into_bytes()); }

const VALUE_LOW_OFFSET: usize = u8_to_u32_offset(RV_TIMER_TIMER_V_LOWER0_REG_OFFSET);

pub fn get_value_low() -> u32 { get_u32(VALUE_LOW_OFFSET) }

pub fn set_value_low(val: u32) { set_u32(VALUE_LOW_OFFSET, val); }

const VALUE_HIGH_OFFSET: usize = u8_to_u32_offset(RV_TIMER_TIMER_V_UPPER0_REG_OFFSET);

pub fn get_value_high() -> u32 { get_u32(VALUE_HIGH_OFFSET) }

pub fn set_value_high(val: u32) { set_u32(VALUE_HIGH_OFFSET, val); }

const COMPARE_LOW_OFFSET: usize = u8_to_u32_offset(RV_TIMER_COMPARE_LOWER0_0_REG_OFFSET);

pub fn get_compare_low() -> u32 { get_u32(COMPARE_LOW_OFFSET) }

pub fn set_compare_low(val: u32) { set_u32(COMPARE_LOW_OFFSET, val); }

const COMPARE_HIGH_OFFSET: usize = u8_to_u32_offset(RV_TIMER_COMPARE_UPPER0_0_REG_OFFSET);

pub fn get_compare_high() -> u32 { get_u32(COMPARE_HIGH_OFFSET) }

pub fn set_compare_high(val: u32) { set_u32(COMPARE_HIGH_OFFSET, val); }

const INTR_ENABLE_OFFSET: usize = u8_to_u32_offset(RV_TIMER_INTR_ENABLE0_REG_OFFSET);

pub fn get_intr_enable() -> Intr { Intr::from_bytes(get_bytes(INTR_ENABLE_OFFSET)) }

pub fn set_intr_enable(intr_enable: Intr) {
    set_bytes(INTR_ENABLE_OFFSET, intr_enable.into_bytes());
}

const INTR_STATE_OFFSET: usize = u8_to_u32_offset(RV_TIMER_INTR_STATE0_REG_OFFSET);

pub fn get_intr_state() -> Intr { Intr::from_bytes(get_bytes(INTR_STATE_OFFSET)) }

pub fn set_intr_state(intr_state: Intr) { set_bytes(INTR_STATE_OFFSET, intr_state.into_bytes()); }

const INTR_TEST_OFFSET: usize = u8_to_u32_offset(RV_TIMER_INTR_TEST0_REG_OFFSET);

pub fn get_intr_test() -> Intr { Intr::from_bytes(get_bytes(INTR_TEST_OFFSET)) }

pub fn set_intr_test(intr_test: Intr) { set_bytes(INTR_TEST_OFFSET, intr_test.into_bytes()); }
