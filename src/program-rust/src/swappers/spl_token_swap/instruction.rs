use solana_program::{
  instruction::{AccountMeta, Instruction},
  program_error::ProgramError,
  pubkey::Pubkey,
};
use std::mem::size_of;

/// Swap instruction data
#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub struct Swap {
  /// SOURCE amount to transfer, output to DESTINATION is based on the exchange rate
  pub amount_in: u64,
  /// Minimum amount of DESTINATION token to output, prevents excessive slippage
  pub minimum_amount_out: u64,
}

/// Instructions supported by the token swap program.
#[repr(C)]
#[derive(Debug, PartialEq)]
pub enum SwapInstruction {
  ///   Swap the tokens in the pool.
  ///
  ///   0. `[]` Token-swap
  ///   1. `[]` swap authority
  ///   2. `[]` user transfer authority
  ///   3. `[writable]` token_(A|B) SOURCE Account, amount is transferable by user transfer authority,
  ///   4. `[writable]` token_(A|B) Base Account to swap INTO.  Must be the SOURCE token.
  ///   5. `[writable]` token_(A|B) Base Account to swap FROM.  Must be the DESTINATION token.
  ///   6. `[writable]` token_(A|B) DESTINATION Account assigned to USER as the owner.
  ///   7. `[writable]` Pool token mint, to generate trading fees
  ///   8. `[writable]` Fee account, to receive trading fees
  ///   9. '[]` Token program id
  ///   10 `[optional, writable]` Host fee account to receive additional trading fees
  Swap(Swap),
}

impl SwapInstruction {
  /// Packs a [SwapInstruction](enum.SwapInstruction.html) into a byte buffer.
  pub fn pack(&self) -> Vec<u8> {
    let mut buf = Vec::with_capacity(size_of::<Self>());
    match &*self {
      Self::Swap(Swap {
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
#[allow(clippy::too_many_arguments)]
pub fn swap(
  program_id: &Pubkey,
  token_program_id: &Pubkey,
  swap_pubkey: &Pubkey,
  authority_pubkey: &Pubkey,
  user_transfer_authority_pubkey: &Pubkey,
  source_pubkey: &Pubkey,
  swap_source_pubkey: &Pubkey,
  swap_destination_pubkey: &Pubkey,
  destination_pubkey: &Pubkey,
  pool_mint_pubkey: &Pubkey,
  pool_fee_pubkey: &Pubkey,
  host_fee_pubkey: Option<&Pubkey>,
  instruction: Swap,
) -> Result<Instruction, ProgramError> {
  let data = SwapInstruction::Swap(instruction).pack();

  let mut accounts = vec![
    AccountMeta::new_readonly(*swap_pubkey, false),
    AccountMeta::new_readonly(*authority_pubkey, false),
    AccountMeta::new_readonly(*user_transfer_authority_pubkey, true),
    AccountMeta::new(*source_pubkey, false),
    AccountMeta::new(*swap_source_pubkey, false),
    AccountMeta::new(*swap_destination_pubkey, false),
    AccountMeta::new(*destination_pubkey, false),
    AccountMeta::new(*pool_mint_pubkey, false),
    AccountMeta::new(*pool_fee_pubkey, false),
    AccountMeta::new_readonly(*token_program_id, false),
  ];
  if let Some(host_fee_pubkey) = host_fee_pubkey {
    accounts.push(AccountMeta::new(*host_fee_pubkey, false));
  }

  Ok(Instruction {
    program_id: *program_id,
    accounts,
    data,
  })
}
