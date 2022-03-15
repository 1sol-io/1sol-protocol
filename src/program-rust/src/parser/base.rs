use crate::{
  declare_validated_account_wrapper,
  error::{ProtocolError, ProtocolResult},
  spl_token,
  state::SwapInfo,
};
use arrayref::{array_ref, array_refs};
use solana_program::{account_info::AccountInfo, msg, program_pack::Pack, pubkey::Pubkey, sysvar};

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
  let key = Pubkey::create_program_address(&[base_key, &[nonce]], program_id).map_err(|e| {
    msg!("create_program_address failed: {}, nonce: {}", e, nonce);
    ProtocolError::InvalidProgramAddress
  })?;
  if key != *authority {
    return Err(ProtocolError::InvalidAuthority);
  }
  Ok(())
}
