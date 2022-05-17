#![no_std]

// ML Coordinator Design Doc: go/sparrow-ml-doc

extern crate alloc;

use alloc::string::String;
use cstr_core::CStr;
use cantrip_ml_interface::MlCoordinatorInterface;
use cantrip_ml_interface::MlCoreInterface;
use cantrip_os_common::allocator;
use cantrip_os_common::cspace_slot::CSpaceSlot;
use cantrip_os_common::logger::CantripLogger;
use cantrip_os_common::sel4_sys;
use cantrip_os_common::slot_allocator::CANTRIP_CSPACE_SLOTS;
use cantrip_security_interface::*;
use cantrip_vec_core::MlCore;
use log::{error, info, trace};

use sel4_sys::seL4_CPtr;

extern "C" {
    static SELF_CNODE_FIRST_SLOT: seL4_CPtr;
    static SELF_CNODE_LAST_SLOT: seL4_CPtr;
}

pub struct MLCoordinator {
    loaded_bundle: Option<String>,
    loaded_model: Option<String>,
    is_running: bool,
    continous_mode: bool,
    ml_core: MlCore,
}

pub static mut ML_COORD: MLCoordinator = MLCoordinator {
    loaded_bundle: None,
    loaded_model: None,
    is_running: false,
    continous_mode: false,
    ml_core: MlCore {},
};

impl MLCoordinator {
    fn init(&mut self) {
        self.ml_core.enable_interrupts(true);
    }

    fn is_loaded(&self) -> bool {
        self.loaded_bundle.is_some() && self.loaded_model.is_some()
    }

    fn cmp_loaded(&self, bundle_id: &str, model_id: &str) -> bool {
        self.loaded_bundle.as_deref() == Some(bundle_id) &&
        self.loaded_model.as_deref() == Some(model_id)
    }

    fn handle_return_interrupt(&mut self) {
        extern "C" {
            fn finish_acknowledge() -> u32;
        }

        // TODO(jesionowski): Move the result from TCM to SRAM,
        // update the input/model.
        let return_code = MlCore::get_return_code();
        let fault = MlCore::get_fault_register();

        if return_code != 0 {
            error!(
                "{}: vctop execution failed with code {}, fault pc: {:#010X}",
                self.loaded_model.as_ref().unwrap(), return_code, fault
            );
            self.continous_mode = false;
        }

        self.is_running = false;
        if self.continous_mode {
            // TODO(sleffler): can !is_loaded happen?
            // XXX needs proper state machine
            // XXX what is the threading/locking model?
            if self.is_loaded() {
                self.ml_core.run(); // Unhalt, start at default PC.
                self.is_running = true;
            }
        }

        MlCore::clear_finish();
        assert!(unsafe { finish_acknowledge() == 0 });
    }
}

impl MlCoordinatorInterface for MLCoordinator {
    fn execute(&mut self, bundle_id: &str, model_id: &str) {
        if self.is_running {
            trace!("Skip execute with {}:{} already running",
                   self.loaded_bundle.as_ref().unwrap(),
                   self.loaded_model.as_ref().unwrap());
            return;
        }

        if !self.cmp_loaded(bundle_id, model_id) {
            // Loads |model_id| associated with |bundle_id| from the
            // SecurityCoordinator. The data are returned as unmapped
            // page frames in a CNode container left in |container_slot|.
            // To load the model into the vector core the pages must be
            // mapped into the MlCoordinator's VSpace before being copied
            // to their destination.
            let container_slot = CSpaceSlot::new();
            match cantrip_security_load_model(bundle_id, model_id, &container_slot) {
                Ok(model_frames) => {
                    if let Err(e) = self.ml_core.load_image(&model_frames) {
                        error!("Load of {}:{} failed: {:?}",
                               bundle_id, model_id, e);
                        // NB: may have corrupted TCM, clear loaded state
                        self.loaded_bundle = None;
                        self.loaded_model = None;
                    } else {
                        info!("Load successful.");
                        self.loaded_bundle = Some(String::from(bundle_id));
                        self.loaded_model = Some(String::from(model_id));
                    }
                }
                Err(e) => {
                    error!("LoadModel of bundle {} model {} failed: {:?}",
                           bundle_id, model_id, e);
                }
            }
        }

        if self.is_loaded() {
            self.ml_core.run(); // Unhalt, start at default PC.
            self.is_running = true;
        }
    }

    fn set_continuous_mode(&mut self, continous: bool) {
        self.continous_mode = continous;
    }
}

#[no_mangle]
pub extern "C" fn pre_init() {
    static CANTRIP_LOGGER: CantripLogger = CantripLogger;
    log::set_logger(&CANTRIP_LOGGER).unwrap();
    log::set_max_level(log::LevelFilter::Trace);

    // TODO(sleffler): temp until we integrate with seL4
    static mut HEAP_MEMORY: [u8; 4 * 1024] = [0; 4 * 1024];
    unsafe {
        allocator::ALLOCATOR.init(HEAP_MEMORY.as_mut_ptr() as usize, HEAP_MEMORY.len());
        trace!(
            "setup heap: start_addr {:p} size {}",
            HEAP_MEMORY.as_ptr(),
            HEAP_MEMORY.len()
        );
    }

    unsafe {
        CANTRIP_CSPACE_SLOTS.init(
            /*first_slot=*/ SELF_CNODE_FIRST_SLOT,
            /*size=*/ SELF_CNODE_LAST_SLOT - SELF_CNODE_FIRST_SLOT
        );
        trace!("setup cspace slots: first slot {} free {}",
               CANTRIP_CSPACE_SLOTS.base_slot(),
               CANTRIP_CSPACE_SLOTS.free_slots());
    }
}

#[no_mangle]
pub extern "C" fn mlcoord__init() {
    unsafe {
        ML_COORD.init();
    }
}

#[no_mangle]
pub extern "C" fn mlcoord_execute(
    c_bundle_id: *const cstr_core::c_char,
    c_model_id: *const cstr_core::c_char,
) {
    unsafe {
        match CStr::from_ptr(c_bundle_id).to_str() {
            Ok(bundle_id) => match CStr::from_ptr(c_model_id).to_str() {
                Ok(model_id) => {
                    ML_COORD.execute(bundle_id, model_id)
                }
                _ => error!("Invalid model_id"),
            }
            _ => error!("Invalid bundle_id"),
        }
    }
}

#[no_mangle]
pub extern "C" fn mlcoord_set_continuous_mode(mode: bool) {
    unsafe {
        ML_COORD.set_continuous_mode(mode);
    }
}

#[no_mangle]
pub extern "C" fn host_req_handle() {
    extern "C" {
        fn host_req_acknowledge() -> u32;
    }
    MlCore::clear_host_req();
    assert!(unsafe { host_req_acknowledge() == 0 });
}

#[no_mangle]
pub extern "C" fn finish_handle() {
    unsafe {
        ML_COORD.handle_return_interrupt();
    }
}

#[no_mangle]
pub extern "C" fn instruction_fault_handle() {
    extern "C" {
        fn instruction_fault_acknowledge() -> u32;
    }
    error!("Instruction fault in Vector Core.");
    MlCore::clear_instruction_fault();
    assert!(unsafe { instruction_fault_acknowledge() == 0 });
}

#[no_mangle]
pub extern "C" fn data_fault_handle() {
    extern "C" {
        fn data_fault_acknowledge() -> u32;
    }
    error!("Data fault in Vector Core.");
    MlCore::clear_data_fault();
    assert!(unsafe { data_fault_acknowledge() == 0 });
}
