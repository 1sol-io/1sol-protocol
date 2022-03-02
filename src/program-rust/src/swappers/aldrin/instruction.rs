use std::mem::size_of;

use num_enum::{IntoPrimitive, TryFromPrimitive};
use solana_program::{
  instruction::{AccountMeta, Instruction},
  program_error::ProgramError,
  pubkey::Pubkey,
};

#[derive(Eq, PartialEq, Copy, Clone, TryFromPrimitive, IntoPrimitive, Debug)]
#[repr(u8)]
pub enum Side {
  Bid = 0,
  Ask = 1,
}

#[derive(Clone, Debug, PartialEq)]
struct Swap {
  /// SOURCE amount to transfer, output to DESTINATION is based on the exchange rate
  pub amount_in: u64,
  /// Minimum amount of DESTINATION token to output, prevents excessive slippage
  pub minimum_amount_out: u64,
  pub side: Side,
}

#[derive(Debug, PartialEq)]
enum SwapInstrution {
  Swap(Swap),
}

impl SwapInstrution {
  pub fn pack(&self) -> Vec<u8> {
    let mut buf = Vec::with_capacity(size_of::<Self>());
    match &*self {
      Self::Swap(Swap {
        amount_in,
        minimum_amount_out,
        side,
      }) => {
        buf.extend_from_slice(&[248, 198, 158, 145, 225, 117, 135, 200]);
        buf.extend_from_slice(&amount_in.to_le_bytes());
        buf.extend_from_slice(&minimum_amount_out.to_le_bytes());
        buf.push(Side::into(*side));
      }
    };
    buf
  }
}

#[allow(clippy::too_many_arguments)]
pub fn swap_instruction(
  program_id: &Pubkey,
  pool_key: &Pubkey,
  pool_signer: &Pubkey,
  pool_mint: &Pubkey,
  pool_coin_token_vault: &Pubkey,
  pool_pc_token_vault: &Pubkey,
  pool_fee_account: &Pubkey,
  pool_curve_key: &Pubkey,
  user_coin_token_account: &Pubkey,
  user_pc_token_account: &Pubkey,
  user_authority: &Pubkey,
  token_program_id: &Pubkey,
  amount_in: u64,
  minimum_amount_out: u64,
  side: Side,
) -> Result<Instruction, ProgramError> {
  let data = SwapInstrution::Swap(Swap {
    amount_in,
    minimum_amount_out,
    side,
  })
  .pack();

  let accounts = vec![
    AccountMeta::new_readonly(*pool_key, false),
    AccountMeta::new_readonly(*pool_signer, false),
    AccountMeta::new(*pool_mint, false),
    AccountMeta::new(*pool_coin_token_vault, false),
    AccountMeta::new(*pool_pc_token_vault, false),
    AccountMeta::new(*pool_fee_account, false),
    AccountMeta::new_readonly(*user_authority, true),
    AccountMeta::new(*user_coin_token_account, false),
    AccountMeta::new(*user_pc_token_account, false),
    AccountMeta::new_readonly(*pool_curve_key, false),
    AccountMeta::new_readonly(*token_program_id, false),
  ];

  Ok(Instruction {
    program_id: *program_id,
    accounts,
    data,
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  pub fn test_pack_swap_instruction() {
    let data = SwapInstrution::Swap(Swap {
      amount_in: 100,
      minimum_amount_out: 99,
      side: Side::Bid,
    })
    .pack();
    assert!(
      data
        == vec![
          248, 198, 158, 145, 225, 117, 135, 200, 100, 0, 0, 0, 0, 0, 0, 0, 99, 0, 0, 0, 0, 0, 0,
          0, 0
        ]
    );
    let data = SwapInstrution::Swap(Swap {
      amount_in: 100,
      minimum_amount_out: 99,
      side: Side::Ask,
    })
    .pack();
    assert!(
      data
        == vec![
          248, 198, 158, 145, 225, 117, 135, 200, 100, 0, 0, 0, 0, 0, 0, 0, 99, 0, 0, 0, 0, 0, 0,
          0, 1
        ]
    );
    let data = SwapInstrution::Swap(Swap {
      amount_in: 101,
      minimum_amount_out: 99,
      side: Side::Ask,
    })
    .pack();
    assert!(
      data
        != vec![
          248, 198, 158, 145, 225, 117, 135, 200, 100, 0, 0, 0, 0, 0, 0, 0, 99, 0, 0, 0, 0, 0, 0,
          0, 1
        ]
    );
  }
}
