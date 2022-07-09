//! The Timer Service provides multiplexed access to a hardware timer.
#![no_std]
#![allow(clippy::missing_safety_doc)]

use core::time::Duration;
use cantrip_os_common::camkes::Camkes;
use cantrip_os_common::sel4_sys::seL4_Word;
use cantrip_timer_interface::{TimerId, TimerServiceError};
use cantrip_timer_service::TIMER_SRV;

static mut CAMKES: Camkes = Camkes::new("TimerService");

#[no_mangle]
pub unsafe extern "C" fn pre_init() {
    static mut HEAP_MEMORY: [u8; 4 * 1024] = [0; 4 * 1024];
    CAMKES.pre_init(
        log::LevelFilter::Debug,
        &mut HEAP_MEMORY,
    );
}

#[no_mangle]
pub unsafe extern "C" fn timer__init() {
    TIMER_SRV.lock().init();
}

extern "C" {
    fn timer_get_sender_id() -> seL4_Word;
}

#[no_mangle]
pub unsafe extern "C" fn timer_completed_timers() -> u32 {
    let client_id = timer_get_sender_id();
    return TIMER_SRV.lock().completed_timers(client_id);
}

#[no_mangle]
pub unsafe extern "C" fn timer_oneshot(
    timer_id: TimerId,
    duration_ms: u32
) -> TimerServiceError {
    let duration = Duration::from_millis(duration_ms as u64);
    let is_periodic = false;
    let client_id = timer_get_sender_id();
    match TIMER_SRV
        .lock()
        .add(client_id, timer_id, duration, is_periodic)
    {
        Err(e) => e,
        Ok(()) => TimerServiceError::TimerOk,
    }
}

#[no_mangle]
pub unsafe extern "C" fn timer_periodic(
    timer_id: TimerId,
    duration_ms: u32
) -> TimerServiceError {
    let duration = Duration::from_millis(duration_ms as u64);
    let is_periodic = true;
    let client_id = timer_get_sender_id();
    match TIMER_SRV
        .lock()
        .add(client_id, timer_id, duration, is_periodic)
    {
        Err(e) => e,
        Ok(()) => TimerServiceError::TimerOk,
    }
}

#[no_mangle]
pub unsafe extern "C" fn timer_cancel(
    timer_id: TimerId
) -> TimerServiceError {
    let client_id = timer_get_sender_id();
    match TIMER_SRV.lock().cancel(client_id, timer_id) {
        Err(e) => e,
        Ok(()) => TimerServiceError::TimerOk,
    }
}

#[no_mangle]
pub unsafe extern "C" fn timer_interrupt_handle() {
    extern "C" {
        fn timer_interrupt_acknowledge() -> u32;
    }
    TIMER_SRV.lock().service_interrupt();
    assert!(timer_interrupt_acknowledge() == 0);
}

#[no_mangle]
pub unsafe extern "C" fn timer_capscan() {
    let _ = Camkes::capscan();
}
