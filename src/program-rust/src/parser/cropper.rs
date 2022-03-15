use super::base::{validate_authority_pubkey, TokenAccount, TokenMint};
use crate::{
  declare_validated_account_wrapper,
  error::{ProtocolError, ProtocolResult},
};
use arrayref::array_ref;
use solana_program::{account_info::AccountInfo, msg, pubkey::Pubkey};

declare_validated_account_wrapper!(CropperSwapV1, |account: &AccountInfo| {
  let account_data = account
    .try_borrow_data()
    .map_err(|_| ProtocolError::BorrowAccountDataError)?;
  if account_data.len() != 291 {
    return Err(ProtocolError::InvalidCremaSwapAccountData);
  }
  Ok(())
});

impl<'a, 'b: 'a> CropperSwapV1<'a, 'b> {
  pub fn nonce(self) -> ProtocolResult<u8> {
    Ok(
      self
        .inner()
        .try_borrow_data()
        .map_err(|_| ProtocolError::BorrowAccountDataError)?[2],
    )
  }

  pub fn token_a_account(self) -> ProtocolResult<Pubkey> {
    let data = self
      .inner()
      .try_borrow_data()
      .map_err(|_| ProtocolError::BorrowAccountDataError)?;
    Ok(Pubkey::new_from_array(*array_ref![data, 131, 32]))
  }

  pub fn token_b_account(self) -> ProtocolResult<Pubkey> {
    let data = self
      .inner()
      .try_borrow_data()
      .map_err(|_| ProtocolError::BorrowAccountDataError)?;
    Ok(Pubkey::new_from_array(*array_ref![data, 163, 32]))
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

  pub fn pool_mint(self) -> ProtocolResult<Pubkey> {
    let data = self
      .inner()
      .try_borrow_data()
      .map_err(|_| ProtocolError::BorrowAccountDataError)?;
    Ok(Pubkey::new_from_array(*array_ref![data, 195, 32]))
  }
}

declare_validated_account_wrapper!(CropperProgramState, |account: &AccountInfo| {
  let account_data = account
    .try_borrow_data()
    .map_err(|_| ProtocolError::BorrowAccountDataError)?;
  if account_data.len() != 130 {
    return Err(ProtocolError::InvalidCremaSwapAccountData);
  }
  Ok(())
});

impl<'a, 'b: 'a> CropperProgramState<'a, 'b> {
  pub fn fee_owner(self) -> ProtocolResult<Pubkey> {
    let data = self
      .inner()
      .try_borrow_data()
      .map_err(|_| ProtocolError::BorrowAccountDataError)?;
    Ok(Pubkey::new_from_array(*array_ref![data, 33, 32]))
  }
}

#[derive(Copy, Clone)]
pub struct CropperArgs<'a, 'b: 'a> {
  pub swap_info: CropperSwapV1<'a, 'b>,
  pub authority: &'a AccountInfo<'b>,
  pub program_state: CropperProgramState<'a, 'b>,
  pub token_a_account: TokenAccount<'a, 'b>,
  pub token_b_account: TokenAccount<'a, 'b>,
  pub pool_mint: TokenMint<'a, 'b>,
  pub fee_account: TokenAccount<'a, 'b>,
  pub program_id: &'a AccountInfo<'b>,
}

impl<'a, 'b: 'a> CropperArgs<'a, 'b> {
  pub fn with_parsed_args(accounts: &'a [AccountInfo<'b>]) -> ProtocolResult<Self> {
    const MIN_ACCOUNTS: usize = 8;

    if accounts.len() != MIN_ACCOUNTS {
      return Err(ProtocolError::InvalidAccountsLength);
    }
    let &[
      ref swap_info_acc,
      ref authority,
      ref program_state_acc,
      ref token_a_account_acc,
      ref token_b_account_acc,
      ref pool_mint_acc,
      ref fee_account_acc,
      ref program_id,
    ]: &'a[AccountInfo<'b>; MIN_ACCOUNTS] = array_ref![accounts, 0, MIN_ACCOUNTS];

    let swap_info = CropperSwapV1::new(swap_info_acc)?;
    if !program_id.executable || *swap_info_acc.owner != *program_id.key {
      return Err(ProtocolError::InvalidProgramAddress);
    }

    if *pool_mint_acc.key != swap_info.pool_mint()? {
      return Err(ProtocolError::InvalidPoolMint);
    }
    let token_1_account = TokenAccount::new(token_a_account_acc)?;
    let token_2_account = TokenAccount::new(token_b_account_acc)?;

    let pool_token_a = swap_info.token_a_account()?;
    let pool_token_b = swap_info.token_b_account()?;

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

    validate_authority_pubkey(
      authority.key,
      program_id.key,
      &swap_info_acc.key.to_bytes(),
      swap_info.nonce()?,
    )?;

    let program_state = CropperProgramState::new(program_state_acc)?;
    let fee_account = TokenAccount::new(fee_account_acc)?;

    if fee_account.owner()? != program_state.fee_owner()? {
      msg!(
        "cropper_finance, fee_account.owner[{}] != program_state.fee_owner[{}]",
        fee_account.owner()?,
        program_state.fee_owner()?
      );
    }
    let pool_mint = TokenMint::new(pool_mint_acc)?;
    if swap_info.pool_mint()? != *pool_mint.pubkey() {
      msg!(
        "cropper_finance, pool_mint is {}, expect {}",
        pool_mint.pubkey(),
        swap_info.pool_mint()?
      );
    }

    Ok(Self {
      swap_info,
      authority,
      program_state,
      token_a_account,
      token_b_account,
      pool_mint,
      fee_account,
      program_id,
    })
  }
}

#[cfg(test)]
mod test {
  use super::*;
  use bs58;
  use solana_sdk::{account::Account, account_info::IntoAccountInfo};
  use std::str::FromStr;

  #[test]
  pub fn test_parse_cropper_swap_info() {
    let pubkey = Pubkey::from_str("7NWyuTfpb8gfRpgm67yv5GkdX2EM3WkefGSwHZfNVzTW").unwrap();
    let program_id = Pubkey::from_str("CTMAxxk34HjKWxQ3QLZK1HpaLXmBveao3ESePXbiyfzh").unwrap();
    let account_data = "2C1RW18oraJgyUDV6gjSYhkVvyAJktsiS4Hq3dn2J6HQn3EDc1L9HW8uVLZPAGvfJxLeVQBinxLGGpcySQeD1sfrUiPRYy3u
GqmEhSz8LxYtVh2a8qBpQwPnrExV2EGqdvii6s3KUdxayiDAiEv8pUoF5xDHGQNHwYnA8r76yiFkc8RMom5pahKvqH4vBeJ2ypMBCqXos98PB4p9s7HanZQJ
wwNsNLBhoPbzt4ETyew4TPGnb5dAuvQtmLRmHmiMrMv4hjcbn3yBYrtzyfFs774i28HRTL9n9S3DbgYsUmJPBBbJjU3TaJBxLyiWASQDrd4snbxpcWbgTo95
WiQ3pv9mtcjZxcGchY1hw4AGj83tmHeah5EE5cRWrhqemnT9TZLoFHzoVRZBW";
    let mut test_account = Account {
      lamports: 2916240,
      data: bs58::decode(account_data.replace('\n', ""))
        .into_vec()
        .unwrap(),
      owner: program_id,
      executable: false,
      rent_epoch: 283,
    };
    let account_info = (&pubkey, &mut test_account).into_account_info();
    assert!(*account_info.key == pubkey);
    let c = CropperSwapV1::new(&account_info).unwrap();
    assert_eq!(
      c.token_a_account().unwrap().to_string(),
      "5DuRdWMtLQ51Ld534PsjDbudPGVnYkCwiGKo5EKcoaaL".to_string()
    );
    assert_eq!(
      c.token_b_account().unwrap().to_string(),
      "4G9Hp77tNNfMuYgD2DfFxZAAaruXuJigfbU2FjBreSdn".to_string()
    );
    assert_eq!(
      c.token_a_mint().unwrap().to_string(),
      "So11111111111111111111111111111111111111112".to_string()
    );
    assert_eq!(
      c.token_b_mint().unwrap().to_string(),
      "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string()
    );
    assert_eq!(
      c.pool_mint().unwrap().to_string(),
      "APTaiNJxUtAZMnhoZCVXdxR5kf7ExYWuET3sfnub59z2".to_string()
    );
    assert_eq!(c.nonce().unwrap(), 253);
  }

  #[test]
  pub fn test_parse_cropper_program_state() {
    let pubkey = Pubkey::from_str("3hsU1VgsBgBgz5jWiqdw9RfGU6TpWdCmdah1oi4kF3Tq").unwrap();
    let program_id = Pubkey::from_str("CTMAxxk34HjKWxQ3QLZK1HpaLXmBveao3ESePXbiyfzh").unwrap();
    let account_data = "49Njgem1Ug3UieKyzb639EJxjKFgsi5hrc3W9TW1wCK7aPTavf6uz7kYFur1f6jo1QKZEseY9EBHDEmrkim8Wc9cb7f7sACW
y2oAoS9LLHyqQpzfLNY48sxw48PxhFPsyfVVasbjFrygCNJjcoNsQqs9UZ1rJYbAGvuf1vosp3zKkLnxF";
    let mut test_account = Account {
      lamports: 1795680,
      data: bs58::decode(account_data.replace('\n', ""))
        .into_vec()
        .unwrap(),
      owner: program_id,
      executable: false,
      rent_epoch: 284,
    };
    let account_info = (&pubkey, &mut test_account).into_account_info();
    assert!(*account_info.key == pubkey);
    let c = CropperProgramState::new(&account_info).unwrap();
    assert_eq!(
      c.fee_owner().unwrap().to_string(),
      "DyDdJM9KVsvosfXbcHDp4pRpmbMHkRq3pcarBykPy4ir".to_string()
    );
  }
}
