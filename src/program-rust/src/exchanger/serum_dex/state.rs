use arrayref::{array_ref, array_refs};
use solana_program::{program_error::ProgramError, pubkey::Pubkey};

const ACCOUNT_HEAD_PADDING: &[u8; 5] = b"serum";
const ACCOUNT_TAIL_PADDING: &[u8; 7] = b"padding";

#[repr(C)]
#[derive(Debug, PartialEq, Clone)]
pub struct MarketState {
  // 0
  pub account_flags: u64, // Initialized, Market
  // 1
  pub own_address: Pubkey,
  // 5
  pub vault_signer_nonce: u64,
  // 6
  pub coin_mint: Pubkey,
  // 10
  pub pc_mint: Pubkey,

  // 14
  pub coin_vault: Pubkey,
  // 18
  pub coin_deposits_total: u64,
  // 19
  pub coin_fees_accrued: u64,

  // 20
  pub pc_vault: Pubkey,
  // 24
  pub pc_deposits_total: u64,
  // 25
  pub pc_fees_accrued: u64,

  // 26
  pub pc_dust_threshold: u64,

  // 27
  pub req_q: Pubkey,
  // 31
  pub event_q: Pubkey,

  // 35
  pub bids: Pubkey,
  // 39
  pub asks: Pubkey,

  // 43
  pub coin_lot_size: u64,
  // 44
  pub pc_lot_size: u64,
  // // 45
  // pub fee_rate_bps: u64,
  // // 46
  // pub referrer_rebates_accrued: u64,
}

impl MarketState {
  #[allow(dead_code)]
  const LEN: usize = 388;

  pub fn unpack_from_slice(input: &[u8]) -> Result<MarketState, ProgramError> {
    if input.len() <= 12 {
      return Err(ProgramError::InvalidAccountData);
    }
    #[allow(clippy::ptr_offset_with_cast)]
    let (head, data, tail) = array_refs![input, 5; ..; 7];
    if head != ACCOUNT_HEAD_PADDING {
      return Err(ProgramError::InvalidAccountData);
    }
    if tail != ACCOUNT_TAIL_PADDING {
      return Err(ProgramError::InvalidAccountData);
    }
    let input = array_ref![data, 0, 360];
    let (
      account_flags_arr,
      own_address_arr,
      vault_signer_nonce,
      coin_mint_arr,
      pc_mint_arr,
      coin_vault_arr,
      coin_deposits_total_arr,
      coin_fees_accrued_arr,
      pc_vault_arr,
      pc_deposits_total_arr,
      pc_fees_accrued_arr,
      pc_dust_threshold_arr,
      req_q_arr,
      event_q_arr,
      bids_arr,
      asks_arr,
      coin_lot_size_arr,
      pc_lot_size_arr,
    ) = array_refs![input, 8, 32, 8, 32, 32, 32, 8, 8, 32, 8, 8, 8, 32, 32, 32, 32, 8, 8];
    Ok(MarketState {
      account_flags: u64::from_le_bytes(*account_flags_arr),
      own_address: Pubkey::new_from_array(*own_address_arr),
      vault_signer_nonce: u64::from_le_bytes(*vault_signer_nonce),
      coin_mint: Pubkey::new_from_array(*coin_mint_arr),
      pc_mint: Pubkey::new_from_array(*pc_mint_arr),
      coin_vault: Pubkey::new_from_array(*coin_vault_arr),
      coin_deposits_total: u64::from_le_bytes(*coin_deposits_total_arr),
      coin_fees_accrued: u64::from_le_bytes(*coin_fees_accrued_arr),
      pc_vault: Pubkey::new_from_array(*pc_vault_arr),
      pc_deposits_total: u64::from_le_bytes(*pc_deposits_total_arr),
      pc_fees_accrued: u64::from_le_bytes(*pc_fees_accrued_arr),
      pc_dust_threshold: u64::from_le_bytes(*pc_dust_threshold_arr),
      req_q: Pubkey::new_from_array(*req_q_arr),
      event_q: Pubkey::new_from_array(*event_q_arr),
      bids: Pubkey::new_from_array(*bids_arr),
      asks: Pubkey::new_from_array(*asks_arr),
      coin_lot_size: u64::from_le_bytes(*coin_lot_size_arr),
      pc_lot_size: u64::from_le_bytes(*pc_lot_size_arr),
    })
  }
}
