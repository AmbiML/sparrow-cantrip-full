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

#![no_std]

// ML Coordinator Design Doc: go/sparrow-ml-doc

use static_assertions::assert_cfg;
assert_cfg!(
    any(feature = "springbok_support", feature = "kelvin_support"),
    "No vector core configured"
);
assert_cfg!(
    not(all(feature = "springbok_support", feature = "kelvin_support")),
    "Only one vector core may be specified"
);

extern crate alloc;
use alloc::vec::Vec;
use cantrip_memory_interface::cantrip_object_free_in_cnode;
use cantrip_ml_interface::MlCoordError;
use cantrip_ml_interface::MlOutput;
use cantrip_ml_interface::MAX_OUTPUT_DATA;
use cantrip_ml_shared::*;
use cantrip_ml_support::image_manager::ImageManager;
use cantrip_os_common::cspace_slot::CSpaceSlot;
use cantrip_os_common::sel4_sys::seL4_Word;
use cantrip_proc_interface::BundleImage;
use cantrip_security_interface::*;
use cantrip_timer_interface::*;
use log::{error, info, trace, warn};

#[cfg(feature = "kelvin_support")]
use kelvin_vec_core as MlCore;
#[cfg(feature = "springbok_support")]
use springbok_vec_core as MlCore;

use MlCore::MAX_MODELS;

/// Loadable model.
#[derive(Debug)]
struct LoadableModel {
    id: ImageId,
    on_flash_sizes: ImageSizes,
    in_memory_sizes: ImageSizes,
    rate_in_ms: Option<u32>,
    client_id: seL4_Word,
    jobnum: usize,
    output_header: Option<OutputHeader>, // Output header from last run.
    output_data: [u8; MAX_OUTPUT_DATA],  // Data returned from last run.
}
impl LoadableModel {
    pub fn new(
        id: ImageId,
        on_flash_sizes: ImageSizes,
        in_memory_sizes: ImageSizes,
        rate_in_ms: Option<u32>,
        client_id: seL4_Word,
    ) -> Self {
        Self {
            id,
            on_flash_sizes,
            in_memory_sizes,
            rate_in_ms,
            client_id,
            jobnum: 0,
            output_header: None,
            output_data: [0; MAX_OUTPUT_DATA],
        }
    }
}

/// Statistics on non-happy-path events.
#[derive(Debug)]
struct Statistics {
    load_failures: u32,
    already_queued: u32,
}

pub struct MLCoordinator {
    /// The currently running model, if any.
    running_model: Option<ImageId>,
    /// A list of all models that have been requested for oneshot or periodic
    /// execution.
    models: [Option<LoadableModel>; MAX_MODELS],
    /// A queue of models that are ready for immediate execution on the vector
    /// core, once the currently running model has finished.
    execution_queue: Vec<ModelIdx>,
    /// Bitmask of completed model runs.
    // XXX needs to be per-client
    completed_job_mask: usize,
    /// The image manager is responsible for tracking, loading, and unloading
    /// images.
    image_manager: ImageManager,
    /// Value associated with each model run.
    /// Returned by get_output to distinguish returned data.
    jobnum: usize,
    statistics: Statistics,
}

// The index of a model in MLCoordinator.models
pub type ModelIdx = usize;

impl MLCoordinator {
    pub const fn new() -> Self {
        // NB: The repeat operand requires a const item.
        const INIT_NONE: Option<LoadableModel> = None;
        MLCoordinator {
            running_model: None,
            models: [INIT_NONE; MAX_MODELS],
            execution_queue: Vec::new(),
            completed_job_mask: 0,
            image_manager: ImageManager::new(),
            jobnum: 0,
            statistics: Statistics {
                load_failures: 0,
                already_queued: 0,
            },
        }
    }

    /// Initialize the vector core.
    pub fn init(&mut self) {
        MlCore::enable_interrupts(true);
        self.execution_queue.reserve(MAX_MODELS);
        self.image_manager.init();
    }

    // Validates the image by ensuring it has all the required loadable
    // sections and that it fits into the TCM. Returns a tuple of
    // |(on_flash_sizes, in_memory_sizes)|.
    fn validate_image(&self, id: &ImageId) -> Option<(ImageSizes, ImageSizes)> {
        let mut container_slot = CSpaceSlot::new();
        match cantrip_security_load_model(&id.bundle_id, &id.model_id, &container_slot) {
            Ok(model_frames) => {
                container_slot.release(); // NB: take ownership
                let mut image = BundleImage::new(&model_frames);
                let result = MlCore::preprocess_image(id, &mut image);
                drop(image); // NB: before releasing objects
                let _ = cantrip_object_free_in_cnode(&model_frames);

                result
            }
            Err(status) => {
                error!("{}: Security Core error {:?}", &id, status);
                None
            }
        }
    }

    // If there is a next model in the queue, load it onto the vector core and
    // start running. If there's already a running model, don't do anything.
    fn schedule_next_model(&mut self) -> Result<(), MlCoordError> {
        if self.running_model.is_some() || self.execution_queue.is_empty() {
            return Ok(());
        }

        let next_idx = self.execution_queue.remove(0);
        let model = self.models[next_idx].as_mut().expect("Model get fail");

        if !self.image_manager.is_loaded(&model.id) {
            // Loads |model_id| associated with |bundle_id| from the
            // SecurityCoordinator. The data are returned as unmapped
            // page frames in a CNode container left in |container_slot|.
            // To load the model into the vector core the pages must be
            // mapped into the MlCoordinator's VSpace before being copied
            // to the TCM.
            let mut container_slot = CSpaceSlot::new();
            match cantrip_security_load_model(
                &model.id.bundle_id,
                &model.id.model_id,
                &container_slot,
            ) {
                Ok(model_frames) => {
                    container_slot.release(); // NB: take ownership
                    let mut image = BundleImage::new(&model_frames);

                    // Ask the image manager to make enough room and get
                    // the starting address for writing the image.
                    let temp_top = self.image_manager.make_space(
                        model.in_memory_sizes.data_top_size(),
                        model.in_memory_sizes.temporary_data,
                    );
                    MlCore::write_image(
                        &mut image,
                        temp_top,
                        &model.on_flash_sizes,
                        &model.in_memory_sizes,
                    )?;
                    info!("Load successful.");

                    drop(image); // NB: before releasing objects
                    let _ = cantrip_object_free_in_cnode(&model_frames);

                    // Inform the image manager the image has been written.
                    self.image_manager
                        .commit_image(model.id.clone(), model.in_memory_sizes);
                }
                Err(e) => {
                    error!("{}: LoadModel failed: {:?}", &model.id, e);
                    self.statistics.load_failures += 1;
                    return Err(MlCoordError::LoadModelFailed);
                }
            }
        }

        // TODO(jesionowski): Investigate if we need to clear the entire
        // temporary data section or just certain parts.
        // TODO(jesionowski): When hardware clear is enabled, we should
        // kick it off after the run instead.
        self.image_manager.clear_temp_data();

        #[cfg(feature = "springbok_support")]
        self.image_manager.set_wmmu(&model.id);

        // Clear output state.
        // TODO(sleffler): defer to give client more time to retrieve? (esp for periodic)
        model.output_header = None;

        // Assign run a new jobnum.
        model.jobnum = self.jobnum;
        self.jobnum = self.jobnum.wrapping_add(1);

        self.running_model = Some(model.id.clone());
        MlCore::run(); // Start core at default PC.

        Ok(())
    }

    pub fn handle_return_interrupt(&mut self) {
        trace!("Vector Core finish.");
        self.process_return_interrupt();
        self.running_model = None;
        if let Err(e) = self.schedule_next_model() {
            error!("Running next model failed with {:?}", e)
        }
        // Clear/ack interrupt.
        MlCore::clear_finish();
    }

    fn process_return_interrupt(&mut self) -> Option<()> {
        let image_id = self.running_model.as_ref().or_else(|| {
            // Should not happen; always complain.
            error!("Vector Core finish with no running model.");
            None
        })?;
        // The image may not be loaded if the job was canceled; ignore.
        let header = self.image_manager.output_header(image_id)?;
        if header.epc.is_some() || header.return_code != 0 {
            // Application is notified below and can ask for status
            // to find return code and any other available info (e.g
            // epc on Springbok).
            error!("{} finished: {:?}", &image_id, &header);
        }

        // The app that started the model may have unloaded the image
        // when stopping; ignore.
        let idx = self.get_model_index(image_id)?;

        // Save output header and any indirect data.
        let model = self.models[idx].as_mut().unwrap();
        model.output_header = Some(header);
        model.output_data.fill(0);
        if header.output_length != 0 {
            trace!("{:#x?}", &header);
            if let Some(output_ptr) = header.output_ptr {
                MlCore::tcm_read(
                    output_ptr as usize,
                    header.output_length as usize,
                    &mut model.output_data,
                );
                trace!("{:#x?}", &model.output_data);
            }
        }

        // Mark the job completed and notify the client.
        self.completed_job_mask |= 1 << idx;
        unsafe {
            extern "Rust" {
                fn mlcoord_emit(badge: seL4_Word);
            }
            mlcoord_emit(model.client_id);
        }
        Some(())
    }

    // Sets up a loadable model for |id|, returning the index of that model.
    fn ready_model(
        &mut self,
        client_id: usize,
        id: ImageId,
        rate_in_ms: Option<u32>,
    ) -> Result<ModelIdx, MlCoordError> {
        // Return NoModelSlotsLeft if all slots are full.
        let index = self
            .models
            .iter()
            .position(|m| m.is_none())
            .ok_or(MlCoordError::NoModelSlotsLeft)?;

        let (on_flash_sizes, in_memory_sizes) =
            self.validate_image(&id).ok_or(MlCoordError::InvalidImage)?;

        self.models[index] = Some(LoadableModel::new(
            id,
            on_flash_sizes,
            in_memory_sizes,
            rate_in_ms,
            client_id,
        ));

        Ok(index)
    }

    // Returns the index for model |id|, if it exists.
    fn get_model_index(&self, id: &ImageId) -> Option<ModelIdx> {
        self.models.iter().position(|opti| {
            if let Some(i) = opti {
                i.id == *id
            } else {
                false
            }
        })
    }

    /// Starts a one-time model execution, to happen immediately.
    pub fn oneshot(&mut self, client_id: usize, id: ImageId) -> Result<(), MlCoordError> {
        // Check if we've loaded this model already.
        let idx = match self.get_model_index(&id) {
            Some(idx) => idx,
            None => self.ready_model(client_id, id, None)?,
        };

        self.execution_queue.push(idx);
        self.schedule_next_model()?;

        Ok(())
    }

    /// Start a periodic model execution, to happen immediately
    /// and repeat every |rate_in_ms|.
    pub fn periodic(
        &mut self,
        client_id: usize,
        id: ImageId,
        rate_in_ms: u32,
    ) -> Result<(), MlCoordError> {
        // XXX mucks with model state before we are assured of succcess
        // Check if we've loaded this model already.
        let idx = match self.get_model_index(&id) {
            Some(idx) => {
                // Force the timer duration in case the image was loaded as a oneshot
                // XXX if was periodic is there a timer running that needs to be canceled?
                self.models[idx].as_mut().unwrap().rate_in_ms = Some(rate_in_ms);
                idx
            }
            None => self.ready_model(client_id, id, Some(rate_in_ms))?,
        };

        match cantrip_timer_periodic(idx as TimerId, rate_in_ms) {
            Ok(_) => {
                self.execution_queue.push(idx);
                self.schedule_next_model()?;
                Ok(())
            }
            Err(e) => {
                error!("cantrip_timer_periodic({}, {}) returns {:?}", idx, rate_in_ms, e);
                // XXX map e so caller gets a more meaningful error?
                Err(MlCoordError::InvalidTimer)
            }
        }
    }

    /// Cancels an outstanding execution.
    pub fn cancel(&mut self, id: &ImageId) -> Result<(), MlCoordError> {
        // Find the model index matching the bundle/model id.
        let model_idx = self.get_model_index(id).ok_or(MlCoordError::NoSuchModel)?;

        // If the model is periodic, cancel the timer.
        if self.models[model_idx]
            .as_ref()
            .unwrap()
            .rate_in_ms
            .is_some()
        {
            // XXX just continue if error
            if let Err(e) = cantrip_timer_cancel(model_idx as TimerId) {
                warn!("Cancel timer {} failed: {:?}", model_idx, e);
            }
        }

        // If the model is scheduled to be executed, remove it.
        let execution_idx = self
            .execution_queue
            .iter()
            .position(|idx| *idx == model_idx);
        if let Some(idx) = execution_idx {
            self.execution_queue.remove(idx);
        }

        self.image_manager.unload_image(id);
        self.completed_job_mask |= 1 << model_idx;

        self.models[model_idx] = None;
        Ok(())
    }

    /// Enqueues the model associated with the completed timer.
    pub fn timer_completed(&mut self, model_idx: ModelIdx) -> Result<(), MlCoordError> {
        // There's a small chance the model was removed at the same time the
        // timer interrupt fires, in which case we just ignore it.
        if self.models[model_idx].is_some() {
            // We don't want the queue to grow unbounded, so don't requeue
            // an execution if there's one scheduled already.
            if self.execution_queue.iter().any(|idx| *idx == model_idx) {
                let model = self.models[model_idx].as_ref().unwrap();
                warn!(
                    "Dropping {}:{} duplicate periodic execution.",
                    &model.id.bundle_id, &model.id.model_id
                );
                self.statistics.already_queued += 1;
                return Ok(());
            }

            self.execution_queue.push(model_idx);
            self.schedule_next_model()?;
        }

        Ok(())
    }

    pub fn completed_jobs(&mut self) -> u32 {
        // XXX restrict mask to client jobs
        let mask = self.completed_job_mask;
        self.completed_job_mask = 0;
        mask as u32
    }

    pub fn get_output(&mut self, id: &ImageId) -> Result<MlOutput, MlCoordError> {
        let idx = self.get_model_index(id).ok_or(MlCoordError::NoSuchModel)?;
        let model = self.models[idx].as_mut().unwrap();
        let header = model.output_header.ok_or(MlCoordError::NoOutputHeader)?;
        Ok(MlOutput {
            jobnum: model.jobnum,
            return_code: header.return_code,
            epc: header.epc,
            data: model.output_data,
        })
    }

    pub fn handle_host_req_interrupt(&self) { MlCore::clear_host_req(); }

    pub fn handle_instruction_fault_interrupt(&self) {
        MlCore::clear_instruction_fault();
        error!("Vector Core instruction fault.");
    }

    #[cfg(feature = "CONFIG_PLAT_SPARROW")]
    pub fn handle_data_fault_interrupt(&self) {
        MlCore::clear_data_fault();
        error!("Vector Core data fault.");
    }

    fn ids_at(&self, idx: ModelIdx) -> (&str, &str) {
        match self.models[idx].as_ref() {
            Some(model) => (&model.id.bundle_id, &model.id.model_id),
            None => ("None", "None"),
        }
    }

    pub fn debug_state(&self) {
        MlCore::debug_state();
        match &self.running_model {
            Some(id) => {
                info!(target: "", "Running model: {}", &id);
            }
            None => info!(target: "", "No running model."),
        }

        info!(target: "", "Loadable Models:");
        for model in self.models.as_ref().iter().flatten() {
            info!(target: "", "  {:X?}", model);
        }

        info!(target: "", "Execution Queue:");
        for idx in &self.execution_queue {
            let (bundle, model) = self.ids_at(*idx);
            info!(target: "", "  {}:{}", bundle, model);
        }

        info!(target: "", "{:?}", self.statistics);
        self.image_manager.debug_state();
    }
}
