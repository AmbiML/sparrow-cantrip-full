#![no_std]

use core::time::Duration;

pub type Ticks = u64;
pub type TimerId = u32;

/// A hardware timer capable of generating interrupts.
pub trait HardwareTimer {
    fn setup(&self);
    fn ack_interrupt(&self);
    // The current value of the timer.
    fn now(&self) -> Ticks;
    // Return the deadline `duration` in the future, in Ticks.
    fn deadline(&self, duration: Duration) -> Ticks;
    fn set_alarm(&self, deadline: Ticks);
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum TimerServiceError {
    TimerOk = 0,
    NoSuchTimer,
    TimerAlreadyExists,
}
