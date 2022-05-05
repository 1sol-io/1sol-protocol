use super::base::TokenAccount;
use crate::{
  declare_validated_account_wrapper,
  error::{ProtocolError, ProtocolResult},
};
use arrayref::array_ref;
use solana_program::{account_info::AccountInfo, pubkey::Pubkey};

declare_validated_account_wrapper!(WhirlpoolInfo, |account: &AccountInfo| {
  let account_data = account
    .try_borrow_data()
    .map_err(|_| ProtocolError::BorrowAccountDataError)?;
  if account_data.len() != 653 {
    return Err(ProtocolError::InvalidCremaSwapAccountData);
  }
  Ok(())
});

impl<'a, 'b: 'a> WhirlpoolInfo<'a, 'b> {
  #[allow(dead_code)]
  pub fn bump(self) -> ProtocolResult<u8> {
    Ok(
      self
        .inner()
        .try_borrow_data()
        .map_err(|_| ProtocolError::BorrowAccountDataError)?[40],
    )
  }

  pub fn token_a_account(self) -> ProtocolResult<Pubkey> {
    let data = self
      .inner()
      .try_borrow_data()
      .map_err(|_| ProtocolError::BorrowAccountDataError)?;
    Ok(Pubkey::new_from_array(*array_ref![data, 133, 32]))
  }

  pub fn token_b_account(self) -> ProtocolResult<Pubkey> {
    let data = self
      .inner()
      .try_borrow_data()
      .map_err(|_| ProtocolError::BorrowAccountDataError)?;
    Ok(Pubkey::new_from_array(*array_ref![data, 213, 32]))
  }

  pub fn token_a_mint(self) -> ProtocolResult<Pubkey> {
    let data = self
      .inner()
      .try_borrow_data()
      .map_err(|_| ProtocolError::BorrowAccountDataError)?;
    Ok(Pubkey::new_from_array(*array_ref![data, 101, 32]))
  }

  pub fn token_b_mint(self) -> ProtocolResult<Pubkey> {
    let data = self
      .inner()
      .try_borrow_data()
      .map_err(|_| ProtocolError::BorrowAccountDataError)?;
    Ok(Pubkey::new_from_array(*array_ref![data, 181, 32]))
  }
}

#[derive(Copy, Clone)]
pub struct WhirlpoolArgs<'a, 'b: 'a> {
  pub pool: WhirlpoolInfo<'a, 'b>,
  pub token_a_account: TokenAccount<'a, 'b>,
  pub token_b_account: TokenAccount<'a, 'b>,
  pub tick_array_0: &'a AccountInfo<'b>,
  pub tick_array_1: &'a AccountInfo<'b>,
  pub tick_array_2: &'a AccountInfo<'b>,
  pub oracle: &'a AccountInfo<'b>,
  pub program_id: &'a AccountInfo<'b>,
}

impl<'a, 'b: 'a> WhirlpoolArgs<'a, 'b> {
  const MIN_ACCOUNTS: usize = 8;

  pub fn with_parsed_args(accounts: &'a [AccountInfo<'b>]) -> ProtocolResult<Self> {
    if accounts.len() != Self::MIN_ACCOUNTS {
      return Err(ProtocolError::InvalidAccountsLength);
    }
    let &[
      ref pool_acc,
      ref token_a_account,
      ref token_b_account,
      ref tick_array_0,
      ref tick_array_1,
      ref tick_array_2,
      ref oracle,
      ref program_id,
    ]: &'a[AccountInfo<'b>; WhirlpoolArgs::MIN_ACCOUNTS] = array_ref![accounts, 0, WhirlpoolArgs::MIN_ACCOUNTS];

    let pool = WhirlpoolInfo::new(pool_acc)?;
    if !program_id.executable || *pool_acc.owner != *program_id.key {
      return Err(ProtocolError::InvalidProgramAddress);
    }

    let token_1_account = TokenAccount::new(token_a_account)?;
    let token_2_account = TokenAccount::new(token_b_account)?;

    let pool_token_a = pool.token_a_account()?;
    let pool_token_b = pool.token_b_account()?;

    // auto invert vault token account
    let (token_a_account, token_b_account) = if *token_1_account.pubkey() == pool_token_a
      && *token_2_account.pubkey() == pool_token_b
    {
      (token_1_account, token_2_account)
    } else if *token_1_account.pubkey() == pool_token_b && *token_2_account.pubkey() == pool_token_a
    {
      (token_2_account, token_1_account)
    } else {
      return Err(ProtocolError::InvalidTokenMint);
    };

    Ok(Self {
      pool,
      token_a_account,
      token_b_account,
      tick_array_0,
      tick_array_1,
      tick_array_2,
      oracle,
      program_id,
    })
  }

  pub fn unpack_input(input: &[u8]) -> ProtocolResult<u128> {
    if input.len() < 16 {
      return Err(ProtocolError::InvalidInput);
    }
    let &sqrt_price_limit = array_ref![input, 0, 16];
    Ok(u128::from_le_bytes(sqrt_price_limit))
  }
}

#[cfg(test)]
mod test {
  use super::*;
  use solana_program::account_info::IntoAccountInfo;
  use solana_sdk::account::Account;
  use std::str::FromStr;

  #[test]
  fn test_whirlpool_info() {
    let pubkey = Pubkey::from_str("4fuUiYxTQ6QCrdSq9ouBYcTM7bqSwYTSyLueGZLTy4T4").unwrap();
    let program_id = Pubkey::from_str("whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc").unwrap();
    let account_data = "6mtVVAzzExTMtcc3avxFq9Ku8rwbiyuFe4fCqF3YaBFk4qBvzVWz5cbEWmgSqWqREa4MQUzikiPtU9nqRMA3taKkZgedKuAL
ntnGX9SLW2fBkui1M6ms2EoEyWcAFk2g5U2NQs116o6Dd5XX1PB9Wdr462MFWPKZZcaDW96hXb8kbjP6GG1hZoTZsLvVkFjM9RoqR81kcncEDPNrKv96b3Rh
WFgGZVMobs7kQh9jPeRNmzBynjDpSvp6328ubJD5jwSyjV1U2ecNVpwS8NtfFJxxN5KsLCtCvJk1WFJ1YJbWaS9WZigdcx7WRHquU6BpVPi7T9daJb4GHhP3
1hVsVLTX3i2M8UMPHCUXTRyeHLBDLaiM1GY85C7MdRMpy5YLbbvEvzQ1w3CTz9HmcRRE3SX3SLKNb7YFviYLurx1YPHXnFafEdf9wGQX7qrnYbKP9FqD2SsJ
fAdbp8XaN345172gxfjQk574EWxvanVwqYyyzZoYtYDXV2BKDdkjUUBGRBmfugHeiAEZsThxkEQdpKSUfcTcHfAgohF6E1fXHXC9rkmfB6pHbZNeyAuvkDLN
FTY6QQAQVzCdzqKuxijaapH15JRaaWABY78MijEpvuBsT3QcsrL78adXw8mWfgGWk6tGS23GdveVYv5kSGaFccmrH76CZoJimUNpztCXDPfuBXSYYMnfTsFc
U7BjCt9v4waWk6m64oSjjcy9Rn3fBhmFD8LejB84TFs21uutoYMNetXXpgpu9rRdjSNiyRrH1RAgnRHb6cR5FpjDoFsPygQB1nKUcgGrSXP9PafVCSn6zpJE
N6NCg4jeSqKAij88mkmmPeCRVhFKiN9HLZXc6LXDnNikbR6bxizcUNQ7joRcRDRKU7H7oVLCv8Yf";
    let mut test_account = Account {
      lamports: 5435760,
      data: bs58::decode(account_data.replace('\n', ""))
        .into_vec()
        .unwrap(),
      owner: program_id,
      executable: false,
      rent_epoch: 302,
    };
    let account_info = (&pubkey, &mut test_account).into_account_info();
    assert!(*account_info.key == pubkey);
    let pool = WhirlpoolInfo::new(&account_info).unwrap();
    assert_eq!(pool.bump().unwrap(), 254);
    assert_eq!(
      pool.token_a_account().unwrap(),
      Pubkey::from_str("4oY1eVHJrt7ywuFoQnAZwto4qcQip1QhYMAhD11PU4QL").unwrap()
    );
    assert_eq!(
      pool.token_b_account().unwrap(),
      Pubkey::from_str("4dSG9tKHZR4CAictyEnH9XuGZyKapodWXq5xyg7uFwE9").unwrap()
    );
  }
}
