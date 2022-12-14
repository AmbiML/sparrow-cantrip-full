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

extern crate alloc;

use alloc::vec::Vec;
use cantrip_memory_interface::cantrip_object_free_in_cnode;
use cantrip_ml_interface::MlCoordError;
use cantrip_ml_shared::*;
use cantrip_ml_support::image_manager::ImageManager;
use cantrip_os_common::cspace_slot::CSpaceSlot;
use cantrip_os_common::sel4_sys;
use cantrip_proc_interface::BundleImage;
use cantrip_security_interface::*;
use cantrip_timer_interface::*;
use cantrip_vec_core as MlCore;
use log::{error, info, trace, warn};

use sel4_sys::seL4_Word;

/// Represents a single loadable model.
#[derive(Debug)]
struct LoadableModel {
    id: ImageId,
    on_flash_sizes: ImageSizes,
    in_memory_sizes: ImageSizes,
    rate_in_ms: Option<u32>,
    client_id: seL4_Word,
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
    statistics: Statistics,
}

// The index of a model in MLCoordinator.models
pub type ModelIdx = usize;

// NB: Can't use `None` as it's Option<T>, need to clarify its Option<Model>
const INIT_NONE: Option<LoadableModel> = None;

impl MLCoordinator {
    pub const fn new() -> Self {
        MLCoordinator {
            running_model: None,
            models: [INIT_NONE; MAX_MODELS],
            execution_queue: Vec::new(),
            completed_job_mask: 0,
            image_manager: ImageManager::new(),
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

                let mut on_flash_sizes = ImageSizes::default();
                let mut in_memory_sizes = ImageSizes::default();

                while let Some(section) = image.next_section() {
                    match section.vaddr {
                        TEXT_VADDR => {
                            on_flash_sizes.text = section.fsize;
                            in_memory_sizes.text = round_up(section.msize, WMMU_PAGE_SIZE);
                        }
                        CONST_DATA_VADDR => {
                            on_flash_sizes.constant_data = section.fsize;
                            in_memory_sizes.constant_data = round_up(section.msize, WMMU_PAGE_SIZE);
                        }
                        MODEL_OUTPUT_VADDR => {
                            on_flash_sizes.model_output = section.fsize;
                            in_memory_sizes.model_output = round_up(section.msize, WMMU_PAGE_SIZE);
                        }
                        STATIC_DATA_VADDR => {
                            on_flash_sizes.static_data = section.fsize;
                            in_memory_sizes.static_data = round_up(section.msize, WMMU_PAGE_SIZE);
                        }
                        TEMP_DATA_VADDR => {
                            on_flash_sizes.temporary_data = section.fsize;
                            in_memory_sizes.temporary_data =
                                round_up(section.msize, WMMU_PAGE_SIZE);
                        }
                        vaddr => {
                            warn!("{}: skipping unexpected section at {:#x}", &id, vaddr);
                        }
                    }
                }

                if !in_memory_sizes.is_valid() {
                    error!("{} invalid, section missing: {:?}", &id, in_memory_sizes);
                    return None;
                }
                if in_memory_sizes.total_size() > TCM_SIZE {
                    error!("{} too big to fit in TCM: {:?}", &id, in_memory_sizes);
                    return None;
                }

                drop(image);
                let _ = cantrip_object_free_in_cnode(&model_frames);

                Some((on_flash_sizes, in_memory_sizes))
            }
            Err(status) => {
                error!("{}: Security Core error {:?}", &id, status);
                None
            }
        }
    }

    fn reload_static_data(&self, model: &LoadableModel) -> Result<(), MlCoordError> {
        let mut container_slot = CSpaceSlot::new();
        let model_frames =
            cantrip_security_load_model(&model.id.bundle_id, &model.id.model_id, &container_slot)
                .map_err(|_| MlCoordError::LoadModelFailed)?;
        container_slot.release(); // NB: take ownership

        let mut image = BundleImage::new(&model_frames);

        // Find top address for loading the data segment.
        let mut temp_top = self.image_manager.get_top_addr(&model.id).unwrap();
        trace!("reload {} temp_top {:#x}", &model.id, temp_top);

        while let Some(section) = image.next_section() {
            if section.vaddr == TEXT_VADDR {
                temp_top += model.in_memory_sizes.text;
            } else if section.vaddr == CONST_DATA_VADDR {
                temp_top += model.in_memory_sizes.constant_data;
            } else if section.vaddr == MODEL_OUTPUT_VADDR {
                temp_top += model.in_memory_sizes.model_output;
            } else if section.vaddr == STATIC_DATA_VADDR {
                MlCore::write_image_part(
                    &mut image,
                    temp_top,
                    model.on_flash_sizes.static_data,
                    model.in_memory_sizes.static_data,
                )
                .ok_or(MlCoordError::LoadModelFailed)?;
                break;
            }
        }
        drop(image);
        let _ = cantrip_object_free_in_cnode(&model_frames);

        Ok(())
    }

    // If there is a next model in the queue, load it onto the vector core and
    // start running. If there's already a running model, don't do anything.
    fn schedule_next_model(&mut self) -> Result<(), MlCoordError> {
        if self.running_model.is_some() || self.execution_queue.is_empty() {
            return Ok(());
        }

        let next_idx = self.execution_queue.remove(0);
        let model = self.models[next_idx].as_ref().expect("Model get fail");

        let image_is_loaded = self.image_manager.is_loaded(&model.id);
        if !image_is_loaded {
            // Loads |model_id| associated with |bundle_id| from the
            // SecurityCoordinator. The data are returned as unmapped
            // page frames in a CNode container left in |container_slot|.
            // To load the model into the vector core the pages must be
            // mapped into the MlCoordinator's VSpace before being copied
            // to their destination.
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
                    // the address to write to.
                    let mut temp_top = self.image_manager.make_space(
                        model.in_memory_sizes.data_top_size(),
                        model.in_memory_sizes.temporary_data,
                    );
                    trace!("first load {} temp_top {:#x}", &model.id, temp_top);

                    while let Some(section) = image.next_section() {
                        // TODO(jesionowski): Ensure these are in order.
                        if section.vaddr == TEXT_VADDR {
                            MlCore::write_image_part(
                                &mut image,
                                temp_top,
                                model.on_flash_sizes.text,
                                model.in_memory_sizes.text,
                            )
                            .ok_or(MlCoordError::LoadModelFailed)?;

                            temp_top += model.in_memory_sizes.text;
                        } else if section.vaddr == CONST_DATA_VADDR {
                            MlCore::write_image_part(
                                &mut image,
                                temp_top,
                                model.on_flash_sizes.constant_data,
                                model.in_memory_sizes.constant_data,
                            )
                            .ok_or(MlCoordError::LoadModelFailed)?;

                            temp_top += model.in_memory_sizes.constant_data;
                        } else if section.vaddr == MODEL_OUTPUT_VADDR {
                            // Don't load, but do skip.
                            temp_top += model.in_memory_sizes.model_output;
                        } else if section.vaddr == STATIC_DATA_VADDR {
                            MlCore::write_image_part(
                                &mut image,
                                temp_top,
                                model.on_flash_sizes.static_data,
                                model.in_memory_sizes.static_data,
                            )
                            .ok_or(MlCoordError::LoadModelFailed)?;

                            temp_top += model.in_memory_sizes.static_data;
                        }
                    }
                    info!("Load successful.");

                    // Inform the image manager the image has been written.
                    self.image_manager
                        .commit_image(model.id.clone(), model.in_memory_sizes);

                    drop(image);
                    let _ = cantrip_object_free_in_cnode(&model_frames);
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

        if image_is_loaded {
            // TODO(b/258304148): reload .data section to workaround corruption
            self.reload_static_data(&model)?;
        }

        self.image_manager.set_wmmu(&model.id);

        self.running_model = Some(model.id.clone());
        MlCore::run(); // Start core at default PC.

        Ok(())
    }

    pub fn handle_return_interrupt(&mut self) {
        extern "C" {
            fn finish_acknowledge() -> u32;
            fn mlcoord_emit(badge: seL4_Word);
        }

        if let Some(image_id) = self.running_model.as_ref() {
            if let Some(output_header) = self.image_manager.output_header(image_id) {
                // TODO(jesionowski): Move the result from TCM to SRAM,
                // update the input/model.

                if output_header.return_code != 0 {
                    // TODO(jesionowski): Signal the application that there was a failure.
                    error!(
                        "vctop execution failed with code {}, fault pc: {:#010X}",
                        output_header.return_code, output_header.epc
                    );
                }
            } else {
                // This can happen during normal execution if mlcancel happens
                // during an execution.
                warn!("Executable finished running but image is not loaded.");
            }

            // NB: an application that started the model may have unloaded
            //   the image when stopping; just do nothing
            if let Some(idx) = self.get_model_index(image_id) {
                self.completed_job_mask |= 1 << idx;
                unsafe {
                    mlcoord_emit(self.models[idx].as_ref().unwrap().client_id);
                }
            }
        } else {
            error!("Unexpected return interrupt with no running model.")
        }

        self.running_model = None;

        // TODO(jesionowski): Signal the application that owns this model
        // that there was a failure.
        if let Err(e) = self.schedule_next_model() {
            error!("Running next model failed with {:?}", e)
        }

        MlCore::clear_finish();
        assert!(unsafe { finish_acknowledge() == 0 });
    }

    // Constructs a new model and add to an open slot, returning the index
    // of that slot.
    fn ready_model(
        &mut self,
        id: ImageId,
        rate_in_ms: Option<u32>,
    ) -> Result<ModelIdx, MlCoordError> {
        // Return None if all slots are full.
        let index = self
            .models
            .iter()
            .position(|m| m.is_none())
            .ok_or(MlCoordError::NoModelSlotsLeft)?;

        let (on_flash_sizes, in_memory_sizes) =
            self.validate_image(&id).ok_or(MlCoordError::InvalidImage)?;

        extern "C" {
            fn mlcoord_get_sender_id() -> seL4_Word;
        }
        self.models[index] = Some(LoadableModel {
            id,
            on_flash_sizes,
            in_memory_sizes,
            rate_in_ms,
            client_id: unsafe { mlcoord_get_sender_id() },
        });

        Ok(index)
    }

    // Returns the index for model |id| if it exists.
    fn get_model_index(&self, id: &ImageId) -> Option<ModelIdx> {
        self.models.iter().position(|opti| {
            if let Some(i) = opti {
                i.id == *id
            } else {
                false
            }
        })
    }

    /// Start a one-time model execution, to be executed immediately.
    pub fn oneshot(&mut self, id: ImageId) -> Result<(), MlCoordError> {
        // Check if we've loaded this model already.
        let idx = match self.get_model_index(&id) {
            Some(idx) => idx,
            None => self.ready_model(id, None)?,
        };

        self.execution_queue.push(idx);
        self.schedule_next_model()?;

        Ok(())
    }

    /// Start a periodic model execution, to be executed immediately and
    /// then every rate_in_ms.
    pub fn periodic(&mut self, id: ImageId, rate_in_ms: u32) -> Result<(), MlCoordError> {
        // XXX mucks with model state before we are assured of succcess
        // Check if we've loaded this model already.
        let idx = match self.get_model_index(&id) {
            Some(idx) => {
                // Force the timer duration in case the image was loaded as a oneshot
                // XXX if was periodic is there a timer running that needs to be canceled?
                self.models[idx].as_mut().unwrap().rate_in_ms = Some(rate_in_ms);
                idx
            }
            None => self.ready_model(id, Some(rate_in_ms))?,
        };

        match cantrip_timer_periodic(idx as TimerId, rate_in_ms) {
            Ok(_) => {
                self.execution_queue.push(idx);
                self.schedule_next_model()?;
                Ok(())
            }
            Err(e) => {
                error!("cantrip_timer_periodic({}, {}) returns {:?}", idx, rate_in_ms, e);
                // XXX map error?
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
                    "Dropping {}:{} periodic execution as it has an execution outstanding already.",
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

    pub fn handle_host_req_interrupt(&self) {
        extern "C" {
            fn host_req_acknowledge() -> u32;
        }
        MlCore::clear_host_req();
        unsafe {
            assert!(host_req_acknowledge() == 0);
        }
    }

    pub fn handle_instruction_fault_interrupt(&self) {
        extern "C" {
            fn instruction_fault_acknowledge() -> u32;
        }
        error!("Instruction fault in Vector Core.");
        MlCore::clear_instruction_fault();
        unsafe {
            assert!(instruction_fault_acknowledge() == 0);
        }
    }

    pub fn handle_data_fault_interrupt(&self) {
        extern "C" {
            fn data_fault_acknowledge() -> u32;
        }
        error!("Data fault in Vector Core.");
        MlCore::clear_data_fault();
        unsafe {
            assert!(data_fault_acknowledge() == 0);
        }
    }

    fn ids_at(&self, idx: ModelIdx) -> (&str, &str) {
        match self.models[idx].as_ref() {
            Some(model) => (&model.id.bundle_id, &model.id.model_id),
            None => ("None", "None"),
        }
    }

    pub fn debug_state(&self) {
        match &self.running_model {
            Some(id) => {
                info!("Running model: {}:{}", id.bundle_id, id.model_id);
            }
            None => info!("No running model."),
        }

        info!("Loadable Models:");
        for model in self.models.as_ref().iter().flatten() {
            info!("  {:x?}", model);
        }

        info!("Execution Queue:");
        for idx in &self.execution_queue {
            let (bundle, model) = self.ids_at(*idx);
            info!("  {}:{}", bundle, model);
        }

        info!("Statistics: {:?}", self.statistics);

        self.image_manager.debug_state();
    }
}
