//! Instruction types

use crate::error::OneSolError;
use solana_program::program_error::ProgramError;
use std::convert::TryInto;

/// Initialize instruction data
#[derive(Clone, Debug, PartialEq)]
pub struct Initialize {
    /// nonce used to create validate program address
    pub nonce: u8,
}

/// Swap instruction data
#[derive(Clone, Debug, PartialEq)]
pub struct Swap {
    /// SOURCE amount to transfer, output to DESTINATION is based on the exchange rate
    pub amount_in: u64,
    /// Minimum amount of DESTINATION token to output, prevents excessive slippage
    pub minimum_amount_out: u64,
    /// dexes configs
    pub dex_configs: [(bool, usize); 2],
    // /// supportTokenSwap
    // pub token_swap_config: (bool, usize),
    // /// second token swap config
    // pub token_swap_2_config: (bool, usize),
}

/// Instructions supported by the 1sol constracts program
#[repr(C)]
#[derive(Debug, PartialEq)]
pub enum OneSolInstruction {
    /// Initializes a new 1solProtocol
    /// 0. `[writable, signer]` New 1solProtocol to create.
    /// 1. `[]` swap authority derived from `create_program_address(&[Token-swap account])`
    /// 2. `[]` token Account. Must be non zero, owned by 1sol.
    /// 3. '[]` Token program id
    Initialize(Initialize),

    /// Swap the tokens in the pool.
    ///
    ///   0. `[]` onesolProotcol account
    ///   1. `[]` onesolProotcol authority
    ///   2. `[]` user transfer authority
    ///   3. `[writeable]` onesolProotcol token account
    ///   4. `[writable]` token_A SOURCE Account, amount is transferable by user transfer authority,
    ///   5. `[writable]` token_B DESTINATION Account to swap FROM.  Must be the DESTINATION token.
    ///   6. '[]` Token program id
    ///
    ///   7. `[]` token-swap account
    ///   8. `[]` token-swap authority
    ///   9. `[writable]` token_A Base Account to swap INTO.  Must be the SOURCE token.
    ///   10. `[writable]` token_B Base Account to swap FROM.  Must be the DESTINATION token.
    ///   11. `[writable]` Pool token mint, to generate trading fees
    ///   12. `[writable]` Fee account, to receive trading fees
    ///   13. '[]` Token-Swap program id
    ///   14 `[optional, writable]` Host fee account to receive additional trading fees
    Swap(Swap),
}

impl OneSolInstruction {
    /// Unpacks a byte buffer into a [OneSolInstruction](enum.OneSolInstruction.html).
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (&tag, rest) = input.split_first().ok_or(OneSolError::InvalidInput)?;
        Ok(match tag {
            0 => {
                let (&nonce, _rest) = rest.split_first().ok_or(OneSolError::InvalidInput)?;
                Self::Initialize(Initialize { nonce })
            }
            1 => {
                let (amount_in, _rest) = Self::unpack_u64(rest)?;
                let (minimum_amount_out, _rest) = Self::unpack_u64(_rest)?;
                let (_dexes_configs, _rest) = Self::unpack_dexes_configs(_rest)?;

                if _dexes_configs.len() != 2 {
                    return Err(OneSolError::InvalidInstruction.into());
                }
                // let token_swap_config = dexes_configs[0];
                // let token_swap_2_config = dexes_configs[1];
                // let token_swap_config = (true, true);
                let dex_configs = [_dexes_configs[0], _dexes_configs[1]];

                // let (&support_token_swap, _rest) = _rest.split_first().ok_or(OneSolError::InvalidInput)?;
                Self::Swap(Swap {
                    amount_in,
                    minimum_amount_out,
                    dex_configs,
                    // token_swap_config,
                    // token_swap_2_config,
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

    fn unpack_dexes_configs(input: &[u8]) -> Result<(Vec<(bool, usize)>, &[u8]), ProgramError> {
        let (&dexes_config_size, _rest) = input.split_first().ok_or(OneSolError::InvalidInput)?;
        if dexes_config_size < 1 {
            return Err(OneSolError::InvalidInput.into());
        }
        let dexes_config_real_size = (dexes_config_size * 2) as usize;
        if _rest.len() < dexes_config_real_size {
            return Err(OneSolError::InvalidInput.into());
        }
        let (dexes_configs, _rest) = _rest.split_at(dexes_config_real_size);
        let mut dexes_iter = dexes_configs.chunks(2);
        let mut result = vec![];
        loop {
            let next = dexes_iter.next();
            if next.is_none() {
                break;
            }
            let r = next.unwrap();
            result.push((r[0] != 0, r[1] as usize));
        }
        Ok((result, _rest))
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_unpack_dexes_configs() {
        let r = OneSolInstruction::unpack_dexes_configs(&[0]);
        assert_eq!(r.is_err(), true);
        let r = OneSolInstruction::unpack_dexes_configs(&[1]);
        assert_eq!(r.is_err(), true);
        let r = OneSolInstruction::unpack_dexes_configs(&[1, 0]);
        assert_eq!(r.is_err(), true);
        let r = OneSolInstruction::unpack_dexes_configs(&[1, 1, 1]);
        assert_eq!(r.is_ok(), true);
        let (v, rest) = r.unwrap();
        assert_eq!(v, vec![(true, 1)]);
        assert_eq!(rest.len(), 0);
        let r = OneSolInstruction::unpack_dexes_configs(&[1, 1, 1, 2]);
        assert_eq!(r.is_ok(), true);
        let (v, rest) = r.unwrap();
        assert_eq!(v, vec![(true, 1)]);
        assert_eq!(rest.len(), 1);
        assert_eq!(rest, &[2]);
    }
}
