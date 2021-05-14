//! Program state processor

use crate::{
    error::OneSolError,
    instruction::{Initialize, OneSolInstruction, Swap},
    instructions::token_swap::{self},
};
use num_traits::FromPrimitive;
use solana_program::{
    account_info::{AccountInfo, next_account_info},
    decode_error::DecodeError, 
    entrypoint::ProgramResult, 
    msg, 
    program::invoke, 
    program_error::PrintProgramError, 
    pubkey::Pubkey};

/// Program state handler.
pub struct Processor {}
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
        let instruction = token_swap::Swap{
            amount_in: amount_in,
            minimum_amount_out: minimum_amount_out
        };
        let swap = token_swap::swap(
            program_id, 
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
            instruction)?;
        invoke(&swap, accounts)
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
