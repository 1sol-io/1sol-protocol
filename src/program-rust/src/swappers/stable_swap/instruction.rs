//! Instruction types

#![allow(clippy::too_many_arguments)]

use std::mem::size_of;

use solana_program::{
  instruction::{AccountMeta, Instruction},
  program_error::ProgramError,
  pubkey::Pubkey,
};

/// Swap instruction data
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "fuzz", derive(arbitrary::Arbitrary))]
pub struct SwapData {
  /// SOURCE amount to transfer, output to DESTINATION is based on the exchange rate
  pub amount_in: u64,
  /// Minimum amount of DESTINATION token to output, prevents excessive slippage
  pub minimum_amount_out: u64,
}

/// Instructions supported by the SwapInfo program.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "fuzz", derive(arbitrary::Arbitrary))]
pub enum SwapInstruction {
  /// Swap the tokens in the pool.
  ///
  /// 0. `[]`StableSwap
  /// 1. `[]` $authority
  /// 2. `[signer]` User authority.
  /// 3. `[writable]` token_(A|B) SOURCE Account, amount is transferable by $authority,
  /// 4. `[writable]` token_(A|B) Base Account to swap INTO.  Must be the SOURCE token.
  /// 5. `[writable]` token_(A|B) Base Account to swap FROM.  Must be the DESTINATION token.
  /// 6. `[writable]` token_(A|B) DESTINATION Account assigned to USER as the owner.
  /// 7. `[writable]` token_(A|B) admin fee Account. Must have same mint as DESTINATION token.
  /// 8. `[]` Token program id
  Swap(SwapData),
}

impl SwapInstruction {
  /// Packs a [SwapInstruction](enum.SwapInstruction.html) into a byte buffer.
  pub fn pack(&self) -> Vec<u8> {
    let mut buf = Vec::with_capacity(size_of::<Self>());
    match *self {
      Self::Swap(SwapData {
        amount_in,
        minimum_amount_out,
      }) => {
        buf.push(1);
        buf.extend_from_slice(&amount_in.to_le_bytes());
        buf.extend_from_slice(&minimum_amount_out.to_le_bytes());
      }
    }
    buf
  }
}

/// Creates a 'swap' instruction.
#[inline(always)]
pub fn swap(
  program_id: &Pubkey,
  token_program_id: &Pubkey,
  swap_pubkey: &Pubkey,
  swap_authority_key: &Pubkey,
  user_authority_key: &Pubkey,
  source_pubkey: &Pubkey,
  swap_source_pubkey: &Pubkey,
  swap_destination_pubkey: &Pubkey,
  destination_pubkey: &Pubkey,
  admin_fee_destination_pubkey: &Pubkey,
  amount_in: u64,
  minimum_amount_out: u64,
) -> Result<Instruction, ProgramError> {
  let data = SwapInstruction::Swap(SwapData {
    amount_in,
    minimum_amount_out,
  })
  .pack();

  let accounts = vec![
    AccountMeta::new_readonly(*swap_pubkey, false),
    AccountMeta::new_readonly(*swap_authority_key, false),
    AccountMeta::new_readonly(*user_authority_key, true),
    AccountMeta::new(*source_pubkey, false),
    AccountMeta::new(*swap_source_pubkey, false),
    AccountMeta::new(*swap_destination_pubkey, false),
    AccountMeta::new(*destination_pubkey, false),
    AccountMeta::new(*admin_fee_destination_pubkey, false),
    AccountMeta::new_readonly(*token_program_id, false),
  ];

  Ok(Instruction {
    program_id: *program_id,
    accounts,
    data,
  })
}
