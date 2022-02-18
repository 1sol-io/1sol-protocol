use super::base::{validate_authority_pubkey, TokenAccount};
use crate::{
  declare_validated_account_wrapper,
  error::{ProtocolError, ProtocolResult},
};
use arrayref::array_ref;
use solana_program::{account_info::AccountInfo, pubkey::Pubkey};

declare_validated_account_wrapper!(AldrinPool, |account: &AccountInfo| {
  let account_data = account
    .try_borrow_data()
    .map_err(|_| ProtocolError::BorrowAccountDataError)?;
  if account_data.len() != 472 {
    return Err(ProtocolError::InvalidCremaSwapAccountData);
  }
  if account_data[33] != 1 {
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

  pub fn authority(self) -> ProtocolResult<Pubkey> {
    let data = self
      .inner()
      .try_borrow_data()
      .map_err(|_| ProtocolError::BorrowAccountDataError)?;
    Ok(Pubkey::new_from_array(*array_ref![data, 240, 32]))
  }

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
    Ok(Pubkey::new_from_array(*array_ref![data, 104, 32]))
  }
}

#[derive(Copy, Clone)]
pub struct AldrinPoolArgs<'a, 'b: 'a> {
  pub pool_info: AldrinPool<'a, 'b>,
  pub authority: &'a AccountInfo<'b>,
  pub pool_coin_vault: TokenAccount<'a, 'b>,
  pub pool_pc_vault: TokenAccount<'a, 'b>,
  pub fee_account: &'a AccountInfo<'b>,
  pub curve: &'a AccountInfo<'b>,
  pub program_id: &'a AccountInfo<'b>,
}

impl<'a, 'b: 'a> AldrinPoolArgs<'a, 'b> {
  pub fn with_parsed_args(accounts: &'a [AccountInfo<'b>]) -> ProtocolResult<Self> {
    const MIN_ACCOUNTS: usize = 7;

    if accounts.len() != MIN_ACCOUNTS {
      return Err(ProtocolError::InvalidAccountsLength);
    }
    let &[
      ref pool_info_acc,
      ref authority,
      ref pool_coin_vault_acc,
      ref pool_pc_vault_acc,
      ref fee_account,
      ref curve,
      ref program_id,
    ]: &'a[AccountInfo<'b>; MIN_ACCOUNTS] = array_ref![accounts, 0, MIN_ACCOUNTS];

    let pool_info = AldrinPool::new(pool_info_acc)?;
    if !program_id.executable || *pool_info_acc.owner != *program_id.key {
      return Err(ProtocolError::InvalidProgramAddress);
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
      pool_coin_vault: TokenAccount::new(pool_coin_vault_acc)?,
      pool_pc_vault: TokenAccount::new(pool_pc_vault_acc)?,
      fee_account,
      curve,
      program_id,
    })
  }
}
