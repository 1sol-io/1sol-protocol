use super::base::{validate_authority_pubkey, TokenAccount};
use crate::{
  declare_validated_account_wrapper,
  error::{ProtocolError, ProtocolResult},
};
use arrayref::array_ref;
use solana_program::{account_info::AccountInfo, pubkey::Pubkey};

declare_validated_account_wrapper!(CremaSwapV1, |account: &AccountInfo| {
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

impl<'a, 'b: 'a> CremaSwapV1<'a, 'b> {
  pub fn nonce(self) -> ProtocolResult<u8> {
    Ok(
      self
        .inner()
        .try_borrow_data()
        .map_err(|_| ProtocolError::BorrowAccountDataError)?[34],
    )
  }

  pub fn token_a(self) -> ProtocolResult<Pubkey> {
    let data = self
      .inner()
      .try_borrow_data()
      .map_err(|_| ProtocolError::BorrowAccountDataError)?;
    Ok(Pubkey::new_from_array(*array_ref![data, 163, 32]))
  }

  pub fn token_b(self) -> ProtocolResult<Pubkey> {
    let data = self
      .inner()
      .try_borrow_data()
      .map_err(|_| ProtocolError::BorrowAccountDataError)?;
    Ok(Pubkey::new_from_array(*array_ref![data, 195, 32]))
  }

  pub fn token_a_mint(self) -> ProtocolResult<Pubkey> {
    let data = self
      .inner()
      .try_borrow_data()
      .map_err(|_| ProtocolError::BorrowAccountDataError)?;
    Ok(Pubkey::new_from_array(*array_ref![data, 227, 32]))
  }

  pub fn token_b_mint(self) -> ProtocolResult<Pubkey> {
    let data = self
      .inner()
      .try_borrow_data()
      .map_err(|_| ProtocolError::BorrowAccountDataError)?;
    Ok(Pubkey::new_from_array(*array_ref![data, 259, 32]))
  }
}

#[derive(Copy, Clone)]
pub struct CremaSwapV1Args<'a, 'b: 'a> {
  pub swap_info: CremaSwapV1<'a, 'b>,
  pub authority: &'a AccountInfo<'b>,
  pub pool_token_a: TokenAccount<'a, 'b>,
  pub pool_token_b: TokenAccount<'a, 'b>,
  pub tick_dst: &'a AccountInfo<'b>,
  pub program_id: &'a AccountInfo<'b>,
}

impl<'a, 'b: 'a> CremaSwapV1Args<'a, 'b> {
  pub fn with_parsed_args(accounts: &'a [AccountInfo<'b>]) -> ProtocolResult<Self> {
    const MIN_ACCOUNTS: usize = 6;

    if accounts.len() != MIN_ACCOUNTS {
      return Err(ProtocolError::InvalidAccountsLength);
    }
    let &[
      ref swap_info_acc,
      ref authority,
      ref pool_token_a_acc,
      ref pool_token_b_acc,
      ref tick_dst_acc,
      ref program_id,
    ]: &'a[AccountInfo<'b>; MIN_ACCOUNTS] = array_ref![accounts, 0, MIN_ACCOUNTS];

    if !swap_info_acc.is_writable {
      return Err(ProtocolError::ReadonlyAccount);
    }
    let swap_info = CremaSwapV1::new(swap_info_acc)?;
    if !program_id.executable || *swap_info_acc.owner != *program_id.key {
      return Err(ProtocolError::InvalidProgramAddress);
    }

    validate_authority_pubkey(
      authority.key,
      program_id.key,
      &swap_info_acc.key.to_bytes(),
      swap_info.nonce()?,
    )?;
    let swap_token_a = swap_info.token_a()?;
    let swap_token_b = swap_info.token_b()?;

    let (pool_token_a, pool_token_b) =
      if *pool_token_a_acc.key == swap_token_a && *pool_token_b_acc.key == swap_token_b {
        (
          TokenAccount::new(pool_token_a_acc)?,
          TokenAccount::new(pool_token_b_acc)?,
        )
      } else if *pool_token_a_acc.key == swap_token_b && *pool_token_b_acc.key == swap_token_a {
        (
          TokenAccount::new(pool_token_b_acc)?,
          TokenAccount::new(pool_token_a_acc)?,
        )
      } else {
        return Err(ProtocolError::InvalidTokenAccount);
      };

    Ok(Self {
      swap_info,
      authority,
      pool_token_a,
      pool_token_b,
      tick_dst: tick_dst_acc,
      program_id,
    })
  }

  pub fn find_token_pair(&self, source_mint_key: &Pubkey) -> ProtocolResult<(&TokenAccount<'a, 'b>, &TokenAccount<'a, 'b>)> {
    let pool_token_a_mint = self.swap_info.token_a_mint()?;
    let pool_token_b_mint = self.swap_info.token_b_mint()?;
    if *source_mint_key == pool_token_a_mint {
      return Ok((&self.pool_token_a, &self.pool_token_b))
    } else if *source_mint_key == pool_token_b_mint {
      return Ok((&self.pool_token_b, &self.pool_token_a))
    }
    Err(ProtocolError::InvalidTokenMint)
  }
}
