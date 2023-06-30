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

// Hardware structs for OpenTitan timers.
// https://docs.opentitan.org/hw/ip/rv_timer/doc/

use modular_bitfield::prelude::*;
use reg_constants::timer::*;

unsafe fn get_timer(offset: usize) -> *const u32 {
    extern "Rust" {
        fn get_csr() -> &'static [u8];
    }
    get_csr().as_ptr().add(offset).cast::<u32>()
}
unsafe fn get_timer_mut(offset: usize) -> *mut u32 {
    extern "Rust" {
        fn get_csr_mut() -> &'static mut [u8];
    }
    get_csr_mut().as_mut_ptr().add(offset).cast::<u32>()
}

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

// Control register
#[bitfield]
pub struct Ctrl {
    pub active: bool,
    #[skip]
    __: B31,
}
pub fn get_ctrl() -> Ctrl {
    unsafe {
        Ctrl::from_bytes(
            get_timer(RV_TIMER_CTRL_REG_OFFSET)
                .read_volatile()
                .to_ne_bytes(),
        )
    }
}
pub fn set_ctrl(ctrl: Ctrl) {
    unsafe {
        get_timer_mut(RV_TIMER_CTRL_REG_OFFSET)
            .write_volatile(u32::from_ne_bytes(ctrl.into_bytes()))
    }
}

// Interrupt Enable
#[bitfield]
pub struct IntrEnable {
    pub timer0: bool,
    #[skip]
    __: B31,
}
pub fn get_intr_enable() -> IntrEnable {
    unsafe {
        IntrEnable::from_bytes(
            get_timer(RV_TIMER_INTR_ENABLE0_REG_OFFSET)
                .read_volatile()
                .to_ne_bytes(),
        )
    }
}
pub fn set_intr_enable(intr_enable: IntrEnable) {
    unsafe {
        get_timer_mut(RV_TIMER_INTR_ENABLE0_REG_OFFSET)
            .write_volatile(u32::from_ne_bytes(intr_enable.into_bytes()))
    }
}

// Interrupt Status
#[bitfield]
pub struct IntrStatus {
    pub timer0: bool,
    #[skip]
    __: B31,
}
pub fn get_intr_status() -> IntrStatus {
    unsafe {
        IntrStatus::from_bytes(
            get_timer(RV_TIMER_INTR_STATE0_REG_OFFSET)
                .read_volatile()
                .to_ne_bytes(),
        )
    }
}
pub fn set_intr_status(intr_status: IntrStatus) {
    unsafe {
        get_timer_mut(RV_TIMER_INTR_STATE0_REG_OFFSET)
            .write_volatile(u32::from_ne_bytes(intr_status.into_bytes()))
    }
}

// Interrupt test register
#[bitfield]
pub struct IntrTest {
    pub timer0: bool,
    #[skip]
    __: B31,
}
pub fn get_intr_test() -> IntrTest {
    unsafe {
        IntrTest::from_bytes(
            get_timer(RV_TIMER_INTR_TEST0_REG_OFFSET)
                .read_volatile()
                .to_ne_bytes(),
        )
    }
}
pub fn set_intr_test(intr_test: IntrTest) {
    unsafe {
        get_timer_mut(RV_TIMER_INTR_TEST0_REG_OFFSET)
            .write_volatile(u32::from_ne_bytes(intr_test.into_bytes()))
    }
}

// Configuration for Hart 0
#[bitfield]
pub struct Config {
    pub prescale: B12,
    #[skip]
    __: B4,
    pub step: u8,
    #[skip]
    __: B8,
}
pub fn get_config() -> Config {
    unsafe {
        Config::from_bytes(
            get_timer(RV_TIMER_CFG0_REG_OFFSET)
                .read_volatile()
                .to_ne_bytes(),
        )
    }
}
pub fn set_config(config: Config) {
    unsafe {
        get_timer_mut(RV_TIMER_CFG0_REG_OFFSET)
            .write_volatile(u32::from_ne_bytes(config.into_bytes()))
    }
}

// Timer value Lower
pub fn get_value_low() -> u32 {
    unsafe { get_timer(RV_TIMER_TIMER_V_LOWER0_REG_OFFSET).read_volatile() }
}
pub fn set_value_low(value: u32) {
    unsafe { get_timer_mut(RV_TIMER_TIMER_V_LOWER0_REG_OFFSET).write_volatile(value) }
}

// Timer value Upper
pub fn get_value_high() -> u32 {
    unsafe { get_timer(RV_TIMER_TIMER_V_UPPER0_REG_OFFSET).read_volatile() }
}
pub fn set_value_high(value: u32) {
    unsafe { get_timer_mut(RV_TIMER_TIMER_V_UPPER0_REG_OFFSET).write_volatile(value) }
}

// Timer compare value Lower
pub fn get_compare_low() -> u32 {
    unsafe { get_timer(RV_TIMER_COMPARE_LOWER0_0_REG_OFFSET).read_volatile() }
}
pub fn set_compare_low(value: u32) {
    unsafe { get_timer_mut(RV_TIMER_COMPARE_LOWER0_0_REG_OFFSET).write_volatile(value) }
}

// Timer compare value Upper
pub fn get_compare_high() -> u32 {
    unsafe { get_timer(RV_TIMER_COMPARE_UPPER0_0_REG_OFFSET).read_volatile() }
}
pub fn set_compare_high(value: u32) {
    unsafe { get_timer_mut(RV_TIMER_COMPARE_UPPER0_0_REG_OFFSET).write_volatile(value) }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Validate modular_bitfield defs against regotool-generated SOT.

    fn bit(x: u32) -> u32 { 1 << x }
    fn field(v: u32, mask: u32, shift: usize) -> u32 { (v & mask) << shift }

    #[test]
    fn ctrl() {
        assert_eq!(
            u32::from_ne_bytes(Ctrl::new().with_active(true).into_bytes()),
            bit(RV_TIMER_CTRL_ACTIVE_0_BIT)
        );
    }
    #[test]
    fn intr_status() {
        assert_eq!(
            u32::from_ne_bytes(IntrStatus::new().with_timer0(true).into_bytes()),
            bit(RV_TIMER_INTR_ENABLE0_IE_0_BIT)
        );
    }
    #[test]
    fn intr_enable() {
        assert_eq!(
            u32::from_ne_bytes(IntrEnable::new().with_timer0(true).into_bytes()),
            bit(RV_TIMER_INTR_ENABLE0_IE_0_BIT)
        );
    }
    #[test]
    fn intr_test() {
        assert_eq!(
            u32::from_ne_bytes(IntrTest::new().with_timer0(true).into_bytes()),
            bit(RV_TIMER_INTR_TEST0_T_0_BIT)
        );
    }
    #[test]
    fn config() {
        assert_eq!(RV_TIMER_CFG0_PRESCALE_MASK, (1 << 12) - 1);
        for prescale in 1..RV_TIMER_CFG0_PRESCALE_MASK {
            assert_eq!(
                u32::from_ne_bytes(Config::new().with_prescale(prescale as u16).into_bytes()),
                field(prescale, RV_TIMER_CFG0_PRESCALE_MASK, RV_TIMER_CFG0_PRESCALE_OFFSET)
            );
        }
        assert_eq!(RV_TIMER_CFG0_STEP_MASK, u8::MAX as u32);
        for step in 1..RV_TIMER_CFG0_STEP_MASK {
            assert_eq!(
                u32::from_ne_bytes(Config::new().with_step(step as u8).into_bytes()),
                field(step, RV_TIMER_CFG0_STEP_MASK, RV_TIMER_CFG0_STEP_OFFSET)
            );
        }
    }
}
