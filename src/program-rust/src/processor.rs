//! Program state processor

use crate::{
    error::OneSolError,
    instruction::{OneSolInstruction, Swap},
    instructions::token_swap,
};

use core::i64::MIN;
use num_traits::FromPrimitive;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    decode_error::DecodeError,
    entrypoint::ProgramResult,
    log::sol_log_compute_units,
    msg,
    program::{invoke, invoke_signed},
    program_error::{PrintProgramError, ProgramError},
    program_pack::Pack,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction::create_account,
    sysvar::Sysvar,
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
            OneSolInstruction::Swap(Swap {
                amount_in,
                minimum_amount_out,
                nonce,
                token_swap_config,
            }) => {
                msg!("Instruction: Swap");
                Self::process_swap(
                    program_id,
                    amount_in,
                    minimum_amount_out,
                    nonce,
                    token_swap_config,
                    accounts,
                )
            }
        }
    }

    /// Processes an [Swap](enum.Instruction.html).
    pub fn process_swap(
        _program_id: &Pubkey,
        amount_in: u64,
        minimum_amount_out: u64,
        nonce: u8,
        token_swap_config: (bool, usize),
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        msg!("start process swap");

        let account_info_iter = &mut accounts.iter();
        msg!("start process swap, accounts.len: {}", accounts.len());
        let middle_owner_info = next_account_info(account_info_iter)?;
        let user_transfer_authority_info = next_account_info(account_info_iter)?;
        // let onesol_authority_info = next_account_info(account_info_iter)?;
        // let user_transfer_authority_info = next_account_info(account_info_iter)?;
        let source_info = next_account_info(account_info_iter)?;
        let middle_source_info = next_account_info(account_info_iter)?;
        let middle_destination_info = next_account_info(account_info_iter)?;
        let destination_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;

        let token_program_id = *token_program_info.key;
        let middle_source_token =
            Self::unpack_token_account(middle_source_info, &token_program_id)?;
        let source_token = Self::unpack_token_account(source_info, &token_program_id)?;
        if middle_source_token.mint != source_token.mint {
            return Err(OneSolError::InvalidInput.into());
        }
        // if *user_transfer_authority_info.key != middle_source_token.delegate {
        //     return Err(OneSolError::InvalidOwner.into());
        // }

        let middle_destination_token =
            Self::unpack_token_account(middle_destination_info, &token_program_id)?;
        let destination_token = Self::unpack_token_account(destination_info, &token_program_id)?;
        if middle_destination_token.mint != destination_token.mint {
            return Err(OneSolError::InvalidInput.into());
        }
        // if *user_transfer_authority_info.key != middle_destination_token.owner {
        //     return Err(OneSolError::InvalidOwner.into());
        // }
        let mut calculate_swaps = vec![];

        // load token-swap data
        let token_swap_data = if token_swap_config.0 {
            let swap_info = next_account_info(account_info_iter)?;
            let swap_authority_info = next_account_info(account_info_iter)?;
            let swap_source_info = next_account_info(account_info_iter)?;
            let swap_destination_info = next_account_info(account_info_iter)?;
            let pool_mint_info = next_account_info(account_info_iter)?;
            let pool_fee_account_info = next_account_info(account_info_iter)?;
            let token_swap_program_info = next_account_info(account_info_iter)?;
            let host_fee_account_info = if token_swap_config.1 == 8 {
                Some(next_account_info(account_info_iter)?)
            } else {
                None
            };
            let mut host_fee_pubkey: Option<&Pubkey> = None;
            if let Some(_host_fee_account_info) = host_fee_account_info {
                host_fee_pubkey = Some(_host_fee_account_info.key);
            }

            let keys = vec![
                Some(token_swap_program_info.key),
                Some(token_program_info.key),
                Some(swap_info.key),
                Some(swap_authority_info.key),
                Some(user_transfer_authority_info.key),
                Some(middle_source_info.key),
                Some(swap_source_info.key),
                Some(swap_destination_info.key),
                Some(middle_destination_info.key),
                Some(pool_mint_info.key),
                Some(pool_fee_account_info.key),
                host_fee_pubkey,
            ];
            let mut swap_accounts = vec![
                swap_info.clone(),
                swap_authority_info.clone(),
                user_transfer_authority_info.clone(),
                middle_source_info.clone(),
                swap_source_info.clone(),
                swap_destination_info.clone(),
                middle_destination_info.clone(),
                pool_mint_info.clone(),
                pool_fee_account_info.clone(),
                token_program_info.clone(),
            ];
            if let Some(_host_fee_account_info) = host_fee_account_info {
                swap_accounts.push(_host_fee_account_info.clone());
            }
            calculate_swaps.push(vec![
                swap_info.clone(),
                swap_source_info.clone(),
                swap_destination_info.clone(),
            ]);
            Some((swap_accounts, keys))
        } else {
            None
        };

        msg!("transfer AliceA -> onesolA");
        // transfer AliceA -> OnesolA
        Self::token_transfer(
            middle_owner_info.key,
            token_program_info.clone(),
            source_info.clone(),
            middle_source_info.clone(),
            user_transfer_authority_info.clone(),
            nonce,
            amount_in,
        )
        .unwrap();

        let dest_account1 =
            spl_token::state::Account::unpack(&middle_destination_info.data.borrow())?;

        let amount1 = dest_account1.amount;

        let (best, parts) = if calculate_swaps.len() > 1 {
            let _parts = find_best_parts(amount_in);
            msg!("best parts: {}", _parts);
            sol_log_compute_units();
            let _best = Self::get_expected_return_with_gas(amount_in, _parts, &calculate_swaps[..]);
            sol_log_compute_units();
            msg!("Best split is {:?}", _best);
            (_best, _parts)
        } else {
            (vec![1], 1)
        };

        let mut best_index: usize = 0;

        if token_swap_config.0 {
            // run Token Swap swap
            let token_swap_amount_in = best[best_index] * amount_in / parts;
            let token_swap_minimum_amount_out = best[best_index] * minimum_amount_out / parts;

            best_index += 1;

            // TODO calculate minium_amount_out for token_swap

            // Swap OnesolA -> OnesolB
            msg!(
                "swap onesolA -> onesolB using token-swap, amount_in: {}",
                token_swap_amount_in
            );
            // let token_swap_program_id = Pubkey::from_str(TOKEN_SWAP_PROGRAM_ADDRESS).unwrap();
            let d = token_swap_data.unwrap();
            let token_swap_accounts = d.0;
            let keys = d.1;

            let instruction = token_swap::Swap {
                // amount_in: token_swap_amount_in,
                amount_in: token_swap_amount_in,
                minimum_amount_out: token_swap_minimum_amount_out,
            };

            let swap = token_swap::swap(
                keys[0].unwrap(),
                keys[1].unwrap(),
                keys[2].unwrap(),
                keys[3].unwrap(),
                keys[4].unwrap(),
                keys[5].unwrap(),
                keys[6].unwrap(),
                keys[7].unwrap(),
                keys[8].unwrap(),
                keys[9].unwrap(),
                keys[10].unwrap(),
                keys[11],
                instruction,
            )?;
            // invoke token-swap
            invoke(&swap, &token_swap_accounts[..])?;
        }
        // invoke_signed(&swap, &swap_accounts[..], &[&[swap_info]])

        let dest_account =
            spl_token::state::Account::unpack(&middle_destination_info.data.borrow())?;
        msg!(
            "onesol_destination amount: {}, should tranfer: {}",
            dest_account.amount,
            dest_account.amount - amount1
        );
        // Transfer OnesolB -> AliceB
        // TODO 这里应该确定一下 amout_out
        msg!("transfer OneSolB -> AliceB");
        Self::token_transfer(
            middle_owner_info.key,
            token_program_info.clone(),
            middle_destination_info.clone(),
            destination_info.clone(),
            user_transfer_authority_info.clone(),
            // _token_swap.nonce(),
            nonce,
            // _token_swap.nonce(),
            dest_account.amount - amount1,
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

    /// create token account
    pub fn token_create_account<'a>(
        token_program: AccountInfo<'a>,
        payer_info: AccountInfo<'a>,
        token_info: AccountInfo<'a>,
        onesol_account_info: AccountInfo<'a>,
        token_account_info: AccountInfo<'a>,
    ) -> Result<(), ProgramError> {
        let rent = &Rent::from_account_info(&token_program)?;
        let l = 1.max(rent.minimum_balance(spl_token::state::Account::get_packed_len()));

        let create_account_instruction = create_account(
            payer_info.key,
            token_account_info.key,
            l,
            spl_token::state::Account::get_packed_len() as u64,
            token_program.key,
        );

        invoke(
            &create_account_instruction,
            &[payer_info.clone(), token_account_info.clone()],
        )?;

        let token_create_init_instruction = spl_token::instruction::initialize_account(
            token_program.key,
            token_account_info.key,
            token_info.key,
            onesol_account_info.key,
        )?;

        invoke(&token_create_init_instruction, &[])
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

        for j in 0..(s + 1) as usize {
            answer[0][j] = amounts[0][j] as i64;
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
        distribution
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
        accounts: &[Vec<AccountInfo>],
    ) -> Vec<u64> {
        let mut at_least_one_positive = false;
        let size = accounts.len();
        let mut matrix: Vec<Vec<i64>> = vec![vec![0; (parts + 1) as usize]; size];
        let mut gases = vec![0; size];

        for i in 0..size {
            let (rets, gas) = match Self::calculate_swap(i, amount, parts, &(accounts[i])[..]) {
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
            OneSolError::InvalidInput => msg!("Error: InvalidInput"),
            OneSolError::InvalidOwner => msg!("Error: InvalidOwner"),
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
    if amount > 50 {
        50
    } else if amount < 2 {
        2
    } else {
        amount
    }
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
