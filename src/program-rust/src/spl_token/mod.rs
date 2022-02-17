pub mod error;
pub mod instruction;

use solana_program::{entrypoint::ProgramResult, program_error::ProgramError, pubkey::Pubkey};
use std::str::FromStr;

solana_program::declare_id!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

/// Checks that the supplied program ID is the correct one for SPL-token
pub fn check_program_account(spl_token_program_id: &Pubkey) -> ProgramResult {
  if spl_token_program_id != &id() {
    return Err(ProgramError::IncorrectProgramId);
  }
  Ok(())
}

lazy_static::lazy_static! {
  pub static ref PROGRAM_ID: Pubkey = Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap();
}

pub const ACCOUNT_LEN: usize = 165;
pub const MINT_LEN: usize = 82;
