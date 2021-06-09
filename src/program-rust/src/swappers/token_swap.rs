//! TokenSwap
use crate::swappers;
use solana_program::{account_info::AccountInfo, program_error::ProgramError};

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum SwapperType {
    SplTokenSwap,
    Test,
}

pub trait Swapper {
    fn calculate_swap(&self, amount: u64, parts: u64) -> Result<(Vec<u64>, u64), ProgramError>;

    fn invoke_swap(&self, amount_in: u64, minimum_amount_out: u64) -> Result<(), ProgramError>;
}

#[derive(Clone, Debug)]
pub struct TokenSwap<'a> {
    swapper_type: SwapperType,
    accounts: Vec<AccountInfo<'a>>,
}

impl<'a> TokenSwap<'a> {
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
    fn calculate_swap(
        &self,
        amount: u64,
        parts: u64,
        // accounts: &[AccountInfo],
    ) -> Result<(Vec<u64>, u64), ProgramError> {
        match self.swapper_type {
            SwapperType::SplTokenSwap => {
                return swappers::spl_token_swap::process_token_swap_calculate_swap(
                    &self.accounts[..],
                    amount,
                    parts,
                );
            }
            SwapperType::Test => {
                let amounts = swappers::linear_interpolation(amount, parts);
                // let mut rets = vec![0; amounts.len()];
                let rets = (0..amounts.len())
                    .map(|_| amount / amounts.len() as u64)
                    .collect::<Vec<u64>>();
                return Ok((rets, 0));
            }
        }
    }

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
