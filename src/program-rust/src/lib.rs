//! OnesolProtocol - DEX Aggregator

mod account_parser;
mod constraints;
pub mod error;
pub mod instruction;
pub mod processor;
mod spl_token;
pub mod state;
mod swappers;

#[cfg(not(feature = "no-entrypoint"))]
mod entrypoint;

// export
pub use solana_program;
