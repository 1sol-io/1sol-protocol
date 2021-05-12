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
}

/// Instructions supported by the 1sol constracts program
#[repr(C)]
#[derive(Debug, PartialEq)]
pub enum OneSolInstruction {
    ///   Initializes
    ///
    ///   0. `[writable, signer]` New Token-swap to create.
    ///   1. `[]` swap authority derived from `create_program_address(&[Token-swap account])`
    ///   2. `[]` token_a Account. Must be non zero, owned by swap authority.
    ///   3. `[]` token_b Account. Must be non zero, owned by swap authority.
    ///   4. `[writable]` Pool Token Mint. Must be empty, owned by swap authority.
    ///   5. `[]` Pool Token Account to deposit trading and withdraw fees.
    ///   Must be empty, not owned by swap authority
    ///   6. `[writable]` Pool Token Account to deposit the initial pool token
    ///   supply.  Must be empty, not owned by swap authority.
    ///   7. '[]` Token program id
    Initialize(Initialize),

    /// Swap the tokens in the pool.
    ///
    /// 0. `[]` Token-swap
    /// 1. `[]` swap authority
    /// 2. `[]` user transfer authority
    ///   3. `[writable]` token_(A|B) SOURCE Account, amount is transferable by user transfer authority,
    ///   4. `[writable]` token_(A|B) Base Account to swap INTO.  Must be the SOURCE token.
    ///   5. `[writable]` token_(A|B) Base Account to swap FROM.  Must be the DESTINATION token.
    ///   6. `[writable]` token_(A|B) DESTINATION Account assigned to USER as the owner.
    ///   7. `[writable]` Pool token mint, to generate trading fees
    ///   8. `[writable]` Fee account, to receive trading fees
    ///   9. '[]` Token program id
    ///   10 `[optional, writable]` Host fee account to receive additional trading fees
    Swap(Swap),
}

impl OneSolInstruction {
    /// Unpacks a byte buffer into a [OneSolInstruction](enum.OneSolInstruction.html).
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (&tag, rest) = input.split_first().ok_or(OneSolError::InvalidInstruction)?;
        Ok(match tag {
            0 => {
                let (&nonce, rest) = rest.split_first().ok_or(OneSolError::InvalidInstruction)?;
                Self::Initialize(Initialize {})
            }
            1 => {
                let (amount_in, rest) = Self::unpack_u64(rest)?;
                let (minimum_amount_out, _rest) = Self::unpack_u64(rest)?;
                Self::Swap(Swap {
                    amount_in,
                    minimum_amount_out,
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
