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
}

impl ExchangerType {
  pub fn from(value: u8) -> Option<Self> {
    match value {
      0 => Some(ExchangerType::SplTokenSwap),
      1 => Some(ExchangerType::SerumDex),
      2 => Some(ExchangerType::StableSwap),
      _ => None,
    }
  }
}

#[derive(Clone, Debug, PartialEq, Copy)]
pub struct ExchangeStep {
  pub exchanger_type: ExchangerType,
  pub accounts_count: usize,
}

impl ExchangeStep {
  pub const LEN: usize = 2;

  pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
    let arr_data = array_ref![input, 0, ExchangeStep::LEN];
    let (&[exchanger_type], &[accounts_count]) = array_refs![arr_data, 1, 1];
    Ok(Self {
      exchanger_type: ExchangerType::from(exchanger_type)
        .ok_or(ProgramError::InvalidInstructionData)?,
      accounts_count: accounts_count as usize,
    })
  }
}

/// Initialize instruction data
#[derive(Clone, Debug, PartialEq)]
pub struct Initialize {
  /// nonce used to create validate program address
  pub nonce: u8,
}

/// Swap from multiple exchanger
#[derive(Clone, Debug, PartialEq, Copy)]
pub struct SwapTwoStepsInstruction {
  /// the amount to swap *from*
  pub amount_in: NonZeroU64,
  /// expect amount of tokens to swap
  pub expect_amount_out: NonZeroU64,
  /// Minimum amount of DESTINATION token to output, prevents excessive slippage
  pub minimum_amount_out: NonZeroU64,
  /// Step1
  pub step1: ExchangeStep,
  /// Step1
  pub step2: ExchangeStep,
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
  /// use full amount of source account
  pub use_full: bool,
}

// Instructions supported by the 1sol protocol program
#[repr(C)]
#[derive(Debug, PartialEq)]
pub enum OneSolInstruction {
  /// Initialize Token pair
  ///
  /// 0. `[writable, signer]` New 1solProtocol AmmInfo account to create.
  /// 1. `[]` $authority derived from `create_program_address(&[1solProtocolAmmInfo account])`
  /// 2. `[]` Owner account
  /// 3. `[]` token_a_vault Account. Must be non zero, owned by $authority.
  /// 4. `[]` token_a mint Account.
  /// 5. `[]` token_b_vault Account. Must owned by $authority.
  /// 6. `[]` token_b_mint Account.
  /// 7. '[]` Spl-Token program id
  InitializeAmmInfo(Initialize),

  /// Create Dex Market
  ///
  /// 0. `[]` AmmInfo account.
  /// 1. `[]` $authority `AmmInfo's authority`
  /// 2. `[writable, signer]` new DexMarketInfo account to create.
  /// 3. `[writable]` market account. SerumDexMarket account.
  /// 4. `[writable]` open_orders account. SerumDexOpenOrders account.
  /// 5. `[]` the rend sysvar.
  /// 6. `[]` SerumDex ProgramId.
  InitDexMarketOpenOrders(Initialize),

  /// Update DexMarket OpenOrders
  ///
  /// 0. `[]` AmmInfo account.
  /// 1. `[]` $authority `AmmInfo's authority`
  /// 2. `[writable]` DexMarketInfo account to update.
  /// 3. `[writable]` market account. SerumDexMarket account.
  /// 4. `[writable]` open_orders account. SerumDexOpenOrders account.
  /// 5. `[]` the rend sysvar.
  /// 6. `[]` SerumDex ProgramId.
  UpdateDexMarketOpenOrders,

  /// Withdraw all swap fees
  ///
  /// 0. `[]` AmmInfo account.
  /// 1. `[]` $authority derived from `create_program_address(&[AmmInfo account])`
  /// 2. `[writable]` token_a_vault Account. Must be non zero, owned by $authority.
  /// 3. `[writable]` token_b_vault Account. Must owned by $authority.
  /// 4. '[]` Spl-Token program id
  /// 5. `[writable]` token_a_destination token account.
  /// 6. `[writable]` token_b_destination token account.
  SwapFees,

  /// Swap the tokens in the pool.
  ///
  ///   user accounts
  ///   0. `[writable]` User token SOURCE Account, (coin_wallet)
  ///   1. `[writable]` User token DESTINATION Account to swap INTO. Must be the DESTINATION token.
  ///   2. `[signer]` User token SOURCE account OWNER (or Authority) account.
  ///   spl token program
  ///   3. '[]` Token program id
  ///   amm_info accounts
  ///   4. `[writable]` OneSolProtocol AmmInfo
  ///   5. `[]` OneSolProtocol AmmInfo authority
  ///   6. `[writable]` OneSolProtocol AmmInfo token a account
  ///   7. `[writable]` OneSolProtocol AmmInfo token b account
  ///   token_swap accounts
  ///   8. `[]` TokenSwap swap_info account
  ///   9. `[]` TokenSwap swap_info authority
  ///   10. `[writable]` TokenSwap token_A Account.
  ///   11. `[writable]` TokenSwap token_B Account.
  ///   12. `[writable]` TokenSwap Pool token mint, to generate trading fees
  ///   13. `[writable]` TokenSwap Fee account, to receive trading fees
  ///   14. '[]` Token-Swap program id
  ///   15. `[optional, writable]` Host fee account to receive additional trading fees
  SwapSplTokenSwap(SwapInstruction),

  /// Swap the tokens in the serum dex market.
  ///
  ///     0. `[writable]` User token SOURCE Account, (coin_wallet)
  ///     1. `[writable]` User token DESTINATION Account to swap INTO. Must be the DESTINATION token.
  ///     2. `[signer]` User token SOURCE account OWNER (or Authority) account.
  ///     3. '[]` Token program id
  ///     4. `[writable]` OneSolProtocol AmmInfo
  ///     5. `[]` OneSolProtocol AmmInfo authority
  ///     6. `[writable]` OneSolProtocol AmmInfo token a account
  ///     7. `[writable]` OneSolProtocol AmmInfo token b account
  ///     8. `[writable]`  serum-dex market
  ///     9. `[writable]`  serum-dex request_queue
  ///     10. `[writable]`  serum-dex event_queue
  ///     11. `[writable]`  serum-dex market_bids
  ///     12. `[writable]`  serum-dex market_asks
  ///     13. `[writable]`  serum-dex coin_vault
  ///     14. `[writable]`  serum-dex pc_vault
  ///     15. `[]`  serum-dex vault_signer for settleFunds
  ///     16. `[writable]`  serum-dex open_orders
  ///     17. `[]`  serum-dex rent_sysvar
  ///     18. `[]`  serum-dex serum_dex_program_id
  SwapSerumDex(SwapInstruction),

  /// Swap tokens through Saber StableSwap
  ///
  ///     0. `[writable]` User token SOURCE Account, (coin_wallet).
  ///     1. `[writable]` User token DESTINATION Account to swap INTO. Must be the DESTINATION token.
  ///     2. `[signer]` User token SOURCE account OWNER (or Authority) account.
  ///     3. '[]` Token program id.
  ///     4. `[writable]` OneSolProtocol AmmInfo.
  ///     5. `[]` OneSolProtocol AmmInfo authority.
  ///     6. `[writable]` OneSolProtocol AmmInfo token a account.
  ///     7. `[writable]` OneSolProtocol AmmInfo token b account.
  ///     8. `[]` StableSwap info.
  ///     9. `[]` StableSwap authority.
  ///     10. `[writable]` StableSwap token a account.
  ///     11. `[writable]` StableSwap token b account.
  ///     12. `[writable]` StableSwap admin fee account. Must have same mint as User DESTINATION token account.
  ///     13. `[]` StableSwap clock id.
  ///     14. `[]` StableSwap program id.
  SwapStableSwap(SwapInstruction),

  /// Swap Two Steps
  ///   Define:
  ///     TokenSwap Accounts
  ///       0. `[]` TokenSwap swap_info account
  ///       1. `[]` TokenSwap swap_info authority
  ///       2. `[writable]` TokenSwap token_A Base Account to swap FROM.  Must be the SOURCE token.
  ///       3. `[writable]` TokenSwap token_B Base Account to swap INTO.  Must be the DESTINATION token.
  ///       4. `[writable]` TokenSwap Pool token mint, to generate trading fees
  ///       5. `[writable]` TokenSwap Fee account, to receive trading fees
  ///       6. '[]` Token-Swap program id
  ///       7. `[optional, writable]` Host fee account to receive additional trading fee
  ///     SerumDex Accounts
  ///       0. `[writable]`  serum-dex market
  ///       1. `[writable]`  serum-dex request_queue
  ///       2. `[writable]`  serum-dex event_queue
  ///       3. `[writable]`  serum-dex market_bids
  ///       4. `[writable]`  serum-dex market_asks
  ///       5. `[writable]`  serum-dex coin_vault
  ///       6. `[writable]`  serum-dex pc_vault
  ///       7. `[]`  serum-dex vault_signer for settleFunds
  ///       8. `[writable]`  serum-dex open_orders
  ///       9. `[]`  serum-dex rent_sysvar
  ///       10. `[]`  serum-dex serum_dex_program_id
  ///     Saber StableSwap accounts
  ///       0. `[]` StableSwap info.
  ///       1. `[]` StableSwap authority.
  ///       2. `[writable]` StableSwap token a account.
  ///       3. `[writable]` StableSwap token b account.
  ///       4. `[writable]` StableSwap admin fee account. Must have same mint as User DESTINATION token account.
  ///       5. `[]` StableSwap clock id.
  ///       6. `[]` StableSwap program id.
  ///
  ///   All Accounts:
  ///     0. `[writable]` User token SOURCE Account, (coin_wallet)
  ///     1. `[writable]` User token DESTINATION Account to swap INTO. Must be the DESTINATION token.
  ///     2. `[signer]` User token SOURCE account OWNER (or Authority) account.
  ///     3. '[]` Token program id
  ///   Step0
  ///     TokenSwap Accounts or SerumDex Accounts
  ///   Step1
  ///     0. `[writable]` OneSolProtocol AmmInfo2
  ///     1. `[]` OneSolProtocol AmmInfo2 authority
  ///     2. `[writeable]` OneSolProtocol AmmInfo2 token a account
  ///     4. `[writeable]` OneSolProtocol AmmInfo2 token b account
  ///     TokenSwap Accounts or SerumDex Accounts or Saber StableSwap Accounts
  SwapTwoSteps(SwapTwoStepsInstruction),
}

impl OneSolInstruction {
  /// Unpacks a byte buffer into a [OneSolInstruction](enum.OneSolInstruction.html).
  pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
    let (&tag, rest) = input.split_first().ok_or(ProtocolError::InvalidInput)?;
    Ok(match tag {
      1 => Self::InitializeAmmInfo(Initialize::unpack(rest)?),
      2 => Self::InitDexMarketOpenOrders(Initialize::unpack(rest)?),
      3 => Self::SwapSplTokenSwap(SwapInstruction::unpack(rest)?),
      4 => Self::SwapSerumDex(SwapInstruction::unpack(rest)?),
      5 => Self::SwapTwoSteps(SwapTwoStepsInstruction::unpack(rest)?),
      6 => Self::SwapStableSwap(SwapInstruction::unpack(rest)?),
      7 => Self::UpdateDexMarketOpenOrders,
      8 => Self::SwapFees,
      _ => return Err(ProtocolError::InvalidInstruction.into()),
    })
  }
}

impl Initialize {
  fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
    if input.len() < 1 {
      return Err(ProtocolError::InvalidInput.into());
    }
    let (&nonce, _rest) = input.split_first().ok_or(ProtocolError::InvalidInput)?;
    Ok(Initialize { nonce })
  }
}

impl SwapInstruction {
  const DATA_LEN: usize = 25;

  // size = 1 or 3
  // flag[0/1], [account_size], [amount_in], [minium_amount_out]
  fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
    if input.len() < SwapInstruction::DATA_LEN {
      return Err(ProtocolError::InvalidInput.into());
    }
    let arr_data = array_ref![input, 0, SwapInstruction::DATA_LEN];
    let (&amount_in_arr, &expect_amount_out_arr, &minimum_amount_out_arr, &[use_full]) =
      array_refs![arr_data, 8, 8, 8, 1];
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
      use_full: use_full == 1,
    })
  }
}

impl SwapTwoStepsInstruction {
  const DATA_LEN: usize = 28;

  /// u64, u64, u64, u8,
  fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
    if input.len() < SwapTwoStepsInstruction::DATA_LEN {
      return Err(ProtocolError::InvalidInput.into());
    }
    let arr_data = array_ref![input, 0, SwapTwoStepsInstruction::DATA_LEN];
    let (&amount_in_arr, &expect_amount_out_arr, &minimum_amount_out_arr, &step1_arr, &step2_arr) =
      array_refs![arr_data, 8, 8, 8, 2, 2];
    let amount_in =
      NonZeroU64::new(u64::from_le_bytes(amount_in_arr)).ok_or(ProtocolError::InvalidInput)?;
    let expect_amount_out = NonZeroU64::new(u64::from_le_bytes(expect_amount_out_arr))
      .ok_or(ProtocolError::InvalidInput)?;
    let minimum_amount_out = NonZeroU64::new(u64::from_le_bytes(minimum_amount_out_arr))
      .ok_or(ProtocolError::InvalidInput)?;
    if expect_amount_out.get() < minimum_amount_out.get() || expect_amount_out.get() == 0 {
      return Err(ProtocolError::InvalidExpectAmountOut.into());
    }
    let step1 = ExchangeStep::unpack(&step1_arr)?;
    let step2 = ExchangeStep::unpack(&step2_arr)?;
    Ok(SwapTwoStepsInstruction {
      amount_in,
      expect_amount_out,
      minimum_amount_out,
      step1,
      step2,
    })
  }
}

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
    let use_full = 1u8;
    let mut buf = Vec::with_capacity(SwapInstruction::DATA_LEN);
    buf.extend_from_slice(&amount_in.to_le_bytes());
    buf.extend_from_slice(&expect_amount_out.to_le_bytes());
    buf.extend_from_slice(&minimum_amount_out.to_le_bytes());
    buf.push(use_full);
    // buf.insert(, element)

    let i = SwapInstruction::unpack(&buf[..]).unwrap();
    assert_eq!(i.amount_in.get(), amount_in);
    assert_eq!(i.expect_amount_out.get(), expect_amount_out);
    assert_eq!(i.minimum_amount_out.get(), minimum_amount_out);
    assert_eq!(i.use_full, true);

    let amount_in = 120000u64;
    let minimum_amount_out = 1080222u64;
    let expect_amount_out = 1090000u64;
    let use_full = 0u8;
    let mut buf = Vec::with_capacity(SwapInstruction::DATA_LEN);
    buf.extend_from_slice(&amount_in.to_le_bytes());
    buf.extend_from_slice(&expect_amount_out.to_le_bytes());
    buf.extend_from_slice(&minimum_amount_out.to_le_bytes());
    buf.push(use_full);

    let i = SwapInstruction::unpack(&buf[..]).unwrap();
    assert_eq!(i.amount_in.get(), amount_in);
    assert_eq!(i.expect_amount_out.get(), expect_amount_out);
    assert_eq!(i.minimum_amount_out.get(), minimum_amount_out);
    assert_eq!(i.use_full, false);
  }
}
