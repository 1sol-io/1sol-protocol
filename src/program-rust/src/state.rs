//! State transition types
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_program::{
  program_error::ProgramError,
  program_option::COption,
  program_pack::{IsInitialized, Pack, Sealed},
  pubkey::Pubkey,
};

/// Onesol program serum dex info state.
#[repr(C)]
#[derive(PartialEq, Debug, Clone, Copy)]
pub struct DexMarketInfo {
  /// Initialized state.
  pub is_initialized: u8,
  /// status
  pub status: u8,
  /// nonce
  pub nonce: u8,
  /// market address
  pub market: Pubkey,
  /// pc_mint
  pub pc_mint: Pubkey,
  /// coin_mint
  pub coin_mint: Pubkey,
  /// open orders account
  pub open_orders: Pubkey,
  /// SerumDex program id
  pub dex_program_id: Pubkey,
}

impl Sealed for DexMarketInfo {}
impl IsInitialized for DexMarketInfo {
  fn is_initialized(&self) -> bool {
    self.is_initialized == 1
  }
}

impl Pack for DexMarketInfo {
  const LEN: usize = 163;

  fn pack_into_slice(&self, output: &mut [u8]) {
    let output = array_mut_ref![output, 0, 163];
    #[rustfmt::skip]
    let (
      is_initialized,
      status,
      nonce,
      market,
      pc_mint,
      coin_mint,
      open_orders,
      dex_program_id,
    ) = mut_array_refs![output, 1, 1, 1, 32, 32, 32, 32, 32];
    is_initialized.copy_from_slice(&[self.is_initialized]);
    status.copy_from_slice(&[self.status]);
    nonce.copy_from_slice(&[self.nonce]);
    market.copy_from_slice(self.market.as_ref());
    pc_mint.copy_from_slice(self.pc_mint.as_ref());
    coin_mint.copy_from_slice(self.coin_mint.as_ref());
    open_orders.copy_from_slice(self.open_orders.as_ref());
    dex_program_id.copy_from_slice(self.dex_program_id.as_ref());
  }

  fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
    let input = array_ref![input, 0, 163];
    #[rustfmt::skip]
    let (
      &[is_initialized],
      &[status],
      &[nonce],
      market,
      pc_mint,
      coin_mint,
      open_orders,
      dex_program_id
    ) =
      array_refs![input, 1, 1, 1, 32, 32, 32, 32, 32];
    Ok(Self {
      is_initialized: is_initialized,
      status: status,
      nonce: nonce,
      market: Pubkey::new(market),
      pc_mint: Pubkey::new(pc_mint),
      coin_mint: Pubkey::new(coin_mint),
      open_orders: Pubkey::new(open_orders),
      dex_program_id: Pubkey::new(dex_program_id),
    })
  }
}

#[repr(C)]
#[derive(PartialEq, Debug, Clone, Copy)]
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
