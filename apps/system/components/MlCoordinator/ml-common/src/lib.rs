// TODO(jesionowski): What are the actual errors we may encounter?
#[derive(Copy, Clone, Debug)]
pub enum ExecutionError {
    ModelNotLoaded,
    InvalidInstruction,
    InvalidFetch,
    CoreReset,
}

// The abstraction layer over the "hardware" of running an execution.
// Returns a slice of bytes, which is the output data, or an ExecutionError.
pub trait ExecutiveInterface {
    fn run_model(&self, model: &Model) -> Result<&[u8], ExecutionError>;
}

// Options for execution that may be set by the application.
pub struct ModelOptions {
    pub rate: u32,
}

// Immutable model attributes.
#[derive(PartialEq, Debug)]
pub struct Model {
    pub output_activations_len: usize,
}

pub const MAX_MODELS: u32 = 10;
