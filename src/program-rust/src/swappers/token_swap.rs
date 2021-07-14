//! TokenSwap
use crate::swappers;
use solana_program::{account_info::AccountInfo, program_error::ProgramError};
// use spl_token_swap::curve::base::SwapCurve;

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum SwapperType {
    SplTokenSwap,
    Test,
}

pub trait Swapper {
    fn invoke_swap(&self, amount_in: u64, minimum_amount_out: u64) -> Result<(), ProgramError>;
}

#[derive(Clone, Debug)]
pub struct TokenSwap<'a> {
    swapper_type: SwapperType,
    accounts: Vec<AccountInfo<'a>>,
}

impl<'a> TokenSwap<'a> {
    /// accounts
    ///   0. `[]` token_program_info
    ///   1. `[]` user_transfer_authority_info
    ///   2. `[]` middle_source_info
    ///   3. `[]` middle_destination_info
    ///   4. `[]` token-swap account
    ///   5. `[]` token-swap authority
    ///   6. `[writable]` token_A Base Account to swap INTO.  Must be the SOURCE token.
    ///   7. `[writable]` token_B Base Account to swap FROM.  Must be the DESTINATION token.
    ///   8. `[writable]` Pool token mint, to generate trading fees
    ///   9. `[writable]` Fee account, to receive trading fees
    ///   10. '[]` Token-Swap program id
    ///   11 `[optional, writable]` Host fee account to receive additional trading fees
    pub fn new_spl_token_swap(
        token_program_info: AccountInfo<'a>,
        user_transfer_authority_info: AccountInfo<'a>,
        middle_source_info: AccountInfo<'a>,
        middle_destination_info: AccountInfo<'a>,
        accounts: &[AccountInfo<'a>],
    ) -> Result<TokenSwap<'a>, ProgramError> {
        let mut a = vec![
            token_program_info,
            user_transfer_authority_info,
            middle_source_info,
            middle_destination_info,
        ];
        a.extend(accounts.iter().cloned());
        Ok(TokenSwap {
            swapper_type: SwapperType::SplTokenSwap,
            accounts: a,
        })
    }

    #[allow(dead_code)]
    pub fn new_test_swap() -> Result<TokenSwap<'a>, ProgramError> {
        Ok(TokenSwap {
            swapper_type: SwapperType::Test,
            accounts: vec![],
        })
    }
}

impl<'a> Swapper for TokenSwap<'a> {
    fn invoke_swap(&self, amount_in: u64, minimum_amount_out: u64) -> Result<(), ProgramError> {
        match self.swapper_type {
            SwapperType::SplTokenSwap => swappers::spl_token_swap::process_token_swap_invoke_swap(
                &self.accounts[..],
                amount_in,
                minimum_amount_out,
            ),
            SwapperType::Test => Ok(()),
        }
    }
}
