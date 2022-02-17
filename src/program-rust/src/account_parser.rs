use crate::swappers::serum_dex::matching::Side as DexSide;
use crate::{
  check_unreachable,
  error::{ProtocolError, ProtocolResult},
  spl_token,
  state::SwapInfo,
};
use arrayref::{array_ref, array_refs};
use solana_program::{account_info::AccountInfo, msg, program_pack::Pack, pubkey::Pubkey, sysvar};

pub trait ArgsSize {
  const ARGS_SIZE: usize;
}

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

          #[inline(always)]
          #[allow(unused)]
          pub fn check_writable(self) -> ProtocolResult<()> {
            if !self.inner().is_writable {
              return Err(ProtocolError::ReadonlyAccount)
            }
            Ok(())
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
  if data.len() != spl_token::ACCOUNT_LEN {
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
  if data.len() != spl_token::MINT_LEN {
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
fn unpack_coption_key(src: &[u8; 36]) -> ProtocolResult<Option<Pubkey>> {
  let (tag, body) = array_refs![src, 4, 32];
  match *tag {
    [0, 0, 0, 0] => Ok(None),
    [1, 0, 0, 0] => Ok(Some(Pubkey::new_from_array(*body))),
    _ => Err(ProtocolError::InvalidAccountData),
  }
}

#[allow(dead_code)]
/// Calculates the authority id by generating a program address.
pub fn validate_authority_pubkey(
  authority: &Pubkey,
  program_id: &Pubkey,
  base_key: &[u8],
  nonce: u8,
) -> Result<(), ProtocolError> {
  let key = Pubkey::create_program_address(&[base_key, &[nonce]], program_id)
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

  pub fn check_owner(self, authority: &Pubkey, strict: bool) -> ProtocolResult<()> {
    let owner = self.owner()?;
    if *authority == owner {
      return Ok(());
    }
    if strict {
      return Err(ProtocolError::InvalidOwner);
    }
    let delegate = self.delegate()?;
    if let Some(d) = delegate {
      if d == *authority {
        return Ok(());
      }
    }
    Err(ProtocolError::InvalidOwner)
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
pub struct TokenAccountAndMint<'a, 'b: 'a> {
  pub account: TokenAccount<'a, 'b>,
  pub mint: TokenMint<'a, 'b>,
}

#[allow(unused)]
impl<'a, 'b: 'a> TokenAccountAndMint<'a, 'b> {
  pub fn new(account: TokenAccount<'a, 'b>, mint: TokenMint<'a, 'b>) -> ProtocolResult<Self> {
    if *mint.pubkey() != account.mint()? {
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
  pub source_account_owner: &'a AccountInfo<'b>,
}

impl<'a, 'b: 'a> UserArgs<'a, 'b> {
  pub fn with_parsed_args(accounts: &'a [AccountInfo<'b>]) -> ProtocolResult<Self> {
    const MIN_ACCOUNTS: usize = 3;
    if accounts.len() != MIN_ACCOUNTS {
      return Err(ProtocolError::InvalidAccountsLength);
    }

    let &[
      ref token_source_acc_info,
      ref token_destination_acc_info,
      ref source_account_owner,
    ]: &'a[AccountInfo<'b>; MIN_ACCOUNTS] = array_ref![accounts, 0, MIN_ACCOUNTS];

    let token_source_account = TokenAccount::new(token_source_acc_info)?;
    let token_destination_account = TokenAccount::new(token_destination_acc_info)?;
    // let source_account_owner = SignerAccount::new(source_account_owner)?;
    token_source_account.check_owner(source_account_owner.key, false)?;

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

pub struct SwapInfoArgs<'a, 'b: 'a> {
  pub swap_info: SwapInfo,
  pub swap_info_acc: &'a AccountInfo<'b>,
}

impl<'a, 'b: 'a> SwapInfoArgs<'a, 'b> {
  pub fn with_parsed_args(
    account: &'a AccountInfo<'b>,
    program_id: &'a Pubkey,
  ) -> ProtocolResult<Self> {
    if *account.owner != *program_id {
      return Err(ProtocolError::InvalidOwner);
    }
    let swap_info =
      SwapInfo::unpack(&account.data.borrow()).map_err(|_| ProtocolError::InvalidAccountData)?;
    Ok(Self {
      swap_info,
      swap_info_acc: account,
    })
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
  fn test_serum_dex_market() {
    let market_data = "GmH4gu6PYUUKDZqX8AT2ZH7MKQkqEiK1rkgus44yrCJvP7UDfLpQzbFKzfgUx1oSffopN2NGno33fnjhD37awk2MPJrXgRiQjwQWWwspgrrjXVKhP87vynWu4FzjGgx8USsnBa5mNEZb2rKvNmVZKekzZUpdSAiXEMbVvEpAn1tQTderQCh69t84sPfcVfseAPEKyJYcAiFLCTrKFmQ3SVQiartpqiySprqLqkqto5Z3LAVRGBvVvcinYuZBN49ZbBaMGxXS9wt6tXN8ZqmoZMfYvc3un68DuJ5vyRPyiYz56LqovWnbjjXY76rRPzsbXR3EqYNMyCFjoqxnsH3LLJVYXwT11ggvUery3J8bhDbdvSJaacCyTEuaMuWXjJMcsBxW2NQLAPzasX8vu1uTDjqnvCkZKhYcGtCpiLddLQEMXu6mTEE6ZmT73rHCLaoGKPSYxuVkunGb4AtkU4mSUfWw3EbKc6s6sEvgi5Ec47RYGdNDMK31jENakYtSAweGRSin1iB7G11FU1xhNE";
    let mut data = bs58::decode(market_data).into_vec().unwrap();
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
    let market_data = "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";
    let mut data = bs58::decode(market_data).into_vec().unwrap();
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

  #[test]
  fn test_raydium_struct() {
    let raydium_program_id =
      Pubkey::from_str("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8").unwrap();
    let raydium_pubkey = Pubkey::from_str("DVa7Qmb5ct9RCpaU7UTpSaf3GVMYz17vNVU67XpdCRut").unwrap();
    let raydium_data = "Csa6r43w6Tksashc251QAkcpr6D4zyiWB4sSrw5xDZzoH9FsPfiZDXJSNMMTFHVsbKqVyDZb32anWxQNNk9FL7bCpKPZ7qMdCe6eCkjjRbbdiYvHBV1TrhWWwQ6pKP3rNVfae2R25Hj8ttD9CwVTz2CRzcDDdu88N5T6J67xVhcBKwEmJB3itxbnWWnvHf95TBXbmmAZFrbfPm6153Re8mjTUVswfNCRVC2ypRV8jzZoBbohMWrbPxKW4VXZdaEE8JwVU5QrPFvKFJKkmeReiBreb7Huy52gGioSCu8FLWg8JYQHMzgnr31tR5sDa1WSVJVPUQ4t4rRazqcdALsdSKZHUrnZACbLTsEgiXQWn4Ncc9eVciH78oQsXgvPsWC4qSURfyQZoe7QUZ5pb6YtY5A4YASwim5JauPHVGdd6sLFTea3DK7RUdmpDcmyKbnQKBVE3mTMA6useCSrUtHChwpETDkTC1ghEQtZQTVdefcPsAGLXEy3LioEqfnny3huwYxuTnT6LYt7KYP1FqqRoff7zQUvWn8xRq45pxWjbm3HLGimno7tCWYVRUwMH74vDfgg7AebDUTdRA72GhBUG1Y2852URSs3crQ4qDs9z62AS2ymyMZ8Qicz9RmimyU9iCU8n96pZ7Y57XKydcW8aDKF1gBi3bdLDGyUAdYYb51Jijykz38oM6KPswC7rAxgTVVgiMu4JvKmVwecn7NCP4iWoM9k8vrYaa8tS3VBZtAMCkVtuwpQeYVZ9HPZkwVPV9o6oFXBidkZaQukNQ7sfZSCEGj6vKv4fGJNpuDJDZiUXhveEjnbYffrm5Gnfz2kvSSdCgotWNJwcJZkfv5LsMkprfTXodEXXnLqqHj3LM8tNSFuCqhMRFKbuHdZt1EfvFWcyxNukAhUXZn5k4MVNQdhQZ5poqMfUa6AzgXBMVAYCoFrsKF9qHbCEHFLNcznS3J3go3xcCnigQtQEctXawtxg5yoJmS91iDZt2nTceatH7LN78fA5DxmJDn8kpF3F2";
    let mut raydium_data = bs58::decode(raydium_data).into_vec().unwrap();
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

    // let serum_program_id = Pubkey::from_str("9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin").unwrap();
    // let market_pubkey = Pubkey::from_str("teE55QrL4a4QSfydR9dnHF97jgCfptpuigbb53Lo95g").unwrap();
    // let market_data = "GmH4gu6PYUUKDZqX89gcVw36feKDpPigWxwhfjousKHgERh6FL6GJQSHjNiGNn7uKv6dMmNrovsKNP7miL9GZ9qhkL9XtR1841zKFNG4LUnPNXcJRrumQN7TDZuvHrKnyhZWrsBqFYKEEX1Qr9eSWivWHdCMBGy31yo95k6JQ125kW26jNqD9mGqx7MfPHdZRL3WeB3RxRp68Hgvs4iEn9NYHJCVEF2Lf8vY7hgDgk9YgY7XwoWKPBZ7n8gp2NbzRnTtGs2nhLLUm2XRRY8wFr2t7rEHUwdk9vuxxJakf7vixVmSrouKqzYyXpNZP9Qv1JSUvavZq3mLQCH9uwUBghbvcpU24csgyVZFzYZezWvDUEPgU49C7z5WwYQqpsjGFXfRC2jtrK7CYstV5itGC8v3Bv3bfD4w3MS4bAZaL3UiRYxAnvPf42N8jAkVev8rdprs3vrxUkM6vc3nCDZsMeSik2uVssbEBKye5uqhicdegqLLXQUTJsjqTX19rKiSpU4G42WkC2vNd2a8gW";
    // let mut market_data = bs58::decode(market_data).into_vec().unwrap();
    // let mut market_lamports = 3591360u64;
    // let market_account_info = AccountInfo::new(
    //   &market_pubkey,
    //   false,
    //   true,
    //   &mut market_lamports,
    //   &mut market_data[..],
    //   &serum_program_id,
    //   false,
    //   248,
    // );
  }
}
