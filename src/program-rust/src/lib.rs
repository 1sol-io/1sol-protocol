#![deny(missing_docs)]
#![forbid(unsafe_code)]

//! OneSol - DEX Aggregator

pub mod error;
pub mod instruction;
pub mod processor;
pub mod state;
mod swappers;
mod util;

#[cfg(not(feature = "no-entrypoint"))]
mod entrypoint;

// export
pub use solana_program;
