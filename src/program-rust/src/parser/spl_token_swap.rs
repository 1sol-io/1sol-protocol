use arrayref::array_refs;
use solana_program::{account_info::AccountInfo, msg, pubkey::Pubkey};

use crate::{
  check_unreachable, declare_validated_account_wrapper,
  error::{ProtocolError, ProtocolResult},
};

use super::base::{TokenAccount, TokenMint};

declare_validated_account_wrapper!(SplTokenSwapInfo, |account: &AccountInfo| {
  let data = account
    .try_borrow_data()
    .map_err(|_| ProtocolError::BorrowAccountDataError)?;
  if data.len() != 324 {
    msg!(
      "spl-tokenswap-info, data.len(): {}, is_initialized: {}",
      data.len(),
      data[1]
    );
    return Err(ProtocolError::InvalidSplTokenSwapInfoAccount);
  }
  let version = data[0];
  if version != 1u8 {
    msg!("spl-tokenswap-info, version: {}", data[0]);
    return Err(ProtocolError::InvalidSplTokenSwapInfoAccount);
  }
  let is_initialized = data[1];
  if is_initialized != 1u8 {
    msg!(
      "spl-tokenswap-info, data.len(): {}, is_initialized: {}",
      data.len(),
      data[1]
    );
    return Err(ProtocolError::InvalidSplTokenSwapInfoAccount);
  }
  Ok(())
});

impl<'a, 'b: 'a> SplTokenSwapInfo<'a, 'b> {}

#[derive(Copy, Clone)]
pub struct SplTokenSwapArgs<'a, 'b: 'a> {
  pub swap_info: SplTokenSwapInfo<'a, 'b>,
  pub authority_acc_info: &'a AccountInfo<'b>,
  pub token_a_account: TokenAccount<'a, 'b>,
  pub token_b_account: TokenAccount<'a, 'b>,
  pub pool_mint: TokenMint<'a, 'b>,
  pub fee_account: TokenAccount<'a, 'b>,
  pub program: &'a AccountInfo<'b>,
  pub host_fee_account: Option<TokenAccount<'a, 'b>>,
}

impl<'a, 'b: 'a> SplTokenSwapArgs<'a, 'b> {
  pub fn with_parsed_args(accounts: &'a [AccountInfo<'b>]) -> ProtocolResult<Self> {
    const MIN_ACCOUNTS: usize = 7;
    if !(accounts.len() == MIN_ACCOUNTS || accounts.len() == MIN_ACCOUNTS + 1) {
      return Err(ProtocolError::InvalidAccountsLength);
    }
    #[allow(clippy::ptr_offset_with_cast)]
    let (fixed_accounts, host_fee_account): (
      &'a [AccountInfo<'b>; MIN_ACCOUNTS],
      &'a [AccountInfo<'b>],
    ) = array_refs![accounts, MIN_ACCOUNTS; .. ;];
    let &[
      ref swap_info_acc,
      ref authority_acc,
      ref token_a_acc,
      ref token_b_acc,
      ref pool_mint_acc,
      ref fee_acc,
      ref program_acc,
    ]: &'a [AccountInfo<'b>; MIN_ACCOUNTS] = fixed_accounts;
    let host_fee_acc = match host_fee_account {
      [] => None,
      [ref acc] => Some(TokenAccount::new(acc)?),
      _ => check_unreachable!()?,
    };
    let swap_info = SplTokenSwapInfo::new(swap_info_acc)?;
    if *swap_info.inner().owner != *program_acc.key {
      return Err(ProtocolError::InvalidProgramAddress);
    }
    // other checks will run in spl-token-swap
    Ok(SplTokenSwapArgs {
      swap_info,
      authority_acc_info: authority_acc,
      token_a_account: TokenAccount::new(token_a_acc)?,
      token_b_account: TokenAccount::new(token_b_acc)?,
      pool_mint: TokenMint::new(pool_mint_acc)?,
      fee_account: TokenAccount::new(fee_acc)?,
      program: program_acc,
      host_fee_account: host_fee_acc,
    })
  }

  pub fn find_token_pair(
    &self,
    source_token_account_mint: &Pubkey,
  ) -> ProtocolResult<(&TokenAccount<'a, 'b>, &TokenAccount<'a, 'b>)> {
    if *source_token_account_mint == self.token_a_account.mint()? {
      Ok((&self.token_a_account, &self.token_b_account))
    } else {
      Ok((&self.token_b_account, &self.token_a_account))
    }
  }
}
