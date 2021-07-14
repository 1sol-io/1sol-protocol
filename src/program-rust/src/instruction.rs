//! Instruction types

use crate::{error::OneSolError, swappers::serum_dex_order};
use arrayref::{array_ref, array_refs};
use solana_program::program_error::ProgramError;
use std::num::NonZeroU64;

/// Initialize instruction data
#[derive(Clone, Debug, PartialEq)]
pub struct Initialize {
  /// nonce used to create validate program address
  pub nonce: u8,
}

/// Swap instruction data
#[derive(Clone, Debug, PartialEq)]
pub struct SwapTokenSwap {
  /// amount of tokens to swap
  pub amount_in: NonZeroU64,
  /// Minimum amount of DESTINATION token to output, prevents excessive slippage
  pub minimum_amount_out: NonZeroU64,
}

/// Swap instruction data
#[derive(Clone, Debug, PartialEq, Copy)]
pub struct SwapSerumDex {
  /// the amount to swap *from*
  pub amount_in: u64,
  /// the direct to swap,
  ///   when side is "bid" the swaps B to A.
  ///   when side is "ask" the swaps A to B
  pub side: serum_dex::matching::Side,
  /// the exchange range to use when determining whether the transaction should abort
  pub min_exchange_rate: serum_dex_order::ExchangeRate,
}

/// Instructions supported by the 1sol protocol program
#[repr(C)]
#[derive(Debug, PartialEq)]
pub enum OneSolInstruction {
  /// Initializes a new 1solProtocol
  /// 0. `[writable, signer]` New 1solProtocol to create.
  /// 1. `[]` swap authority derived from `create_program_address(&[Token-swap account])`
  /// 2. `[]` token Account. Must be non zero, owned by 1sol.
  /// 3. '[]` Token program id
  Initialize(Initialize),

  /// Swap the tokens in the pool.
  ///
  ///   0. `[]` OneSol Protocol account
  ///   1. `[]` OneSol Protocol authority
  ///   2. `[writeable]` OneSol Protocol token account
  ///   3. `[writable]` token_A SOURCE Account,
  ///   4. `[writable]` token_B DESTINATION Account to swap INTO. Must be the DESTINATION token.
  ///   5. '[]` Token program id
  ///
  ///   6. `[signer]` user_transfer_authority account
  ///   7. `[]` token-swap account
  ///   8. `[]` token-swap authority
  ///   9. `[writable]` token_A Base Account to swap FROM.  Must be the SOURCE token.
  ///   10. `[writable]` token_B Base Account to swap INTO.  Must be the DESTINATION token.
  ///   11. `[writable]` Pool token mint, to generate trading fees
  ///   12. `[writable]` Fee account, to receive trading fees
  ///   13. '[]` Token-Swap program id
  ///   14. `[optional, writable]` Host fee account to receive additional trading fees
  SwapTokenSwap(SwapTokenSwap),

  /// Swap the tokens in the serum dex market.
  ///
  ///   0. `[]` OneSol Protocol account
  ///   1. `[]` OneSol Protocol authority
  ///   2. `[writeable]` OneSol Protocol token account
  ///   3. `[writable]` token_(A|B) SOURCE Account, (coin_wallet)
  ///   4. `[writable]` token_(A|B) DESTINATION Account to swap INTO. Must be the DESTINATION token.
  ///   5. '[]` Token program id
  ///
  ///   6. `[writable]`  serum-dex market
  ///   7. `[writable]`  serum-dex request_queue
  ///   8. `[writable]`  serum-dex event_queue
  ///   9. `[writable]`  serum-dex market_bids
  ///   10. `[writable]`  serum-dex market_asks
  ///   11. `[writable]`  serum-dex coin_vault
  ///   12. `[writable]`  serum-dex pc_vault
  ///   13. `[writable]`  serum-dex vault_signer for settleFunds
  ///   14. `[writable]`  serum-dex open_orders
  ///   15. `[signer]`  serum-dex open_orders_owner
  ///   16. `[]`  serum-dex rent_sysvar
  ///   17. `[]`  serum-dex serum_dex_program_id
  SwapSerumDex(SwapSerumDex),
}

impl OneSolInstruction {
  /// Unpacks a byte buffer into a [OneSolInstruction](enum.OneSolInstruction.html).
  pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
    let (&tag, rest) = input.split_first().ok_or(OneSolError::InvalidInput)?;
    Ok(match tag {
      0 => Self::Initialize(Initialize::unpack(rest)?),
      1 => Self::SwapTokenSwap(SwapTokenSwap::unpack(rest)?),
      2 => Self::SwapSerumDex(SwapSerumDex::unpack(rest)?),
      _ => return Err(OneSolError::InvalidInstruction.into()),
    })
  }
}

impl Initialize {
  fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
    if input.len() < 1 {
      return Err(OneSolError::InvalidInput.into());
    }
    let (&nonce, _rest) = input.split_first().ok_or(OneSolError::InvalidInput)?;
    Ok(Initialize { nonce })
  }
}

impl SwapTokenSwap {
  // size = 1 or 3
  // flag[0/1], [account_size], [amount_in], [minium_amount_out]
  fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
    const DATA_LEN: usize = 16;
    if input.len() < DATA_LEN {
      return Err(OneSolError::InvalidInput.into());
    }
    let arr_data = array_ref![input, 0, DATA_LEN];
    let (&amount_in_arr, &minimum_amount_out_arr) = array_refs![arr_data, 8, 8];
    let amount_in =
      NonZeroU64::new(u64::from_le_bytes(amount_in_arr)).ok_or(OneSolError::InvalidInput)?;
    let minimum_amount_out = NonZeroU64::new(u64::from_le_bytes(minimum_amount_out_arr))
      .ok_or(OneSolError::InvalidInput)?;
    Ok(SwapTokenSwap {
      amount_in,
      minimum_amount_out,
    })
  }
}

impl SwapSerumDex {
  fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
    const DATA_LEN: usize = 20;
    if input.len() < DATA_LEN {
      return Err(OneSolError::InvalidInput.into());
    }
    let arr_data = array_ref![input, 0, DATA_LEN];
    let (&amount_in_arr, &[side], &rate_arr, &[from_decimals], &[quote_decimals], &[strict]) =
      array_refs![arr_data, 8, 1, 8, 1, 1, 1];
    let amount_in = u64::from_le_bytes(amount_in_arr);
    let rate = u64::from_le_bytes(rate_arr);
    Ok(SwapSerumDex {
      amount_in: amount_in,
      side: if side == 0 {
        serum_dex::matching::Side::Bid
      } else {
        serum_dex::matching::Side::Ask
      },
      min_exchange_rate: serum_dex_order::ExchangeRate {
        rate: rate,
        from_decimals: from_decimals,
        quote_decimals: quote_decimals,
        strict: if strict == 0 { false } else { true },
      },
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use arrayref::mut_array_refs;
  use bytemuck::bytes_of;

  #[test]
  fn test_unpack_initialize() {
    let nonce = 101;
    let input = bytes_of(&nonce);
    let i = Initialize::unpack(input).unwrap();
    assert_eq!(i.nonce, nonce)
  }

  #[test]
  fn test_unpack_swap_token_swap() {
    let amount_in = 120000u64;
    let minimum_amount_out = 1080222u64;
    let mut input = [0u8; 16];
    let (amount_in_arr, minimum_amount_out_arr) = mut_array_refs![&mut input, 8, 8];
    amount_in_arr.copy_from_slice(&bytes_of(&amount_in));
    minimum_amount_out_arr.copy_from_slice(&bytes_of(&minimum_amount_out));

    let i = SwapTokenSwap::unpack(&input).unwrap();
    assert_eq!(i.amount_in.get(), amount_in);
    assert_eq!(i.minimum_amount_out.get(), minimum_amount_out);
  }

  #[test]
  fn test_unpack_swap_serum_dex() {
    let amount_in = 120000u64;
    let side = 1u8;
    let rate = 120u64;
    let from_decimals = 6u8;
    let quote_decimals = 9u8;
    let strict = 1u8;

    let mut buf = Vec::with_capacity(20);
    buf.extend_from_slice(&amount_in.to_le_bytes());
    buf.push(side);
    buf.extend_from_slice(&rate.to_le_bytes());
    buf.push(from_decimals);
    buf.push(quote_decimals);
    buf.push(strict);
    let i = SwapSerumDex::unpack(&buf[..]).unwrap();
    assert_eq!(i.amount_in, amount_in);
    assert_eq!(i.side, serum_dex::matching::Side::Ask);
    assert_eq!(i.min_exchange_rate.rate, rate);
    assert_eq!(i.min_exchange_rate.from_decimals, from_decimals);
    assert_eq!(i.min_exchange_rate.quote_decimals, quote_decimals);
    assert_eq!(i.min_exchange_rate.strict, strict == 1);
  }
}
