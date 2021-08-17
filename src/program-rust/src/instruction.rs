//! Instruction types

use crate::error::OneSolError;
use solana_program::program_error::ProgramError;
use std::convert::TryInto;
use std::num::NonZeroU64;

use arrayref::{array_ref, array_refs};

/// Initialize instruction data
#[derive(Clone, Debug, PartialEq)]
pub struct Initialize {
    /// nonce used to create validate program address
    pub nonce: u8,
}

/// Swap instruction data
#[derive(Clone, Debug, PartialEq)]
pub struct Swap {
    /// Minimum amount of DESTINATION token to output, prevents excessive slippage
    pub minimum_amount_out: u64,
    /// spl_token_swap_data
    pub spl_token_swap_data: Option<SplTokenSwapData>,
    /// spl_token_swap_data
    pub serum_dex_order_data: Option<SerumDexOrderData>,
}

/// SplTokenSwapData
#[derive(Clone, Debug, PartialEq, Copy)]
pub struct SplTokenSwapData {
    /// account_size: the size of accountInfos
    pub account_size: usize,
    // /// ratio: the ratio of exchange
    // pub ratio: u8,
    /// SOURCE amount to transfer, output to DESTINATION is based on the exchange rate
    pub amount_in: u64,
    /// Minimum amount of DESTINATION token to output, prevents excessive slippage
    pub minimum_amount_out: u64,
}

/// SerumDex data
#[derive(Clone, Debug, PartialEq)]
pub struct SerumDexOrderData {
    /// account_size: the size of accountInfos
    pub account_size: usize,
    /// limit_price: serum dex order data
    pub limit_price: NonZeroU64,
    /// max_coin_qty: serum dex order data
    pub max_coin_qty: NonZeroU64,
    /// client_order_id: serum dex order data
    pub client_order_id: u64,
    /// self_trade_behavior: serum dex order data
    pub self_trade_behavior: serum_dex::instruction::SelfTradeBehavior,
    // /// limit: serum dex order data
    // pub limit: u16,
    /// max_native_pc_qty_including_fees: serum dex order data
    pub max_native_pc_qty_including_fees: NonZeroU64,
    /// side
    pub side: serum_dex::matching::Side,
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
    ///   2. `[]` wallet owner
    ///   3. `[writeable]` onesolProotcol token account
    ///   4. `[writable]` token_A SOURCE Account,
    ///   5. `[writable]` token_B DESTINATION Account to swap INTO. Must be the DESTINATION token.
    ///   6. '[]` Token program id
    ///
    ///   7. `[]` token-swap account
    ///   8. `[]` token-swap authority
    ///   9. `[writable]` token_A Base Account to swap FROM.  Must be the SOURCE token.
    ///   10. `[writable]` token_B Base Account to swap INTO.  Must be the DESTINATION token.
    ///   11. `[writable]` Pool token mint, to generate trading fees
    ///   12. `[writable]` Fee account, to receive trading fees
    ///   13. '[]` Token-Swap program id
    ///   14. `[optional, writable]` Host fee account to receive additional trading fees
    ///
    ///   15. `[writable]`  serum-dex market
    ///   16. `[writable]`  serum-dex open_orders
    ///   17. `[writable]`  serum-dex request_queue
    ///   18. `[writable]`  serum-dex event_queue
    ///   19. `[writable]`  serum-dex market_bids
    ///   20. `[writable]`  serum-dex market_asks
    ///   21. `[writable]`  serum-dex coin_vault
    ///   22. `[writable]`  serum-dex pc_vault
    ///   23. `[writable]`  serum-dex vault_signer for settleFunds
    ///   24. `[]`  serum-dex rent_sysvar
    ///   25. `[]`  serum-dex serum_dex_program account
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
                // let (amount_in, _rest) = Self::unpack_u64(rest)?;
                let (minimum_amount_out, _rest) = Self::unpack_u64(rest)?;
                let (spl_token_swap_data, _rest) = Self::unpack_spl_token_swap_data(_rest)?;
                let (serum_dex_order_data, _rest) = Self::unpack_serum_dex_order_data(_rest)?;
                Self::Swap(Swap {
                    // amount_in,
                    minimum_amount_out,
                    spl_token_swap_data,
                    serum_dex_order_data,
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
                .ok_or(OneSolError::InvalidInput)?;
            Ok((amount, rest))
        } else {
            Err(OneSolError::InvalidInput.into())
        }
    }

    fn unpack_u64_2(input: &[u8]) -> Result<u64, ProgramError> {
        if input.len() == 8 {
            let v = input
                .get(..8)
                .and_then(|slice| slice.try_into().ok())
                .map(u64::from_le_bytes)
                .ok_or(OneSolError::InvalidInput)?;
            Ok(v)
        } else {
            Err(OneSolError::InvalidInput.into())
        }
    }

    // size = 1 or 3
    // flag[0/1], [account_size], [amount_in], [minium_amount_out]
    fn unpack_spl_token_swap_data(
        input: &[u8],
    ) -> Result<(Option<SplTokenSwapData>, &[u8]), ProgramError> {
        let (&flag, rest) = input.split_first().ok_or(OneSolError::InvalidInput)?;
        if flag == 0 {
            return Ok((None, rest));
        }
        const DATA_LEN: usize = 17;
        if rest.len() < DATA_LEN {
            return Err(OneSolError::InvalidInput.into());
        }
        let (data, rest) = if rest.len() == DATA_LEN {
            (rest, &[] as &[u8])
        } else {
            rest.split_at(DATA_LEN)
        };
        let arr_data = array_ref![data, 0, DATA_LEN];
        let (&account_size_arr, &amount_in_arr, &minimum_amount_out_arr) =
            array_refs![arr_data, 1, 8, 8];
        let amount_in = Self::unpack_u64_2(&amount_in_arr)?;
        let minimum_amount_out = Self::unpack_u64_2(&minimum_amount_out_arr)?;
        Ok((
            Some(SplTokenSwapData {
                account_size: account_size_arr[0] as usize,
                amount_in: amount_in,
                minimum_amount_out: minimum_amount_out,
            }),
            rest,
        ))
    }

    fn unpack_serum_dex_order_data(
        input: &[u8],
    ) -> Result<(Option<SerumDexOrderData>, &[u8]), ProgramError> {
        let (&flag, rest) = input.split_first().ok_or(OneSolError::InvalidInput)?;
        if flag == 0 {
            return Ok((None, rest));
        }
        const DATA_LEN: usize = 34;
        if rest.len() < DATA_LEN {
            return Err(OneSolError::InvalidInput.into());
        }
        let (data, rest) = if rest.len() == DATA_LEN {
            (rest, &[] as &[u8])
        } else {
            rest.split_at(DATA_LEN)
        };
        let arr_data = array_ref![data, 0, DATA_LEN];
        let (
            &account_size_arr,
            &side_arr,
            &price_arr,
            &max_coin_qty_arr,
            &max_pc_qty_arr,
            &client_order_id_bytes,
        ) = array_refs![arr_data, 1, 1, 8, 8, 8, 8];

        let account_size = account_size_arr[0];
        // account size may be 12
        if account_size == 0 {
            return Err(OneSolError::InvalidInput.into());
        }
        let side = if side_arr[0] == 0 {
            serum_dex::matching::Side::Ask
        } else {
            serum_dex::matching::Side::Bid
        };
        solana_program::msg!("[serum_dex] side: {:?}", side);

        // let price_u64 = Self::unpack_u64_2(&price_arr)?;
        // solana_program::msg!("price_u64: {}", price_u64);
        let limit_price =
            NonZeroU64::new(Self::unpack_u64_2(&price_arr)?).ok_or(OneSolError::InvalidInput)?;

        let max_coin_qty = NonZeroU64::new(Self::unpack_u64_2(&max_coin_qty_arr)?)
            .ok_or(OneSolError::InvalidInput)?;

        let max_pc_qty = NonZeroU64::new(Self::unpack_u64_2(&max_pc_qty_arr)?)
            .ok_or(OneSolError::InvalidInput)?;

        let client_order_id = Self::unpack_u64_2(&client_order_id_bytes)?;
        let self_trade_behavior = serum_dex::instruction::SelfTradeBehavior::AbortTransaction;
        Ok((
            Some(SerumDexOrderData {
                account_size: account_size as usize,
                limit_price,
                max_coin_qty,
                client_order_id,
                self_trade_behavior,
                max_native_pc_qty_including_fees: max_pc_qty,
                side:side,
            }),
            rest,
        ))
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_unpack_spl_token_swap_data() {
        let r = OneSolInstruction::unpack_spl_token_swap_data(&[0]);
        assert_eq!(r.is_err(), false);
        assert_eq!(r.unwrap().0.is_none(), true);

        let r = OneSolInstruction::unpack_spl_token_swap_data(&[1, 1]);
        assert_eq!(r.is_err(), true);

        let r = OneSolInstruction::unpack_spl_token_swap_data(&[
            1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
        ]);
        assert_eq!(r.is_err(), false);
        let data_opt = r.unwrap().0;
        assert_eq!(data_opt.is_some(), true);
        let data: SplTokenSwapData = data_opt.unwrap();
        // assert_eq!(data.ratio , 1);
        assert_eq!(data.account_size, 1);
        assert_eq!(data.amount_in, 0);
    }

    #[test]
    fn test_unpack_serum_dex_order_data() {
        let r = OneSolInstruction::unpack_serum_dex_order_data(&[0]);
        assert_eq!(r.is_err(), false);
        assert_eq!(r.unwrap().0.is_none(), true);

        let r = OneSolInstruction::unpack_serum_dex_order_data(&[
            1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
        ]);
        assert_eq!(r.is_err(), true);

        let r = OneSolInstruction::unpack_serum_dex_order_data(&[
            1, 8, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0,
            0, 0, 0, 0, 1, 1, 2, 3,
        ]);
        assert_eq!(r.is_err(), false);

        let r = OneSolInstruction::unpack_serum_dex_order_data(&[
            1, 8, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0,
            0, 0, 0, 0, 1, 1, 2, 3,
        ]);
        assert_eq!(r.is_err(), false);
        let data_opt = r.unwrap().0;
        assert_eq!(data_opt.is_some(), true);
        let data: SerumDexOrderData = data_opt.unwrap();
        assert_eq!(data.account_size, 8);

        let r = OneSolInstruction::unpack_serum_dex_order_data(&[
            1, 8, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0,
            0, 0, 0, 0, 1, 1, 2, 3,
        ]);
        assert_eq!(r.is_err(), false);
        let data_opt = r.unwrap().0;
        assert_eq!(data_opt.is_some(), true);
        let data: SerumDexOrderData = data_opt.unwrap();
        assert_eq!(data.account_size, 8);
    }
}
