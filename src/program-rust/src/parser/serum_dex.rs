use crate::{
  declare_validated_account_wrapper,
  error::{ProtocolError, ProtocolResult},
  exchanger::serum_dex::matching::Side as DexSide,
  parser::base::TokenAccount,
};
use arrayref::{array_ref, array_refs};
use solana_program::{account_info::AccountInfo, msg, pubkey::Pubkey};

declare_validated_account_wrapper!(SerumDexMarket, |account: &AccountInfo| {
  if !account.is_writable {
    return Err(ProtocolError::ReadonlyAccount);
  }
  let data = account
    .try_borrow_data()
    .map_err(|_| ProtocolError::BorrowAccountDataError)?;
  // [5,8,32,8,32,32,32,8,8,32,8,8,8,32,32,32,32,8,8,8,8,7]
  const MARKET_LEN: usize = 388;
  if data.len() != MARKET_LEN {
    return Err(ProtocolError::InvalidSerumDexMarketAccount);
  }
  let flag_data = u64::from_le_bytes(*array_ref![data, 5, 8]);
  /**
   *  Initialized = 1u64 << 0,
   *  Market = 1u64 << 1,
   */
  if flag_data != 3 {
    // if flag_data != (SerumAccountFlag::Initialized | SerumAccountFlag::Market).bits() {
    msg!("flag_data: {:?}, expect: {:?}", flag_data, 3,);
    return Err(ProtocolError::InvalidSerumDexMarketAccount);
  }
  Ok(())
});

declare_validated_account_wrapper!(SerumDexOpenOrders, |account: &AccountInfo| {
  if !account.is_writable {
    return Err(ProtocolError::ReadonlyAccount);
  }
  let account_data = account
    .try_borrow_data()
    .map_err(|_| ProtocolError::BorrowAccountDataError)?;
  // [5,8,32,32,8,8,8,8,16,16,16*128,8*128,8,7]
  const MARKET_LEN: usize = 3228;
  if account_data.len() != MARKET_LEN {
    return Err(ProtocolError::InvalidSerumDexMarketAccount);
  }
  #[allow(clippy::ptr_offset_with_cast)]
  let (_, data, _) = array_refs![&account_data, 5; ..; 7];
  let flag_data = u64::from_le_bytes(*array_ref![data, 0, 8]);
  /**
   *  Initialized = 1u64 << 0,
   *  Market = 1u64 << 1,
   */
  // BitFlags::
  if flag_data != 5 {
    // if flag_data != (SerumAccountFlag::Initialized | SerumAccountFlag::OpenOrders).bits() {
    msg!("flag_data: {:?}, expect: {:?}", flag_data, 5,);
    return Err(ProtocolError::InvalidOpenOrdersAccount);
  }
  Ok(())
});

#[allow(unused)]
impl<'a, 'b: 'a> SerumDexMarket<'a, 'b> {
  pub fn coin_mint(self) -> ProtocolResult<Pubkey> {
    let account_data = self
      .inner()
      .try_borrow_data()
      .map_err(|_| ProtocolError::BorrowAccountDataError)?;
    #[allow(clippy::ptr_offset_with_cast)]
    let (_, data, _) = array_refs![&account_data, 5; ..; 7];
    Ok(Pubkey::new_from_array(*array_ref![data, 48, 32]))
  }

  pub fn pc_mint(self) -> ProtocolResult<Pubkey> {
    let account_data = self
      .inner()
      .try_borrow_data()
      .map_err(|_| ProtocolError::BorrowAccountDataError)?;
    #[allow(clippy::ptr_offset_with_cast)]
    let (_, data, _) = array_refs![&account_data, 5; ..; 7];
    Ok(Pubkey::new_from_array(*array_ref![data, 80, 32]))
  }
}

#[allow(unused)]
impl<'a, 'b: 'a> SerumDexOpenOrders<'a, 'b> {
  pub fn market(self) -> ProtocolResult<Pubkey> {
    let account_data = self
      .inner()
      .try_borrow_data()
      .map_err(|_| ProtocolError::BorrowAccountDataError)?;
    #[allow(clippy::ptr_offset_with_cast)]
    let (_, data, _) = array_refs![&account_data, 5; ..; 7];
    Ok(Pubkey::new_from_array(*array_ref![data, 8, 32]))
  }

  pub fn owner(self) -> ProtocolResult<Pubkey> {
    let account_data = self
      .inner()
      .try_borrow_data()
      .map_err(|_| ProtocolError::BorrowAccountDataError)?;
    #[allow(clippy::ptr_offset_with_cast)]
    let (_, data, _) = array_refs![&account_data, 5; ..; 7];
    Ok(Pubkey::new_from_array(*array_ref![data, 40, 32]))
  }
}

#[derive(Copy, Clone)]
pub struct SerumDexArgs<'a, 'b: 'a> {
  pub open_orders: SerumDexOpenOrders<'a, 'b>,
  pub market: SerumDexMarket<'a, 'b>,
  pub request_queue_acc: &'a AccountInfo<'b>,
  pub event_queue_acc: &'a AccountInfo<'b>,
  pub bids_acc: &'a AccountInfo<'b>,
  pub asks_acc: &'a AccountInfo<'b>,
  pub coin_vault_acc: TokenAccount<'a, 'b>,
  pub pc_vault_acc: TokenAccount<'a, 'b>,
  pub vault_signer_acc: &'a AccountInfo<'b>,
  pub rent_sysvar_acc: &'a AccountInfo<'b>,
  pub program_acc: &'a AccountInfo<'b>,
}

impl<'a, 'b: 'a> SerumDexArgs<'a, 'b> {
  pub fn with_parsed_args(accounts: &'a [AccountInfo<'b>]) -> ProtocolResult<Self> {
    const MIN_ACCOUNTS: usize = 11;
    if accounts.len() != MIN_ACCOUNTS {
      return Err(ProtocolError::InvalidAccountsLength);
    }
    let &[
      ref open_orders_acc,
      ref market_acc,
      ref request_queue_acc,
      ref event_queue_acc,
      ref bids_acc,
      ref asks_acc,
      ref coin_vault_acc,
      ref pc_vault_acc,
      ref vault_signer_acc,
      ref rent_sysvar_acc,
      ref serum_program_acc,
    ]: &'a[AccountInfo<'b>; MIN_ACCOUNTS] = array_ref![accounts, 0, MIN_ACCOUNTS];

    let market = SerumDexMarket::new(market_acc)?;
    if *market.inner().owner != *serum_program_acc.key {
      return Err(ProtocolError::InvalidProgramAddress);
    }
    let open_orders = SerumDexOpenOrders::new(open_orders_acc)?;
    if *open_orders.inner().owner != *serum_program_acc.key {
      return Err(ProtocolError::InvalidProgramAddress);
    }
    // if open_orders.market()? != *market.pubkey() {
    //   return Err(ProtocolError::InvalidSerumDexMarketAccount);
    // }

    Ok(SerumDexArgs {
      open_orders,
      market,
      request_queue_acc,
      event_queue_acc,
      bids_acc,
      asks_acc,
      coin_vault_acc: TokenAccount::new(coin_vault_acc)?,
      pc_vault_acc: TokenAccount::new(pc_vault_acc)?,
      vault_signer_acc,
      rent_sysvar_acc,
      program_acc: serum_program_acc,
    })
  }

  pub fn find_side(&self, source_mint: &Pubkey) -> ProtocolResult<DexSide> {
    if *source_mint == self.coin_vault_acc.mint()? {
      Ok(DexSide::Ask)
    } else {
      Ok(DexSide::Bid)
    }
  }

  // pub fn check_open_orders_owner(&self, target: &Pubkey) -> ProtocolResult<()> {
  //   if self.open_orders.owner()? != *target {
  //     return Err(ProtocolError::InvalidOpenOrdersAccount);
  //   }
  //   Ok(())
  // }
}

#[cfg(test)]
mod tests {
  use super::*;
  use solana_sdk::{account_info::AccountInfo, pubkey::Pubkey};
  use std::str::FromStr;

  #[test]
  fn test_serum_dex_market() {
    let market_data = r#"GmH4gu6PYUUKDZqX8AT2ZH7MKQkqEiK1rkgus44yrCJvP7UDfLpQzbFKzfg
Ux1oSffopN2NGno33fnjhD37awk2MPJrXgRiQjwQWWwspgrrjXVKhP87vynWu4FzjGgx8USsnBa5
mNEZb2rKvNmVZKekzZUpdSAiXEMbVvEpAn1tQTderQCh69t84sPfcVfseAPEKyJYcAiFLCTrKFmQ3
SVQiartpqiySprqLqkqto5Z3LAVRGBvVvcinYuZBN49ZbBaMGxXS9wt6tXN8ZqmoZMfYvc3un68Du
J5vyRPyiYz56LqovWnbjjXY76rRPzsbXR3EqYNMyCFjoqxnsH3LLJVYXwT11ggvUery3J8bhDbdvS
JaacCyTEuaMuWXjJMcsBxW2NQLAPzasX8vu1uTDjqnvCkZKhYcGtCpiLddLQEMXu6mTEE6ZmT73rH
CLaoGKPSYxuVkunGb4AtkU4mSUfWw3EbKc6s6sEvgi5Ec47RYGdNDMK31jENakYtSAweGRSin1iB7
G11FU1xhNE"#;
    let mut data = bs58::decode(market_data.replace('\n', ""))
      .into_vec()
      .unwrap();
    let pubkey = Pubkey::from_str("9wFFyRfZBsuAha4YcuxcXLKwMxJR43S7fPfQLusDBzvT").unwrap();
    let owner = Pubkey::from_str("9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin").unwrap();
    let mut lamports = 1003591360u64;
    let account_info = AccountInfo::new(
      &pubkey,
      false,
      true,
      &mut lamports,
      &mut data[..],
      &owner,
      false,
      246,
    );
    let market = SerumDexMarket::new(&account_info).unwrap();
    let expect_coin_mint = Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();
    let expect_pc_mint = Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap();
    assert_eq!(market.coin_mint().unwrap(), expect_coin_mint);
    assert_eq!(market.pc_mint().unwrap(), expect_pc_mint);
  }

  #[test]
  fn test_serum_dex_open_orders() {
    let market_data = r#"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
UhNFaoyqJ7aZm14QZS9c9FHesiGEqUFNiCZfkWz"#;
    let mut data = bs58::decode(market_data.replace('\n', ""))
      .into_vec()
      .unwrap();
    let pubkey = Pubkey::from_str("HRk9CMrpq7Jn9sh7mzxE8CChHG8dneX9p475QKz4Fsfc").unwrap();
    let owner = Pubkey::from_str("9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin").unwrap();
    let mut lamports = 1003591360u64;
    let account_info = AccountInfo::new(
      &pubkey,
      false,
      true,
      &mut lamports,
      &mut data[..],
      &owner,
      false,
      246,
    );
    let open_orders = SerumDexOpenOrders::new(&account_info).unwrap();
    let expect_market = Pubkey::from_str("9wFFyRfZBsuAha4YcuxcXLKwMxJR43S7fPfQLusDBzvT").unwrap();
    let expect_owner = Pubkey::from_str("5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1").unwrap();
    assert_eq!(open_orders.market().unwrap(), expect_market);
    assert_eq!(open_orders.owner().unwrap(), expect_owner);
  }
}
