//! Instruction types
#![allow(clippy::ptr_offset_with_cast)]

use crate::{error::ProtocolError, parser::whirlpool};
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
  /// Raydium swap slim
  RaydiumSwapSlim,
  /// CremaFinance
  CremaFinance,
  /// AldrinExchange
  AldrinExchange,
  /// CropperFinance
  CropperFinance,
  /// Whirlpool
  Whirlpool,
}

impl ExchangerType {
  pub fn from(value: u8) -> Option<Self> {
    match value {
      0 => Some(ExchangerType::SplTokenSwap),
      1 => Some(ExchangerType::SerumDex),
      2 => Some(ExchangerType::StableSwap),
      3 => Some(ExchangerType::RaydiumSwap),
      4 => Some(ExchangerType::RaydiumSwapSlim),
      5 => Some(ExchangerType::CremaFinance),
      6 => Some(ExchangerType::AldrinExchange),
      7 => Some(ExchangerType::CropperFinance),
      8 => Some(ExchangerType::Whirlpool),
      _ => None,
    }
  }
}

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
  /// sqrt_price_limit
  pub sqrt_price_limit: Option<u128>,
}

/// Swap instruction data
#[derive(Clone, Debug, PartialEq)]
pub struct SwapInInstruction {
  /// amount of tokens to swap
  pub amount_in: NonZeroU64,
  /// sqrt_price_limit
  pub sqrt_price_limit: Option<u128>,
}

/// Swap instruction data
#[derive(Clone, Debug, PartialEq)]
pub struct SwapOutInstruction {
  /// expect amount of tokens to swap
  pub expect_amount_out: NonZeroU64,
  /// Minimum amount of DESTINATION token to output, prevents excessive slippage
  pub minimum_amount_out: NonZeroU64,
  /// sqrt_price_limit
  pub sqrt_price_limit: Option<u128>,
}

// Instructions supported by the 1sol protocol program
#[repr(C)]
#[derive(Debug, PartialEq)]
pub enum ProtocolInstruction {
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
  ///     5. `[writable]`  user market open_orders
  ///     6. `[writable]`  serum-dex market
  ///     7. `[writable]`  serum-dex request_queue
  ///     8. `[writable]`  serum-dex event_queue
  ///     9. `[writable]`  serum-dex market_bids
  ///     10. `[writable]`  serum-dex market_asks
  ///     11. `[writable]`  serum-dex coin_vault
  ///     12. `[writable]`  serum-dex pc_vault
  ///     13. `[]`  serum-dex vault_signer for settleFunds
  ///     14. `[]`  serum-dex rent_sysvar
  ///     15. `[]`  serum-dex serum_dex_program_id
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
  ///   1. `[writable]` The swapInfo account for setup
  ///   2. `[]` TokenAccount to set
  SetupSwapInfo,

  /// Close SwapInfo account
  ///   1. `[writable]` The swapInfo account for close
  ///   2. `[signer]` owner account
  ///   3. `[writable]` destination account
  CloseSwapInfo,

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
  ///     5. `[writable]`  user market open_orders
  ///     6. `[writable]`  serum-dex market
  ///     7. `[writable]`  serum-dex request_queue
  ///     8. `[writable]`  serum-dex event_queue
  ///     9. `[writable]`  serum-dex market_bids
  ///     10. `[writable]`  serum-dex market_asks
  ///     11. `[writable]`  serum-dex coin_vault
  ///     12. `[writable]`  serum-dex pc_vault
  ///     13. `[]`  serum-dex vault_signer for settleFunds
  ///     14. `[]`  serum-dex rent_sysvar
  ///     15. `[]`  serum-dex serum_dex_program_id
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
  ///     6. `[writable]`  user market open_orders
  ///     7. `[writable]`  serum-dex market
  ///     8. `[writable]`  serum-dex request_queue
  ///     9. `[writable]`  serum-dex event_queue
  ///     10. `[writable]`  serum-dex market_bids
  ///     11. `[writable]`  serum-dex market_asks
  ///     12. `[writable]`  serum-dex coin_vault
  ///     13. `[writable]`  serum-dex pc_vault
  ///     14. `[]`  serum-dex vault_signer for settleFunds
  ///     15. `[]`  serum-dex rent_sysvar
  ///     16. `[]`  serum-dex serum_dex_program_id
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
  ///     8. `[writable]` raydium pool_token_coin account.
  ///     9. `[writable]` raydium pool_token_pc account.
  ///     10. `[]` serum-dex program id.
  ///     11. `[writable]` raydium serum_market account.
  ///     12. `[writable]` raydium bids account.
  ///     13. `[writable]` raydium asks account.
  ///     14. `[writable]` raydium event_q account.
  ///     15. `[writable]` raydium coin_vault account.
  ///     16. `[writable]` raydium pc_vault account.
  ///     17. `[]` raydium vault_signer account.
  ///     18. `[]` raydium program id.
  SwapRaydiumIn2(SwapInInstruction),

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
  SwapRaydiumOut2(SwapOutInstruction),

  /// Swap direct by CremaFinance
  ///
  ///   0. `[writable]` User token SOURCE Account, (coin_wallet)
  ///   1. `[writable]` User token DESTINATION Account to swap INTO. Must be the DESTINATION token.
  ///   2. `[signer]` User token SOURCE account OWNER (or Authority) account.
  ///   3. '[]` Token program id
  ///   4. `[writable]` fee token account
  ///
  ///   5. `[writable]` CremaFinance swap_info account
  ///   6. `[]` CremaFinance authority
  ///   7. `[writable]` CremaFinance token_A Account.
  ///   8. `[writable]` CremaFinance token_B Account.
  ///   9. `[writable]` CremaFinance tick dst Account.
  ///   10. '[]` CremaFinance program id
  SwapCremaFinance(SwapInstruction),

  /// SwapIn by CremaFinance
  ///   0. `[writable]` User token SOURCE Account, (coin_wallet).
  ///   1. `[writable]` User token DESTINATION Account to swap INTO. Must be the DESTINATION token.
  ///   2. `[signer]` User token SOURCE account OWNER (or Authority) account.
  ///   3. '[writable]` Protocol SwapInfo account
  ///   4. '[]` Token program id.
  ///
  ///   5. `[writable]` CremaFinance swap_info account
  ///   6. `[]` CremaFinance authority
  ///   7. `[writable]` CremaFinance token_A Account.
  ///   8. `[writable]` CremaFinance token_B Account.
  ///   9. `[writable]` CremaFinance tick dst Account.
  ///   10. '[]` CremaFinance program id
  SwapCremaFinanceIn(SwapInInstruction),

  /// SwapOut by CremaFinance
  ///   0. `[writable]` User token SOURCE Account, (coin_wallet).
  ///   1. `[writable]` User token DESTINATION Account to swap INTO. Must be the DESTINATION token.
  ///   2. `[signer]` User token SOURCE account OWNER (or Authority) account.
  ///   3. '[writable]` SwapInfo account
  ///   4. '[]` Token program id.
  ///   5. `[writable]` fee token account.
  ///
  ///   6. `[writable]` CremaFinance swap_info account
  ///   7. `[]` CremaFinance authority
  ///   8. `[writable]` CremaFinance token_A Account.
  ///   9. `[writable]` CremaFinance token_B Account.
  ///   10. `[writable]` CremaFinance tick dst Account.
  ///   11. '[]` CremaFinance program id
  SwapCremaFinanceOut(SwapOutInstruction),

  /// Swap direct by AldrinExchange
  ///
  ///   0. `[writable]` User token SOURCE Account, (coin_wallet)
  ///   1. `[writable]` User token DESTINATION Account to swap INTO. Must be the DESTINATION token.
  ///   2. `[signer]` User token SOURCE account OWNER (or Authority) account.
  ///   3. '[]` Token program id
  ///   4. `[writable]` fee token account
  ///
  ///   5. `[]` AldrinExchange pool_info account.
  ///   6. `[]` AldrinExchange pool authority.
  ///   7. `[writable]` AldrinExchange pool mint account.
  ///   8. `[writable]` AldrinExchange pool coin vault account.
  ///   9. `[writable]` AldrinExchange pool pc vault account.
  ///   10. `[writable]` AldrinExchange Pool fee account.
  ///   11. `[]` AldrinExchange Pool curve_key account.
  ///   12. '[]` AldrinExchange program id.
  SwapAldrinExchange(SwapInstruction),

  /// SwapIn by AldrinExchange
  ///   0. `[writable]` User token SOURCE Account, (coin_wallet).
  ///   1. `[writable]` User token DESTINATION Account to swap INTO. Must be the DESTINATION token.
  ///   2. `[signer]` User token SOURCE account OWNER (or Authority) account.
  ///   3. '[writable]` Protocol SwapInfo account
  ///   4. '[]` Token program id.
  ///
  ///   5. `[]` AldrinExchange pool_info account.
  ///   6. `[]` AldrinExchange pool authority.
  ///   7. `[writable]` AldrinExchange pool mint account.
  ///   8. `[writable]` AldrinExchange pool coin vault account.
  ///   9. `[writable]` AldrinExchange pool pc vault account.
  ///   10. `[writable]` AldrinExchange Pool fee account.
  ///   11. `[]` AldrinExchange Pool curve_key account.
  ///   12. '[]` AldrinExchange program id.
  SwapAldrinExchangeIn(SwapInInstruction),

  /// SwapOut by AldrinExchange
  ///   0. `[writable]` User token SOURCE Account, (coin_wallet).
  ///   1. `[writable]` User token DESTINATION Account to swap INTO. Must be the DESTINATION token.
  ///   2. `[signer]` User token SOURCE account OWNER (or Authority) account.
  ///   3. '[writable]` SwapInfo account
  ///   4. '[]` Token program id.
  ///   5. `[writable]` fee token account.
  ///
  ///   6. `[]` AldrinExchange pool_info account.
  ///   7. `[]` AldrinExchange pool authority.
  ///   8. `[writable]` AldrinExchange pool mint account.
  ///   9. `[writable]` AldrinExchange pool coin vault account.
  ///   10. `[writable]` AldrinExchange pool pc vault account.
  ///   11. `[writable]` AldrinExchange Pool fee account.
  ///   12. `[]` AldrinExchange Pool curve_key account.
  ///   13. '[]` AldrinExchange program id.
  SwapAldrinExchangeOut(SwapOutInstruction),

  /// Swap direct by CropperFinance
  ///
  ///   0. `[writable]` User token SOURCE Account, (coin_wallet)
  ///   1. `[writable]` User token DESTINATION Account to swap INTO. Must be the DESTINATION token.
  ///   2. `[signer]` User token SOURCE account OWNER (or Authority) account.
  ///   3. '[]` Token program id
  ///   4. `[writable]` fee token account
  ///
  ///   5. `[]` CropperFinance swap_info account.
  ///   6. `[]` CropperFinance pool authority.
  ///   7. `[]` CropperFinance program state [3hsU1VgsBgBgz5jWiqdw9RfGU6TpWdCmdah1oi4kF3Tq].
  ///   8. `[writable]` AldrinExchange pool token_a account.
  ///   9. `[writable]` AldrinExchange pool token_b account.
  ///   10. `[writable]` AldrinExchange pool mint account.
  ///   11. `[writable]` AldrinExchange Pool fee account.
  ///   12. '[]` AldrinExchange program id.
  SwapCropperFinance(SwapInstruction),

  /// SwapIn by CropperFinance
  ///   0. `[writable]` User token SOURCE Account, (coin_wallet).
  ///   1. `[writable]` User token DESTINATION Account to swap INTO. Must be the DESTINATION token.
  ///   2. `[signer]` User token SOURCE account OWNER (or Authority) account.
  ///   3. '[writable]` Protocol SwapInfo account
  ///   4. '[]` Token program id.
  ///
  ///   5. `[]` CropperFinance swap_info account.
  ///   6. `[]` CropperFinance pool authority.
  ///   7. `[]` CropperFinance program state [3hsU1VgsBgBgz5jWiqdw9RfGU6TpWdCmdah1oi4kF3Tq].
  ///   8. `[writable]` AldrinExchange pool token_a account.
  ///   9. `[writable]` AldrinExchange pool token_b account.
  ///   10. `[writable]` AldrinExchange pool mint account.
  ///   11. `[writable]` AldrinExchange Pool fee account.
  ///   12. '[]` AldrinExchange program id.
  SwapCropperFinanceIn(SwapInInstruction),

  /// SwapOut by CropperFinance
  ///   0. `[writable]` User token SOURCE Account, (coin_wallet).
  ///   1. `[writable]` User token DESTINATION Account to swap INTO. Must be the DESTINATION token.
  ///   2. `[signer]` User token SOURCE account OWNER (or Authority) account.
  ///   3. '[writable]` SwapInfo account
  ///   4. '[]` Token program id.
  ///   5. `[writable]` fee token account.
  ///
  ///   6. `[]` CropperFinance swap_info account.
  ///   7. `[]` CropperFinance pool authority.
  ///   8. `[]` CropperFinance program state [3hsU1VgsBgBgz5jWiqdw9RfGU6TpWdCmdah1oi4kF3Tq].
  ///   9. `[writable]` AldrinExchange pool token_a account.
  ///   10. `[writable]` AldrinExchange pool token_b account.
  ///   11. `[writable]` AldrinExchange pool mint account.
  ///   12. `[writable]` AldrinExchange Pool fee account.
  ///   13. '[]` AldrinExchange program id.
  SwapCropperFinanceOut(SwapOutInstruction),

  /// Swap direct by Whirlpool
  ///
  ///   0. `[writable]` User token SOURCE Account, (coin_wallet)
  ///   1. `[writable]` User token DESTINATION Account to swap INTO. Must be the DESTINATION token.
  ///   2. `[signer]` User token SOURCE account OWNER (or Authority) account.
  ///   3. '[]` Token program id
  ///   4. `[writable]` fee token account
  ///
  ///   5. `[writable]` Whirlpool pool account.
  ///   6. `[writable]` Whirlpool token vault a.
  ///   7. `[writable]` Whirlpool token vault b.
  ///   8. `[writable]` Whirlpool TickArray 0.
  ///   9. `[writable]` Whirlpool TickArray 1.
  ///   10. `[writable]` Whirlpool TickArray 2.
  ///   11. `[]` Whirlpool Oracle Account.
  ///   12. '[]` Whirlpool program id.
  SwapWhirlpool(SwapInstruction),

  /// SwapIn by Whirlpool
  ///
  ///   0. `[writable]` User token SOURCE Account, (coin_wallet).
  ///   1. `[writable]` User token DESTINATION Account to swap INTO. Must be the DESTINATION token.
  ///   2. `[signer]` User token SOURCE account OWNER (or Authority) account.
  ///   3. '[writable]` Protocol SwapInfo account
  ///   4. '[]` Token program id.
  ///
  ///   5. `[writable]` Whirlpool pool account.
  ///   6. `[writable]` Whirlpool token vault a.
  ///   7. `[writable]` Whirlpool token vault b.
  ///   8. `[writable]` Whirlpool TickArray 0.
  ///   9. `[writable]` Whirlpool TickArray 1.
  ///   10. `[writable]` Whirlpool TickArray 2.
  ///   11. `[]` Whirlpool Oracle Account.
  ///   12. '[]` Whirlpool program id.
  SwapInWhirlpool(SwapInInstruction),

  /// SwapOut by Whirlpool
  ///
  ///   0. `[writable]` User token SOURCE Account, (coin_wallet).
  ///   1. `[writable]` User token DESTINATION Account to swap INTO. Must be the DESTINATION token.
  ///   2. `[signer]` User token SOURCE account OWNER (or Authority) account.
  ///   3. '[writable]` SwapInfo account
  ///   4. '[]` Token program id.
  ///   5. `[writable]` fee token account.
  ///
  ///   6. `[writable]` Whirlpool pool account.
  ///   7. `[writable]` Whirlpool token vault a.
  ///   8. `[writable]` Whirlpool token vault b.
  ///   9. `[writable]` Whirlpool TickArray 0.
  ///   10. `[writable]` Whirlpool TickArray 1.
  ///   11. `[writable]` Whirlpool TickArray 2.
  ///   12. `[]` Whirlpool Oracle Account.
  ///   13. '[]` Whirlpool program id.
  SwapOutWhirlpool(SwapOutInstruction),
}

impl ProtocolInstruction {
  /// Unpacks a byte buffer into a [OneSolInstruction](enum.OneSolInstruction.html).
  pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
    let (&tag, rest) = input.split_first().ok_or(ProtocolError::InvalidInput)?;
    Ok(match tag {
      3 => Self::SwapSplTokenSwap(SwapInstruction::unpack(rest, ExchangerType::SplTokenSwap)?),
      4 => Self::SwapSerumDex(SwapInstruction::unpack(rest, ExchangerType::SerumDex)?),
      5 => return Err(ProtocolError::InvalidInstruction.into()),
      6 => Self::SwapStableSwap(SwapInstruction::unpack(rest, ExchangerType::StableSwap)?),
      8 => return Err(ProtocolError::InvalidInstruction.into()),
      9 => Self::SwapRaydiumSwap(SwapInstruction::unpack(rest, ExchangerType::RaydiumSwap)?),
      10 => Self::InitializeSwapInfo,
      11 => Self::SetupSwapInfo,
      12 => Self::SwapSplTokenSwapIn(SwapInInstruction::unpack(
        rest,
        ExchangerType::SplTokenSwap,
      )?),
      13 => Self::SwapSplTokenSwapOut(SwapOutInstruction::unpack(
        rest,
        ExchangerType::SplTokenSwap,
      )?),
      14 => Self::SwapSerumDexIn(SwapInInstruction::unpack(rest, ExchangerType::SerumDex)?),
      15 => Self::SwapSerumDexOut(SwapOutInstruction::unpack(rest, ExchangerType::SerumDex)?),
      16 => Self::SwapStableSwapIn(SwapInInstruction::unpack(rest, ExchangerType::StableSwap)?),
      17 => Self::SwapStableSwapOut(SwapOutInstruction::unpack(rest, ExchangerType::StableSwap)?),
      // Self::SwapRaydiumIn(SwapInInstruction::unpack(rest)?)
      18 => return Err(ProtocolError::InvalidInstruction.into()),
      //, Self::SwapRaydiumOut(SwapOutInstruction::unpack(rest)?),
      19 => return Err(ProtocolError::InvalidInstruction.into()),
      20 => Self::SwapRaydiumIn2(SwapInInstruction::unpack(rest, ExchangerType::RaydiumSwap)?),
      21 => Self::SwapRaydiumOut2(SwapOutInstruction::unpack(
        rest,
        ExchangerType::RaydiumSwap,
      )?),
      22 => Self::SwapCremaFinance(SwapInstruction::unpack(rest, ExchangerType::CremaFinance)?),
      23 => Self::SwapCremaFinanceIn(SwapInInstruction::unpack(
        rest,
        ExchangerType::CremaFinance,
      )?),
      24 => Self::SwapCremaFinanceOut(SwapOutInstruction::unpack(
        rest,
        ExchangerType::CremaFinance,
      )?),
      25 => Self::SwapAldrinExchange(SwapInstruction::unpack(
        rest,
        ExchangerType::AldrinExchange,
      )?),
      26 => Self::SwapAldrinExchangeIn(SwapInInstruction::unpack(
        rest,
        ExchangerType::AldrinExchange,
      )?),
      27 => Self::SwapAldrinExchangeOut(SwapOutInstruction::unpack(
        rest,
        ExchangerType::AldrinExchange,
      )?),
      28 => Self::SwapCropperFinance(SwapInstruction::unpack(
        rest,
        ExchangerType::CropperFinance,
      )?),
      29 => Self::SwapCropperFinanceIn(SwapInInstruction::unpack(
        rest,
        ExchangerType::CropperFinance,
      )?),
      30 => Self::SwapCropperFinanceOut(SwapOutInstruction::unpack(
        rest,
        ExchangerType::CropperFinance,
      )?),
      31 => Self::CloseSwapInfo,
      32 => Self::SwapWhirlpool(SwapInstruction::unpack(rest, ExchangerType::Whirlpool)?),
      33 => Self::SwapInWhirlpool(SwapInInstruction::unpack(rest, ExchangerType::Whirlpool)?),
      34 => Self::SwapOutWhirlpool(SwapOutInstruction::unpack(rest, ExchangerType::Whirlpool)?),
      _ => return Err(ProtocolError::InvalidInstruction.into()),
    })
  }
}

impl SwapInstruction {
  const MIN_DATA_LEN: usize = 24;

  // size = 1 or 3
  // flag[0/1], [account_size], [amount_in], [minium_amount_out]
  fn unpack(input: &[u8], exchanger_type: ExchangerType) -> Result<Self, ProgramError> {
    if input.len() < SwapInstruction::MIN_DATA_LEN {
      return Err(ProtocolError::InvalidInput.into());
    }
    let (fixed_arr, other_arr) = array_refs![input, SwapInstruction::MIN_DATA_LEN; ..;];
    let (&amount_in_arr, &expect_amount_out_arr, &minimum_amount_out_arr) =
      array_refs![fixed_arr, 8, 8, 8];

    let amount_in =
      NonZeroU64::new(u64::from_le_bytes(amount_in_arr)).ok_or(ProtocolError::InvalidInput)?;
    let expect_amount_out = NonZeroU64::new(u64::from_le_bytes(expect_amount_out_arr))
      .ok_or(ProtocolError::InvalidInput)?;
    let minimum_amount_out = NonZeroU64::new(u64::from_le_bytes(minimum_amount_out_arr))
      .ok_or(ProtocolError::InvalidInput)?;
    if expect_amount_out.get() < minimum_amount_out.get() || expect_amount_out.get() == 0 {
      return Err(ProtocolError::InvalidExpectAmountOut.into());
    }
    let sqrt_price_limit = match exchanger_type {
      ExchangerType::Whirlpool => Some(whirlpool::WhirlpoolArgs::unpack_input(other_arr)?),
      _ => None,
    };
    Ok(SwapInstruction {
      amount_in,
      expect_amount_out,
      minimum_amount_out,
      sqrt_price_limit,
    })
  }
}

impl SwapInInstruction {
  const MIN_DATA_LEN: usize = 8;

  // size = 1 or 3
  // flag[0/1], [account_size], [amount_in], [minium_amount_out]
  fn unpack(input: &[u8], exchanger_type: ExchangerType) -> Result<Self, ProgramError> {
    if input.len() < SwapInInstruction::MIN_DATA_LEN {
      return Err(ProtocolError::InvalidInput.into());
    }
    let (fixed_arr, other_arr) = array_refs![input, SwapInInstruction::MIN_DATA_LEN; ..;];
    let &amount_in_arr = array_ref![fixed_arr, 0, 8];
    let amount_in =
      NonZeroU64::new(u64::from_le_bytes(amount_in_arr)).ok_or(ProtocolError::InvalidInput)?;

    let sqrt_price_limit = match exchanger_type {
      ExchangerType::Whirlpool => Some(whirlpool::WhirlpoolArgs::unpack_input(other_arr)?),
      _ => None,
    };
    Ok(Self {
      amount_in,
      sqrt_price_limit,
    })
  }
}

impl SwapOutInstruction {
  // size = 1 or 3
  // flag[0/1], [account_size], [amount_in], [minium_amount_out]
  fn unpack(input: &[u8], exchanger_type: ExchangerType) -> Result<Self, ProgramError> {
    let (expect_amount_out, minimum_amount_out, other_arr) = match exchanger_type {
      ExchangerType::RaydiumSwapSlim => {
        if input.len() < 8 {
          return Err(ProtocolError::InvalidInput.into());
        }
        let (fixed_arr, other_arr) = array_refs![input, 8; ..;];
        let &minimum_amount_out = array_ref![fixed_arr, 0, 8];
        let minimum_amount_out = NonZeroU64::new(u64::from_le_bytes(minimum_amount_out))
          .ok_or(ProtocolError::InvalidInput)?;
        (minimum_amount_out, minimum_amount_out, other_arr)
      }
      _ => {
        if input.len() < 16 {
          return Err(ProtocolError::InvalidInput.into());
        }
        let (fixed_arr, other_arr) = array_refs![input, 16; ..;];
        let (&expect_amount_out_arr, &minimum_amount_out_arr) = array_refs![fixed_arr, 8, 8];
        let expect_amount_out = NonZeroU64::new(u64::from_le_bytes(expect_amount_out_arr))
          .ok_or(ProtocolError::InvalidInput)?;
        let minimum_amount_out = NonZeroU64::new(u64::from_le_bytes(minimum_amount_out_arr))
          .ok_or(ProtocolError::InvalidInput)?;
        if expect_amount_out.get() < minimum_amount_out.get() || expect_amount_out.get() == 0 {
          return Err(ProtocolError::InvalidExpectAmountOut.into());
        }
        (expect_amount_out, minimum_amount_out, other_arr)
      }
    };

    let sqrt_price_limit = if exchanger_type == ExchangerType::Whirlpool {
      Some(whirlpool::WhirlpoolArgs::unpack_input(other_arr)?)
    } else {
      None
    };

    Ok(Self {
      expect_amount_out,
      minimum_amount_out,
      sqrt_price_limit,
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_unpack_swap_token_swap() {
    let amount_in = 120000u64;
    let minimum_amount_out = 1080222u64;
    let expect_amount_out = 1090000u64;
    let mut buf = Vec::with_capacity(SwapInstruction::MIN_DATA_LEN);
    buf.extend_from_slice(&amount_in.to_le_bytes());
    buf.extend_from_slice(&expect_amount_out.to_le_bytes());
    buf.extend_from_slice(&minimum_amount_out.to_le_bytes());
    // buf.insert(, element)

    let i = SwapInstruction::unpack(&buf[..], ExchangerType::SplTokenSwap).unwrap();
    assert_eq!(i.amount_in.get(), amount_in);
    assert_eq!(i.expect_amount_out.get(), expect_amount_out);
    assert_eq!(i.minimum_amount_out.get(), minimum_amount_out);
    assert_eq!(i.sqrt_price_limit, None);
  }
}
