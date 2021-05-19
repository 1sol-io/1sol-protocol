//! Program state processor

use crate::{
    error::OneSolError,
    instruction::{Initialize, OneSolInstruction, Swap},
    instructions::{
        token_swap,
        spl_token,
    }
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
use std::str::FromStr;

/// Program state handler.
pub struct Processor {
}

// #[cfg(debug_assertions)]
const TOKEN_SWAP_PROGRAM_ADDRESS: &str = "BgGyXsZxLbug3f4q7W5d4EtsqkQjH1M9pJxUSGQzVGyf";
// #[cfg(not(debug_assertions))]
// const TOKEN_SWAP_PROGRAM_ADDRESS: &str = &"SwaPpA9LAaLfeLi3a68M4DjnLqgtticKg6CnyNwgAC8";

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
    pub fn process_initialize(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
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
            amount_in
        ).unwrap();
        
        // Swap OnesolA -> OnesolB
        msg!("swap onesolA -> onesolB using token-swap");
        let token_swap_program_id = Pubkey::from_str(TOKEN_SWAP_PROGRAM_ADDRESS).unwrap();
        // TODO do swap here
        let instruction = token_swap::Swap {
            amount_in: amount_in,
            minimum_amount_out: minimum_amount_out,
        };
        // let swap_key = Pubkey::new_unique();
        // let (authority_key, _nonce) = Pubkey::find_program_address(&[&swap_key.to_bytes()], &token_swap_program_id);

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
        msg!("swap onesolA -> onesolB invoke token_swap {}, {}", swap.accounts.len(), swap_accounts.len());
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
        ).unwrap();



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
            invoke_signed(
                &ix,
                &[source, destination, authority, token_program],
                signers,
            )
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
