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

#[cfg(test)]
mod tests {
    use super::*;

    // Validate modular_bitfield defs against regotool-generated SOT.

    fn bit(x: u32) -> u32 { 1 << x }
    fn field(v: u32, mask: u32, shift: usize) -> u32 { (v & mask) << shift }

    #[test]
    fn intr_state() {
        assert_eq!(
            u32::from_ne_bytes(IntrState::new().with_tx_watermark(true).into_bytes()),
            bit(UART_INTR_STATE_TX_WATERMARK_BIT)
        );
        assert_eq!(
            u32::from_ne_bytes(IntrState::new().with_rx_watermark(true).into_bytes()),
            bit(UART_INTR_STATE_RX_WATERMARK_BIT)
        );
        assert_eq!(
            u32::from_ne_bytes(IntrState::new().with_tx_empty(true).into_bytes()),
            bit(UART_INTR_STATE_TX_EMPTY_BIT)
        );
        assert_eq!(
            u32::from_ne_bytes(IntrState::new().with_rx_overflow(true).into_bytes()),
            bit(UART_INTR_STATE_RX_OVERFLOW_BIT)
        );
        assert_eq!(
            u32::from_ne_bytes(IntrState::new().with_rx_frame_err(true).into_bytes()),
            bit(UART_INTR_STATE_RX_FRAME_ERR_BIT)
        );
        assert_eq!(
            u32::from_ne_bytes(IntrState::new().with_rx_break_err(true).into_bytes()),
            bit(UART_INTR_STATE_RX_BREAK_ERR_BIT)
        );
        assert_eq!(
            u32::from_ne_bytes(IntrState::new().with_rx_timeout(true).into_bytes()),
            bit(UART_INTR_STATE_RX_TIMEOUT_BIT)
        );
        assert_eq!(
            u32::from_ne_bytes(IntrState::new().with_rx_parity_err(true).into_bytes()),
            bit(UART_INTR_STATE_RX_PARITY_ERR_BIT)
        );
    }
    #[test]
    fn intr_enable() {
        assert_eq!(
            u32::from_ne_bytes(IntrEnable::new().with_tx_watermark(true).into_bytes()),
            bit(UART_INTR_ENABLE_TX_WATERMARK_BIT)
        );
        assert_eq!(
            u32::from_ne_bytes(IntrEnable::new().with_rx_watermark(true).into_bytes()),
            bit(UART_INTR_ENABLE_RX_WATERMARK_BIT)
        );
        assert_eq!(
            u32::from_ne_bytes(IntrEnable::new().with_tx_empty(true).into_bytes()),
            bit(UART_INTR_ENABLE_TX_EMPTY_BIT)
        );
        assert_eq!(
            u32::from_ne_bytes(IntrEnable::new().with_rx_overflow(true).into_bytes()),
            bit(UART_INTR_ENABLE_RX_OVERFLOW_BIT)
        );
        assert_eq!(
            u32::from_ne_bytes(IntrEnable::new().with_rx_frame_err(true).into_bytes()),
            bit(UART_INTR_ENABLE_RX_FRAME_ERR_BIT)
        );
        assert_eq!(
            u32::from_ne_bytes(IntrEnable::new().with_rx_break_err(true).into_bytes()),
            bit(UART_INTR_ENABLE_RX_BREAK_ERR_BIT)
        );
        assert_eq!(
            u32::from_ne_bytes(IntrEnable::new().with_rx_timeout(true).into_bytes()),
            bit(UART_INTR_ENABLE_RX_TIMEOUT_BIT)
        );
        assert_eq!(
            u32::from_ne_bytes(IntrEnable::new().with_rx_parity_err(true).into_bytes()),
            bit(UART_INTR_ENABLE_RX_PARITY_ERR_BIT)
        );
    }
    #[test]
    fn intr_test() {
        assert_eq!(
            u32::from_ne_bytes(IntrTest::new().with_tx_watermark(true).into_bytes()),
            bit(UART_INTR_TEST_TX_WATERMARK_BIT)
        );
        assert_eq!(
            u32::from_ne_bytes(IntrTest::new().with_rx_watermark(true).into_bytes()),
            bit(UART_INTR_TEST_RX_WATERMARK_BIT)
        );
        assert_eq!(
            u32::from_ne_bytes(IntrTest::new().with_tx_empty(true).into_bytes()),
            bit(UART_INTR_TEST_TX_EMPTY_BIT)
        );
        assert_eq!(
            u32::from_ne_bytes(IntrTest::new().with_rx_overflow(true).into_bytes()),
            bit(UART_INTR_TEST_RX_OVERFLOW_BIT)
        );
        assert_eq!(
            u32::from_ne_bytes(IntrTest::new().with_rx_frame_err(true).into_bytes()),
            bit(UART_INTR_TEST_RX_FRAME_ERR_BIT)
        );
        assert_eq!(
            u32::from_ne_bytes(IntrTest::new().with_rx_break_err(true).into_bytes()),
            bit(UART_INTR_TEST_RX_BREAK_ERR_BIT)
        );
        assert_eq!(
            u32::from_ne_bytes(IntrTest::new().with_rx_timeout(true).into_bytes()),
            bit(UART_INTR_TEST_RX_TIMEOUT_BIT)
        );
        assert_eq!(
            u32::from_ne_bytes(IntrTest::new().with_rx_parity_err(true).into_bytes()),
            bit(UART_INTR_TEST_RX_PARITY_ERR_BIT)
        );
    }
    #[test]
    fn ctrl() {
        assert_eq!(
            u32::from_ne_bytes(Ctrl::new().with_tx(true).into_bytes()),
            bit(UART_CTRL_TX_BIT)
        );
        assert_eq!(
            u32::from_ne_bytes(Ctrl::new().with_rx(true).into_bytes()),
            bit(UART_CTRL_RX_BIT)
        );
        assert_eq!(
            u32::from_ne_bytes(Ctrl::new().with_nf(true).into_bytes()),
            bit(UART_CTRL_NF_BIT)
        );
        assert_eq!(
            u32::from_ne_bytes(Ctrl::new().with_slpbk(true).into_bytes()),
            bit(UART_CTRL_SLPBK_BIT)
        );
        assert_eq!(
            u32::from_ne_bytes(Ctrl::new().with_llpbk(true).into_bytes()),
            bit(UART_CTRL_LLPBK_BIT)
        );
        assert_eq!(
            u32::from_ne_bytes(Ctrl::new().with_parity_en(true).into_bytes()),
            bit(UART_CTRL_PARITY_EN_BIT)
        );
        assert_eq!(
            u32::from_ne_bytes(Ctrl::new().with_parity_odd(true).into_bytes()),
            bit(UART_CTRL_PARITY_ODD_BIT)
        );

        assert_eq!(
            u32::from_ne_bytes(Ctrl::new().with_rxblvl(RxBLvl::Break2).into_bytes()),
            field(
                UART_CTRL_RXBLVL_VALUE_BREAK2,
                UART_CTRL_RXBLVL_MASK,
                UART_CTRL_RXBLVL_OFFSET
            )
        );
        assert_eq!(
            u32::from_ne_bytes(Ctrl::new().with_rxblvl(RxBLvl::Break4).into_bytes()),
            field(
                UART_CTRL_RXBLVL_VALUE_BREAK4,
                UART_CTRL_RXBLVL_MASK,
                UART_CTRL_RXBLVL_OFFSET
            )
        );
        assert_eq!(
            u32::from_ne_bytes(Ctrl::new().with_rxblvl(RxBLvl::Break8).into_bytes()),
            field(
                UART_CTRL_RXBLVL_VALUE_BREAK8,
                UART_CTRL_RXBLVL_MASK,
                UART_CTRL_RXBLVL_OFFSET
            )
        );
        assert_eq!(
            u32::from_ne_bytes(Ctrl::new().with_rxblvl(RxBLvl::Break16).into_bytes()),
            field(
                UART_CTRL_RXBLVL_VALUE_BREAK16,
                UART_CTRL_RXBLVL_MASK,
                UART_CTRL_RXBLVL_OFFSET
            )
        );

        assert_eq!(UART_CTRL_NCO_MASK, u16::MAX as u32); // Verify field width
        for nco in 1..UART_CTRL_NCO_MASK {
            assert_eq!(
                u32::from_ne_bytes(Ctrl::new().with_nco(nco as u16).into_bytes()),
                field(nco, UART_CTRL_NCO_MASK, UART_CTRL_NCO_OFFSET)
            );
        }
    }
    #[test]
    fn status() {
        assert_eq!(
            u32::from_ne_bytes(Status::new().with_txfull(true).into_bytes()),
            bit(UART_STATUS_TXFULL_BIT)
        );
        assert_eq!(
            u32::from_ne_bytes(Status::new().with_rxfull(true).into_bytes()),
            bit(UART_STATUS_RXFULL_BIT)
        );
        assert_eq!(
            u32::from_ne_bytes(Status::new().with_txempty(true).into_bytes()),
            bit(UART_STATUS_TXEMPTY_BIT)
        );
        assert_eq!(
            u32::from_ne_bytes(Status::new().with_txidle(true).into_bytes()),
            bit(UART_STATUS_TXIDLE_BIT)
        );
        assert_eq!(
            u32::from_ne_bytes(Status::new().with_rxidle(true).into_bytes()),
            bit(UART_STATUS_RXIDLE_BIT)
        );
        assert_eq!(
            u32::from_ne_bytes(Status::new().with_rxempty(true).into_bytes()),
            bit(UART_STATUS_RXEMPTY_BIT)
        );
    }
    #[test]
    fn rdata() {
        assert_eq!(UART_RDATA_RDATA_MASK, u8::MAX as u32); // Verify field width
        for rdata in 1..UART_RDATA_RDATA_MASK {
            assert_eq!(
                u32::from_ne_bytes(RData::new().with_rdata(rdata as u8).into_bytes()),
                field(rdata, UART_RDATA_RDATA_MASK, UART_RDATA_RDATA_OFFSET)
            );
        }
    }
    #[test]
    fn wdata() {
        assert_eq!(UART_WDATA_WDATA_MASK, u8::MAX as u32); // Verify field width
        for wdata in 1..UART_WDATA_WDATA_MASK {
            assert_eq!(
                u32::from_ne_bytes(WData::new().with_wdata(wdata as u8).into_bytes()),
                field(wdata, UART_WDATA_WDATA_MASK, UART_WDATA_WDATA_OFFSET)
            );
        }
    }
    #[test]
    fn fifo_ctrl() {
        assert_eq!(
            u32::from_ne_bytes(FifoCtrl::new().with_rxrst(true).into_bytes()),
            bit(UART_FIFO_CTRL_RXRST_BIT)
        );
        assert_eq!(
            u32::from_ne_bytes(FifoCtrl::new().with_txrst(true).into_bytes()),
            bit(UART_FIFO_CTRL_TXRST_BIT)
        );

        assert_eq!(
            u32::from_ne_bytes(FifoCtrl::new().with_rxilvl(RxILvl::Level1).into_bytes()),
            field(
                UART_FIFO_CTRL_RXILVL_VALUE_RXLVL1,
                UART_FIFO_CTRL_RXILVL_MASK,
                UART_FIFO_CTRL_RXILVL_OFFSET
            )
        );
        assert_eq!(
            u32::from_ne_bytes(FifoCtrl::new().with_rxilvl(RxILvl::Level4).into_bytes()),
            field(
                UART_FIFO_CTRL_RXILVL_VALUE_RXLVL4,
                UART_FIFO_CTRL_RXILVL_MASK,
                UART_FIFO_CTRL_RXILVL_OFFSET
            )
        );
        assert_eq!(
            u32::from_ne_bytes(FifoCtrl::new().with_rxilvl(RxILvl::Level8).into_bytes()),
            field(
                UART_FIFO_CTRL_RXILVL_VALUE_RXLVL8,
                UART_FIFO_CTRL_RXILVL_MASK,
                UART_FIFO_CTRL_RXILVL_OFFSET
            )
        );
        assert_eq!(
            u32::from_ne_bytes(FifoCtrl::new().with_rxilvl(RxILvl::Level16).into_bytes()),
            field(
                UART_FIFO_CTRL_RXILVL_VALUE_RXLVL16,
                UART_FIFO_CTRL_RXILVL_MASK,
                UART_FIFO_CTRL_RXILVL_OFFSET
            )
        );
        assert_eq!(
            u32::from_ne_bytes(FifoCtrl::new().with_rxilvl(RxILvl::Level30).into_bytes()),
            field(
                UART_FIFO_CTRL_RXILVL_VALUE_RXLVL30,
                UART_FIFO_CTRL_RXILVL_MASK,
                UART_FIFO_CTRL_RXILVL_OFFSET
            )
        );

        assert_eq!(
            u32::from_ne_bytes(FifoCtrl::new().with_txilvl(TxILvl::Level1).into_bytes()),
            field(
                UART_FIFO_CTRL_TXILVL_VALUE_TXLVL1,
                UART_FIFO_CTRL_TXILVL_MASK,
                UART_FIFO_CTRL_TXILVL_OFFSET
            )
        );
        assert_eq!(
            u32::from_ne_bytes(FifoCtrl::new().with_txilvl(TxILvl::Level4).into_bytes()),
            field(
                UART_FIFO_CTRL_TXILVL_VALUE_TXLVL4,
                UART_FIFO_CTRL_TXILVL_MASK,
                UART_FIFO_CTRL_TXILVL_OFFSET
            )
        );
        assert_eq!(
            u32::from_ne_bytes(FifoCtrl::new().with_txilvl(TxILvl::Level8).into_bytes()),
            field(
                UART_FIFO_CTRL_TXILVL_VALUE_TXLVL8,
                UART_FIFO_CTRL_TXILVL_MASK,
                UART_FIFO_CTRL_TXILVL_OFFSET
            )
        );
        assert_eq!(
            u32::from_ne_bytes(FifoCtrl::new().with_txilvl(TxILvl::Level16).into_bytes()),
            field(
                UART_FIFO_CTRL_TXILVL_VALUE_TXLVL16,
                UART_FIFO_CTRL_TXILVL_MASK,
                UART_FIFO_CTRL_TXILVL_OFFSET
            )
        );
    }
    #[test]
    fn fifo_status() {
        assert_eq!(UART_FIFO_STATUS_TXLVL_MASK, (1 << 6) - 1); // Verify field width
        for txlvl in 1..UART_FIFO_STATUS_TXLVL_MASK {
            assert_eq!(
                u32::from_ne_bytes(FifoStatus::new().with_txlvl(txlvl as u8).into_bytes()),
                field(txlvl, UART_FIFO_STATUS_TXLVL_MASK, UART_FIFO_STATUS_TXLVL_OFFSET)
            );
        }

        assert_eq!(UART_FIFO_STATUS_RXLVL_MASK, (1 << 6) - 1); // Verify field width
        for rxlvl in 1..UART_FIFO_STATUS_RXLVL_MASK {
            assert_eq!(
                u32::from_ne_bytes(FifoStatus::new().with_rxlvl(rxlvl as u8).into_bytes()),
                field(rxlvl, UART_FIFO_STATUS_RXLVL_MASK, UART_FIFO_STATUS_RXLVL_OFFSET)
            );
        }
    }
    #[test]
    fn timeout_ctrl() {
        assert_eq!(UART_TIMEOUT_CTRL_VAL_MASK, (1 << 24) - 1); // Verify field width

        // NB: checking all 24-bit values takes too long; reduce the range
        //   since this register isn't used
        for val in 1..(UART_TIMEOUT_CTRL_VAL_MASK >> 8) {
            assert_eq!(
                u32::from_ne_bytes(TimeoutCtrl::new().with_val(val as u32).into_bytes()),
                field(val, UART_TIMEOUT_CTRL_VAL_MASK, UART_TIMEOUT_CTRL_VAL_OFFSET)
            );
        }
        assert_eq!(
            u32::from_ne_bytes(TimeoutCtrl::new().with_en(true).into_bytes()),
            bit(UART_TIMEOUT_CTRL_EN_BIT)
        );
    }
}
