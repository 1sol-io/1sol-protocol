//! TokenSwap
use crate::{
    swappers,
    util::{to_u128, to_u64},
};
use solana_program::{account_info::AccountInfo, program_error::ProgramError};
// use spl_token_swap::curve::base::SwapCurve;

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
                return process_test_calculate_swap(&[], amount, parts);
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

pub fn process_test_calculate_swap(
    _accounts: &[AccountInfo],
    amount: u64,
    parts: u64,
) -> Result<(Vec<u64>, u64), ProgramError> {
    let amounts = swappers::linear_interpolation(amount, parts);
    let trade_direction = spl_token_swap::curve::calculator::TradeDirection::AtoB;

    let curve = spl_token_swap::curve::base::SwapCurve {
        curve_type: spl_token_swap::curve::base::CurveType::ConstantProduct,
        calculator: Box::new(spl_token_swap::curve::constant_product::ConstantProductCurve {}),
    };
    // curve.swap_without_fees(source_amount: u128, _swap_source_amount: u128, _swap_destination_amount: u128, trade_direction: TradeDirection)
    let source_amount = amount;
    let destination_amount = 1000000;
    let mut rets = vec![0; amounts.len()];

    for i in 0..amounts.len() {
        let result = curve
            .swap(
                to_u128(amounts[i])?,
                to_u128(source_amount)?,
                destination_amount,
                trade_direction,
                &spl_token_swap::curve::fees::Fees {
                    trade_fee_numerator: 0,
                    trade_fee_denominator: 0,
                    owner_trade_fee_numerator: 0,
                    owner_trade_fee_denominator: 0,
                    owner_withdraw_fee_numerator: 0,
                    owner_withdraw_fee_denominator: 0,
                    host_fee_numerator: 0,
                    host_fee_denominator: 0,
                },
            )
            .ok_or(ProgramError::Custom(1))?;
        rets[i] = to_u64(result.destination_amount_swapped)?;
    }
    Ok((rets, 0))
}
