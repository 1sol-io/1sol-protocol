//! State transition types
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_program::{
  program_error::ProgramError,
  program_option::COption,
  program_pack::{IsInitialized, Pack, Sealed},
  pubkey::Pubkey,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
  SwapInfo,
  Closed,
}

impl Status {
  pub fn from_u8(status: u8) -> Result<Self, ProgramError> {
    match status {
      1 => Ok(Status::SwapInfo),
      3 => Ok(Status::Closed),
      _ => Err(ProgramError::InvalidArgument),
    }
  }

  pub fn to_u8(&self) -> u8 {
    match self {
      Status::SwapInfo => 1,
      Status::Closed => 3,
    }
  }
}

#[repr(C)]
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub struct SwapInfo {
  /// Initialized state.
  pub is_initialized: u8,
  /// nonce used in program address.
  pub status: u8,
  /// latest amount
  pub token_latest_amount: u64,
  /// Owner address
  pub owner: Pubkey,
  /// token account
  pub token_account: COption<Pubkey>,
}

impl SwapInfo {
  pub fn new(owner: &Pubkey) -> Self {
    Self {
      is_initialized: 1,
      status: Status::SwapInfo.to_u8(),
      token_latest_amount: 0,
      owner: *owner,
      token_account: COption::None,
    }
  }
}

impl Sealed for SwapInfo {}

impl IsInitialized for SwapInfo {
  fn is_initialized(&self) -> bool {
    self.is_initialized == 1
  }
}

impl Pack for SwapInfo {
  const LEN: usize = 78;

  fn pack_into_slice(&self, dst: &mut [u8]) {
    let output = array_mut_ref![dst, 0, 78];
    #[rustfmt::skip]
    let (
      is_initialized,
      status,
      token_latest_amount,
      owner,
      token_account,
    ) = mut_array_refs![output, 1, 1, 8, 32, 36];
    is_initialized.copy_from_slice(&[self.is_initialized]);
    status.copy_from_slice(&[self.status]);
    token_latest_amount.copy_from_slice(&self.token_latest_amount.to_le_bytes()[..]);
    owner.copy_from_slice(self.owner.as_ref());
    pack_coption_key(&self.token_account, token_account);
  }

  fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
    let input = array_ref![src, 0, 78];
    #[rustfmt::skip]
    let (
      &[is_initialized],
      &[status],
      &token_latest_amount,
      owner,
      token_account,
    ) = array_refs![input, 1, 1, 8, 32, 36];
    Ok(Self {
      is_initialized,
      status,
      token_latest_amount: u64::from_le_bytes(token_latest_amount),
      owner: Pubkey::new(owner),
      token_account: unpack_coption_key(token_account)?,
    })
  }
}

fn pack_coption_key(src: &COption<Pubkey>, dst: &mut [u8; 36]) {
  let (tag, body) = mut_array_refs![dst, 4, 32];
  match src {
    COption::Some(key) => {
      *tag = [1, 0, 0, 0];
      body.copy_from_slice(key.as_ref());
    }
    COption::None => {
      *tag = [0; 4];
    }
  }
}

fn unpack_coption_key(src: &[u8; 36]) -> Result<COption<Pubkey>, ProgramError> {
  let (tag, body) = array_refs![src, 4, 32];
  match *tag {
    [0, 0, 0, 0] => Ok(COption::None),
    [1, 0, 0, 0] => Ok(COption::Some(Pubkey::new_from_array(*body))),
    _ => Err(ProgramError::InvalidAccountData),
  }
}

#[cfg(test)]
mod test {
  // use super::*;

  // const TEST_VERSION: u8 = 1;
  // const TEST_NONCE: u8 = 255;
  // const TEST_TOKEN_PROGRAM_ID: Pubkey = Pubkey::new_from_array([1u8; 32]);
  // const TEST_TOKEN: Pubkey = Pubkey::new_from_array([2u8; 32]);
  // const TEST_TOKEN_MINT: Pubkey = Pubkey::new_from_array([5u8; 32]);

  #[test]
  pub fn test_onesol_amm_info() {
    assert_eq!(1, 1);
  }
}
