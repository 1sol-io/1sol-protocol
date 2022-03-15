use crate::{
  declare_validated_account_wrapper,
  error::{ProtocolError, ProtocolResult},
};
use arrayref::array_ref;
use solana_program::{account_info::AccountInfo, pubkey::Pubkey};

use super::{
  base::TokenAccount,
  serum_dex::{SerumDexMarket, SerumDexOpenOrders},
};

declare_validated_account_wrapper!(RaydiumAmmInfo, |account: &AccountInfo| {
  const DATA_LEN: usize = 752;
  let data = account
    .try_borrow_data()
    .map_err(|_| ProtocolError::BorrowAccountDataError)?;
  if data.len() != DATA_LEN {
    return Err(ProtocolError::InvalidTokenAccount);
  };
  let status = u64::from_le_bytes(*array_ref![data, 0, 8]);
  if status != 1u64 {
    return Err(ProtocolError::InvalidAccountFlags);
  };
  Ok(())
});

#[allow(dead_code)]
impl<'a, 'b: 'a> RaydiumAmmInfo<'a, 'b> {
  pub fn token_coin(self) -> ProtocolResult<Pubkey> {
    let data = self
      .inner()
      .try_borrow_data()
      .map_err(|_| ProtocolError::BorrowAccountDataError)?;
    // 128 + 208
    Ok(Pubkey::new_from_array(*array_ref![data, 336, 32]))
  }

  pub fn token_pc(self) -> ProtocolResult<Pubkey> {
    let data = self
      .inner()
      .try_borrow_data()
      .map_err(|_| ProtocolError::BorrowAccountDataError)?;
    // 128 + 208
    Ok(Pubkey::new_from_array(*array_ref![data, 368, 32]))
  }

  pub fn coin_mint(self) -> ProtocolResult<Pubkey> {
    let data = self
      .inner()
      .try_borrow_data()
      .map_err(|_| ProtocolError::BorrowAccountDataError)?;
    // 128 + 208
    Ok(Pubkey::new_from_array(*array_ref![data, 400, 32]))
  }

  pub fn pc_mint(self) -> ProtocolResult<Pubkey> {
    let data = self
      .inner()
      .try_borrow_data()
      .map_err(|_| ProtocolError::BorrowAccountDataError)?;
    // 128 + 208
    Ok(Pubkey::new_from_array(*array_ref![data, 432, 32]))
  }

  pub fn open_orders(self) -> ProtocolResult<Pubkey> {
    let data = self
      .inner()
      .try_borrow_data()
      .map_err(|_| ProtocolError::BorrowAccountDataError)?;
    // 128 + 208
    Ok(Pubkey::new_from_array(*array_ref![data, 496, 32]))
  }

  pub fn market(self) -> ProtocolResult<Pubkey> {
    let data = self
      .inner()
      .try_borrow_data()
      .map_err(|_| ProtocolError::BorrowAccountDataError)?;
    // 128 + 208
    Ok(Pubkey::new_from_array(*array_ref![data, 528, 32]))
  }

  pub fn serum_dex(self) -> ProtocolResult<Pubkey> {
    let data = self
      .inner()
      .try_borrow_data()
      .map_err(|_| ProtocolError::BorrowAccountDataError)?;
    // 128 + 208
    Ok(Pubkey::new_from_array(*array_ref![data, 560, 32]))
  }
}

#[derive(Copy, Clone)]
pub struct RaydiumSwapArgs<'a, 'b: 'a> {
  pub amm_info: RaydiumAmmInfo<'a, 'b>,
  pub authority: &'a AccountInfo<'b>,
  pub open_orders: SerumDexOpenOrders<'a, 'b>,
  pub target_orders: &'a AccountInfo<'b>,
  pub pool_token_coin: TokenAccount<'a, 'b>,
  pub pool_token_pc: TokenAccount<'a, 'b>,
  pub serum_dex_program_id: &'a AccountInfo<'b>,
  pub serum_market: SerumDexMarket<'a, 'b>,
  pub bids: &'a AccountInfo<'b>,
  pub asks: &'a AccountInfo<'b>,
  pub event_q: &'a AccountInfo<'b>,
  pub coin_vault: TokenAccount<'a, 'b>,
  pub pc_vault: TokenAccount<'a, 'b>,
  pub vault_signer: &'a AccountInfo<'b>,
  pub program_id: &'a AccountInfo<'b>,
}

impl<'a, 'b: 'a> RaydiumSwapArgs<'a, 'b> {
  pub fn with_parsed_args(accounts: &'a [AccountInfo<'b>]) -> ProtocolResult<Self> {
    const MIN_ACCOUNTS: usize = 15;
    if accounts.len() != MIN_ACCOUNTS {
      return Err(ProtocolError::InvalidAccountsLength);
    }
    let &[
      ref amm_info_acc,
      ref authority,
      ref open_orders_acc,
      ref target_orders_acc,
      ref pool_token_coin_acc,
      ref pool_token_pc_acc,
      ref serum_dex_program_id,
      ref serum_market_acc,
      ref bids,
      ref asks,
      ref event_q,
      ref coin_vault_acc,
      ref pc_vault_acc,
      ref vault_signer,
      ref program_id,
    ]: &'a[AccountInfo<'b>; MIN_ACCOUNTS] = array_ref![accounts, 0, MIN_ACCOUNTS];

    if !amm_info_acc.is_writable {
      return Err(ProtocolError::ReadonlyAccount);
    }
    let amm_info = RaydiumAmmInfo::new(amm_info_acc)?;

    if amm_info.token_coin()? != *pool_token_coin_acc.key {
      return Err(ProtocolError::InvalidTokenAccount);
    }
    if amm_info.token_pc()? != *pool_token_pc_acc.key {
      return Err(ProtocolError::InvalidTokenAccount);
    }
    if amm_info.open_orders()? != *open_orders_acc.key {
      return Err(ProtocolError::InvalidRaydiumAmmInfoAccount);
    }
    if amm_info.market()? != *serum_market_acc.key {
      return Err(ProtocolError::InvalidRaydiumAmmInfoAccount);
    }
    if !open_orders_acc.is_writable {
      return Err(ProtocolError::ReadonlyAccount);
    }
    if amm_info.serum_dex()? != *serum_dex_program_id.key {
      return Err(ProtocolError::InvalidSerumDexProgramId);
    }

    let market = SerumDexMarket::new(serum_market_acc)?;
    if *market.inner().owner != *serum_dex_program_id.key {
      return Err(ProtocolError::InvalidSerumDexMarketAccount);
    }
    if *bids.owner != *serum_dex_program_id.key {
      return Err(ProtocolError::InvalidSerumDexMarketAccount);
    }
    if *asks.owner != *serum_dex_program_id.key {
      return Err(ProtocolError::InvalidSerumDexMarketAccount);
    }
    if *event_q.owner != *serum_dex_program_id.key {
      return Err(ProtocolError::InvalidSerumDexMarketAccount);
    }
    Ok(Self {
      amm_info,
      authority,
      open_orders: SerumDexOpenOrders::new(open_orders_acc)?,
      target_orders: target_orders_acc,
      pool_token_coin: TokenAccount::new(pool_token_coin_acc)?,
      pool_token_pc: TokenAccount::new(pool_token_pc_acc)?,
      serum_dex_program_id,
      serum_market: market,
      bids,
      asks,
      event_q,
      coin_vault: TokenAccount::new(coin_vault_acc)?,
      pc_vault: TokenAccount::new(pc_vault_acc)?,
      vault_signer,
      program_id,
    })
  }

  // pub fn find_token_pair(
  //   &self,
  //   source_token_account_mint: &Pubkey,
  // ) -> ProtocolResult<(&TokenAccount<'a, 'b>, &TokenAccount<'a, 'b>)> {
  //   if *source_token_account_mint == self.token_a.mint()? {
  //     Ok((&self.token_a, &self.token_b))
  //   } else {
  //     Ok((&self.token_b, &self.token_a))
  //   }
  // }
}

#[derive(Copy, Clone)]
pub struct RaydiumSwapArgs2<'a, 'b: 'a> {
  pub amm_info: RaydiumAmmInfo<'a, 'b>,
  pub authority: &'a AccountInfo<'b>,
  pub open_orders: SerumDexOpenOrders<'a, 'b>,
  pub pool_token_coin: TokenAccount<'a, 'b>,
  pub pool_token_pc: TokenAccount<'a, 'b>,
  pub serum_dex_program_id: &'a AccountInfo<'b>,
  pub serum_market: SerumDexMarket<'a, 'b>,
  pub bids: &'a AccountInfo<'b>,
  pub asks: &'a AccountInfo<'b>,
  pub event_q: &'a AccountInfo<'b>,
  pub coin_vault: TokenAccount<'a, 'b>,
  pub pc_vault: TokenAccount<'a, 'b>,
  pub vault_signer: &'a AccountInfo<'b>,
  pub program_id: &'a AccountInfo<'b>,
}

impl<'a, 'b: 'a> RaydiumSwapArgs2<'a, 'b> {
  pub fn with_parsed_args(accounts: &'a [AccountInfo<'b>]) -> ProtocolResult<Self> {
    const MIN_ACCOUNTS: usize = 14;
    if accounts.len() != MIN_ACCOUNTS {
      return Err(ProtocolError::InvalidAccountsLength);
    }
    let &[
      ref amm_info_acc,
      ref authority,
      ref open_orders_acc,
      ref pool_token_coin_acc,
      ref pool_token_pc_acc,
      ref serum_dex_program_id,
      ref serum_market_acc,
      ref bids,
      ref asks,
      ref event_q,
      ref coin_vault_acc,
      ref pc_vault_acc,
      ref vault_signer,
      ref program_id,
    ]: &'a[AccountInfo<'b>; MIN_ACCOUNTS] = array_ref![accounts, 0, MIN_ACCOUNTS];

    if !amm_info_acc.is_writable {
      return Err(ProtocolError::ReadonlyAccount);
    }
    let amm_info = RaydiumAmmInfo::new(amm_info_acc)?;

    if amm_info.token_coin()? != *pool_token_coin_acc.key {
      return Err(ProtocolError::InvalidTokenAccount);
    }
    if amm_info.token_pc()? != *pool_token_pc_acc.key {
      return Err(ProtocolError::InvalidTokenAccount);
    }
    if amm_info.open_orders()? != *open_orders_acc.key {
      return Err(ProtocolError::InvalidRaydiumAmmInfoAccount);
    }
    if amm_info.market()? != *serum_market_acc.key {
      return Err(ProtocolError::InvalidRaydiumAmmInfoAccount);
    }
    if !open_orders_acc.is_writable {
      return Err(ProtocolError::ReadonlyAccount);
    }
    if amm_info.serum_dex()? != *serum_dex_program_id.key {
      return Err(ProtocolError::InvalidSerumDexProgramId);
    }

    let market = SerumDexMarket::new(serum_market_acc)?;
    if *market.inner().owner != *serum_dex_program_id.key {
      return Err(ProtocolError::InvalidSerumDexMarketAccount);
    }
    if *bids.owner != *serum_dex_program_id.key {
      return Err(ProtocolError::InvalidSerumDexMarketAccount);
    }
    if *asks.owner != *serum_dex_program_id.key {
      return Err(ProtocolError::InvalidSerumDexMarketAccount);
    }
    if *event_q.owner != *serum_dex_program_id.key {
      return Err(ProtocolError::InvalidSerumDexMarketAccount);
    }
    Ok(Self {
      amm_info,
      authority,
      open_orders: SerumDexOpenOrders::new(open_orders_acc)?,
      pool_token_coin: TokenAccount::new(pool_token_coin_acc)?,
      pool_token_pc: TokenAccount::new(pool_token_pc_acc)?,
      serum_dex_program_id,
      serum_market: market,
      bids,
      asks,
      event_q,
      coin_vault: TokenAccount::new(coin_vault_acc)?,
      pc_vault: TokenAccount::new(pc_vault_acc)?,
      vault_signer,
      program_id,
    })
  }

  // pub fn find_token_pair(
  //   &self,
  //   source_token_account_mint: &Pubkey,
  // ) -> ProtocolResult<(&TokenAccount<'a, 'b>, &TokenAccount<'a, 'b>)> {
  //   if *source_token_account_mint == self.token_a.mint()? {
  //     Ok((&self.token_a, &self.token_b))
  //   } else {
  //     Ok((&self.token_b, &self.token_a))
  //   }
  // }
}

#[cfg(test)]
mod tests {
  use super::*;
  use solana_sdk::{account_info::AccountInfo, pubkey::Pubkey};
  use std::str::FromStr;

  #[test]
  fn test_raydium_struct() {
    let raydium_program_id =
      Pubkey::from_str("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8").unwrap();
    let raydium_pubkey = Pubkey::from_str("DVa7Qmb5ct9RCpaU7UTpSaf3GVMYz17vNVU67XpdCRut").unwrap();
    let raydium_data =
      "Csa6r43w6Tksashc251QAkcpr6D4zyiWB4sSrw5xDZzoH9FsPfiZDXJSNMMTFHVsbKqVyDZb32anWxQN
Nk9FL7bCpKPZ7qMdCe6eCkjjRbbdiYvHBV1TrhWWwQ6pKP3rNVfae2R25Hj8ttD9CwVTz2CRzcDDdu88N5T6J67xVhcBKwEmJB3i
txbnWWnvHf95TBXbmmAZFrbfPm6153Re8mjTUVswfNCRVC2ypRV8jzZoBbohMWrbPxKW4VXZdaEE8JwVU5QrPFvKFJKkmeReiBre
b7Huy52gGioSCu8FLWg8JYQHMzgnr31tR5sDa1WSVJVPUQ4t4rRazqcdALsdSKZHUrnZACbLTsEgiXQWn4Ncc9eVciH78oQsXgvP
sWC4qSURfyQZoe7QUZ5pb6YtY5A4YASwim5JauPHVGdd6sLFTea3DK7RUdmpDcmyKbnQKBVE3mTMA6useCSrUtHChwpETDkTC1gh
EQtZQTVdefcPsAGLXEy3LioEqfnny3huwYxuTnT6LYt7KYP1FqqRoff7zQUvWn8xRq45pxWjbm3HLGimno7tCWYVRUwMH74vDfgg
7AebDUTdRA72GhBUG1Y2852URSs3crQ4qDs9z62AS2ymyMZ8Qicz9RmimyU9iCU8n96pZ7Y57XKydcW8aDKF1gBi3bdLDGyUAdYY
b51Jijykz38oM6KPswC7rAxgTVVgiMu4JvKmVwecn7NCP4iWoM9k8vrYaa8tS3VBZtAMCkVtuwpQeYVZ9HPZkwVPV9o6oFXBidkZ
aQukNQ7sfZSCEGj6vKv4fGJNpuDJDZiUXhveEjnbYffrm5Gnfz2kvSSdCgotWNJwcJZkfv5LsMkprfTXodEXXnLqqHj3LM8tNSFu
CqhMRFKbuHdZt1EfvFWcyxNukAhUXZn5k4MVNQdhQZ5poqMfUa6AzgXBMVAYCoFrsKF9qHbCEHFLNcznS3J3go3xcCnigQtQEctX
awtxg5yoJmS91iDZt2nTceatH7LN78fA5DxmJDn8kpF3F2";
    let mut raydium_data = bs58::decode(raydium_data.replace('\n', ""))
      .into_vec()
      .unwrap();
    let mut raydium_lamports = 6124800u64;
    let raydium_account_info = AccountInfo::new(
      &raydium_pubkey,
      false,
      true,
      &mut raydium_lamports,
      &mut raydium_data[..],
      &raydium_program_id,
      false,
      248,
    );
    let raydium_info = RaydiumAmmInfo::new(&raydium_account_info).unwrap();
    assert_eq!(*raydium_info.pubkey(), raydium_pubkey);
    assert_eq!(*raydium_info.inner().owner, raydium_program_id);
    assert_eq!(
      raydium_info.open_orders().unwrap().to_string(),
      "7UF3m8hDGZ6bNnHzaT2YHrhp7A7n9qFfBj6QEpHPv5S8"
    );
    assert_eq!(
      raydium_info.market().unwrap().to_string(),
      "teE55QrL4a4QSfydR9dnHF97jgCfptpuigbb53Lo95g"
    );
    assert_eq!(
      raydium_info.token_coin().unwrap().to_string(),
      "3wqhzSB9avepM9xMteiZnbJw75zmTBDVmPFLTQAGcSMN"
    );
    assert_eq!(
      raydium_info.token_pc().unwrap().to_string(),
      "5GtSbKJEPaoumrDzNj4kGkgZtfDyUceKaHrPziazALC1"
    );
    assert_eq!(
      raydium_info.coin_mint().unwrap().to_string(),
      "4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R"
    );
    assert_eq!(
      raydium_info.pc_mint().unwrap().to_string(),
      "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB"
    );
    assert_eq!(
      raydium_info.serum_dex().unwrap().to_string(),
      "9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin"
    );
  }
}
