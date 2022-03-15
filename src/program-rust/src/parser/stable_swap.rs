use crate::{
  declare_validated_account_wrapper,
  error::{ProtocolError, ProtocolResult},
};
use arrayref::{array_ref, array_refs};
use solana_program::{account_info::AccountInfo, pubkey::Pubkey};

use super::base::TokenAccount;

declare_validated_account_wrapper!(StableSwapInfo, |account: &AccountInfo| {
  let data = account
    .try_borrow_data()
    .map_err(|_| ProtocolError::BorrowAccountDataError)?;
  if data.len() != 395 {
    return Err(ProtocolError::InvalidStableSwapAccount);
  }
  let is_initialized = data[0];
  if is_initialized != 1u8 {
    return Err(ProtocolError::InvalidStableSwapAccountState);
  }
  let is_paused = data[1];
  if is_paused == 1u8 {
    return Err(ProtocolError::InvalidStableSwapAccountState);
  }
  Ok(())
});

#[allow(dead_code)]
impl<'a, 'b: 'a> StableSwapInfo<'a, 'b> {
  pub fn token_a(self) -> ProtocolResult<Pubkey> {
    let data = self
      .inner()
      .try_borrow_data()
      .map_err(|_| ProtocolError::BorrowAccountDataError)?;
    Ok(Pubkey::new_from_array(*array_ref![data, 107, 32]))
  }

  pub fn token_b(self) -> ProtocolResult<Pubkey> {
    let data = self
      .inner()
      .try_borrow_data()
      .map_err(|_| ProtocolError::BorrowAccountDataError)?;
    Ok(Pubkey::new_from_array(*array_ref![data, 139, 32]))
  }

  pub fn token_a_mint(self) -> ProtocolResult<Pubkey> {
    let data = self
      .inner()
      .try_borrow_data()
      .map_err(|_| ProtocolError::BorrowAccountDataError)?;
    Ok(Pubkey::new_from_array(*array_ref![data, 203, 32]))
  }

  pub fn token_b_mint(self) -> ProtocolResult<Pubkey> {
    let data = self
      .inner()
      .try_borrow_data()
      .map_err(|_| ProtocolError::BorrowAccountDataError)?;
    Ok(Pubkey::new_from_array(*array_ref![data, 235, 32]))
  }
  pub fn admin_fee_key_a(self) -> ProtocolResult<Pubkey> {
    let data = self
      .inner()
      .try_borrow_data()
      .map_err(|_| ProtocolError::BorrowAccountDataError)?;
    Ok(Pubkey::new_from_array(*array_ref![data, 267, 32]))
  }

  pub fn admin_fee_key_b(self) -> ProtocolResult<Pubkey> {
    let data = self
      .inner()
      .try_borrow_data()
      .map_err(|_| ProtocolError::BorrowAccountDataError)?;
    Ok(Pubkey::new_from_array(*array_ref![data, 299, 32]))
  }

  pub fn nonce(self) -> ProtocolResult<u8> {
    let data = self
      .inner()
      .try_borrow_data()
      .map_err(|_| ProtocolError::BorrowAccountDataError)?;
    Ok(data[2])
  }
}

#[derive(Copy, Clone)]
pub struct StableSwapArgs<'a, 'b: 'a> {
  pub swap_info: StableSwapInfo<'a, 'b>,
  pub authority_acc: &'a AccountInfo<'b>,
  pub token_a: TokenAccount<'a, 'b>,
  pub token_b: TokenAccount<'a, 'b>,
  pub admin_fee_acc: &'a AccountInfo<'b>,
  pub program_acc: &'a AccountInfo<'b>,
}

impl<'a, 'b: 'a> StableSwapArgs<'a, 'b> {
  const MIN_ACCOUNTS: usize = 6;

  pub fn with_parsed_args(accounts: &'a [AccountInfo<'b>]) -> ProtocolResult<Self> {
    if accounts.len() < Self::MIN_ACCOUNTS {
      return Err(ProtocolError::InvalidAccountsLength);
    }
    #[allow(clippy::ptr_offset_with_cast)]
    let (fixed_accounts, other_accounts) = array_refs![accounts, 5; ..;];

    let &[
      ref swap_info_acc,
      ref authority_acc,
      ref token_a_acc,
      ref token_b_acc,
      ref admin_fee_acc,
      //ref clock_sysvar_acc,
      //ref program_acc,
    ]: &'a[AccountInfo<'b>; 5] = array_ref![fixed_accounts, 0, 5];

    let program_acc = if other_accounts.len() == 1 {
      other_accounts.get(0).unwrap()
    } else {
      other_accounts.get(1).unwrap()
    };

    let swap_info = StableSwapInfo::new(swap_info_acc)?;

    if swap_info.token_a()? != *token_a_acc.key {
      return Err(ProtocolError::InvalidTokenAccount);
    }
    if swap_info.token_b()? != *token_b_acc.key {
      return Err(ProtocolError::InvalidTokenAccount);
    }
    if !(swap_info.admin_fee_key_a()? == *admin_fee_acc.key
      || swap_info.admin_fee_key_b()? == *admin_fee_acc.key)
    {
      return Err(ProtocolError::InvalidStableSwapAccount);
    }

    // validate_authority_pubkey(
    //   authority_acc.key,
    //   program_acc.key,
    //   &swap_info_acc.key.to_bytes(),
    //   swap_info.nonce()?,
    // )?;

    Ok(StableSwapArgs {
      swap_info,
      authority_acc,
      token_a: TokenAccount::new(token_a_acc)?,
      token_b: TokenAccount::new(token_b_acc)?,
      admin_fee_acc,
      program_acc,
    })
  }

  pub fn find_token_pair(
    &self,
    source_token_account_mint: &Pubkey,
  ) -> ProtocolResult<(&TokenAccount<'a, 'b>, &TokenAccount<'a, 'b>)> {
    if *source_token_account_mint == self.token_a.mint()? {
      Ok((&self.token_a, &self.token_b))
    } else {
      Ok((&self.token_b, &self.token_a))
    }
  }
}
