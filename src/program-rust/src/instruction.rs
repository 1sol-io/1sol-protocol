//! Instruction types

use crate::error::ProtocolError;
use arrayref::{array_ref, array_refs};
use solana_program::program_error::ProgramError;
use std::num::NonZeroU64;

/// ExchangerType
#[derive(Clone, Debug, PartialEq, Copy)]
pub enum ExchangerType {
  /// ExchangerType SplTokenSwap
  SplTokenSwap,
  /// ExchangerType SerumDex
  SerumDex,
  /// Saber StableSwap
  StableSwap,
  /// Raydium swap
  RaydiumSwap,
}

impl ExchangerType {
  pub fn from(value: u8) -> Option<Self> {
    match value {
      0 => Some(ExchangerType::SplTokenSwap),
      1 => Some(ExchangerType::SerumDex),
      2 => Some(ExchangerType::StableSwap),
      3 => Some(ExchangerType::RaydiumSwap),
      _ => None,
    }
  }
}

// #[derive(Clone, Debug, PartialEq, Copy)]
// pub struct ExchangeStep {
//   pub exchanger_type: ExchangerType,
//   pub accounts_count: usize,
// }

// impl ExchangeStep {
//   pub const LEN: usize = 2;

//   pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
//     let arr_data = array_ref![input, 0, ExchangeStep::LEN];
//     let (&[exchanger_type], &[accounts_count]) = array_refs![arr_data, 1, 1];
//     Ok(Self {
//       exchanger_type: ExchangerType::from(exchanger_type)
//         .ok_or(ProgramError::InvalidInstructionData)?,
//       accounts_count: accounts_count as usize,
//     })
//   }
// }

/// Initialize instruction data
#[derive(Clone, Debug, PartialEq)]
pub struct Initialize {
  /// nonce used to create validate program address
  pub nonce: u8,
}

/// Swap instruction data
#[derive(Clone, Debug, PartialEq)]
pub struct SwapInstruction {
  /// amount of tokens to swap
  pub amount_in: NonZeroU64,
  /// expect amount of tokens to swap
  pub expect_amount_out: NonZeroU64,
  /// Minimum amount of DESTINATION token to output, prevents excessive slippage
  pub minimum_amount_out: NonZeroU64,
}

/// Swap instruction data
#[derive(Clone, Debug, PartialEq)]
pub struct SwapInInstruction {
  /// amount of tokens to swap
  pub amount_in: NonZeroU64,
}

/// Swap instruction data
#[derive(Clone, Debug, PartialEq)]
pub struct SwapOutInstruction {
  /// expect amount of tokens to swap
  pub expect_amount_out: NonZeroU64,
  /// Minimum amount of DESTINATION token to output, prevents excessive slippage
  pub minimum_amount_out: NonZeroU64,
}

// /// Swap from multiple exchanger
// #[derive(Clone, Debug, PartialEq, Copy)]
// pub struct SwapTwoStepsInstruction {
//   /// the amount to swap *from*
//   pub amount_in: NonZeroU64,
//   /// expect amount of tokens to swap
//   pub expect_amount_out: NonZeroU64,
//   /// Minimum amount of DESTINATION token to output, prevents excessive slippage
//   pub minimum_amount_out: NonZeroU64,
//   /// Step1
//   pub step1: ExchangeStep,
//   /// Step1
//   pub step2: ExchangeStep,
// }

// Instructions supported by the 1sol protocol program
#[repr(C)]
#[derive(Debug, PartialEq)]
pub enum OneSolInstruction {
  /// Create Dex Market
  ///
  /// 0. `[writable, signer]` new DexMarketInfo account to create.
  /// 1. `[]` $authority `DexMarketInfo's authority`
  /// 2. `[writable]` market account. SerumDexMarket account.
  /// 3. `[writable]` open_orders account. SerumDexOpenOrders account.
  /// 4. `[]` the rend sysvar.
  /// 5. `[]` SerumDex ProgramId.
  InitDexMarketOpenOrders(Initialize),

  /// Update DexMarket OpenOrders
  ///
  /// 0. `[writable]` DexMarketInfo account to update.
  /// 1. `[]` $authority `AmmInfo's authority`
  /// 2. `[writable]` market account. SerumDexMarket account.
  /// 3. `[writable]` open_orders account. SerumDexOpenOrders account.
  /// 4. `[]` the rend sysvar.
  /// 5. `[]` SerumDex ProgramId.
  UpdateDexMarketOpenOrders,

  /// Swap the tokens in the pool.
  ///
  ///   user accounts
  ///   0. `[writable]` User token SOURCE Account, (coin_wallet)
  ///   1. `[writable]` User token DESTINATION Account to swap INTO. Must be the DESTINATION token.
  ///   2. `[signer]` User token SOURCE account OWNER (or Authority) account.
  ///   3. '[]` Token program id
  ///   4. `[writable]` fee token account
  ///   5. `[]` TokenSwap swap_info account
  ///   6. `[]` TokenSwap swap_info authority
  ///   7. `[writable]` TokenSwap token_A Account.
  ///   8. `[writable]` TokenSwap token_B Account.
  ///   9. `[writable]` TokenSwap Pool token mint, to generate trading fees
  ///   10. `[writable]` TokenSwap Fee account, to receive trading fees
  ///   11. '[]` Token-Swap program id
  ///   12. `[optional, writable]` Host fee account to receive additional trading fees
  SwapSplTokenSwap(SwapInstruction),

  /// Swap the tokens in the serum dex market.
  ///
  ///     0. `[writable]` User token SOURCE Account, (coin_wallet)
  ///     1. `[writable]` User token DESTINATION Account to swap INTO. Must be the DESTINATION token.
  ///     2. `[signer]` User token SOURCE account OWNER (or Authority) account.
  ///     3. '[]` Token program id
  ///     4. `[writable]` fee token account
  ///     5. `[writable]` dex-market-info account
  ///     6. `[]` dex-market-info authority
  ///     7. `[writable]`  dex-market-info open_orders
  ///     8. `[writable]`  serum-dex market
  ///     9. `[writable]`  serum-dex request_queue
  ///     10. `[writable]`  serum-dex event_queue
  ///     11. `[writable]`  serum-dex market_bids
  ///     12. `[writable]`  serum-dex market_asks
  ///     13. `[writable]`  serum-dex coin_vault
  ///     14. `[writable]`  serum-dex pc_vault
  ///     15. `[]`  serum-dex vault_signer for settleFunds
  ///     16. `[]`  serum-dex rent_sysvar
  ///     17. `[]`  serum-dex serum_dex_program_id
  SwapSerumDex(SwapInstruction),

  /// Swap tokens through Saber StableSwap
  ///
  ///     0. `[writable]` User token SOURCE Account, (coin_wallet).
  ///     1. `[writable]` User token DESTINATION Account to swap INTO. Must be the DESTINATION token.
  ///     2. `[-signer]` User token SOURCE account OWNER (or Authority) account.
  ///     3. '[]` Token program id.
  ///     4. `[writable]` fee token account.
  ///     6. `[]` StableSwap info.
  ///     7. `[]` StableSwap authority.
  ///     8. `[writable]` StableSwap token a account.
  ///     9. `[writable]` StableSwap token b account.
  ///     10. `[writable]` StableSwap admin fee account. Must have same mint as User DESTINATION token account.
  ///     11. `[]` StableSwap clock id.
  ///     12. `[]` StableSwap program id.
  SwapStableSwap(SwapInstruction),

  /// Swap tokens through Raydium-Swap
  ///
  ///     0. `[writable]` User token SOURCE Account, (coin_wallet).
  ///     1. `[writable]` User token DESTINATION Account to swap INTO. Must be the DESTINATION token.
  ///     2. `[-signer]` User token SOURCE account OWNER (or Authority) account.
  ///     3. '[]` Token program id.
  ///     4. `[writable]` fee token account.
  ///     6. `[writable]` raydium amm account.
  ///     7. `[]` raydium $authority.
  ///     8. `[writable]` raydium open_orders account.
  ///     9. `[writable]` raydium target_orders account.
  ///     10. `[writable]` raydium pool_token_coin account.
  ///     11. `[writable]` raydium pool_token_pc account.
  ///     12. `[]` serum-dex program id.
  ///     13. `[writable]` raydium serum_market account.
  ///     14. `[writable]` raydium bids account.
  ///     15. `[writable]` raydium asks account.
  ///     16. `[writable]` raydium event_q account.
  ///     17. `[writable]` raydium coin_vault account.
  ///     18. `[writable]` raydium pc_vault account.
  ///     19. `[]` raydium vault_signer account.
  ///     20. `[]` raydium program id.
  SwapRaydiumSwap(SwapInstruction),

  /// Initialize a new swap info account
  ///   1. `[writable, signer]` The swapInfo account for initializing
  ///   2. `[signer]` User account
  InitializeSwapInfo,

  /// Setup SwapInfo account
  ///   1. `[writable]` The swapInfo account for initializing
  ///   2. `[]` TokenAccount to set
  SetupSwapInfo,

  /// Swap the tokens in the pool.
  ///     0. `[writable]` User token SOURCE Account, (coin_wallet)
  ///     1. `[writable]` User token DESTINATION Account to swap INTO. Must be the DESTINATION token.
  ///     2. `[signer]` User token SOURCE account OWNER (or Authority) account.
  ///     3. '[writable]` SwapInfo account
  ///     4. '[]` Token program id
  ///     5. `[]` TokenSwap swap_info account
  ///     6. `[]` TokenSwap swap_info authority
  ///     7. `[writable]` TokenSwap token_A Account.
  ///     8. `[writable]` TokenSwap token_B Account.
  ///     9. `[writable]` TokenSwap Pool token mint, to generate trading fees
  ///     10. `[writable]` TokenSwap Fee account, to receive trading fees
  ///     11. '[]` Token-Swap program id
  SwapSplTokenSwapIn(SwapInInstruction),

  /// Swap the tokens in the serum dex market.
  ///
  ///     0. `[writable]` User token SOURCE Account, (coin_wallet)
  ///     1. `[writable]` User token DESTINATION Account to swap INTO. Must be the DESTINATION token.
  ///     2. `[signer]` User token SOURCE account OWNER (or Authority) account.
  ///     3. '[writable]` SwapInfo account
  ///     4. '[]` Token program id
  ///     5. `[writable]` dex-market-info account
  ///     6. `[writable]` dex-market-info authority
  ///     7. `[writable]`  dex-market-info open_orders
  ///     8. `[writable]`  serum-dex market
  ///     9. `[writable]`  serum-dex request_queue
  ///     10. `[writable]`  serum-dex event_queue
  ///     11. `[writable]`  serum-dex market_bids
  ///     12. `[writable]`  serum-dex market_asks
  ///     13. `[writable]`  serum-dex coin_vault
  ///     14. `[writable]`  serum-dex pc_vault
  ///     15. `[]`  serum-dex vault_signer for settleFunds
  ///     16. `[]`  serum-dex rent_sysvar
  ///     17. `[]`  serum-dex serum_dex_program_id
  SwapSerumDexIn(SwapInInstruction),

  /// Swap tokens through Saber StableSwap
  ///
  ///     0. `[writable]` User token SOURCE Account, (coin_wallet).
  ///     1. `[writable]` User token DESTINATION Account to swap INTO. Must be the DESTINATION token.
  ///     2. `[signer]` User token SOURCE account OWNER (or Authority) account.
  ///     3. '[writable]` SwapInfo account
  ///     4. '[]` Token program id.
  ///     5. `[]` StableSwap info.
  ///     6. `[]` StableSwap authority.
  ///     7. `[writable]` StableSwap token a account.
  ///     8. `[writable]` StableSwap token b account.
  ///     9. `[writable]` StableSwap admin fee account. Must have same mint as User DESTINATION token account.
  ///     10. `[]` StableSwap clock id.
  ///     11. `[]` StableSwap program id.
  SwapStableSwapIn(SwapInInstruction),

  /// Swap tokens through Raydium-Swap
  ///
  ///     0. `[writable]` User token SOURCE Account, (coin_wallet).
  ///     1. `[writable]` User token DESTINATION Account to swap INTO. Must be the DESTINATION token.
  ///     2. `[signer]` User token0 SOURCE account OWNER (or Authority) account.
  ///     3. '[writable]` SwapInfo account
  ///     4. '[]` Token program id.
  ///     5. `[writable]` raydium amm account.
  ///     6. `[]` raydium $authority.
  ///     7. `[writable]` raydium open_orders account.
  ///     8. `[writable]` raydium target_orders account.
  ///     9. `[writable]` raydium pool_token_coin account.
  ///     10. `[writable]` raydium pool_token_pc account.
  ///     11. `[]` serum-dex program id.
  ///     12. `[writable]` raydium serum_market account.
  ///     13. `[writable]` raydium bids account.
  ///     14. `[writable]` raydium asks account.
  ///     15. `[writable]` raydium event_q account.
  ///     16. `[writable]` raydium coin_vault account.
  ///     17. `[writable]` raydium pc_vault account.
  ///     18. `[]` raydium vault_signer account.
  ///     19. `[]` raydium program id.
  SwapRaydiumIn(SwapInInstruction),

  /// Swap the tokens in the pool.
  ///
  ///     0. `[writable]` User token SOURCE Account, (coin_wallet)
  ///     1. `[writable]` User token DESTINATION Account to swap INTO. Must be the DESTINATION token.
  ///     2. `[signer]` User token SOURCE account OWNER (or Authority) account.
  ///     3. '[writable]` SwapInfo account
  ///     4. '[]` Token program id
  ///     5. `[writable]` fee token account
  ///     6. `[]` TokenSwap swap_info account
  ///     7. `[]` TokenSwap swap_info authority
  ///     8. `[writable]` TokenSwap token_A Account.
  ///     9. `[writable]` TokenSwap token_B Account.
  ///     10. `[writable]` TokenSwap Pool token mint, to generate trading fees
  ///     11. `[writable]` TokenSwap Fee account, to receive trading fees
  ///     12. '[]` Token-Swap program id
  SwapSplTokenSwapOut(SwapOutInstruction),

  /// Swap the tokens in the serum dex market.
  ///
  ///     0. `[writable]` User token SOURCE Account, (coin_wallet)
  ///     1. `[writable]` User token DESTINATION Account to swap INTO. Must be the DESTINATION token.
  ///     2. `[signer]` User token SOURCE account OWNER (or Authority) account.
  ///     3. '[writable]` SwapInfo account
  ///     4. '[]` Token program id
  ///     5. `[writable]` fee token account
  ///     6. `[writable]` dex-market-info account
  ///     7. `[writable]` dex-market-info authority
  ///     8. `[writable]`  dex-market-info open_orders
  ///     9. `[writable]`  serum-dex market
  ///     10. `[writable]`  serum-dex request_queue
  ///     11. `[writable]`  serum-dex event_queue
  ///     12. `[writable]`  serum-dex market_bids
  ///     13. `[writable]`  serum-dex market_asks
  ///     14. `[writable]`  serum-dex coin_vault
  ///     15. `[writable]`  serum-dex pc_vault
  ///     16. `[]`  serum-dex vault_signer for settleFunds
  ///     17. `[]`  serum-dex rent_sysvar
  ///     18. `[]`  serum-dex serum_dex_program_id
  SwapSerumDexOut(SwapOutInstruction),

  /// Swap tokens through Saber StableSwap
  ///
  ///     0. `[writable]` User token SOURCE Account, (coin_wallet).
  ///     1. `[writable]` User token DESTINATION Account to swap INTO. Must be the DESTINATION token.
  ///     2. `[signer]` User token SOURCE account OWNER (or Authority) account.
  ///     3. '[writable]` SwapInfo account
  ///     4. '[]` Token program id.
  ///     5. `[writable]` fee token account.
  ///     6. `[]` StableSwap info.
  ///     7. `[]` StableSwap authority.
  ///     8. `[writable]` StableSwap token a account.
  ///     9. `[writable]` StableSwap token b account.
  ///     10. `[writable]` StableSwap admin fee account. Must have same mint as User DESTINATION token account.
  ///     11. `[]` StableSwap clock id.
  ///     12. `[]` StableSwap program id.
  SwapStableSwapOut(SwapOutInstruction),

  /// Swap tokens through Raydium-Swap
  ///
  ///     0. `[writable]` User token SOURCE Account, (coin_wallet).
  ///     1. `[writable]` User token DESTINATION Account to swap INTO. Must be the DESTINATION token.
  ///     2. `[signer]` User token SOURCE account OWNER (or Authority) account.
  ///     3. '[writable]` SwapInfo account
  ///     4. '[]` Token program id.
  ///     5. `[writable]` fee token account.
  ///     6. `[writable]` raydium amm account.
  ///     7. `[]` raydium $authority.
  ///     8. `[writable]` raydium open_orders account.
  ///     9. `[writable]` raydium target_orders account.
  ///     10. `[writable]` raydium pool_token_coin account.
  ///     11. `[writable]` raydium pool_token_pc account.
  ///     12. `[]` serum-dex program id.
  ///     13. `[writable]` raydium serum_market account.
  ///     14. `[writable]` raydium bids account.
  ///     15. `[writable]` raydium asks account.
  ///     16. `[writable]` raydium event_q account.
  ///     17. `[writable]` raydium coin_vault account.
  ///     18. `[writable]` raydium pc_vault account.
  ///     10. `[]` raydium vault_signer account.
  ///     20. `[]` raydium program id.
  SwapRaydiumOut(SwapOutInstruction),
}

impl OneSolInstruction {
  /// Unpacks a byte buffer into a [OneSolInstruction](enum.OneSolInstruction.html).
  pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
    let (&tag, rest) = input.split_first().ok_or(ProtocolError::InvalidInput)?;
    Ok(match tag {
      2 => Self::InitDexMarketOpenOrders(Initialize::unpack(rest)?),
      3 => Self::SwapSplTokenSwap(SwapInstruction::unpack(rest)?),
      4 => Self::SwapSerumDex(SwapInstruction::unpack(rest)?),
      5 => return Err(ProtocolError::InvalidInstruction.into()),
      6 => Self::SwapStableSwap(SwapInstruction::unpack(rest)?),
      7 => Self::UpdateDexMarketOpenOrders,
      8 => return Err(ProtocolError::InvalidInstruction.into()),
      9 => Self::SwapRaydiumSwap(SwapInstruction::unpack(rest)?),
      10 => Self::InitializeSwapInfo,
      11 => Self::SetupSwapInfo,
      12 => Self::SwapSplTokenSwapIn(SwapInInstruction::unpack(rest)?),
      13 => Self::SwapSplTokenSwapOut(SwapOutInstruction::unpack(rest)?),
      14 => Self::SwapSerumDexIn(SwapInInstruction::unpack(rest)?),
      15 => Self::SwapSerumDexOut(SwapOutInstruction::unpack(rest)?),
      16 => Self::SwapStableSwapIn(SwapInInstruction::unpack(rest)?),
      17 => Self::SwapStableSwapOut(SwapOutInstruction::unpack(rest)?),
      18 => Self::SwapRaydiumIn(SwapInInstruction::unpack(rest)?),
      19 => Self::SwapRaydiumOut(SwapOutInstruction::unpack(rest)?),
      _ => return Err(ProtocolError::InvalidInstruction.into()),
    })
  }
}

impl Initialize {
  const DATA_LEN: usize = 1;

  fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
    if input.len() < Initialize::DATA_LEN {
      return Err(ProtocolError::InvalidInput.into());
    }
    let &[nonce] = array_ref![input, 0, Initialize::DATA_LEN];
    Ok(Self { nonce })
  }
}

impl SwapInstruction {
  const DATA_LEN: usize = 24;

  // size = 1 or 3
  // flag[0/1], [account_size], [amount_in], [minium_amount_out]
  fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
    if input.len() < SwapInstruction::DATA_LEN {
      return Err(ProtocolError::InvalidInput.into());
    }
    let arr_data = array_ref![input, 0, SwapInstruction::DATA_LEN];
    let (&amount_in_arr, &expect_amount_out_arr, &minimum_amount_out_arr) =
      array_refs![arr_data, 8, 8, 8];
    let amount_in =
      NonZeroU64::new(u64::from_le_bytes(amount_in_arr)).ok_or(ProtocolError::InvalidInput)?;
    let expect_amount_out = NonZeroU64::new(u64::from_le_bytes(expect_amount_out_arr))
      .ok_or(ProtocolError::InvalidInput)?;
    let minimum_amount_out = NonZeroU64::new(u64::from_le_bytes(minimum_amount_out_arr))
      .ok_or(ProtocolError::InvalidInput)?;
    if expect_amount_out.get() < minimum_amount_out.get() || expect_amount_out.get() == 0 {
      return Err(ProtocolError::InvalidExpectAmountOut.into());
    }
    Ok(SwapInstruction {
      amount_in,
      expect_amount_out,
      minimum_amount_out,
    })
  }
}

impl SwapInInstruction {
  const DATA_LEN: usize = 8;

  // size = 1 or 3
  // flag[0/1], [account_size], [amount_in], [minium_amount_out]
  fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
    if input.len() < SwapInInstruction::DATA_LEN {
      return Err(ProtocolError::InvalidInput.into());
    }
    let &amount_in_arr = array_ref![input, 0, SwapInInstruction::DATA_LEN];
    let amount_in =
      NonZeroU64::new(u64::from_le_bytes(amount_in_arr)).ok_or(ProtocolError::InvalidInput)?;
    Ok(Self { amount_in })
  }
}

impl SwapOutInstruction {
  const DATA_LEN: usize = 16;

  // size = 1 or 3
  // flag[0/1], [account_size], [amount_in], [minium_amount_out]
  fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
    if input.len() < SwapOutInstruction::DATA_LEN {
      return Err(ProtocolError::InvalidInput.into());
    }
    let arr_data = array_ref![input, 0, SwapOutInstruction::DATA_LEN];
    let (&expect_amount_out_arr, &minimum_amount_out_arr) = array_refs![arr_data, 8, 8];
    let expect_amount_out = NonZeroU64::new(u64::from_le_bytes(expect_amount_out_arr))
      .ok_or(ProtocolError::InvalidInput)?;
    let minimum_amount_out = NonZeroU64::new(u64::from_le_bytes(minimum_amount_out_arr))
      .ok_or(ProtocolError::InvalidInput)?;
    if expect_amount_out.get() < minimum_amount_out.get() || expect_amount_out.get() == 0 {
      return Err(ProtocolError::InvalidExpectAmountOut.into());
    }
    Ok(Self {
      expect_amount_out,
      minimum_amount_out,
    })
  }
}

// impl SwapTwoStepsInstruction {
//   const DATA_LEN: usize = 28;

//   /// u64, u64, u64, u8,
//   fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
//     if input.len() < SwapTwoStepsInstruction::DATA_LEN {
//       return Err(ProtocolError::InvalidInput.into());
//     }
//     let arr_data = array_ref![input, 0, SwapTwoStepsInstruction::DATA_LEN];
//     let (&amount_in_arr, &expect_amount_out_arr, &minimum_amount_out_arr, &step1_arr, &step2_arr) =
//       array_refs![arr_data, 8, 8, 8, 2, 2];
//     let amount_in =
//       NonZeroU64::new(u64::from_le_bytes(amount_in_arr)).ok_or(ProtocolError::InvalidInput)?;
//     let expect_amount_out = NonZeroU64::new(u64::from_le_bytes(expect_amount_out_arr))
//       .ok_or(ProtocolError::InvalidInput)?;
//     let minimum_amount_out = NonZeroU64::new(u64::from_le_bytes(minimum_amount_out_arr))
//       .ok_or(ProtocolError::InvalidInput)?;
//     if expect_amount_out.get() < minimum_amount_out.get() || expect_amount_out.get() == 0 {
//       return Err(ProtocolError::InvalidExpectAmountOut.into());
//     }
//     let step1 = ExchangeStep::unpack(&step1_arr)?;
//     let step2 = ExchangeStep::unpack(&step2_arr)?;
//     Ok(SwapTwoStepsInstruction {
//       amount_in,
//       expect_amount_out,
//       minimum_amount_out,
//       step1,
//       step2,
//     })
//   }
// }

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_unpack_initialize() {
    let nonce = 101;
    let mut buf = Vec::with_capacity(1);
    buf.push(nonce);

    let i = Initialize::unpack(&buf[..]).unwrap();

    assert_eq!(i.nonce, nonce)
  }

  #[test]
  fn test_unpack_swap_token_swap() {
    let amount_in = 120000u64;
    let minimum_amount_out = 1080222u64;
    let expect_amount_out = 1090000u64;
    let mut buf = Vec::with_capacity(SwapInstruction::DATA_LEN);
    buf.extend_from_slice(&amount_in.to_le_bytes());
    buf.extend_from_slice(&expect_amount_out.to_le_bytes());
    buf.extend_from_slice(&minimum_amount_out.to_le_bytes());
    // buf.insert(, element)

    let i = SwapInstruction::unpack(&buf[..]).unwrap();
    assert_eq!(i.amount_in.get(), amount_in);
    assert_eq!(i.expect_amount_out.get(), expect_amount_out);
    assert_eq!(i.minimum_amount_out.get(), minimum_amount_out);
  }
}
