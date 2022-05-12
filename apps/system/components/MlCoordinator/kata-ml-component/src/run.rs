#![no_std]
#![allow(clippy::missing_safety_doc)]

extern crate alloc;

use alloc::string::String;
use cstr_core::CStr;
use cantrip_ml_coordinator::MLCoordinator;
use cantrip_ml_coordinator::ModelIdx;
use cantrip_ml_interface::MlCoordError;
use cantrip_os_common::allocator;
use cantrip_os_common::logger::CantripLogger;
use cantrip_os_common::sel4_sys;
use cantrip_os_common::slot_allocator::CANTRIP_CSPACE_SLOTS;
use cantrip_timer_interface::*;
use log::{error, trace};
use sel4_sys::seL4_CPtr;
use spin::Mutex;

static mut ML_COORD: Mutex<MLCoordinator> = Mutex::new(MLCoordinator::new());

extern "C" {
    static SELF_CNODE_FIRST_SLOT: seL4_CPtr;
    static SELF_CNODE_LAST_SLOT: seL4_CPtr;
}

#[no_mangle]
pub unsafe extern "C" fn pre_init() {
    static CANTRIP_LOGGER: CantripLogger = CantripLogger;
    log::set_logger(&CANTRIP_LOGGER).unwrap();
    log::set_max_level(log::LevelFilter::Trace);

    // TODO(sleffler): temp until we integrate with seL4
    static mut HEAP_MEMORY: [u8; 4 * 1024] = [0; 4 * 1024];
    allocator::ALLOCATOR.init(HEAP_MEMORY.as_mut_ptr() as usize, HEAP_MEMORY.len());
    trace!(
        "setup heap: start_addr {:p} size {}",
        HEAP_MEMORY.as_ptr(),
        HEAP_MEMORY.len()
    );

    CANTRIP_CSPACE_SLOTS.init(
        /*first_slot=*/ SELF_CNODE_FIRST_SLOT,
        /*size=*/ SELF_CNODE_LAST_SLOT - SELF_CNODE_FIRST_SLOT,
    );
    trace!(
        "setup cspace slots: first slot {} free {}",
        CANTRIP_CSPACE_SLOTS.base_slot(),
        CANTRIP_CSPACE_SLOTS.free_slots()
    );
}

#[no_mangle]
pub unsafe extern "C" fn mlcoord__init() {
    ML_COORD.lock().init();
}

#[no_mangle]
pub extern "C" fn run() {
    loop {
        timer_service_wait();
        let completed = timer_service_completed_timers();

        for i in 0..31 {
            let idx: u32 = 1 << i;
            if completed & idx != 0 {
                unsafe {
                    if let Err(e) = ML_COORD.lock().timer_completed(i as ModelIdx) {
                        error!("Error when trying to run periodic model: {:?}", e);
                    }
                }
            }
        }
    }
}

unsafe fn validate_ids(
    c_bundle_id: *const cstr_core::c_char,
    c_model_id: *const cstr_core::c_char,
) -> Result<(String, String), MlCoordError> {
    let bundle_id = CStr::from_ptr(c_bundle_id)
        .to_str()
        .map_err(|_| MlCoordError::InvalidBundleId)?;
    let model_id = CStr::from_ptr(c_model_id)
        .to_str()
        .map_err(|_| MlCoordError::InvalidModelId)?;
    Ok((String::from(bundle_id), String::from(model_id)))
}

#[no_mangle]
pub unsafe extern "C" fn mlcoord_oneshot(
    c_bundle_id: *const cstr_core::c_char,
    c_model_id: *const cstr_core::c_char,
) -> MlCoordError {
    let (bundle_id, model_id) = match validate_ids(c_bundle_id, c_model_id) {
        Ok(ids) => ids,
        Err(e) => return e,
    };

    if let Err(e) = ML_COORD.lock().oneshot(bundle_id, model_id) {
        return e;
    }

    MlCoordError::MlCoordOk
}

#[no_mangle]
pub unsafe extern "C" fn mlcoord_periodic(
    c_bundle_id: *const cstr_core::c_char,
    c_model_id: *const cstr_core::c_char,
    rate_in_ms: u32,
) -> MlCoordError {
    let (bundle_id, model_id) = match validate_ids(c_bundle_id, c_model_id) {
        Ok(ids) => ids,
        Err(e) => return e,
    };
    if let Err(e) = ML_COORD.lock().periodic(bundle_id, model_id, rate_in_ms) {
        return e;
    }

    MlCoordError::MlCoordOk
}

#[no_mangle]
pub unsafe extern "C" fn mlcoord_cancel(
    c_bundle_id: *const cstr_core::c_char,
    c_model_id: *const cstr_core::c_char,
) -> MlCoordError {
    let (bundle_id, model_id) = match validate_ids(c_bundle_id, c_model_id) {
        Ok(ids) => ids,
        Err(e) => return e,
    };

    if let Err(e) = ML_COORD.lock().cancel(bundle_id, model_id) {
        return e;
    }

    MlCoordError::MlCoordOk
}

#[no_mangle]
pub unsafe extern "C" fn host_req_handle() {
    ML_COORD.lock().handle_host_req_interrupt();
}

#[no_mangle]
pub unsafe extern "C" fn finish_handle() {
    ML_COORD.lock().handle_return_interrupt();
}

#[no_mangle]
pub unsafe extern "C" fn instruction_fault_handle() {
    ML_COORD.lock().handle_instruction_fault_interrupt();
}

#[no_mangle]
pub unsafe extern "C" fn data_fault_handle() {
    ML_COORD.lock().handle_data_fault_interrupt();
}

#[no_mangle]
pub unsafe extern "C" fn mlcoord_debug_state() {
    ML_COORD.lock().debug_state();
}
