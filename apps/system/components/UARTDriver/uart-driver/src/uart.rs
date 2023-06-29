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

// Helpers to read/write MMIO registers.

use modular_bitfield::prelude::*;
use reg_constants::uart::*;

unsafe fn get_uart(offset: usize) -> *const u32 {
    crate::MMIO_REGION.data.as_ptr().add(offset).cast::<u32>()
}
unsafe fn get_uart_mut(offset: usize) -> *mut u32 {
    crate::MMIO_REGION
        .data
        .as_mut_ptr()
        .add(offset)
        .cast::<u32>()
}

// Interrupt State register.
#[bitfield]
pub struct IntrState {
    pub tx_watermark: bool,
    pub rx_watermark: bool,
    pub tx_empty: bool,
    pub rx_overflow: bool,
    pub rx_frame_err: bool,
    pub rx_break_err: bool,
    pub rx_timeout: bool,
    pub rx_parity_err: bool,
    #[skip]
    __: B24,
}
pub fn get_intr_state() -> IntrState {
    unsafe {
        IntrState::from_bytes(
            get_uart(UART_INTR_STATE_REG_OFFSET)
                .read_volatile()
                .to_ne_bytes(),
        )
    }
}
pub fn set_intr_state(state: IntrState) {
    unsafe {
        get_uart_mut(UART_INTR_STATE_REG_OFFSET)
            .write_volatile(u32::from_ne_bytes(state.into_bytes()))
    }
}

// Interrupt Enable register.
#[bitfield]
pub struct IntrEnable {
    pub tx_watermark: bool,
    pub rx_watermark: bool,
    pub tx_empty: bool,
    pub rx_overflow: bool,
    pub rx_frame_err: bool,
    pub rx_break_err: bool,
    pub rx_timeout: bool,
    pub rx_parity_err: bool,
    #[skip]
    __: B24,
}
pub fn get_intr_enable() -> IntrEnable {
    unsafe {
        IntrEnable::from_bytes(
            get_uart(UART_INTR_ENABLE_REG_OFFSET)
                .read_volatile()
                .to_ne_bytes(),
        )
    }
}
pub fn set_intr_enable(enable: IntrEnable) {
    unsafe {
        get_uart_mut(UART_INTR_ENABLE_REG_OFFSET)
            .write_volatile(u32::from_ne_bytes(enable.into_bytes()))
    }
}

// Interrupt Test register.
#[bitfield]
pub struct IntrTest {
    pub tx_watermark: bool,
    pub rx_watermark: bool,
    pub tx_empty: bool,
    pub rx_overflow: bool,
    pub rx_frame_err: bool,
    pub rx_break_err: bool,
    pub rx_timeout: bool,
    pub rx_parity_err: bool,
    #[skip]
    __: B24,
}
pub fn get_intr_test() -> IntrTest {
    unsafe {
        IntrTest::from_bytes(
            get_uart(UART_INTR_TEST_REG_OFFSET)
                .read_volatile()
                .to_ne_bytes(),
        )
    }
}
pub fn set_intr_test(test: IntrTest) {
    unsafe {
        get_uart_mut(UART_INTR_TEST_REG_OFFSET)
            .write_volatile(u32::from_ne_bytes(test.into_bytes()))
    }
}

// Alert Test register (unused)

// UART control register.
#[repr(u32)]
#[derive(BitfieldSpecifier)]
pub enum RxBLvl {
    Break2 = UART_CTRL_RXBLVL_VALUE_BREAK2,
    Break4 = UART_CTRL_RXBLVL_VALUE_BREAK4,
    Break8 = UART_CTRL_RXBLVL_VALUE_BREAK8,
    Break16 = UART_CTRL_RXBLVL_VALUE_BREAK16,
}
#[bitfield]
pub struct Ctrl {
    pub tx: bool,
    pub rx: bool,
    pub nf: bool,
    #[skip]
    __: B1,
    pub slpbk: bool,
    pub llpbk: bool,
    pub parity_en: bool,
    pub parity_odd: bool,
    #[bits = 2]
    pub rxblvl: RxBLvl,
    #[skip]
    __: B6,
    pub nco: B16,
}
pub fn get_ctrl() -> Ctrl {
    unsafe { Ctrl::from_bytes(get_uart(UART_CTRL_REG_OFFSET).read_volatile().to_ne_bytes()) }
}
pub fn set_ctrl(ctrl: Ctrl) {
    unsafe {
        get_uart_mut(UART_CTRL_REG_OFFSET).write_volatile(u32::from_ne_bytes(ctrl.into_bytes()))
    }
}

// UART live status register (RO).
#[bitfield]
pub struct Status {
    pub txfull: bool,
    pub rxfull: bool,
    pub txempty: bool,
    pub txidle: bool,
    pub rxidle: bool,
    pub rxempty: bool,
    #[skip]
    __: B26,
}
pub fn get_status() -> Status {
    unsafe {
        Status::from_bytes(
            get_uart(UART_STATUS_REG_OFFSET)
                .read_volatile()
                .to_ne_bytes(),
        )
    }
}

// UART read data (RO).
#[bitfield]
pub struct RData {
    pub rdata: u8,
    #[skip]
    __: B24,
}
pub fn get_rdata() -> u8 {
    unsafe {
        RData::from_bytes(
            get_uart(UART_RDATA_REG_OFFSET)
                .read_volatile()
                .to_ne_bytes(),
        )
        .rdata()
    }
}

// UART write data (WO).
#[bitfield]
pub struct WData {
    pub wdata: u8,
    #[skip]
    __: B24,
}
pub fn set_wdata(wdata: u8) {
    unsafe {
        get_uart_mut(UART_WDATA_REG_OFFSET)
            .write_volatile(u32::from_ne_bytes(WData::new().with_wdata(wdata).into_bytes()))
    }
}

// UART FIFO control register.
#[repr(u32)]
#[derive(BitfieldSpecifier)]
#[bits = 3]
pub enum RxILvl {
    Level1 = UART_FIFO_CTRL_RXILVL_VALUE_RXLVL1,
    Level4 = UART_FIFO_CTRL_RXILVL_VALUE_RXLVL4,
    Level8 = UART_FIFO_CTRL_RXILVL_VALUE_RXLVL8,
    Level16 = UART_FIFO_CTRL_RXILVL_VALUE_RXLVL16,
    Level30 = UART_FIFO_CTRL_RXILVL_VALUE_RXLVL30,
}
#[repr(u32)]
#[derive(BitfieldSpecifier)]
pub enum TxILvl {
    Level1 = UART_FIFO_CTRL_TXILVL_VALUE_TXLVL1,
    Level4 = UART_FIFO_CTRL_TXILVL_VALUE_TXLVL4,
    Level8 = UART_FIFO_CTRL_TXILVL_VALUE_TXLVL8,
    Level16 = UART_FIFO_CTRL_TXILVL_VALUE_TXLVL16,
}
#[bitfield]
pub struct FifoCtrl {
    pub rxrst: bool,
    pub txrst: bool,
    #[bits = 3]
    pub rxilvl: RxILvl,
    #[bits = 2]
    pub txilvl: TxILvl,
    #[skip]
    __: B25,
}
pub fn get_fifo_ctrl() -> FifoCtrl {
    unsafe {
        FifoCtrl::from_bytes(
            get_uart(UART_FIFO_CTRL_REG_OFFSET)
                .read_volatile()
                .to_ne_bytes(),
        )
    }
}
pub fn set_fifo_ctrl(ctrl: FifoCtrl) {
    unsafe {
        get_uart_mut(UART_FIFO_CTRL_REG_OFFSET)
            .write_volatile(u32::from_ne_bytes(ctrl.into_bytes()))
    }
}

// UART FIFO status register (RO).
#[bitfield]
pub struct FifoStatus {
    pub txlvl: B6,
    #[skip]
    __: B10,
    pub rxlvl: B6,
    #[skip]
    __: B10,
}
pub fn get_fifo_status() -> FifoStatus {
    unsafe {
        FifoStatus::from_bytes(
            get_uart(UART_FIFO_STATUS_REG_OFFSET)
                .read_volatile()
                .to_ne_bytes(),
        )
    }
}

// TX pin override control (unused)
// UART oversmapled values (unused)

// UART RX timeout control.
#[bitfield]
pub struct TimeoutCtrl {
    pub val: B24,
    #[skip]
    __: B7,
    pub en: bool,
}
pub fn get_timeout_ctrl() -> TimeoutCtrl {
    unsafe {
        TimeoutCtrl::from_bytes(
            get_uart(UART_TIMEOUT_CTRL_REG_OFFSET)
                .read_volatile()
                .to_ne_bytes(),
        )
    }
}
pub fn set_timeout_ctrl(timeout_ctrl: TimeoutCtrl) {
    unsafe {
        get_uart_mut(UART_TIMEOUT_CTRL_REG_OFFSET)
            .write_volatile(u32::from_ne_bytes(timeout_ctrl.into_bytes()))
    }
}
