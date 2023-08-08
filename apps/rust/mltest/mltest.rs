/*
 * Copyright 2021, Google LLC
 *
 * SPDX-License-Identifier: Apache-2.0
 */
#![no_std]
#![no_main]

use libcantrip::sdk_init;
use log::{error, info};
use sdk_interface::*;

#[no_mangle]
pub fn main() {
    static mut HEAP: [u8; 4096] = [0; 4096];
    sdk_init(unsafe { &mut HEAP });

    const NONEXISTENT_ID: &str = "nonexistent";
    //    info!(
    //        "sdk_model_cancel returned {:?} with nothing running",
    //        sdk_model_cancel(0)
    //    );
    info!(
        "sdk_model_oneshot({}) returned {:?} (as expected)",
        NONEXISTENT_ID,
        sdk_model_oneshot(NONEXISTENT_ID),
    );

    // Probe for a model.
    let model_id: &str = [
        "conv1x1_test_emitc_static.kelvin",
        "hello_world.kelvin",
        "mobilenet_v1_emitc_static.model",
    ]
    .iter()
    .find(|model| match sdk_model_oneshot(model) {
        Ok(id) => {
            info!("{} started, id {}", model, id);
            match sdk_model_wait() {
                Ok(_) => {
                    info!("{} completed", model);
                    let output = sdk_model_output(id);
                    info!("{:?}", &output);
                }
                Err(e) => error!("sdk_model_wait failed: {:?}", e),
            }
            true
        }
        Err(_) => false,
    })
    .or(Some(&NONEXISTENT_ID))
    .unwrap();

    const DURATION: TimerDuration = 1000; // 1s
    match sdk_model_periodic(model_id, DURATION) {
        Ok(id) => {
            let _ = info!("Model {} started, id {}", model_id, id);
            let mut ms: TimerDuration = 0;
            for _ in 0..10 {
                let mask = sdk_model_wait().unwrap();
                if (mask & (1 << id)) != 0 {
                    ms += DURATION;
                }
                info!("Model completed: mask {:#06b} ms {}", mask, ms);
                let output = sdk_model_output(id);
                info!("{:?}", &output);
            }
            if let Err(e) = sdk_model_cancel(id) {
                error!("sdk_tmodel_cancel failed: {:?}", e);
            }
            info!("Model {} canceled", id);
        }
        Err(e) => {
            error!("sdk_model_periodic({}) failed: {:?}", model_id, e);
        }
    }
    info!("DONE!");
}
