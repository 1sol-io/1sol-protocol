//! State transition types
use crate::error::ProtocolError;
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use enumflags2::{bitflags, BitFlags};
use solana_program::{
  program_error::ProgramError,
  program_pack::{IsInitialized, Pack, Sealed},
  pubkey::Pubkey,
};

/// AccountFlag
#[bitflags(default = Initialized)]
#[repr(u16)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum AccountFlag {
  /// Initialized
  Initialized = 1u16 << 1,
  /// Disabled
  Disabled = 1u16 << 2,
  /// Closed
  Closed = 1u16 << 3,
  /// AmmInfo
  AmmInfo = 1u16 << 4,
  /// Serum Dex Info
  DexMarketInfo = 1u16 << 5,
}

/// Onesol program ammInfo state.
#[repr(C)]
#[derive(PartialEq, Debug, Clone, Copy)]
pub struct AmmInfo {
  /// Initialized state.
  pub account_flags: u16,
  /// nonce used in program address.
  pub nonce: u8,
  /// Owner address
  pub owner: Pubkey,
  /// Program ID of the tokens
  pub token_program_id: Pubkey,
  /// Token token_a
  pub token_a_vault: Pubkey,
  /// TokenMint pc
  pub token_a_mint: Pubkey,
  /// TokenAccount coin
  pub token_b_vault: Pubkey,
  /// TokenMint coin
  pub token_b_mint: Pubkey,
  /// output data
  pub output_data: OutputData,
}

/// load account flags
pub fn account_flags(account_data: &[u8]) -> Result<BitFlags<AccountFlag>, ProtocolError> {
  let mut flag_bytes = [0u8; 2];
  flag_bytes.copy_from_slice(&account_data[0..2]);
  BitFlags::from_bits(u16::from_le_bytes(flag_bytes))
    .map_err(|_| ProtocolError::InvalidAccountFlags)
    .map(Into::into)
}

impl AmmInfo {
  /// flags
  pub fn flags(&self) -> Result<BitFlags<AccountFlag>, ProgramError> {
    BitFlags::from_bits(self.account_flags)
      .map_err(|_| ProgramError::InvalidAccountData)
      .map(Into::into)
  }
}

impl Sealed for AmmInfo {}

impl IsInitialized for AmmInfo {
  fn is_initialized(&self) -> bool {
    BitFlags::from_bits(self.account_flags)
      .map(|x| x.contains(AccountFlag::Initialized))
      .unwrap_or(false)
  }
}

impl Pack for AmmInfo {
  const LEN: usize = 280;

  fn pack_into_slice(&self, output: &mut [u8]) {
    let output = array_mut_ref![output, 0, 280];
    let (
      account_flags,
      nonce,
      owner,
      token_program_id,
      token_a_vault,
      token_a_mint,
      token_b_vault,
      token_b_mint,
      output_data,
      space,
    ) = mut_array_refs![output, 2, 1, 32, 32, 32, 32, 32, 32, 80, 5];
    account_flags.copy_from_slice(&self.account_flags.to_le_bytes());
    nonce[0] = self.nonce;
    owner.copy_from_slice(self.owner.as_ref());
    token_program_id.copy_from_slice(self.token_program_id.as_ref());
    token_a_vault.copy_from_slice(self.token_a_vault.as_ref());
    token_a_mint.copy_from_slice(self.token_a_mint.as_ref());
    token_b_vault.copy_from_slice(self.token_b_vault.as_ref());
    token_b_mint.copy_from_slice(self.token_b_mint.as_ref());
    self.output_data.pack_into_slice(&mut output_data[..]);
    space.copy_from_slice(&[0; 5]);
  }

  fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
    let input = array_ref![input, 0, 280];
    let (
      &account_flags,
      nonce,
      owner,
      token_program_id,
      token_a_vault,
      token_a_mint,
      token_b_vault,
      token_b_mint,
      output_data,
      _,
    ) = array_refs![input, 2, 1, 32, 32, 32, 32, 32, 32, 80, 5];

    Ok(Self {
      account_flags: u16::from_le_bytes(account_flags),
      nonce: nonce[0],
      owner: Pubkey::new(owner),
      token_program_id: Pubkey::new(token_program_id),
      token_a_vault: Pubkey::new(token_a_vault),
      token_a_mint: Pubkey::new(token_a_mint),
      token_b_vault: Pubkey::new(token_b_vault),
      token_b_mint: Pubkey::new(token_b_mint),
      output_data: OutputData::unpack_from_slice(output_data)?,
    })
  }
}

/// Output data
#[repr(C)]
#[derive(PartialEq, Debug, Clone, Copy, Default)]
pub struct OutputData {
  /// swap token_a in amount
  pub token_a_in_amount: u128,
  /// swap token_b out amount
  pub token_b_out_amount: u128,
  /// swap token_a to token_b fee
  pub token_a2b_fee: u64,
  /// swap token_b in amount
  pub token_b_in_amount: u128,
  /// swap token_a out amount
  pub token_a_out_amount: u128,
  /// swap token_b to token_a fee
  pub token_b2a_fee: u64,
}

impl OutputData {
  /// initialize output data
  pub fn new() -> Self {
    OutputData {
      token_a_in_amount: 0u128,
      token_b_out_amount: 0u128,
      token_a2b_fee: 0u64,
      token_b_in_amount: 0u128,
      token_a_out_amount: 0u128,
      token_b2a_fee: 0u64,
    }
  }
}

impl Sealed for OutputData {}

impl Pack for OutputData {
  const LEN: usize = 80;

  fn pack_into_slice(&self, output: &mut [u8]) {
    let output = array_mut_ref![output, 0, 80];
    let (
      token_a_in_amount,
      token_b_out_amount,
      token_a2b_fee,
      token_b_in_amount,
      token_a_out_amount,
      token_b2a_fee,
    ) = mut_array_refs![output, 16, 16, 8, 16, 16, 8];
    token_a_in_amount.copy_from_slice(&self.token_a_in_amount.to_le_bytes());
    token_b_out_amount.copy_from_slice(&self.token_b_out_amount.to_le_bytes());
    token_a2b_fee.copy_from_slice(&self.token_a2b_fee.to_le_bytes());
    token_b_in_amount.copy_from_slice(&self.token_b_in_amount.to_le_bytes());
    token_a_out_amount.copy_from_slice(&self.token_a_out_amount.to_le_bytes());
    token_b2a_fee.copy_from_slice(&self.token_b2a_fee.to_le_bytes());
  }

  fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
    let input = array_ref![input, 0, 80];
    let (
      &token_a_in_amount,
      &token_b_out_amount,
      &token_a2b_fee,
      &token_b_in_amount,
      &token_a_out_amount,
      &token_b2a_fee,
    ) = array_refs![input, 16, 16, 8, 16, 16, 8];
    Ok(Self {
      token_a_in_amount: u128::from_le_bytes(token_a_in_amount),
      token_b_out_amount: u128::from_le_bytes(token_b_out_amount),
      token_a2b_fee: u64::from_le_bytes(token_a2b_fee),
      token_b_in_amount: u128::from_le_bytes(token_b_in_amount),
      token_a_out_amount: u128::from_le_bytes(token_a_out_amount),
      token_b2a_fee: u64::from_le_bytes(token_b2a_fee),
    })
  }
}

/// Onesol program serum dex info state.
#[repr(C)]
#[derive(PartialEq, Debug, Clone, Copy)]
pub struct DexMarketInfo {
  /// Initialized state.
  pub account_flags: u16,
  /// owner address
  pub amm_info: Pubkey,
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

impl DexMarketInfo {
  /// flags
  pub fn flags(&self) -> Result<BitFlags<AccountFlag>, ProgramError> {
    BitFlags::from_bits(self.account_flags)
      .map_err(|_| ProgramError::InvalidAccountData)
      .map(Into::into)
  }
}

impl Sealed for DexMarketInfo {}
impl IsInitialized for DexMarketInfo {
  fn is_initialized(&self) -> bool {
    BitFlags::from_bits(self.account_flags)
      .map(|x| x.contains(AccountFlag::Initialized))
      .unwrap_or(false)
  }
}

impl Pack for DexMarketInfo {
  const LEN: usize = 194;

  fn pack_into_slice(&self, output: &mut [u8]) {
    let output = array_mut_ref![output, 0, 194];
    #[rustfmt::skip]
    let (
      account_flags,
      amm_info,
      market,
      pc_mint,
      coin_mint,
      open_orders,
      dex_program_id,
    ) = mut_array_refs![output,2, 32, 32, 32, 32, 32, 32];
    account_flags.copy_from_slice(&self.account_flags.to_le_bytes()[..]);
    amm_info.copy_from_slice(self.amm_info.as_ref());
    market.copy_from_slice(self.market.as_ref());
    pc_mint.copy_from_slice(self.pc_mint.as_ref());
    coin_mint.copy_from_slice(self.coin_mint.as_ref());
    open_orders.copy_from_slice(self.open_orders.as_ref());
    dex_program_id.copy_from_slice(self.dex_program_id.as_ref());
  }

  fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
    let input = array_ref![input, 0, 194];
    #[rustfmt::skip]
    let (
      &account_flags,
      amm_info,
      market,
      pc_mint,
      coin_mint,
      open_orders,
      dex_program_id
    ) =
      array_refs![input, 2, 32, 32, 32, 32, 32, 32];
    Ok(Self {
      account_flags: u16::from_le_bytes(account_flags),
      amm_info: Pubkey::new(amm_info),
      market: Pubkey::new(market),
      pc_mint: Pubkey::new(pc_mint),
      coin_mint: Pubkey::new(coin_mint),
      open_orders: Pubkey::new(open_orders),
      dex_program_id: Pubkey::new(dex_program_id),
    })
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
