//! OnesolProtocol - DEX Aggregator

mod constraints;
pub mod error;
mod exchanger;
pub mod instruction;
mod parser;
pub mod processor;
mod spl_token;
pub mod state;

#[cfg(not(feature = "no-entrypoint"))]
mod entrypoint;

// export
pub use solana_program;
