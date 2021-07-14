use crate::error::OneSolError;
use solana_program::{account_info::AccountInfo, program_pack::Pack, pubkey::Pubkey};
// use std::convert::TryInto;

/// Unpacks a spl_token `Account`.
pub fn unpack_token_account(
    account_info: &AccountInfo,
    token_program_id: &Pubkey,
) -> Result<spl_token::state::Account, OneSolError> {
    if account_info.owner != token_program_id {
        Err(OneSolError::IncorrectTokenProgramId)
    } else {
        spl_token::state::Account::unpack(&account_info.data.borrow())
            .map_err(|_| OneSolError::ExpectedAccount)
    }
}

// /// convert u64 to u128
// pub fn to_u128(val: u64) -> Result<u128, OneSolError> {
//     val.try_into().map_err(|_| OneSolError::ConversionFailure)
// }

// /// convert u128 to u64
// pub fn to_u64(val: u128) -> Result<u64, OneSolError> {
//     val.try_into().map_err(|_| OneSolError::ConversionFailure)
// }
