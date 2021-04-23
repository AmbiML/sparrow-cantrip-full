#![no_std]
use arrayvec::ArrayVec;
use ml_common as ml;
use ml_common::ExecutiveInterface;

pub struct MlCoordinator<'a> {
    executive: &'a dyn ExecutiveInterface,
    // Application owns the model struct.
    // ML Coordinator owns a reference to the model.
    models: ArrayVec<[&'a ml::Model; 10]>,
}

impl<'a> MlCoordinator<'a> {
    // Create a new ML Coordinator instance.
    fn new(executive: &'a dyn ExecutiveInterface) -> MlCoordinator<'a> {
        let mut models = ArrayVec::<[&ml::Model; 10]>::new();

        MlCoordinator {
            executive: executive,
            models: models,
        }
    }

    // Returns the index of the model if it has already been loaded.
    fn has_model(&self, model: &'a ml::Model) -> Option<usize> {
        self.models.iter().position(|&x| model == x)
    }

    // Load the passed model. Returns true if the model was loaded
    // successfully, and false if there is no room or the model was already loaded.
    // This function will eventually perform validation of the model, create transfer buffers, etc.
    #[must_use]
    fn load(&mut self, model: &'a ml::Model) -> bool {
        if self.models.is_full() || self.has_model(model).is_some() {
            return false;
        }
        self.models.push(model);
        true
    }

    // Unload the passed model. Returns true if the model was unloaded successfully,
    // false if the model was not loaded in the first place.
    fn unload(&mut self, model: &'a ml::Model) -> bool {
        let res = self.has_model(model);
        if let Some(idx) = res {
            self.models.remove(idx);
        }
        res.is_some()
    }

    // Execute the passed model.
    fn execute(&self, model: &'a ml::Model) -> Result<&'a [u8], ml::ExecutionError> {
        self.has_model(model).ok_or(ml::ExecutionError::ModelNotLoaded)?;
        self.executive.run_model(model)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeExecutive {
        output_memory: [u8; 512],
        fake_error: Option<ml::ExecutionError>,
    }

    impl ExecutiveInterface for FakeExecutive {
        fn run_model(&self, _model: &ml::Model) -> Result<&[u8], ml::ExecutionError> {
            match self.fake_error {
                Some(err) => Err(err),
                None => Ok(&self.output_memory),
            }
        }
    }

    #[test]
    fn test_load_unload() {
        let exec = tests::FakeExecutive {
            output_memory: [0x00; 512],
            fake_error: None,
        };
        let model = ml::Model {
            output_activations_len: 512,
        };
        let mut ml_coord = MlCoordinator::new(&exec);
        assert!(ml_coord.load(&model));
        assert!(ml_coord.unload(&model));
    }

    #[test]
    fn test_unload_noload() {
        let exec = tests::FakeExecutive {
            output_memory: [0x00; 512],
            fake_error: None,
        };
        let model = ml::Model {
            output_activations_len: 512,
        };
        let mut ml_coord = MlCoordinator::new(&exec);
        assert_eq!(ml_coord.unload(&model), false);
    }


    #[test]
    fn test_unload_twice() {
        let exec = tests::FakeExecutive {
            output_memory: [0x00; 512],
            fake_error: None,
        };
        let model = ml::Model {
            output_activations_len: 512,
        };
        let mut ml_coord = MlCoordinator::new(&exec);
        assert!(ml_coord.load(&model));
        assert!(ml_coord.unload(&model));
        assert_eq!(ml_coord.unload(&model), false);
    }

    #[test]
    fn test_load_twice() {
        let exec = tests::FakeExecutive {
            output_memory: [0x00; 512],
            fake_error: None,
        };
        let model = ml::Model {
            output_activations_len: 512,
        };
        let mut ml_coord = MlCoordinator::new(&exec);
        assert!(ml_coord.load(&model));
        assert_eq!(ml_coord.load(&model), false);
    }

    #[test]
    fn test_execute_noload() {
        let exec = tests::FakeExecutive {
            output_memory: [0x00; 512],
            fake_error: None,
        };
        let model = ml::Model {
            output_activations_len: 512,
        };
        let mut ml_coord = MlCoordinator::new(&exec);
        assert!(ml_coord.execute(&model).is_err());
    }

    #[test]
    fn test_execute() {
        let exec = tests::FakeExecutive {
            output_memory: [0xAD; 512],
            fake_error: None,
        };
        let model = ml::Model {
            output_activations_len: 512,
        };
        let mut ml_coord = MlCoordinator::new(&exec);
        assert!(ml_coord.load(&model));

        let res = ml_coord.execute(&model);
        assert_eq!(res.unwrap()[0], 0xAD)
    }
}
