//! OnesolProtocol - DEX Aggregator

mod constraints;
pub mod error;
pub mod instruction;
mod parser;
pub mod processor;
mod spl_token;
pub mod state;
mod swappers;

#[cfg(not(feature = "no-entrypoint"))]
mod entrypoint;

// export
pub use solana_program;
