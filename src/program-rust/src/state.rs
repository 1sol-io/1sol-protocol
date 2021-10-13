//! State transition types
use crate::error::ProtocolError;
use bytemuck::{cast_slice_mut, from_bytes_mut, Pod, Zeroable};
use enumflags2::{bitflags, BitFlags};
use safe_transmute::{self, trivial::TriviallyTransmutable};
use solana_program::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};
use std::cell::RefMut;

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

#[cfg(target_endian = "little")]
unsafe impl Zeroable for AmmInfo {}
#[cfg(target_endian = "little")]
unsafe impl Pod for AmmInfo {}
#[cfg(target_endian = "little")]
unsafe impl TriviallyTransmutable for AmmInfo {}

/// load account flags
pub fn account_flags(account_data: &[u8]) -> Result<BitFlags<AccountFlag>, ProtocolError> {
  let mut flag_bytes = [0u8; 2];
  flag_bytes.copy_from_slice(&account_data[0..2]);
  BitFlags::from_bits(u16::from_le_bytes(flag_bytes))
    .map_err(|_| ProtocolError::InvalidAccountFlags)
    .map(Into::into)
}

impl AmmInfo {
  /// load onesol amm info
  #[inline]
  pub fn load_mut<'a>(
    account: &'a AccountInfo,
    check_flag: bool,
  ) -> Result<RefMut<'a, AmmInfo>, ProtocolError> {
    if check_flag {
      let flags = account_flags(
        &account
          .try_borrow_data()
          .map_err(|_| ProtocolError::BorrowAccountDataError)?,
      )?;
      if !flags.contains(AccountFlag::AmmInfo) {
        return Err(ProtocolError::InvalidAccountFlags);
      }
    }

    let account_data: RefMut<'a, [u8]>;
    let amm_data: RefMut<'a, AmmInfo>;

    account_data = RefMut::map(
      account
        .try_borrow_mut_data()
        .map_err(|_| ProtocolError::BorrowAccountDataError)?,
      |data| *data,
    );
    amm_data = RefMut::map(account_data, |data| from_bytes_mut(cast_slice_mut(data)));
    Ok(amm_data)
  }
  /// flags
  pub fn flags(&self) -> Result<BitFlags<AccountFlag>, ProgramError> {
    BitFlags::from_bits(self.account_flags)
      .map_err(|_| ProgramError::InvalidAccountData)
      .map(Into::into)
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

#[cfg(target_endian = "little")]
unsafe impl Zeroable for DexMarketInfo {}
#[cfg(target_endian = "little")]
unsafe impl Pod for DexMarketInfo {}
#[cfg(target_endian = "little")]
unsafe impl TriviallyTransmutable for DexMarketInfo {}

impl DexMarketInfo {
  /// load onesol amm info
  #[inline]
  pub fn load_mut<'a>(
    account: &'a AccountInfo,
    check_flag: bool,
  ) -> Result<RefMut<'a, DexMarketInfo>, ProtocolError> {
    if check_flag {
      let flags = account_flags(
        &account
          .try_borrow_data()
          .map_err(|_| ProtocolError::BorrowAccountDataError)?,
      )?;
      if !flags.contains(AccountFlag::DexMarketInfo) {
        return Err(ProtocolError::InvalidAccountFlags);
      }
    }

    let account_data: RefMut<'a, [u8]>;
    let amm_data: RefMut<'a, DexMarketInfo>;

    account_data = RefMut::map(
      account
        .try_borrow_mut_data()
        .map_err(|_| ProtocolError::BorrowAccountDataError)?,
      |data| *data,
    );
    amm_data = RefMut::map(account_data, |data| from_bytes_mut(cast_slice_mut(data)));
    Ok(amm_data)
  }
  /// flags
  pub fn flags(&self) -> Result<BitFlags<AccountFlag>, ProgramError> {
    BitFlags::from_bits(self.account_flags)
      .map_err(|_| ProgramError::InvalidAccountData)
      .map(Into::into)
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
