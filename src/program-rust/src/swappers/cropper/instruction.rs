//! Instruction types

#![allow(clippy::too_many_arguments)]

use solana_program::{
  instruction::{AccountMeta, Instruction},
  program_error::ProgramError,
  pubkey::Pubkey,
};
use std::mem::size_of;

/// Swap instruction data
#[cfg_attr(feature = "fuzz", derive(Arbitrary))]
#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub struct SwapInstruction {
  /// SOURCE amount to transfer, output to DESTINATION is based on the exchange rate
  pub amount_in: u64,
  /// Minimum amount of DESTINATION token to output, prevents excessive slippage
  pub minimum_amount_out: u64,
}

/// Instructions supported by the token swap program.
#[repr(C)]
#[derive(Debug, PartialEq)]
pub enum AmmInstruction {
  Swap(SwapInstruction),
}

impl AmmInstruction {
  /// Packs a [AmmInstruction](enum.AmmInstruction.html) into a byte buffer.
  pub fn pack(&self) -> Vec<u8> {
    let mut buf = Vec::with_capacity(size_of::<Self>());
    match &*self {
      Self::Swap(SwapInstruction {
        amount_in,
        minimum_amount_out,
      }) => {
        buf.push(1);
        buf.extend_from_slice(&amount_in.to_le_bytes());
        buf.extend_from_slice(&minimum_amount_out.to_le_bytes());
      }
    };
    buf
  }
}

/// Creates a 'swap' instruction.
pub fn swap_instruction(
  program_id: &Pubkey,
  token_program_id: &Pubkey,
  swap_pubkey: &Pubkey,
  authority_pubkey: &Pubkey,
  user_transfer_authority_pubkey: &Pubkey,
  state_pubkey: &Pubkey,
  source_pubkey: &Pubkey,
  swap_source_pubkey: &Pubkey,
  swap_destination_pubkey: &Pubkey,
  destination_pubkey: &Pubkey,
  pool_mint_pubkey: &Pubkey,
  fee_account_pubkey: &Pubkey,
  amount_in: u64,
  minimum_amount_out: u64,
) -> Result<Instruction, ProgramError> {
  let data = AmmInstruction::Swap(SwapInstruction {
    amount_in,
    minimum_amount_out,
  })
  .pack();

  let accounts = vec![
    AccountMeta::new_readonly(*swap_pubkey, false),
    AccountMeta::new_readonly(*authority_pubkey, false),
    AccountMeta::new_readonly(*user_transfer_authority_pubkey, true),
    AccountMeta::new_readonly(*state_pubkey, false),
    AccountMeta::new(*source_pubkey, false),
    AccountMeta::new(*swap_source_pubkey, false),
    AccountMeta::new(*swap_destination_pubkey, false),
    AccountMeta::new(*destination_pubkey, false),
    AccountMeta::new(*pool_mint_pubkey, false),
    AccountMeta::new(*fee_account_pubkey, false),
    AccountMeta::new_readonly(*token_program_id, false),
  ];

  Ok(Instruction {
    program_id: *program_id,
    accounts,
    data,
  })
}
