use std::cell::RefMut;

use crate::{
  check_unreachable,
  error::{ProtocolError, ProtocolResult},
  state::AmmInfo,
};
use arrayref::{array_ref, array_refs};
use serum_dex::{matching::Side as DexSide, state::AccountFlag};
use solana_program::{account_info::AccountInfo, msg, program_pack::Pack, pubkey::Pubkey, sysvar};

macro_rules! declare_validated_account_wrapper {
  ($WrapperT:ident, $validate:expr $(, $a:ident : $t:ty)*) => {
      #[derive(Copy, Clone)]
      pub struct $WrapperT<'a, 'b: 'a>(&'a AccountInfo<'b>);
      impl<'a, 'b: 'a> $WrapperT<'a, 'b> {
          #[allow(unused)]
          pub fn new(account: &'a AccountInfo<'b> $(,$a: $t)*) -> ProtocolResult<Self> {
              let validate_result: ProtocolResult = $validate(account $(,$a)*);
              validate_result?;
              Ok($WrapperT(account))
          }

          #[inline(always)]
          #[allow(unused)]
          pub fn inner(self) -> &'a AccountInfo<'b> {
              self.0
          }

          #[inline(always)]
          #[allow(unused)]
          pub fn pubkey(self) -> &'b Pubkey {
            self.0.key
          }
      }
  }
}

declare_validated_account_wrapper!(SplTokenProgram, |account: &AccountInfo| {
  if *account.key != spl_token::ID {
    return Err(ProtocolError::IncorrectTokenProgramId);
  };
  Ok(())
});

declare_validated_account_wrapper!(TokenAccount, |account: &AccountInfo| {
  if *account.owner != spl_token::ID {
    return Err(ProtocolError::InvalidTokenAccount);
  }
  let data = account
    .try_borrow_data()
    .map_err(|_| ProtocolError::BorrowAccountDataError)?;
  if data.len() != spl_token::state::Account::LEN {
    return Err(ProtocolError::InvalidTokenAccount);
  };
  let is_initialized = data[0x6c];
  if is_initialized != 1u8 {
    return Err(ProtocolError::InvalidTokenAccount);
  };
  Ok(())
});

declare_validated_account_wrapper!(TokenMint, |mint: &AccountInfo| {
  if *mint.owner != spl_token::ID {
    return Err(ProtocolError::InvalidTokenMint);
  };
  let data = mint
    .try_borrow_data()
    .map_err(|_| ProtocolError::BorrowAccountDataError)?;
  if data.len() != spl_token::state::Mint::LEN {
    return Err(ProtocolError::InvalidTokenMint);
  };
  let is_initialized = data[0x2d];
  if is_initialized != 1u8 {
    return Err(ProtocolError::InvalidTokenMint);
  };
  Ok(())
});

declare_validated_account_wrapper!(SignerAccount, |account: &AccountInfo| {
  if !account.is_signer {
    return Err(ProtocolError::InvalidSignerAccount);
  }
  Ok(())
});

declare_validated_account_wrapper!(SysClockAccount, |account: &AccountInfo| {
  if *account.key != sysvar::clock::id() {
    return Err(ProtocolError::InvalidClockAccount);
  }
  Ok(())
});

declare_validated_account_wrapper!(SplTokenSwapInfo, |account: &AccountInfo| {
  let data = account
    .try_borrow_data()
    .map_err(|_| ProtocolError::BorrowAccountDataError)?;
  if data.len() != 324 {
    msg!(
      "spl-tokenswap-info, data.len(): {}, is_initialized: {}",
      data.len(),
      data[0]
    );
    return Err(ProtocolError::InvalidSplTokenSwapInfoAccount);
  }
  let is_initialized = data[0];
  if is_initialized != 1u8 {
    msg!(
      "spl-tokenswap-info, data.len(): {}, is_initialized: {}",
      data.len(),
      data[0]
    );
    return Err(ProtocolError::InvalidSplTokenSwapInfoAccount);
  }
  Ok(())
});

declare_validated_account_wrapper!(SerumDexMarket, |account: &AccountInfo| {
  if !account.is_writable {
    return Err(ProtocolError::ReadableAccount);
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
  // BitFlags::
  // if flag_data != 3768656749939 {
  if flag_data != (AccountFlag::Initialized | AccountFlag::Market).bits() {
    msg!(
      "flag_data: {:?}, expect: {:?}",
      flag_data,
      (AccountFlag::Initialized | AccountFlag::Market).bits()
    );
    return Err(ProtocolError::InvalidSerumDexMarketAccount);
  }
  Ok(())
});

#[allow(unused)]
fn unpack_coption_key(src: &[u8; 36]) -> ProtocolResult<Option<Pubkey>> {
  let (tag, body) = array_refs![src, 4, 32];
  match *tag {
    [0, 0, 0, 0] => Ok(None),
    [1, 0, 0, 0] => Ok(Some(Pubkey::new_from_array(*body))),
    _ => Err(ProtocolError::InvalidAccountData),
  }
}

/// Calculates the authority id by generating a program address.
pub fn validate_authority_pubkey(
  authority: &Pubkey,
  program_id: &Pubkey,
  info: &Pubkey,
  nonce: u8,
) -> Result<(), ProtocolError> {
  let key = Pubkey::create_program_address(&[&info.to_bytes()[..32], &[nonce]], program_id)
    .map_err(|_| ProtocolError::InvalidProgramAddress)?;
  if key != *authority {
    return Err(ProtocolError::InvalidAuthority);
  }
  Ok(())
}

#[allow(unused)]
impl<'a, 'b: 'a> TokenAccount<'a, 'b> {
  pub fn balance(self) -> ProtocolResult<u64> {
    let data = self
      .inner()
      .try_borrow_data()
      .map_err(|_| ProtocolError::BorrowAccountDataError)?;
    Ok(u64::from_le_bytes(*array_ref![data, 64, 8]))
  }
  pub fn mint(self) -> ProtocolResult<Pubkey> {
    let data = self
      .inner()
      .try_borrow_data()
      .map_err(|_| ProtocolError::BorrowAccountDataError)?;
    Ok(Pubkey::new_from_array(*array_ref![data, 0, 32]))
  }

  pub fn owner(self) -> ProtocolResult<Pubkey> {
    let data = self
      .inner()
      .try_borrow_data()
      .map_err(|_| ProtocolError::BorrowAccountDataError)?;
    Ok(Pubkey::new_from_array(*array_ref![data, 32, 32]))
  }

  pub fn delegate(self) -> ProtocolResult<Option<Pubkey>> {
    let data = self
      .inner()
      .try_borrow_data()
      .map_err(|_| ProtocolError::BorrowAccountDataError)?;
    unpack_coption_key(array_ref![data, 72, 36])
  }

  pub fn check_owner(self, authority: &Pubkey) -> ProtocolResult<()> {
    let owner = self.owner()?;
    if *authority == owner {
      return Ok(());
    }
    let delegate = self.delegate()?;
    match delegate {
      Some(d) => {
        if d == *authority {
          return Ok(());
        }
      }
      None => {}
    }
    return Err(ProtocolError::InvalidOwner);
  }

  // pub fn check_delegate(self, authority: &Pubkey) -> ProtocolResult<()> {
  //   let delegate = self.delegate()?;
  //   match delegate {
  //     Some(d) => {
  //       if d == *authority {
  //         return Ok(());
  //       }
  //     }
  //     None => {}
  //   }
  //   Err(ProtocolError::InvalidDelegate)
  // }
}

#[derive(Copy, Clone)]
pub struct TokenAccountAndMint<'a, 'b: 'a> {
  account: TokenAccount<'a, 'b>,
  mint: TokenMint<'a, 'b>,
}

#[allow(unused)]
impl<'a, 'b: 'a> TokenAccountAndMint<'a, 'b> {
  fn new(account: TokenAccount<'a, 'b>, mint: TokenMint<'a, 'b>) -> ProtocolResult<Self> {
    let account_data = account
      .0
      .try_borrow_data()
      .map_err(|_| ProtocolError::BorrowAccountDataError)?;
    if mint.0.key.as_ref() != &account_data[..32] {
      return Err(ProtocolError::InvalidTokenMint);
    }
    Ok(TokenAccountAndMint { account, mint })
  }

  pub fn get_account(self) -> TokenAccount<'a, 'b> {
    self.account
  }

  pub fn get_mint(self) -> TokenMint<'a, 'b> {
    self.mint
  }
}

impl<'a, 'b: 'a> SplTokenSwapInfo<'a, 'b> {}

#[derive(Copy, Clone)]
pub struct UserArgs<'a, 'b: 'a> {
  pub token_source_account: TokenAccount<'a, 'b>,
  pub token_destination_account: TokenAccount<'a, 'b>,
  pub source_account_owner: SignerAccount<'a, 'b>,
}

impl<'a, 'b: 'a> UserArgs<'a, 'b> {
  pub fn with_parsed_args(accounts: &'a [AccountInfo<'b>]) -> ProtocolResult<Self> {
    const MIN_ACCOUNTS: usize = 3;
    if !(accounts.len() == MIN_ACCOUNTS) {
      return Err(ProtocolError::InvalidAccountsLength);
    }

    let &[
      ref token_source_acc_info,
      ref token_destination_acc_info,
      ref source_account_owner,
    ]: &'a[AccountInfo<'b>; MIN_ACCOUNTS] = array_ref![accounts, 0, MIN_ACCOUNTS];

    let token_source_account = TokenAccount::new(token_source_acc_info)?;
    let token_destination_account = TokenAccount::new(token_destination_acc_info)?;
    let source_account_owner = SignerAccount::new(source_account_owner)?;
    token_source_account.check_owner(source_account_owner.inner().key)?;
    if token_source_account.mint() == token_destination_account.mint() {
      return Err(ProtocolError::InvalidTokenAccount);
    }

    Ok(UserArgs {
      token_source_account,
      token_destination_account,
      source_account_owner,
    })
  }
}

pub struct AmmInfoArgs<'a, 'b: 'a> {
  pub amm_info: RefMut<'a, AmmInfo>,
  pub amm_info_key: &'a Pubkey,
  pub authority_acc_info: &'a AccountInfo<'b>,
  pub token_a_account: TokenAccount<'a, 'b>,
  pub token_b_account: TokenAccount<'a, 'b>,
}

impl<'a, 'b: 'a> AmmInfoArgs<'a, 'b> {
  pub fn with_parsed_args(
    program_id: &'a Pubkey,
    accounts: &'a [AccountInfo<'b>],
  ) -> ProtocolResult<Self> {
    const MIN_ACCOUNTS: usize = 4;
    if !(accounts.len() == MIN_ACCOUNTS) {
      return Err(ProtocolError::InvalidAccountsLength);
    }
    let &[
      ref amm_info_acc,
      ref authority_acc_info,
      ref token_a_acc,
      ref token_b_acc,
    ]: &'a[AccountInfo<'b>; MIN_ACCOUNTS] = array_ref![accounts, 0, MIN_ACCOUNTS];

    if *amm_info_acc.owner != *program_id {
      return Err(ProtocolError::InvalidOwner);
    }
    // Pubkey::create_program_address(seeds, program_id)
    let amm_info = AmmInfo::load_mut(amm_info_acc, true)?;

    validate_authority_pubkey(
      authority_acc_info.key,
      program_id,
      amm_info_acc.key,
      amm_info.nonce,
    )?;

    let token_a_account = TokenAccount::new(token_a_acc)?;
    token_a_account.check_owner(authority_acc_info.key)?;

    let token_b_account = TokenAccount::new(token_b_acc)?;
    token_b_account.check_owner(authority_acc_info.key)?;
    Ok(AmmInfoArgs {
      amm_info,
      amm_info_key: amm_info_acc.key,
      authority_acc_info,
      token_a_account,
      token_b_account,
    })
  }

  pub fn nonce(&self) -> u8 {
    self.amm_info.nonce
  }

  /// find token_pair of amm_info by user's token pair
  ///   0 source_token_account
  ///   1 destination_token_account
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
      &[] => None,
      &[ref acc] => Some(TokenAccount::new(acc)?),
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

#[derive(Copy, Clone)]
pub struct SerumDexArgs<'a, 'b: 'a> {
  pub market: SerumDexMarket<'a, 'b>,
  pub request_queue_acc: &'a AccountInfo<'b>,
  pub event_queue_acc: &'a AccountInfo<'b>,
  pub bids_acc: &'a AccountInfo<'b>,
  pub asks_acc: &'a AccountInfo<'b>,
  pub coin_vault_acc: TokenAccount<'a, 'b>,
  pub pc_vault_acc: TokenAccount<'a, 'b>,
  pub vault_signer_acc: &'a AccountInfo<'b>,
  pub open_order_acc: &'a AccountInfo<'b>,
  pub rent_sysvar_acc: &'a AccountInfo<'b>,
  pub program_acc: &'a AccountInfo<'b>,
}

impl<'a, 'b: 'a> SerumDexArgs<'a, 'b> {
  pub fn with_parsed_args(accounts: &'a [AccountInfo<'b>]) -> ProtocolResult<Self> {
    const MIN_ACCOUNTS: usize = 11;
    if !(accounts.len() == MIN_ACCOUNTS) {
      return Err(ProtocolError::InvalidAccountsLength);
    }
    let &[
      ref market_acc,
      ref request_queue_acc,
      ref event_queue_acc,
      ref bids_acc,
      ref asks_acc,
      ref coin_vault_acc,
      ref pc_vault_acc,
      ref vault_signer_acc,
      ref open_order_acc,
      ref rent_sysvar_acc,
      ref program_acc,
    ]: &'a[AccountInfo<'b>; MIN_ACCOUNTS] = array_ref![accounts, 0, MIN_ACCOUNTS];

    let market = SerumDexMarket::new(market_acc)?;
    if *market.inner().owner != *program_acc.key {
      return Err(ProtocolError::InvalidProgramAddress);
    }

    Ok(SerumDexArgs {
      market,
      request_queue_acc,
      event_queue_acc,
      bids_acc,
      asks_acc,
      coin_vault_acc: TokenAccount::new(coin_vault_acc)?,
      pc_vault_acc: TokenAccount::new(pc_vault_acc)?,
      vault_signer_acc,
      open_order_acc,
      rent_sysvar_acc,
      program_acc,
    })
  }

  pub fn find_side(&self, source_mint: &Pubkey) -> ProtocolResult<DexSide> {
    if *source_mint == self.coin_vault_acc.mint()? {
      Ok(DexSide::Ask)
    } else {
      Ok(DexSide::Bid)
    }
  }
}

declare_validated_account_wrapper!(StableSwapInfo, |account: &AccountInfo| {
  let data = account
    .try_borrow_data()
    .map_err(|_| ProtocolError::BorrowAccountDataError)?;
  if data.len() != stable_swap_client::state::SwapInfo::LEN {
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
  pub clock_sysvar_acc: SysClockAccount<'a, 'b>,
  pub program_acc: &'a AccountInfo<'b>,
}

impl<'a, 'b: 'a> StableSwapArgs<'a, 'b> {
  pub fn with_parsed_args(accounts: &'a [AccountInfo<'b>]) -> ProtocolResult<Self> {
    const MIN_ACCOUNTS: usize = 7;
    if accounts.len() != MIN_ACCOUNTS {
      return Err(ProtocolError::InvalidAccountsLength);
    }
    let &[
      ref swap_info_acc,
      ref authority_acc,
      ref token_a_acc,
      ref token_b_acc,
      ref admin_fee_acc,
      ref clock_sysvar_acc,
      ref program_acc,
    ]: &'a[AccountInfo<'b>; MIN_ACCOUNTS] = array_ref![accounts, 0, MIN_ACCOUNTS];

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

    validate_authority_pubkey(
      authority_acc.key,
      program_acc.key,
      swap_info_acc.key,
      swap_info.nonce()?,
    )?;

    Ok(StableSwapArgs {
      swap_info,
      authority_acc,
      token_a: TokenAccount::new(token_a_acc)?,
      token_b: TokenAccount::new(token_b_acc)?,
      admin_fee_acc,
      clock_sysvar_acc: SysClockAccount::new(clock_sysvar_acc)?,
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
