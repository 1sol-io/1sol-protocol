//! Instruction types

#![allow(clippy::too_many_arguments)]
use solana_program::{
  instruction::{AccountMeta, Instruction},
  pubkey::Pubkey,
};

/// Swap instruction data
/// [
///     {
///         "name": "amount",
///         "type": "u64"
///     },
///     {
///         "name": "otherAmountThreshold",
///         "type": "u64"
///     },
///     {
///         "name": "sqrtPriceLimit",
///         "type": "u128"
///     },
///     {
///         "name": "exactInput",
///         "type": "bool"
///     },
///     {
///         "name": "aToB",
///         "type": "bool"
///     }
/// ]
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Swap {
  pub amount: u64,
  pub other_amount_threshold: u64,
  pub sqrt_price_limit: u128,
  pub exact_input: bool,
  pub a_to_b: bool,
}

/// Instructions supported by the SwapInfo program.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum SwapInstruction {
  Swap(Swap),
}

impl SwapInstruction {
  pub fn pack(&self) -> Vec<u8> {
    let mut buf = Vec::new();
    match self {
      SwapInstruction::Swap(Swap {
        amount,
        other_amount_threshold,
        sqrt_price_limit,
        exact_input,
        a_to_b,
      }) => {
        // swap
        buf.extend_from_slice(&[248, 198, 158, 145, 225, 117, 135, 200]);
        buf.extend_from_slice(&amount.to_le_bytes());
        buf.extend_from_slice(&other_amount_threshold.to_le_bytes());
        buf.extend_from_slice(&sqrt_price_limit.to_le_bytes());
        buf.push(if *exact_input { 1 } else { 0 });
        buf.push(if *a_to_b { 1 } else { 0 });
      }
    }
    buf
  }
}

///! {
///!   "name": "swap",
///!   "accounts": [
///!     {
///!         "name": "tokenProgram",
///!         "isMut": false,
///!         "isSigner": false
///!     },
///!     {
///!         "name": "tokenAuthority",
///!         "isMut": false,
///!         "isSigner": true
///!     },
///!     {
///!         "name": "whirlpool",
///!         "isMut": true,
///!         "isSigner": false
///!     },
///!     {
///!         "name": "tokenOwnerAccountA",
///!         "isMut": true,
///!         "isSigner": false
///!     },
///!     {
///!         "name": "tokenVaultA",
///!         "isMut": true,
///!         "isSigner": false
///!     },
///!     {
///!         "name": "tokenOwnerAccountB",
///!         "isMut": true,
///!         "isSigner": false
///!     },
///!     {
///!         "name": "tokenVaultB",
///!         "isMut": true,
///!         "isSigner": false
///!     },
///!     {
///!         "name": "tickArray0",
///!         "isMut": true,
///!         "isSigner": false
///!     },
///!     {
///!         "name": "tickArray1",
///!         "isMut": true,
///!         "isSigner": false
///!     },
///!     {
///!         "name": "tickArray2",
///!         "isMut": true,
///!         "isSigner": false
///!     },
///!     {
///!         "name": "oracle",
///!         "isMut": false,
///!         "isSigner": false
///!     }
///!   ],
///!   "args": [
///!       {
///!           "name": "amount",
///!           "type": "u64"
///!       },
///!       {
///!           "name": "otherAmountThreshold",
///!           "type": "u64"
///!       },
///!       {
///!           "name": "sqrtPriceLimit",
///!           "type": "u128"
///!       },
///!       {
///!           "name": "exactInput",
///!           "type": "bool"
///!       },
///!       {
///!           "name": "aToB",
///!           "type": "bool"
///!       }
///!   ]
///! },
///! A demo: https://solscan.io/tx/hFJW8skZkzdr8jeBLk2xxGbmtNAsYL2ktyApM3X31zZFjwdPRwHdGY1MKBaxEppoC3EUeDvVzmkR46GaV5iJfVT
#[allow(dead_code)]
pub fn swap(
  program_id: &Pubkey,
  token_program_id: &Pubkey,
  user_authority_key: &Pubkey,
  whirlpool: &Pubkey,
  token_owner_account_a: &Pubkey,
  token_vault_a: &Pubkey,
  token_owner_account_b: &Pubkey,
  token_vault_b: &Pubkey,
  tick_array_0: &Pubkey,
  tick_array_1: &Pubkey,
  tick_array_2: &Pubkey,
  oracle: &Pubkey,
  amount: u64,
  other_amount_threshold: u64,
  sqrt_price_limit: u128,
  exact_input: bool,
  a_to_b: bool,
) -> Instruction {
  let data = SwapInstruction::Swap(Swap {
    amount,
    other_amount_threshold,
    sqrt_price_limit,
    exact_input,
    a_to_b,
  })
  .pack();

  let accounts = vec![
    AccountMeta::new_readonly(*token_program_id, false),
    AccountMeta::new_readonly(*user_authority_key, true),
    AccountMeta::new(*whirlpool, false),
    AccountMeta::new(*token_owner_account_a, false),
    AccountMeta::new(*token_vault_a, false),
    AccountMeta::new(*token_owner_account_b, false),
    AccountMeta::new(*token_vault_b, false),
    AccountMeta::new(*tick_array_0, false),
    AccountMeta::new(*tick_array_1, false),
    AccountMeta::new(*tick_array_2, false),
    AccountMeta::new_readonly(*oracle, false),
  ];
  Instruction {
    program_id: *program_id,
    accounts,
    data,
  }
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn test_pack_swap_instruction() {
    let a = "59p8WydnSZtRpoLy1jH2AXMMgQqWZivjtLP24DhngRF8TmzpmKzziurcyV";
    let data1 = bs58::decode(a).into_vec().unwrap();
    let b = [248, 198, 158, 145, 225, 117, 135, 200];
    let (a1, _a2) = data1.split_at(8);
    assert!(b.eq(a1), "instruction not same");
    let data2 = SwapInstruction::Swap(Swap {
      amount: 2000000000,
      other_amount_threshold: 0,
      sqrt_price_limit: 79226673515401279992447579055,
      exact_input: true,
      a_to_b: false,
    })
    .pack();
    assert_eq!(data1, data2)
  }
}
