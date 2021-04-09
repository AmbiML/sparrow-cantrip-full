use ml_common as ml;
use ml_common::ExecutiveInterface;

pub struct FakeExecutive {
    output_memory: [u8; 512],
    fake_error: Option<ml::ExecutionError>,
}

impl ml::ExecutiveInterface for FakeExecutive {
    fn run_model(&self, _model: &ml::Model) -> Result<&[u8], ml::ExecutionError> {
        match self.fake_error {
            Some(err) => Err(err),
            None => Ok(&self.output_memory),
        }
    }
}

#[test]
fn return_ok() {
    let exec = FakeExecutive {
        output_memory: [0xAD; 512],
        fake_error: None,
    };
    let model = ml::Model {
        output_activations_len: 512,
    };

    let res = exec.run_model(&model);
    assert!(res.is_ok());
    assert_eq!(res.unwrap()[0], 0xAD)
}

#[test]
fn return_err() {
    let exec = FakeExecutive {
        output_memory: [0; 512],
        fake_error: Some(ml::ExecutionError::CoreReset),
    };
    let model = ml::Model {
        output_activations_len: 512,
    };

    assert!(exec.run_model(&model).is_err());
}
