use std::mem::size_of;

use solana_program::{
  instruction::{AccountMeta, Instruction},
  program_error::ProgramError,
  pubkey::Pubkey,
};

#[derive(Clone, Debug, PartialEq)]
struct Swap {
  /// SOURCE amount to transfer, output to DESTINATION is based on the exchange rate
  pub amount_in: u64,
  /// Minimum amount of DESTINATION token to output, prevents excessive slippage
  pub minimum_amount_out: u64,
}

#[derive(Debug, PartialEq)]
enum SwapInstrution {
  Swap(Swap),
}

impl SwapInstrution {
  pub fn pack(&self) -> Vec<u8> {
    let mut buf = Vec::with_capacity(size_of::<Self>());
    match &*self {
      Self::Swap(Swap {
        amount_in,
        minimum_amount_out,
      }) => {
        buf.push(1);
        buf.extend_from_slice(&amount_in.to_le_bytes());
        buf.extend_from_slice(&minimum_amount_out.to_le_bytes());
      }
    };
    buf
  }
}

#[allow(clippy::too_many_arguments)]
pub fn swap_instruction(
  program_id: &Pubkey,
  swap_info_account: &Pubkey,
  swap_authority: &Pubkey,
  user_authority: &Pubkey,
  user_source_token_account: &Pubkey,
  user_destination_token_account: &Pubkey,
  pool_source_token_account: &Pubkey,
  pool_destination_token_account: &Pubkey,
  tick_dist_account: &Pubkey,
  token_program_id: &Pubkey,
  amount_in: u64,
  minimum_amount_out: u64,
) -> Result<Instruction, ProgramError> {
  let data = SwapInstrution::Swap(Swap {
    amount_in,
    minimum_amount_out,
  })
  .pack();
  // let swap_key = Pubkey::from_str(CREMA_SWAP_ACCOUNT).unwrap();
  // let (authority, _) = Pubkey::find_program_address(
  //   &[&swap_key.to_bytes()[..]],
  //   &Pubkey::from_str(CREMA_PROGRAM_ID).unwrap(),
  // );

  let accounts = vec![
    AccountMeta::new(*swap_info_account, false),
    AccountMeta::new_readonly(*swap_authority, false),
    AccountMeta::new_readonly(*user_authority, true),
    AccountMeta::new(*user_source_token_account, false),
    AccountMeta::new(*user_destination_token_account, false),
    AccountMeta::new(*pool_source_token_account, false),
    AccountMeta::new(*pool_destination_token_account, false),
    AccountMeta::new(*tick_dist_account, false),
    AccountMeta::new_readonly(*token_program_id, false),
  ];

  Ok(Instruction {
    program_id: *program_id,
    accounts,
    data,
  })
}
