//! Instruction types

use solana_program::{
  instruction::{AccountMeta, Instruction},
  program_error::ProgramError,
  pubkey::Pubkey,
};
use std::mem::size_of;

use super::check_program_account;

/// Instructions supported by the token program.
#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub enum TokenInstruction {
  /// Transfers tokens from one account to another either directly or via a
  /// delegate.  If this account is associated with the native mint then equal
  /// amounts of SOL and Tokens will be transferred to the destination
  /// account.
  ///
  /// Accounts expected by this instruction:
  ///
  ///   * Single owner/delegate
  ///   0. `[writable]` The source account.
  ///   1. `[writable]` The destination account.
  ///   2. `[signer]` The source account's owner/delegate.
  ///
  ///   * Multisignature owner/delegate
  ///   0. `[writable]` The source account.
  ///   1. `[writable]` The destination account.
  ///   2. `[]` The source account's multisignature owner/delegate.
  ///   3. ..3+M `[signer]` M signer accounts.
  Transfer {
    /// The amount of tokens to transfer.
    amount: u64,
  },

  /// Close an account by transferring all its SOL to the destination account.
  /// Non-native accounts may only be closed if its token amount is zero.
  ///
  /// Accounts expected by this instruction:
  ///
  ///   * Single owner
  ///   0. `[writable]` The account to close.
  ///   1. `[writable]` The destination account.
  ///   2. `[signer]` The account's owner.
  ///
  ///   * Multisignature owner
  ///   0. `[writable]` The account to close.
  ///   1. `[writable]` The destination account.
  ///   2. `[]` The account's multisignature owner.
  ///   3. ..3+M `[signer]` M signer accounts.
  CloseAccount,
}

impl TokenInstruction {
  /// Packs a [TokenInstruction](enum.TokenInstruction.html) into a byte buffer.
  pub fn pack(&self) -> Vec<u8> {
    let mut buf = Vec::with_capacity(size_of::<Self>());
    match self {
      &Self::Transfer { amount } => {
        buf.push(3);
        buf.extend_from_slice(&amount.to_le_bytes());
      }
      Self::CloseAccount => buf.push(9),
    };
    buf
  }
}

/// Creates a `Transfer` instruction.
pub fn transfer(
  token_program_id: &Pubkey,
  source_pubkey: &Pubkey,
  destination_pubkey: &Pubkey,
  authority_pubkey: &Pubkey,
  signer_pubkeys: &[&Pubkey],
  amount: u64,
) -> Result<Instruction, ProgramError> {
  check_program_account(token_program_id)?;
  let data = TokenInstruction::Transfer { amount }.pack();

  let mut accounts = Vec::with_capacity(3 + signer_pubkeys.len());
  accounts.push(AccountMeta::new(*source_pubkey, false));
  accounts.push(AccountMeta::new(*destination_pubkey, false));
  accounts.push(AccountMeta::new_readonly(
    *authority_pubkey,
    signer_pubkeys.is_empty(),
  ));
  for signer_pubkey in signer_pubkeys.iter() {
    accounts.push(AccountMeta::new_readonly(**signer_pubkey, true));
  }

  Ok(Instruction {
    program_id: *token_program_id,
    accounts,
    data,
  })
}

#[allow(dead_code)]
/// Creates a `CloseAccount` instruction.
pub fn close_account(
  token_program_id: &Pubkey,
  account_pubkey: &Pubkey,
  destination_pubkey: &Pubkey,
  owner_pubkey: &Pubkey,
  signer_pubkeys: &[&Pubkey],
) -> Result<Instruction, ProgramError> {
  check_program_account(token_program_id)?;
  let data = TokenInstruction::CloseAccount.pack();

  let mut accounts = Vec::with_capacity(3 + signer_pubkeys.len());
  accounts.push(AccountMeta::new(*account_pubkey, false));
  accounts.push(AccountMeta::new(*destination_pubkey, false));
  accounts.push(AccountMeta::new_readonly(
    *owner_pubkey,
    signer_pubkeys.is_empty(),
  ));
  for signer_pubkey in signer_pubkeys.iter() {
    accounts.push(AccountMeta::new_readonly(**signer_pubkey, true));
  }

  Ok(Instruction {
    program_id: *token_program_id,
    accounts,
    data,
  })
}
