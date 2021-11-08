#![no_std]

// ML Coordinator Design Doc: go/sparrow-ml-doc

extern crate cantrip_panic;

use cantrip_logger::CantripLogger;
use cantrip_ml_interface::MlCoordinatorInterface;
use log::{error, info, trace};

mod mlcore;

pub struct MLCoordinator {
    pub is_loaded: bool,
}

pub static mut ML_COORD: MLCoordinator = MLCoordinator { is_loaded: false };

impl MLCoordinator {
    fn handle_return_interrupt(&self) {
        // TODO(hcindyl): check the return code and fault registers, move the result
        // from TCM to SRAM, update the input/model, and call mlcoord_execute again.
        let return_code = mlcore::get_return_code();
        let fault = mlcore::get_fault_register();

        if return_code != 0 {
            error!(
                "vctop execution failed with code {}, fault pc: {:#010X}",
                return_code, fault
            );
        }
    }
}

impl MlCoordinatorInterface for MLCoordinator {
    fn execute(&mut self) {
        extern "C" {
            fn vctop_set_ctrl(ctrl: u32);
        }

        if !self.is_loaded {
            let res = mlcore::loadelf();
            if let Err(e) = res {
                error!("Load error: {:?}", e);
            } else {
                info!("Load successful.");
                self.is_loaded = true;
            }
        }

        if self.is_loaded {
            // Unhalt, start at default PC.
            unsafe {
                vctop_set_ctrl(vctop_ctrl(0, 0, 0));
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn pre_init() {
    static CANTRIP_LOGGER: CantripLogger = CantripLogger;
    log::set_logger(&CANTRIP_LOGGER).unwrap();
    log::set_max_level(log::LevelFilter::Trace);
}

#[no_mangle]
pub extern "C" fn mlcoord__init() {
    // TODO(sleffler): maybe not needed?
    trace!("init");
}

// TODO: Move out of this file into separate (auto-generated?) file.
// TODO: Consider the modular_bitfield crate to represent bitfields.
fn vctop_ctrl(freeze: u32, vc_reset: u32, pc_start: u32) -> u32 {
    (pc_start << 2) + ((vc_reset & 1) << 1) + (freeze & 1)
}

// TODO: Once multiple model support is in start by name.
#[no_mangle]
pub extern "C" fn mlcoord_execute() {
    unsafe {
        ML_COORD.execute();
    }
}

#[no_mangle]
pub extern "C" fn vctop_return_update_result() {
    unsafe {
        ML_COORD.handle_return_interrupt();
    }
}
