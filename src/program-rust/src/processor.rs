//! Program state processor

use crate::{
    error::OneSolError,
    instruction::{Initialize, OneSolInstruction, Swap},
    instructions::token_swap::{self},
};
use num_traits::FromPrimitive;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    decode_error::DecodeError,
    entrypoint::ProgramResult,
    msg,
    program::invoke,
    program_error::PrintProgramError,
    pubkey::Pubkey,
};
use std::str::FromStr;
use core::i64::MIN;

/// Program state handler.
pub struct Processor {}

#[cfg(debug_assertions)]
const TOKEN_SWAP_ADDRESS: &str = &"GSKD4BfZBFzCtGzZ7qEgPgr4UgkxiCK3bgTV9PQFRMab";
#[cfg(not(debug_assertions))]
const TOKEN_SWAP_ADDRESS: &str = &"SwaPpA9LAaLfeLi3a68M4DjnLqgtticKg6CnyNwgAC8";

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

    /// Processes an [Swap](enum.Instruction.html).
    pub fn process_swap(
        program_id: &Pubkey,
        amount_in: u64,
        minimum_amount_out: u64,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
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
        let mut host_fee_pubkey: Option<&Pubkey> = None;
        if let Ok(host_fee_account_info) = next_account_info(account_info_iter) {
            host_fee_pubkey = Some(host_fee_account_info.key);
        }
        // TODO do swap here
        let instruction = token_swap::Swap {
            amount_in: amount_in,
            minimum_amount_out: minimum_amount_out,
        };
        let token_swap_program_id = &Pubkey::from_str(TOKEN_SWAP_ADDRESS).unwrap();
        let swap = token_swap::swap(
            token_swap_program_id,
            token_program_info.key,
            swap_info.key,
            authority_info.key,
            user_transfer_authority_info.key,
            source_info.key,
            swap_source_info.key,
            swap_destination_info.key,
            destination_info.key,
            pool_mint_info.key,
            pool_fee_account_info.key,
            host_fee_pubkey,
            instruction,
        )?;
        invoke(&swap, accounts)
    }

    fn _findBestDistribution(
        s: u64,                 // parts
        amounts: Vec<Vec<u64>>  // exchangesReturns
    ) -> (i64, Vec<u64>) {
        let n = amounts.len();

        let mut answer:Vec<Vec<i64>> = vec![vec![MIN;(s+1) as usize];n];
        let mut parent = vec![vec![0;(s+1) as usize];n];

        for j in (0..s+1) {
            answer[0][j as usize] = amounts[0][j as usize] as i64;
            // for i in (1..n) {
            //     answer[i as usize][j as usize] = MIN;
            // }
            // parent[0][j as usize] = 0;
        }

        for i in (1..n) {
            for j in (0..s+1) {
                answer[i as usize][j as usize] = answer[(i - 1) as usize][j as usize];
                parent[i as usize][j as usize] = j;

                for k in (1..j+1) {
                    if (answer[(i - 1) as usize][(j - k) as usize] + amounts[i as usize][k as usize] as i64 > answer[i as usize][j as usize]) {
                        answer[i as usize][j as usize] = answer[(i - 1) as usize][(j - k) as usize] + amounts[i as usize][k as usize] as i64;
                        parent[i as usize][j as usize] = j - k;
                    }
                }
            }
        }
        const DEXES_COUNT: usize = 2;
        let mut distribution: Vec<u64> = vec![0;DEXES_COUNT];

        let mut partsLeft = s;
        let mut curExchange: usize = n - 1;
        while partsLeft > 0 {
            distribution[curExchange] = partsLeft - parent[curExchange][partsLeft as usize];
            partsLeft = parent[curExchange][partsLeft as usize];
            partsLeft -= 1;
        }

        let returnAmount = if (answer[(n - 1) as usize][s as usize] == MIN) { 0 } else { answer[(n - 1) as usize][s as usize] };

        return (returnAmount, distribution);
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
        }
    }
}
