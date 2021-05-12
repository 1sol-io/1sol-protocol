//! Program state processor
use crate::{
    error::OneSolError,
    instruction::{Initialize, OneSolInstruction, Swap},
};
use num_traits::FromPrimitive;
use solana_program::{
    account_info::AccountInfo, decode_error::DecodeError, entrypoint::ProgramResult, msg,
    program_error::PrintProgramError, pubkey::Pubkey,
};

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
                Self::processSwap(program_id, amount_in, minimum_amount_out, accounts)
            }
        }
    }

    /// Processes an [Swap](enum.Instruction.html).
    pub fn processSwap(
        program_id: &Pubkey,
        amount_in: u64,
        minimum_amount_out: u64,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        // TODO do swap here
        Ok(())
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
