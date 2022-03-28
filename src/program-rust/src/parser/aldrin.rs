use super::base::{validate_authority_pubkey, TokenAccount, TokenMint};
use crate::{
  declare_validated_account_wrapper,
  error::{ProtocolError, ProtocolResult},
  exchanger::aldrin::instruction::Side,
};
use arrayref::array_ref;
use solana_program::{account_info::AccountInfo, msg, pubkey::Pubkey};

declare_validated_account_wrapper!(AldrinPool, |account: &AccountInfo| {
  let account_data = account
    .try_borrow_data()
    .map_err(|_| ProtocolError::BorrowAccountDataError)?;
  if account_data.len() != 474 {
    return Err(ProtocolError::InvalidCremaSwapAccountData);
  }
  Ok(())
});

impl<'a, 'b: 'a> AldrinPool<'a, 'b> {
  pub fn nonce(self) -> ProtocolResult<u8> {
    Ok(
      self
        .inner()
        .try_borrow_data()
        .map_err(|_| ProtocolError::BorrowAccountDataError)?[232],
    )
  }

  // pub fn authority(self) -> ProtocolResult<Pubkey> {
  //   let data = self
  //     .inner()
  //     .try_borrow_data()
  //     .map_err(|_| ProtocolError::BorrowAccountDataError)?;
  //   Ok(Pubkey::new_from_array(*array_ref![data, 240, 32]))
  // }

  // pub fn pool_signer(self) -> ProtocolResult<Pubkey> {
  //   let data = self
  //     .inner()
  //     .try_borrow_data()
  //     .map_err(|_| ProtocolError::BorrowAccountDataError)?;
  //   Ok(Pubkey::new_from_array(*array_ref![data, 200, 32]))
  // }

  pub fn coin_vault(self) -> ProtocolResult<Pubkey> {
    let data = self
      .inner()
      .try_borrow_data()
      .map_err(|_| ProtocolError::BorrowAccountDataError)?;
    Ok(Pubkey::new_from_array(*array_ref![data, 72, 32]))
  }

  pub fn pc_vault(self) -> ProtocolResult<Pubkey> {
    let data = self
      .inner()
      .try_borrow_data()
      .map_err(|_| ProtocolError::BorrowAccountDataError)?;
    Ok(Pubkey::new_from_array(*array_ref![data, 136, 32]))
  }

  pub fn coin_mint(self) -> ProtocolResult<Pubkey> {
    let data = self
      .inner()
      .try_borrow_data()
      .map_err(|_| ProtocolError::BorrowAccountDataError)?;
    Ok(Pubkey::new_from_array(*array_ref![data, 104, 32]))
  }

  pub fn pc_mint(self) -> ProtocolResult<Pubkey> {
    let data = self
      .inner()
      .try_borrow_data()
      .map_err(|_| ProtocolError::BorrowAccountDataError)?;
    Ok(Pubkey::new_from_array(*array_ref![data, 168, 32]))
  }

  pub fn pool_mint(self) -> ProtocolResult<Pubkey> {
    let data = self
      .inner()
      .try_borrow_data()
      .map_err(|_| ProtocolError::BorrowAccountDataError)?;
    Ok(Pubkey::new_from_array(*array_ref![data, 40, 32]))
  }
}

#[derive(Copy, Clone)]
pub struct AldrinPoolArgs<'a, 'b: 'a> {
  pub pool_info: AldrinPool<'a, 'b>,
  pub authority: &'a AccountInfo<'b>,
  pub pool_mint: TokenMint<'a, 'b>,
  pub pool_coin_vault: TokenAccount<'a, 'b>,
  pub pool_pc_vault: TokenAccount<'a, 'b>,
  pub fee_account: &'a AccountInfo<'b>,
  pub curve_key: &'a AccountInfo<'b>,
  pub program_id: &'a AccountInfo<'b>,
}

impl<'a, 'b: 'a> AldrinPoolArgs<'a, 'b> {
  pub fn with_parsed_args(accounts: &'a [AccountInfo<'b>]) -> ProtocolResult<Self> {
    const MIN_ACCOUNTS: usize = 8;

    if accounts.len() != MIN_ACCOUNTS {
      return Err(ProtocolError::InvalidAccountsLength);
    }
    let &[
      ref pool_info_acc,
      ref authority,
      ref pool_mint_acc,
      ref pool_coin_vault_acc,
      ref pool_pc_vault_acc,
      ref fee_account,
      ref curve_key,
      ref program_id,
    ]: &'a[AccountInfo<'b>; MIN_ACCOUNTS] = array_ref![accounts, 0, MIN_ACCOUNTS];

    let pool_info = AldrinPool::new(pool_info_acc)?;
    if !program_id.executable || *pool_info_acc.owner != *program_id.key {
      return Err(ProtocolError::InvalidProgramAddress);
    }

    if *pool_mint_acc.key != pool_info.pool_mint()? {
      return Err(ProtocolError::InvalidTokenMint);
    }
    let pool_vault_1 = TokenAccount::new(pool_coin_vault_acc)?;
    let pool_vault_2 = TokenAccount::new(pool_pc_vault_acc)?;

    let coin_mint = pool_info.coin_mint()?;
    let pc_mint = pool_info.pc_mint()?;

    // auto invert vault token account
    let (coin_vault, pc_vault) =
      if pool_vault_1.mint()? == coin_mint && pool_vault_2.mint()? == pc_mint {
        (pool_vault_1, pool_vault_2)
      } else if pool_vault_1.mint()? == pc_mint && pool_vault_2.mint()? == coin_mint {
        (pool_vault_2, pool_vault_1)
      } else {
        return Err(ProtocolError::InvalidTokenMint);
      };

    if *coin_vault.inner().key != pool_info.coin_vault()? {
      msg!(
        "coin_vault got {}, expect: {}",
        coin_vault.inner().key,
        pool_info.coin_vault()?
      );
      return Err(ProtocolError::InvalidTokenAccount);
    }

    if *pc_vault.inner().key != pool_info.pc_vault()? {
      msg!(
        "pc_vault got {}, expect: {}",
        pc_vault.inner().key,
        pool_info.pc_vault()?
      );
      return Err(ProtocolError::InvalidTokenAccount);
    }

    validate_authority_pubkey(
      authority.key,
      program_id.key,
      &pool_info_acc.key.to_bytes(),
      pool_info.nonce()?,
    )?;

    Ok(Self {
      pool_info,
      authority,
      pool_mint: TokenMint::new(pool_mint_acc)?,
      pool_coin_vault: coin_vault,
      pool_pc_vault: pc_vault,
      fee_account,
      curve_key,
      program_id,
    })
  }

  pub fn find_side(&self, source_mint: &Pubkey) -> ProtocolResult<Side> {
    let coin_mint = self.pool_info.coin_mint()?;
    if *source_mint == coin_mint {
      Ok(Side::Ask)
    } else {
      Ok(Side::Bid)
    }
  }
}

#[cfg(test)]
mod test {
  use super::*;
  use bs58;
  use solana_sdk::{account::Account, account_info::IntoAccountInfo};
  use std::str::FromStr;

  #[test]
  pub fn test_parse_aldrin_pool_info() {
    let pubkey = Pubkey::from_str("HjZ2zgg4HemPREiJ7he3VWqV6bHV5yhLNkMJahwigbzz").unwrap();
    let program_id = Pubkey::from_str("CURVGoZn8zycx6FXwwevgBTB2gVvdbGTEpvMJDbgs2t4").unwrap();
    let account_data = "4VMpc88zcKRkaAdBUkKmF6xLE8umnyqnBceSPeNcGdFdcHMnVMfjmCnJWCG3dVxvE8LzuisKJDY5cRkFwCpfWM7NtKSKc3dw
aqfLsNsLiR9TZ5gQrptWrv3DKD4GeTc9iUv8ftfEjKARGg4zGhpMzbDWFnsZtL19VU94iCuRfTaspLKEdpAW7qp7KhA3xM4YWBA4d2iPBu1cuQFMiAMocXU4
9YqBeEhajTLbcckBXsnYgN5KhWmcFtRwgzKSEuG3nnu8HDpx5ze8EW1PzYGg2mCsx4KnMUh7prqW2YKuXnrcwBwfe1PMDKdTxCrY17r9tPmaQ3vR4xv7RJA9
GdLrPf1C84LpFgUkbJ72DEgL7PyiXF2tuJofzrt8PXxzWHjL3YPcbJyNtaEWEmem4HwMbw6JYing6X422pLnXAb1zeyGnE1oM4s7d7MFysZ1FMfpWgYJvaD7
11EtACBHwDCPbbcwi588Pdgu1SCHyzCMyX8t8RnShRJGPTwrfDLizxxTQxHTAXRCSMPtJ4RnBYLUwwxCgcPUYRJiFWpV7CuFWMNTxw2rs8skuvYPFh1fj7E2
dVmzXNyJydYDyCE8ntSuc6NQJAnmNYnpMueCof7KfJWJuxVbkZ2jKyWMe349VHLS28sd1Kon";
    let mut test_account = Account {
      lamports: 4189920,
      data: bs58::decode(account_data.replace('\n', ""))
        .into_vec()
        .unwrap(),
      owner: program_id,
      executable: false,
      rent_epoch: 281,
    };
    let account_info = (&pubkey, &mut test_account).into_account_info();
    assert!(*account_info.key == pubkey);
    let c = AldrinPool::new(&account_info).unwrap();
    assert_eq!(
      c.coin_vault().unwrap().to_string(),
      "7auinu2nWzoYfp8NWjGmHrhZgv5X4T9GCfHGT5TRj8tm".to_string()
    );
    assert_eq!(
      c.coin_mint().unwrap().to_string(),
      "7zhbkbKpGaUsJW7AD4yyAfGGoy53Xx2H3Ai5BKcwGKHw".to_string()
    );
    assert_eq!(
      c.pc_vault().unwrap().to_string(),
      "C1j8rurz8aTkjCReDwaHFu6M6j1h8eBEAGrvoMfccwLS".to_string()
    );
    assert_eq!(
      c.pc_mint().unwrap().to_string(),
      "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string()
    );
    assert_eq!(
      c.pool_mint().unwrap().to_string(),
      "8CFVFQGtqdnuYgypdJ2YwexRuaFs9KUPSyP6Gd5LtTqL".to_string()
    );
    assert_eq!(c.nonce().unwrap(), 252);
  }
}
