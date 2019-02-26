/// Error for Sodium lib to use in your code.
use holochain_core_types::error::HolochainError;

#[derive(Debug)]
pub enum SodiumError {
    Generic(String),
    OutputLength(String),
    OutOfMemory,
}

impl SodiumError {
    pub fn new(msg: &str) -> SodiumError {
        SodiumError::Generic(msg.to_string())
    }
}

impl From<SodiumError> for HolochainError {
    fn from(error: SodiumError) -> Self {
        match error {
            SodiumError::Generic(s) => HolochainError::new(&s),
            SodiumError::OutputLength(s) => HolochainError::new(&s),
            SodiumError::OutOfMemory => HolochainError::new("libSodium went out of memory"),
        }
    }
}
