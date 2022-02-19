use super::base::{validate_authority_pubkey, TokenAccount, TokenMint};
use crate::{
  declare_validated_account_wrapper,
  error::{ProtocolError, ProtocolResult},
  swappers::aldrin::instruction::Side,
};
use arrayref::array_ref;
use solana_program::{account_info::AccountInfo, msg, pubkey::Pubkey};

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
