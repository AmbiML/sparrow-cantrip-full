// TODO(jesionowski): What are the actual errors we may encounter?
#[derive(Copy, Clone, Debug)]
pub enum ExecutionError {
    InvalidInstruction,
    InvalidFetch,
    CoreReset
}

// The abstraction layer over the "hardware" of running an execution.
// Returns a slice of bytes, which is the output data, or an ExecutionError.
pub trait ExecutiveInterface {
    fn run_model(&self, model: &Model) -> Result<&[u8], ExecutionError>;
}

pub struct Model {
    pub output_activations_len: usize,
}