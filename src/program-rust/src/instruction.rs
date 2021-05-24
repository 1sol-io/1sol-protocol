//! Instruction types

use crate::error::OneSolError;
use solana_program::program_error::ProgramError;
use std::convert::TryInto;

/// Initialize instruction data
#[derive(Clone, Debug, PartialEq)]
pub struct Initialize {}

/// Swap instruction data
#[derive(Clone, Debug, PartialEq)]
pub struct Swap {
    /// SOURCE amount to transfer, output to DESTINATION is based on the exchange rate
    pub amount_in: u64,
    /// Minimum amount of DESTINATION token to output, prevents excessive slippage
    pub minimum_amount_out: u64,
    /// nonce used to create validate program address
    pub nonce: u8,
}

/// Instructions supported by the 1sol constracts program
#[repr(C)]
#[derive(Debug, PartialEq)]
pub enum OneSolInstruction {
    ///   Initializes
    ///
    /// Do nothing
    Initialize(Initialize),

    /// Swap the tokens in the pool.
    ///
    ///   0. `[]` onesol-protocol account
    ///   1. `[]` token-swap account
    ///   2. `[]` onesol authority
    ///   3. `[]` token-swap authority
    ///   4. `[]` user transfer authority
    ///   5. `[writable]` token_A SOURCE Account, amount is transferable by user transfer authority,
    ///   6. `[writable]` token_A onesol SOURCE Account, amount is transferable by user transfer authority,
    ///   7. `[writable]` token_A Base Account to swap INTO.  Must be the SOURCE token.
    ///   8. `[writable]` token_B Base Account to swap FROM.  Must be the DESTINATION token.
    ///   9. `[writable]` token_B onesol Account to swap FROM.  Must be the DESTINATION token.
    ///   9. `[writable]` token_B DESTINATION Account to swap FROM.  Must be the DESTINATION token.
    ///   7. `[writable]` Pool token mint, to generate trading fees
    ///   8. `[writable]` Fee account, to receive trading fees
    ///   9. '[]` Token program id
    ///   9. '[]` Token-Swap program id
    ///   10 `[optional, writable]` Host fee account to receive additional trading fees
    Swap(Swap),
}

impl OneSolInstruction {
    /// Unpacks a byte buffer into a [OneSolInstruction](enum.OneSolInstruction.html).
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (&tag, rest) = input.split_first().ok_or(OneSolError::InvalidInstruction)?;
        Ok(match tag {
            0 => Self::Initialize(Initialize {}),
            1 => {
                let (amount_in, rest) = Self::unpack_u64(rest)?;
                let (minimum_amount_out, _rest) = Self::unpack_u64(rest)?;
                let (&nonce, _rest) = _rest.split_first().ok_or(OneSolError::InvalidInstruction)?;
                Self::Swap(Swap {
                    amount_in,
                    minimum_amount_out,
                    nonce,
                })
            }
            _ => return Err(OneSolError::InvalidInstruction.into()),
        })
    }

    fn unpack_u64(input: &[u8]) -> Result<(u64, &[u8]), ProgramError> {
        if input.len() >= 8 {
            let (amount, rest) = input.split_at(8);
            let amount = amount
                .get(..8)
                .and_then(|slice| slice.try_into().ok())
                .map(u64::from_le_bytes)
                .ok_or(OneSolError::InvalidInstruction)?;
            Ok((amount, rest))
        } else {
            Err(OneSolError::InvalidInstruction.into())
        }
    }
}
