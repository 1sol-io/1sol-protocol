//! Program state processor

use crate::{
    error::OneSolError,
    instruction::{Initialize, OneSolInstruction, Swap},
    instructions::token_swap,
};

use core::i64::MIN;
use num_traits::FromPrimitive;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    decode_error::DecodeError,
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_error::{PrintProgramError, ProgramError},
    program_pack::Pack,
    pubkey::Pubkey,
};
use std::convert::TryInto;

/// Program state handler.
pub struct Processor {}

impl Processor {
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

    /// Processes an [Instruction](enum.Instruction.html).
    pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]) -> ProgramResult {
        let instruction = OneSolInstruction::unpack(input)?;
        match instruction {
            OneSolInstruction::Initialize(Initialize {}) => {
                msg!("Instruction: Initialize");
                Ok(())
            }
            OneSolInstruction::Swap(Swap {
                amount_in,
                minimum_amount_out,
                nonce,
            }) => {
                msg!("Instruction: Swap");
                Self::process_swap(program_id, amount_in, minimum_amount_out, nonce, accounts)
            }
        }
    }

    /// process
    pub fn process_initialize(program_id: &Pubkey, _accounts: &[AccountInfo]) -> ProgramResult {
        msg!("start process_initialize, {}", program_id);
        Ok(())
    }

    /// Processes an [Swap](enum.Instruction.html).
    pub fn process_swap(
        program_id: &Pubkey,
        amount_in: u64,
        minimum_amount_out: u64,
        nonce: u8,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        msg!("start process swap, accounts.len: {}", accounts.len());
        let onesol_info = next_account_info(account_info_iter)?;
        let swap_info = next_account_info(account_info_iter)?;
        let onesol_authority_info = next_account_info(account_info_iter)?;
        let swap_authority_info = next_account_info(account_info_iter)?;
        let user_transfer_authority_info = next_account_info(account_info_iter)?;
        let source_info = next_account_info(account_info_iter)?;
        let onesol_source_info = next_account_info(account_info_iter)?;
        let swap_source_info = next_account_info(account_info_iter)?;
        let swap_destination_info = next_account_info(account_info_iter)?;
        let onesol_destination_info = next_account_info(account_info_iter)?;
        let destination_info = next_account_info(account_info_iter)?;
        let pool_mint_info = next_account_info(account_info_iter)?;
        let pool_fee_account_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;
        let token_swap_account_info = next_account_info(account_info_iter)?;
        let mut host_fee_pubkey: Option<&Pubkey> = None;
        let host_fee_account_info = next_account_info(account_info_iter);
        if let Ok(_host_fee_account_info) = host_fee_account_info {
            host_fee_pubkey = Some(_host_fee_account_info.key);
        }
        if onesol_info.owner != program_id {
            return Err(ProgramError::IncorrectProgramId);
        };
        if *onesol_authority_info.key != Self::authority_id(program_id, onesol_info.key, nonce)? {
            return Err(OneSolError::InvalidProgramAddress.into());
        }
        // let _token_swap = spl_token_swap::state::SwapVersion::unpack(&swap_info.data.borrow())?;
        // let _token_swap = token_swap::SwapVersion::unpack(&swap_info.data.borrow())?;
        // transfer AliceA -> OnesolA
        msg!("transfer AliceA -> onesolA");
        Self::token_transfer(
            onesol_info.key,
            token_program_info.clone(),
            source_info.clone(),
            onesol_source_info.clone(),
            user_transfer_authority_info.clone(),
            nonce,
            amount_in,
        )
        .unwrap();

        let best = Self::get_expected_return_with_gas(
            amount_in,
            find_best_parts(amount_in), // I don't know which value should be use.
            &[&[
                swap_info.clone(),
                swap_source_info.clone(),
                swap_destination_info.clone(),
            ]],
        );
        msg!("Best split is {:?}", best);
        // TODO 这里需要一个计算 对应交易所 amount 的方法

        let token_swap_amount_in = best[0] * amount_in;
        // TODO calculate minimum_amount_out for Token Swap

        // Swap OnesolA -> OnesolB
        msg!("swap onesolA -> onesolB using token-swap");
        // let token_swap_program_id = Pubkey::from_str(TOKEN_SWAP_PROGRAM_ADDRESS).unwrap();

        let instruction = token_swap::Swap {
            amount_in: token_swap_amount_in,
            minimum_amount_out: minimum_amount_out,
        };

        let swap = token_swap::swap(
            token_swap_account_info.key,
            token_program_info.key,
            swap_info.key,
            swap_authority_info.key,
            user_transfer_authority_info.key,
            onesol_source_info.key,
            swap_source_info.key,
            swap_destination_info.key,
            onesol_destination_info.key,
            pool_mint_info.key,
            pool_fee_account_info.key,
            host_fee_pubkey,
            instruction,
        )?;
        let mut swap_accounts = vec![
            swap_info.clone(),
            swap_authority_info.clone(),
            user_transfer_authority_info.clone(),
            onesol_source_info.clone(),
            swap_source_info.clone(),
            swap_destination_info.clone(),
            onesol_destination_info.clone(),
            pool_mint_info.clone(),
            pool_fee_account_info.clone(),
            token_program_info.clone(),
        ];
        if let Ok(_host_fee_account_info) = host_fee_account_info {
            swap_accounts.push(_host_fee_account_info.clone());
        }
        // invoke tokenswap
        msg!(
            "swap onesolA -> onesolB invoke token_swap {}, {}",
            swap.accounts.len(),
            swap_accounts.len()
        );
        invoke(&swap, &swap_accounts[..])?;
        // invoke_signed(&swap, &swap_accounts[..], &[&[swap_info]])

        let dest_account =
            spl_token::state::Account::unpack(&onesol_destination_info.data.borrow())?;
        msg!("onesol_destination amount: {}", dest_account.amount);
        // Transfer OnesolB -> AliceB
        // TODO 这里应该确定一下 amout_out
        msg!("transfer OneSolB -> AliceB");
        Self::token_transfer(
            onesol_info.key,
            token_program_info.clone(),
            onesol_destination_info.clone(),
            destination_info.clone(),
            user_transfer_authority_info.clone(),
            // _token_swap.nonce(),
            nonce,
            // _token_swap.nonce(),
            dest_account.amount,
        )
        .unwrap();

        Ok(())
    }

    /// Issue a spl_token `Transfer` instruction.
    pub fn token_transfer<'a>(
        swap: &Pubkey,
        token_program: AccountInfo<'a>,
        source: AccountInfo<'a>,
        destination: AccountInfo<'a>,
        authority: AccountInfo<'a>,
        nonce: u8,
        amount: u64,
    ) -> Result<(), ProgramError> {
        let swap_bytes = swap.to_bytes();
        let authority_signature_seeds = [&swap_bytes[..32], &[nonce]];
        let signers = &[&authority_signature_seeds[..]];
        let ix = spl_token::instruction::transfer(
            token_program.key,
            source.key,
            destination.key,
            authority.key,
            &[],
            amount,
        )?;
        // invoke(&ix, &[source, destination, authority, token_program])
        invoke_signed(
            &ix,
            &[source, destination, authority, token_program],
            signers,
        )
    }

    /// Calculates the authority id by generating a program address.
    pub fn authority_id(
        program_id: &Pubkey,
        my_info: &Pubkey,
        nonce: u8,
    ) -> Result<Pubkey, OneSolError> {
        Pubkey::create_program_address(&[&my_info.to_bytes()[..32], &[nonce]], program_id)
            .or(Err(OneSolError::InvalidProgramAddress))
    }

    /// https://github.com/1inch/1inchProtocol/blob/master/contracts/OneSplitBase.sol\#L139
    fn _find_best_distribution(
        s: u64,                 // parts
        amounts: Vec<Vec<i64>>, // exchangesReturns
        size: usize,
    ) -> Vec<u64> {
        let n = amounts.len();

        let mut answer: Vec<Vec<i64>> = vec![vec![MIN; (s + 1) as usize]; n];
        let mut parent: Vec<Vec<u64>> = vec![vec![0; (s + 1) as usize]; n];

        for j in 0..(s + 1) {
            answer[0][j as usize] = amounts[0][j as usize] as i64;
            // Aleardy initlize.
            // for i in (1..n) {
            //     answer[i as usize][j as usize] = MIN;
            // }
            // parent[0][j as usize] = 0;
        }
        // println!("_findBestDistribution: before {:?}", answer);

        for i in 1..n {
            for j in 0..(s + 1) {
                answer[i as usize][j as usize] = answer[(i - 1) as usize][j as usize];
                parent[i as usize][j as usize] = j;

                for k in 1..(j + 1) {
                    let a = answer[(i - 1) as usize][(j - k) as usize]
                        + amounts[i as usize][k as usize] as i64;
                    if a > answer[i as usize][j as usize] {
                        answer[i as usize][j as usize] = a;
                        parent[i as usize][j as usize] = j - k;
                    }
                }
            }
        }
        let mut distribution: Vec<u64> = vec![0; size];

        let mut parts_left = s;
        let mut cur_exchange: i64 = n as i64 - 1;
        while parts_left > 0 {
            distribution[cur_exchange as usize] =
                parts_left - parent[cur_exchange as usize][parts_left as usize];
            parts_left = parent[cur_exchange as usize][parts_left as usize];
            cur_exchange -= 1;
            // Keep safe.
            if cur_exchange < 0 {
                break;
            }
        }

        // Useless.
        // let returnAmount = if (answer[(n - 1) as usize][s as usize] == MIN) { 0 } else { answer[(n - 1) as usize][s as usize] as u64 };

        return distribution;
    }

    /// get expected return with gas
    /// amount:
    /// parts:
    /// accounts:
    ///     1. Token Swap
    ///         * Token Swap Program AccountInfo
    ///         * TokenA AccountInfo
    ///         * TokenB AccountInfo
    fn get_expected_return_with_gas(
        amount: u64,
        parts: u64, // Number of pieces source volume could be splitted
        accounts: &[&[AccountInfo]],
    ) -> Vec<u64> {
        let mut at_least_one_positive = false;
        let size = accounts.len();
        let mut matrix: Vec<Vec<i64>> = vec![vec![0; (parts + 1) as usize]; size];
        let mut gases = vec![0; size];

        for i in 0..size {
            let (rets, gas) = match Self::calculate_swap(i, amount, parts, accounts[i]) {
                Ok((a, b)) => (a, b),
                Err(_) => (vec![0], 0),
            };
            gases[i as usize] = gas;
            for j in 0..rets.len() {
                matrix[i][j + 1] = (rets[j] as i64) - (gas as i64);
                at_least_one_positive = at_least_one_positive || (matrix[i][j + 1] > 0);
            }
        }

        if !at_least_one_positive {
            for i in 0..size {
                for j in 1..parts + 1 {
                    if matrix[i as usize][j as usize] == 0 {
                        matrix[i as usize][j as usize] = MIN;
                    }
                }
            }
        }

        let distribution = Self::_find_best_distribution(parts, matrix, size);

        return distribution;
    }

    fn calculate_swap(
        index: usize,
        amount: u64,
        parts: u64,
        accounts: &[AccountInfo],
    ) -> Result<(Vec<u64>, u64), ProgramError> {
        if index == 0 {
            return Self::calculate_token_swap(amount, parts, accounts);
        }
        // if not support swap return 0
        return Ok((vec![0], 0));
    }

    fn _linear_interpolation(value: u64, parts: u64) -> Vec<u64> {
        let mut rets = vec![0; parts as usize];
        for i in 0..parts {
            rets[i as usize] = value * (i + 1) / parts;
        }
        rets
    }

    fn calculate_token_swap(
        amount: u64,
        parts: u64,
        accounts: &[AccountInfo],
    ) -> Result<(Vec<u64>, u64), ProgramError> {
        let amounts = Self::_linear_interpolation(amount, parts);
        let mut rets = vec![0; amounts.len()];
        let account_iters = &mut accounts.iter();
        let token_swap_program_account = next_account_info(account_iters)?;
        let source_info = next_account_info(account_iters)?;
        let destination_info = next_account_info(account_iters)?;
        let _token_swap =
            spl_token_swap::state::SwapVersion::unpack(&token_swap_program_account.data.borrow())?;
        let source_account =
            Self::unpack_token_account(source_info, &_token_swap.token_program_id())?;
        let dest_account =
            Self::unpack_token_account(destination_info, &_token_swap.token_program_id())?;

        let trade_direction = if *source_info.key == *_token_swap.token_a_account() {
            spl_token_swap::curve::calculator::TradeDirection::AtoB
        } else {
            spl_token_swap::curve::calculator::TradeDirection::BtoA
        };

        let mut source_amount = source_account.amount;
        let mut destination_amount = dest_account.amount;

        for i in 0..amounts.len() {
            let result = _token_swap
                .swap_curve()
                .swap(
                    to_u128(amounts[i])?,
                    to_u128(source_amount)?,
                    to_u128(destination_amount)?,
                    trade_direction,
                    _token_swap.fees(),
                )
                .ok_or(OneSolError::ZeroTradingTokens)?;
            rets[i] = to_u64(result.destination_amount_swapped)?;
            source_amount = to_u64(result.new_swap_source_amount)?;
            destination_amount = to_u64(result.new_swap_destination_amount)?;
        }
        Ok((rets, 0))
    }
}

impl PrintProgramError for OneSolError {
    fn print<E>(&self)
    where
        E: 'static + std::error::Error + DecodeError<E> + PrintProgramError + FromPrimitive,
    {
        match self {
            OneSolError::Unknown => msg!("Error: Unknown"),
            OneSolError::InvalidInstruction => msg!("Error: InvalidInstruction"),
            OneSolError::InvalidProgramAddress => msg!("Error: InvildProgramAddress"),
            OneSolError::ExpectedAccount => msg!("Error: ExpectedAccount"),
            OneSolError::IncorrectTokenProgramId => msg!("Error: IncorrectTokenProgramId"),
            OneSolError::ConversionFailure => msg!("Error: ConversionFailure"),
            OneSolError::ZeroTradingTokens => msg!("Error: ZeroTradingTokens"),
        }
    }
}

fn to_u128(val: u64) -> Result<u128, OneSolError> {
    val.try_into().map_err(|_| OneSolError::ConversionFailure)
}

fn to_u64(val: u128) -> Result<u64, OneSolError> {
    val.try_into().map_err(|_| OneSolError::ConversionFailure)
}

fn find_best_parts(amount: u64) -> u64 {
    if amount > 100 {
        return 100;
    }
    if amount < 2 {
        return 2;
    }
    amount
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_distribution() {
        // let result = Processor::get_expected_return_with_gas(10, 100, vec![token_swap_curve_1, token_swap_curve_2]);
        // println!("getExpectedReturnWithGas: {:?}", result);
        // assert_eq!(result, vec![90, 10]);
    }
}
