#![deny(missing_docs)]
#![forbid(unsafe_code)]

//! OneSol - DEX Aggregator

pub mod error;
pub mod instruction;
pub mod instructions;
pub mod processor;

#[cfg(not(feature = "no-entrypoint"))]
mod entrypoint;

// export
pub use solana_program;
