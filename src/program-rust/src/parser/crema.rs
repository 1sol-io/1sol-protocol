use super::base::TokenAccount;
use crate::{
  declare_validated_account_wrapper,
  error::{ProtocolError, ProtocolResult},
  parser::base::validate_authority_pubkey,
};
use arrayref::array_ref;
use solana_program::{account_info::AccountInfo, msg, pubkey::Pubkey};

declare_validated_account_wrapper!(SwapInfoV1, |account: &AccountInfo| {
  let account_data = account
    .try_borrow_data()
    .map_err(|_| ProtocolError::BorrowAccountDataError)?;
  if account_data.len() != 473 {
    return Err(ProtocolError::InvalidCremaSwapAccountData);
  }
  if account_data[34] != 1 {
    return Err(ProtocolError::InvalidCremaSwapAccountData);
  }
  Ok(())
});

impl<'a, 'b: 'a> SwapInfoV1<'a, 'b> {
  #[allow(dead_code)]
  pub fn nonce(self) -> ProtocolResult<u8> {
    Ok(
      self
        .inner()
        .try_borrow_data()
        .map_err(|_| ProtocolError::BorrowAccountDataError)?[35],
    )
  }

  pub fn token_a(self) -> ProtocolResult<Pubkey> {
    let data = self
      .inner()
      .try_borrow_data()
      .map_err(|_| ProtocolError::BorrowAccountDataError)?;
    Ok(Pubkey::new_from_array(*array_ref![data, 164, 32]))
  }

  pub fn token_b(self) -> ProtocolResult<Pubkey> {
    let data = self
      .inner()
      .try_borrow_data()
      .map_err(|_| ProtocolError::BorrowAccountDataError)?;
    Ok(Pubkey::new_from_array(*array_ref![data, 196, 32]))
  }

  pub fn token_a_mint(self) -> ProtocolResult<Pubkey> {
    let data = self
      .inner()
      .try_borrow_data()
      .map_err(|_| ProtocolError::BorrowAccountDataError)?;
    Ok(Pubkey::new_from_array(*array_ref![data, 228, 32]))
  }

  pub fn token_b_mint(self) -> ProtocolResult<Pubkey> {
    let data = self
      .inner()
      .try_borrow_data()
      .map_err(|_| ProtocolError::BorrowAccountDataError)?;
    Ok(Pubkey::new_from_array(*array_ref![data, 260, 32]))
  }
}

#[derive(Copy, Clone)]
pub struct CremaSwapV1Args<'a, 'b: 'a> {
  pub swap_info: SwapInfoV1<'a, 'b>,
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
    let swap_info = SwapInfoV1::new(swap_info_acc)?;
    if !program_id.executable || *swap_info_acc.owner != *program_id.key {
      msg!(
        "program_id: {}, executable: {}, swap_info: {}, owner: {}",
        program_id.key.to_string(),
        program_id.executable,
        swap_info_acc.key.to_string(),
        swap_info_acc.owner.to_string(),
      );
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

  pub fn find_token_pair(
    &self,
    source_mint_key: &Pubkey,
    destination_mint_key: &Pubkey,
  ) -> ProtocolResult<(&TokenAccount<'a, 'b>, &TokenAccount<'a, 'b>)> {
    let pool_token_a_mint = self.swap_info.token_a_mint()?;
    let pool_token_b_mint = self.swap_info.token_b_mint()?;
    if *source_mint_key == pool_token_a_mint && *destination_mint_key == pool_token_b_mint {
      return Ok((&self.pool_token_a, &self.pool_token_b));
    } else if *source_mint_key == pool_token_b_mint && *destination_mint_key == pool_token_a_mint {
      return Ok((&self.pool_token_b, &self.pool_token_a));
    }
    Err(ProtocolError::InvalidTokenMint)
  }
}

#[cfg(test)]
mod test {
  use super::*;
  use bs58;
  use solana_sdk::{account::Account, account_info::IntoAccountInfo};
  use std::str::FromStr;

  #[test]
  pub fn test_parse_crema_swap_v1() {
    let pubkey = Pubkey::from_str("8J3avAjuRfL2CYFKKDwhhceiRoajhrHv9kN5nUiEnuBG").unwrap();
    let program_id = Pubkey::from_str("6MLxLqiXaaSUpkgMnWDTuejNZEz3kE7k2woyHGVFw319").unwrap();
    let account_data = "GfbXvUuzWx8PEGeQR41UGuxsUTmM7kYjMA5BZZoQv9MAGkNCeEkfcusa5rLVifmCQRSPr8vPwQ8wRFAzuGSXGgH4wUKBph
CBDT9quQHAAvBLUJTqMXaSqYjNtq9s3QSZHsCZE1HA8iBHBUgZzW79KnBqHPEnpENxcsN2fAeM4ZtnptTrTYyvnNHjzkfK15jPhXeBntuYRnrubVfYs5HL8X
WVZrUsGc2FiNmw9DxsgctR1pJUfkqqkUSvXUywbDnSVwgJpjCQUTWJYwGUCfWyKcjezWvVuRJaobis634fDApe3SmXJEFo5KiT3hgVCJWiZcRCie4wR3daiR
YZybDHAn6bUYwVN82MRcq4EyiZrChSXgu3S67uiLfDnR3Wfmgn6nCZG2UnuYT6MiASsNDdxVP2RjMquLYkL8ZU2RHUvVLYUNfXpJArnt95ByCXA9zv4DhRUh
SaE3zxQ9yT9m4eBR3rqsmxsjdpWv7EPezNnqiuKJjWNMrxrEb77ecX6UpsdVn6LWJWKtU67Ug6DjKYGGVcrCw4T7ZGppQr6y5pvXYQLe42RFUh77Jvm6CKqc
WExa6Gae6euRW6eCcTw5Lf4F7y6PZxD3wek4uMrrHnURYHBkaumuCDiy1z3kbrv9R9RGsYT";
    let mut test_account = Account {
      lamports: 4182960,
      data: bs58::decode(account_data.replace('\n', ""))
        .into_vec()
        .unwrap(),
      owner: program_id,
      executable: false,
      rent_epoch: 281,
    };
    let account_info = (&pubkey, &mut test_account).into_account_info();
    assert!(*account_info.key == pubkey);
    let c = SwapInfoV1::new(&account_info).unwrap();
    assert_eq!(
      c.token_a().unwrap().to_string(),
      "FAqsr5LhMZQMYwxXrQuCH5C6bx1mVwuXG3WiQ5YjCEzk".to_string()
    );
    assert_eq!(
      c.token_b().unwrap().to_string(),
      "DwFzRnWVxpvrrMJuQUwhBXhPhqUPMbrmDVJAt75k5ybE".to_string()
    );
    assert_eq!(
      c.token_a_mint().unwrap().to_string(),
      "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB".to_string()
    );
    assert_eq!(
      c.token_b_mint().unwrap().to_string(),
      "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string()
    );
    assert_eq!(c.nonce().unwrap(), 254,);
  }
}
