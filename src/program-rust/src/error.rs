//! Error types

use num_derive::FromPrimitive;
use solana_program::{decode_error::DecodeError, program_error::ProgramError};
use thiserror::Error;

/// Errors that may be returned by the OneSol program.
#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum OneSolError {
    /// Unknown error.
    #[error("Unknown error")]
    Unknown,
}
impl From<OneSolError> for ProgramError {
    fn from(e: OneSolError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
impl<T> DecodeError<T> for OneSolError {
    fn type_of() -> &'static str {
        "OneSolError"
    }
}
