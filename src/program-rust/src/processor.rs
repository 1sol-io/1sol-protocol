//! Program state processor

use crate::{
    error::OneSolError,
    instruction::{Initialize, OneSolInstruction, Swap},
    instructions::{spl_token, token_swap},
};
use num_traits::FromPrimitive;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    decode_error::DecodeError,
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_error::{PrintProgramError, ProgramError},
    pubkey::Pubkey,
};
use core::i64::MIN;
use std::{
    // Hide Result from bindgen gets confused about generics in non-generic type declarations
    result::Result as ResultGeneric,
    convert::TryInto,
};
use spl_math::{checked_ceil_div::CheckedCeilDiv};
use spl_token_swap::{
    error::SwapError,
};

/// Program state handler.
pub struct Processor {}

// #[cfg(debug_assertions)]
// const TOKEN_SWAP_PROGRAM_ADDRESS: &str = "BgGyXsZxLbug3f4q7W5d4EtsqkQjH1M9pJxUSGQzVGyf";
// #[cfg(not(debug_assertions))]
// const TOKEN_SWAP_PROGRAM_ADDRESS: &str = &"SwaPpA9LAaLfeLi3a68M4DjnLqgtticKg6CnyNwgAC8";

/// Supporting DEX
const DEXES_COUNT: usize = 2;
const FLAG_DISABLE_SWAP1: u64 = 0x01;
const FLAG_DISABLE_SWAP2: u64 = 0x02;

impl Processor {
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
            }) => {
                msg!("Instruction: Swap");
                Self::process_swap(program_id, amount_in, minimum_amount_out, accounts)
            }
        }
    }

    /// process
    pub fn process_initialize(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        Ok(())
    }

    /// Processes an [Swap](enum.Instruction.html).
    pub fn process_swap(
        program_id: &Pubkey,
        amount_in: u64,
        minimum_amount_out: u64,
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

        let _token_swap = token_swap::SwapVersion::unpack(&swap_info.data.borrow())?;
        // transfer AliceA -> OnesolA
        msg!("transfer AliceA -> onesolA");
        Self::token_transfer(
            swap_info.key,
            token_program_info.clone(),
            source_info.clone(),
            onesol_source_info.clone(),
            user_transfer_authority_info.clone(),
            _token_swap.nonce(),
            amount_in,
        )
        .unwrap();

        // Swap OnesolA -> OnesolB
        msg!("swap onesolA -> onesolB using token-swap");
        // let token_swap_program_id = Pubkey::from_str(TOKEN_SWAP_PROGRAM_ADDRESS).unwrap();
        // TODO do swap here
        let instruction = token_swap::Swap {
            amount_in: amount_in,
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

        // Transfer OnesolB -> AliceB
        // TODO 这里应该确定一下 amout_out
        msg!("transfer OneSolB -> AliceB");
        Self::token_transfer(
            swap_info.key,
            token_program_info.clone(),
            onesol_destination_info.clone(),
            destination_info.clone(),
            user_transfer_authority_info.clone(),
            // _token_swap.nonce(),
            _token_swap.nonce(),
            minimum_amount_out,
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
        let ix = spl_token::transfer(
            token_program.key,
            source.key,
            destination.key,
            authority.key,
            &[],
            amount,
        )?;
        invoke(&ix, &[source, destination, authority, token_program])
        // invoke_signed(
        //     &ix,
        //     &[source, destination, authority, token_program],
        //     signers,
        // )
    }

    /// https://github.com/1inch/1inchProtocol/blob/master/contracts/OneSplitBase.sol\#L139
    fn _findBestDistribution(
        s: u64,                 // parts
        amounts: Vec<Vec<i64>>  // exchangesReturns
    ) -> Vec<u64> {
        // println!("_findBestDistribution: input {} {:?}", s, amounts);

        let n = amounts.len();

        let mut answer: Vec<Vec<i64>> = vec![vec![MIN;(s+1) as usize];n];
        let mut parent: Vec<Vec<u64>> = vec![vec![0;(s+1) as usize];n];

        for j in (0..s+1) {
            answer[0][j as usize] = amounts[0][j as usize] as i64;
            // Aleardy initlize.
            // for i in (1..n) {
            //     answer[i as usize][j as usize] = MIN;
            // }
            // parent[0][j as usize] = 0;
        }
        // println!("_findBestDistribution: before {:?}", answer);

        for i in (1..n) {
            for j in (0..s+1) {
                answer[i as usize][j as usize] = answer[(i - 1) as usize][j as usize];
                parent[i as usize][j as usize] = j;

                // println!("_findBestDistribution: {} {} {:?}", i, j, answer);

                for k in (1..j+1) {
                    let a = answer[(i - 1) as usize][(j - k) as usize] + amounts[i as usize][k as usize] as i64;
                    if (a > answer[i as usize][j as usize]) {
                        answer[i as usize][j as usize] = a;
                        parent[i as usize][j as usize] = j - k;
                        // println!("_findBestDistribution: {} {} {} {:?} ❌ {:?}", i, j, k, parent, answer);
                    }
                }
            }
        }
        let mut distribution: Vec<u64> = vec![0;DEXES_COUNT];
        // println!("_findBestDistribution: {:?}", answer);
        // println!("_findBestDistribution: {:?}", parent);

        let mut partsLeft = s;
        let mut curExchange: i64 = n as i64 - 1;
        while partsLeft > 0 {
            distribution[curExchange as usize] = partsLeft - parent[curExchange as usize][partsLeft as usize];
            partsLeft = parent[curExchange as usize][partsLeft as usize];
            curExchange -= 1;
            /// Keep safe.
            if curExchange < 0 { break; }
        }

        // Useless.
        // let returnAmount = if (answer[(n - 1) as usize][s as usize] == MIN) { 0 } else { answer[(n - 1) as usize][s as usize] as u64 };

        return distribution;
    }

    /// Flags checking.
    fn _getAllReserves(flags: u64) -> Vec< fn(&[AccountInfo], u64, u64, u64)->(Vec<u64>, u64) > {
        return vec![
            if flags & FLAG_DISABLE_SWAP1 != 0 {Self::_calculateNoReturn} else {Self::_calculateSwap1},
            if flags & FLAG_DISABLE_SWAP2 != 0 {Self::_calculateNoReturn} else {Self::_calculateSwap2},
        ];
    }

    fn getExpectedReturnWithGas(
        accounts: &[AccountInfo],
        amount: u64,
        parts: u64, // Number of pieces source volume could be splitted
        flags: u64, // Flags for enabling and disabling some features
    ) -> Vec<u64> {
        let mut atLeastOnePositive = false;
        let reserves = Self::_getAllReserves(flags);
        // matrix[i] = new int256[](parts + 1);
        let mut matrix: Vec<Vec<i64>> = vec![vec![0;(parts + 1) as usize];DEXES_COUNT];
        let mut gases = vec![0;DEXES_COUNT];

        for i in (0..DEXES_COUNT) {
            let (rets, gas) = reserves[i as usize](accounts, amount, parts, flags);
            gases[i as usize] = gas;
            for j in (0..rets.len()) {
                matrix[i][j + 1] = (rets[j] as i64) - (gas as i64);
                atLeastOnePositive = atLeastOnePositive || (matrix[i][j + 1] > 0);
            }
        }

        if (!atLeastOnePositive) {
            for i in (0..DEXES_COUNT) {
                for j in (1..parts+1) {
                    if (matrix[i as usize][j as usize] == 0) {
                        matrix[i as usize][j as usize] = MIN;
                    }
                }
            }
        }

        let distribution = Self::_findBestDistribution(parts, matrix);
        println!("getExpectedReturnWithGas: {:?}", distribution);

        return distribution
    }

    /// Generate in 0..amount
    fn _linearInterpolation(
        value: u64,
        parts: u64,
    ) -> Vec<u64> {
        let mut rets = vec![0;parts as usize];
        for i in (0..parts) {
            rets[i as usize] = value * (i + 1) / parts;
        }
        return rets
    }

    // TODO: Fix name.
    fn _calculateSwap1(
        accounts: &[AccountInfo],
        amount: u64,
        parts: u64,
        flags: u64
    ) -> (Vec<u64>, u64) {
        let amounts = Self::_linearInterpolation(amount, parts);
        let mut rets = vec![0;amounts.len()];
        for i in (0..amounts.len()) {
            // TODO: Calculate amount out.
            rets[i] = amounts[i];
        }
        return (rets, 0);
    }
    fn _calculateSwap2(
        accounts: &[AccountInfo],
        amount: u64,
        parts: u64,
        flags: u64
    ) -> (Vec<u64>, u64) {
        let amounts = Self::_linearInterpolation(amount, parts);
        let mut rets = vec![0;amounts.len()];
        for i in (0..amounts.len()) {
            // TODO: Calculate amount out.
            let result = Self::_calculate_token_swap(accounts, amount);
            match result {
                Ok(val) => rets[i] = val,
                Err(e) => rets[i] = 0,
            }
        }
        return (rets, 0);
    }

    fn _calculateNoReturn(
        accounts: &[AccountInfo],
        amount: u64,
        parts: u64,
        flags: u64
    ) -> (Vec<u64>, u64) {
        return (vec![0;parts as usize], 0);
    }

    fn _calculate_token_swap (
        accounts: &[AccountInfo],
        amount_in: u64,
    ) -> ResultGeneric<u64, ProgramError> {
        let account_info_iter = &mut accounts.iter();
        let swap_info = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;
        let user_transfer_authority_info = next_account_info(account_info_iter)?;
        let source_info = next_account_info(account_info_iter)?;
        let swap_source_info = next_account_info(account_info_iter)?;
        let swap_destination_info = next_account_info(account_info_iter)?;
        let destination_info = next_account_info(account_info_iter)?;
        let pool_mint_info = next_account_info(account_info_iter)?;
        let pool_fee_account_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;

        let token_swap = spl_token_swap::state::SwapVersion::unpack(&swap_info.data.borrow())?;

        
        let source_account =
            spl_token_swap::processor::Processor::unpack_token_account(swap_source_info, &token_swap.token_program_id())?;
        let dest_account =
            spl_token_swap::processor::Processor::unpack_token_account(swap_destination_info, &token_swap.token_program_id())?;

        
        let result = spl_token_swap::curve::constant_product::swap(
            to_u128(amount_in)?,
            to_u128(source_account.amount)?,
            to_u128(dest_account.amount)?,
        );
        match result {
            None => return Ok(0),
            Some(val) => {
                let val_64 = to_u64(val.destination_amount_swapped);
                match val_64 {
                    Ok(a) => return Ok(a),
                    Err(e) => return Err(SwapError::ConversionFailure.into()),
                }
            }
        }

    }
}

fn to_u128(val: u64) -> Result<u128, SwapError> {
    val.try_into().map_err(|_| SwapError::ConversionFailure)
}
fn to_u64(val: u128) -> Result<u64, SwapError> {
    val.try_into().map_err(|_| SwapError::ConversionFailure)
}

impl PrintProgramError for OneSolError {
    fn print<E>(&self)
    where
        E: 'static + std::error::Error + DecodeError<E> + PrintProgramError + FromPrimitive,
    {
        match self {
            OneSolError::Unknown => msg!("Error: Unknown"),
            OneSolError::InvalidInstruction => msg!("Error: InvalidInstruction"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_program::{
        msg,
    };
    #[test]
    fn test_distribution() {
        // let result = Processor::getExpectedReturnWithGas(10, 100, 0);
        // assert_eq!(
        //     result,
        //     vec![90, 10]
        // );
    }
}
