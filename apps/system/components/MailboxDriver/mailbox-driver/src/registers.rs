use reg_constants::mailbox::*;

const fn u8_to_u32_offset(offset: usize) -> usize {
    assert!(offset % 4 == 0);
    offset >> 2
}

const REG_INTR_STATE: usize = u8_to_u32_offset(TLUL_MAILBOX_INTR_STATE_REG_OFFSET);
const REG_INTR_ENABLE: usize = u8_to_u32_offset(TLUL_MAILBOX_INTR_ENABLE_REG_OFFSET);
const REG_INTR_TEST: usize = u8_to_u32_offset(TLUL_MAILBOX_INTR_TEST_REG_OFFSET);
const REG_MBOXW: usize = u8_to_u32_offset(TLUL_MAILBOX_MBOXW_REG_OFFSET);
const REG_MBOXR: usize = u8_to_u32_offset(TLUL_MAILBOX_MBOXR_REG_OFFSET);
const REG_STATUS: usize = u8_to_u32_offset(TLUL_MAILBOX_STATUS_REG_OFFSET);
const REG_ERROR: usize = u8_to_u32_offset(TLUL_MAILBOX_ERROR_REG_OFFSET);
const REG_WIRQT: usize = u8_to_u32_offset(TLUL_MAILBOX_WIRQT_REG_OFFSET);
const REG_RIRQT: usize = u8_to_u32_offset(TLUL_MAILBOX_RIRQT_REG_OFFSET);
const REG_CTRL: usize = u8_to_u32_offset(TLUL_MAILBOX_CTRL_REG_OFFSET);

pub const INTR_STATE_BIT_WTIRQ: u32 = 0b001;
pub const INTR_STATE_BIT_RTIRQ: u32 = 0b010;
pub const INTR_STATE_BIT_EIRQ: u32 = 0b100;
pub const INTR_STATE_MASK: u32 = 0b111;

pub const INTR_ENABLE_BIT_WTIRQ: u32 = 0b001;
pub const INTR_ENABLE_BIT_RTIRQ: u32 = 0b010;
pub const INTR_ENABLE_BIT_EIRQ: u32 = 0b100;
pub const INTR_ENABLE_MASK: u32 = 0b111;

pub const INTR_TEST_BIT_WTIRQ: u32 = 0b001;
pub const INTR_TEST_BIT_RTIRQ: u32 = 0b010;
pub const INTR_TEST_BIT_EIRQ: u32 = 0b100;
pub const INTR_TEST_MASK: u32 = 0b111;

pub const STATUS_BIT_EMPTY: u32 = 0b0001;
pub const STATUS_BIT_FULL: u32 = 0b0010;
pub const STATUS_BIT_WFIFOL: u32 = 0b0100;
pub const STATUS_BIT_RFIFOL: u32 = 0b1000;
pub const STATUS_MASK: u32 = 0b1111;

pub const ERROR_BIT_READ: u32 = 0b01;
pub const ERROR_BIT_WRITE: u32 = 0b10;
pub const ERROR_MASK: u32 = 0b11;

pub const FIFO_SIZE: u32 = 8;
pub const FIFO_MASK: u32 = FIFO_SIZE - 1;
pub const WIRQT_MASK: u32 = FIFO_MASK;
pub const RIRQT_MASK: u32 = FIFO_MASK;

pub const CTRL_BIT_FLUSH_WFIFO: u32 = 0b01;
pub const CTRL_BIT_FLUSH_RFIFO: u32 = 0b10;
pub const CTRL_MASK: u32 = 0b11;

// The high bit of the message header is used to distinguish between "inline"
// messages that fit in the mailbox and "long" messages that contain the
// physical address of a memory page containing the message.
pub const HEADER_FLAG_LONG_MESSAGE: u32 = 0x80000000;

//------------------------------------------------------------------------------
// Directly manipulate the mailbox registers.

pub unsafe fn get_intr_state(mbox: *const u32) -> u32 { mbox.add(REG_INTR_STATE).read_volatile() }
pub unsafe fn get_INTR_ENABLE(mbox: *const u32) -> u32 { mbox.add(REG_INTR_ENABLE).read_volatile() }
pub unsafe fn get_INTR_TEST(mbox: *const u32) -> u32 { mbox.add(REG_INTR_TEST).read_volatile() }
pub unsafe fn get_MBOXW(mbox: *const u32) -> u32 { mbox.add(REG_MBOXW).read_volatile() }
pub unsafe fn get_MBOXR(mbox: *const u32) -> u32 { mbox.add(REG_MBOXR).read_volatile() }
pub unsafe fn get_STATUS(mbox: *const u32) -> u32 { mbox.add(REG_STATUS).read_volatile() }
pub unsafe fn get_ERROR(mbox: *const u32) -> u32 { mbox.add(REG_ERROR).read_volatile() }
pub unsafe fn get_WIRQT(mbox: *const u32) -> u32 { mbox.add(REG_WIRQT).read_volatile() }
pub unsafe fn get_RIRQT(mbox: *const u32) -> u32 { mbox.add(REG_RIRQT).read_volatile() }
pub unsafe fn get_CTRL(mbox: *const u32) -> u32 { mbox.add(REG_CTRL).read_volatile() }

pub unsafe fn set_INTR_STATE(mbox: *mut u32, x: u32) { mbox.add(REG_INTR_STATE).write_volatile(x); }
pub unsafe fn set_INTR_ENABLE(mbox: *mut u32, x: u32) {
    mbox.add(REG_INTR_ENABLE).write_volatile(x);
}
pub unsafe fn set_INTR_TEST(mbox: *mut u32, x: u32) { mbox.add(REG_INTR_TEST).write_volatile(x); }
pub unsafe fn set_MBOXW(mbox: *mut u32, x: u32) { mbox.add(REG_MBOXW).write_volatile(x); }
pub unsafe fn set_MBOXR(mbox: *mut u32, x: u32) { mbox.add(REG_MBOXR).write_volatile(x); }
pub unsafe fn set_STATUS(mbox: *mut u32, x: u32) { mbox.add(REG_STATUS).write_volatile(x); }
pub unsafe fn set_ERROR(mbox: *mut u32, x: u32) { mbox.add(REG_ERROR).write_volatile(x); }
pub unsafe fn set_WIRQT(mbox: *mut u32, x: u32) { mbox.add(REG_WIRQT).write_volatile(x); }
pub unsafe fn set_RIRQT(mbox: *mut u32, x: u32) { mbox.add(REG_RIRQT).write_volatile(x); }
pub unsafe fn set_CTRL(mbox: *mut u32, x: u32) { mbox.add(REG_CTRL).write_volatile(x); }
