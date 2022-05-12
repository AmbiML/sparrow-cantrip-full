#![no_std]

// ML Coordinator Design Doc: go/sparrow-ml-doc

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use cantrip_ml_interface::MlCoordError;
use cantrip_ml_interface::MlCoreInterface;
use cantrip_os_common::cspace_slot::CSpaceSlot;
use cantrip_security_interface::*;
use cantrip_timer_interface::*;
use cantrip_vec_core::MlCore;
use log::{error, info, trace, warn};

/// The maximum number of models that the MLCoordinator can handle, bounded by
/// timer slots. It's unlikely we'll be anywhere near this.
const MAX_MODELS: usize = 32;

/// Represents a single loadable model.
#[derive(Debug)]
struct LoadableModel {
    bundle_id: String,
    model_id: String,
    rate_in_ms: Option<u32>,
}

/// Statistics on non-happy-path events.
#[derive(Debug)]
struct Statistics {
    load_failures: u32,
    already_queued: u32,
}

pub struct MLCoordinator {
    /// The currently running model index, if any.
    running_model: Option<ModelIdx>,
    /// The currently loaded model.
    // NB: This will be removed once the WMMU allows for multiple models loaded
    loaded_model: Option<ModelIdx>,
    /// A list of all models that have been requested for oneshot or periodic
    /// execution.
    models: [Option<LoadableModel>; MAX_MODELS],
    /// A queue of models that are ready for immediate execution on the vector
    /// core, once the currently running model has finished.
    execution_queue: Vec<ModelIdx>,
    statistics: Statistics,
    ml_core: MlCore,
}

// The index of a model in MLCoordinator.models
pub type ModelIdx = usize;

// NB: Can't use `None` as it's Option<T>, need to clarify its Option<Model>
const INIT_NONE: Option<LoadableModel> = None;

impl MLCoordinator {
    pub const fn new() -> Self {
        MLCoordinator {
            running_model: None,
            loaded_model: None,
            models: [INIT_NONE; MAX_MODELS],
            execution_queue: Vec::new(),
            statistics: Statistics{load_failures: 0, already_queued: 0},
            ml_core: MlCore {},
        }
    }

    /// Initialize the vector core.
    pub fn init(&mut self) {
        self.ml_core.enable_interrupts(true);
        self.execution_queue.reserve(MAX_MODELS);
    }

    /// Load a model by copying it into the Vector Core's TCM, if it's not
    /// already been loaded.
    fn load_model(&mut self, model_idx: ModelIdx) -> Result<(), MlCoordError> {
        if self.loaded_model == Some(model_idx) {
            trace!("Model already loaded, skipping load");
            return Ok(());
        }

        // Ensure we have a model at the passed index. This shouldn't error.
        let model = self.models[model_idx]
            .as_ref()
            .ok_or(MlCoordError::LoadModelFailed)?;

        // Loads |model_id| associated with |bundle_id| from the
        // SecurityCoordinator. The data are returned as unmapped
        // page frames in a CNode container left in |container_slot|.
        // To load the model into the vector core the pages must be
        // mapped into the MlCoordinator's VSpace before being copied
        // to their destination.
        let container_slot = CSpaceSlot::new();
        match cantrip_security_load_model(&model.bundle_id, &model.model_id, &container_slot) {
            Ok(model_frames) => {
                match self.ml_core.load_image(&model_frames) {
                    Err(e) => {
                        error!(
                            "Load of {}:{} failed: {:?}",
                            &model.bundle_id, &model.model_id, e
                        );
                        // May have corrupted TCM.
                        self.loaded_model = None;
                        self.statistics.load_failures += 1;
                        Err(MlCoordError::LoadModelFailed)
                    }
                    Ok(sections) => {
                        info!("Load successful.");
                        self.ml_core.set_wmmu(&sections);
                        self.loaded_model = Some(model_idx);
                        Ok(())
                    }
                }
            }
            Err(e) => {
                error!(
                    "LoadModel of bundle {}:{} failed: {:?}",
                    &model.bundle_id, &model.model_id, e
                );
                self.statistics.load_failures += 1;
                Err(MlCoordError::LoadModelFailed)
            }
        }
    }

    /// If there is a next model in the queue, load it onto the core and start
    /// running. If there's already a running model, don't do anything.
    fn schedule_next_model(&mut self) -> Result<(), MlCoordError> {
        if !self.running_model.is_some() && !self.execution_queue.is_empty() {
            let next_idx = self.execution_queue.remove(0);
            // If load model fails we won't try and re-queue this model.
            // It's very unlikely for load errors to be transient, it should
            // only happen in the case of a mal-formed model.
            self.load_model(next_idx)?;
            self.ml_core.run(); // Unhalt, start at default PC.
            self.running_model = Some(next_idx);
        }

        Ok(())
    }

    pub fn handle_return_interrupt(&mut self) {
        extern "C" {
            fn finish_acknowledge() -> u32;
        }

        // TODO(jesionowski): Move the result from TCM to SRAM,
        // update the input/model.
        let return_code = MlCore::get_return_code();
        let fault = MlCore::get_fault_register();

        // TODO(jesionowski): Signal the application that there was a failure.
        if return_code != 0 {
            error!(
                "vctop execution failed with code {}, fault pc: {:#010X}",
                return_code, fault
            );
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
        bundle_id: String,
        model_id: String,
        rate_in_ms: Option<u32>,
    ) -> Result<ModelIdx, MlCoordError> {
        // Return None if all slots are full.
        let index = self
            .models
            .iter()
            .position(|m| m.is_none())
            .ok_or(MlCoordError::NoModelSlotsLeft)?;

        self.models[index] = Some(LoadableModel {
            bundle_id,
            model_id,
            rate_in_ms,
        });

        Ok(index)
    }

    /// Start a one-time model execution, to be executed immediately.
    pub fn oneshot(&mut self, bundle_id: String, model_id: String) -> Result<(), MlCoordError> {
        let model_idx = self.ready_model(bundle_id, model_id, None)?;

        self.execution_queue.push(model_idx);
        self.schedule_next_model()?;

        Ok(())
    }

    /// Start a periodic model execution, to be executed immediately and
    /// then every rate_in_ms.
    pub fn periodic(
        &mut self,
        bundle_id: String,
        model_id: String,
        rate_in_ms: u32,
    ) -> Result<(), MlCoordError> {
        let model_idx = self.ready_model(bundle_id, model_id, Some(rate_in_ms))?;

        self.execution_queue.push(model_idx);
        self.schedule_next_model()?;

        timer_service_periodic(model_idx as u32, rate_in_ms);

        Ok(())
    }

    /// Cancels an outstanding execution.
    pub fn cancel(&mut self, bundle_id: String, model_id: String) -> Result<(), MlCoordError> {
        // Find the model index matching the bundle/model id.
        let model_idx = self
            .models
            .iter()
            .position(|optm| {
                if let Some(m) = optm {
                    m.bundle_id == bundle_id && m.model_id == model_id
                } else {
                    false
                }
            })
            .ok_or(MlCoordError::NoSuchModel)?;

        // If the model is periodic, cancel the timer.
        if self.models[model_idx]
            .as_ref()
            .unwrap()
            .rate_in_ms
            .is_some()
        {
            timer_service_cancel(model_idx as u32);
        }

        // If the model is scheduled to be executed, remove it.
        let execution_idx = self
            .execution_queue
            .iter()
            .position(|idx| *idx == model_idx);
        if let Some(idx) = execution_idx {
            self.execution_queue.remove(idx);
        }

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
                    &model.bundle_id, &model.model_id
                );
                self.statistics.already_queued += 1;
                return Ok(());
            }

            self.execution_queue.push(model_idx);
            self.schedule_next_model()?;
        }

        Ok(())
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
            Some(model) => (&model.bundle_id, &model.model_id),
            None => ("None", "None"),
        }
    }

    pub fn debug_state(&self) {
        match self.running_model {
            Some(idx) => {
                let (bundle, model) = self.ids_at(idx);
                info!("Running model: {}:{}", bundle, model);
            }
            None => info!("No running model.")
        }

        match self.loaded_model {
            Some(idx) => {
                let (bundle, model) = self.ids_at(idx);
                info!("Loaded model: {}:{}", bundle, model);
            }
            None => info!("No loaded model.")
        }

        info!("Loadable Models:");
        for model in self.models.as_ref() {
            if let Some(m) = model {
                info!("  {:?}", m);
            }
        }

        info!("Execution Queue:");
        for idx in &self.execution_queue {
            let (bundle, model) = self.ids_at(*idx);
            info!("  {}:{}", bundle, model);
        }

        info!("Statistics: {:?}", self.statistics);
    }
}
