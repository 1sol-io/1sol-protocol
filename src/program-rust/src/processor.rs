//! Program state processor

use crate::{
    error::OneSolError,
    instruction::{Initialize, OneSolInstruction, SerumDexOrderData, SplTokenSwapData, Swap},
    state::OneSolState,
    swappers::{serum_dex_order, spl_token_swap},
    util::unpack_token_account,
};
use num_traits::FromPrimitive;
use safe_transmute::to_bytes::transmute_one_to_bytes;
use solana_program::{
    account_info::{next_account_info, next_account_infos, AccountInfo},
    decode_error::DecodeError,
    entrypoint::ProgramResult,
    log::sol_log_compute_units,
    msg,
    program::invoke_signed,
    program_error::{PrintProgramError, ProgramError},
    program_pack::Pack,
    pubkey::Pubkey,
};
use std::convert::identity;

/// Program state handler.
pub struct Processor {}

impl Processor {
    /// Processes an [Instruction](enum.Instruction.html).
    pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]) -> ProgramResult {
        let instruction = OneSolInstruction::unpack(input)?;
        match instruction {
            OneSolInstruction::Initialize(Initialize { nonce }) => {
                msg!("Instruction: Initialize");
                Self::process_initialize(program_id, nonce, accounts)
            }
            OneSolInstruction::Swap(Swap {
                minimum_amount_out,
                spl_token_swap_data,
                serum_dex_order_data,
            }) => {
                msg!("Instruction: Swap");
                Self::process_swap(
                    program_id,
                    minimum_amount_out,
                    spl_token_swap_data,
                    serum_dex_order_data,
                    accounts,
                )
            }
        }
    }

    /// Processes initialize
    pub fn process_initialize(
        program_id: &Pubkey,
        nonce: u8,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let onesol_info = next_account_info(account_info_iter)?;
        let authority_info = next_account_info(account_info_iter)?;
        let token_info = next_account_info(account_info_iter)?;
        let spl_token_program_info = next_account_info(account_info_iter)?;

        let token_program_id = *spl_token_program_info.key;

        if *authority_info.key != Self::authority_id(program_id, onesol_info.key, nonce)? {
            return Err(OneSolError::InvalidProgramAddress.into());
        }
        let token = unpack_token_account(token_info, &token_program_id)?;
        if token.delegate.is_some() {
            if token.delegate.unwrap() != *authority_info.key {
                return Err(OneSolError::InvalidDelegate.into());
            }
        } else if *authority_info.key != token.owner {
            return Err(OneSolError::InvalidOwner.into());
        }
        // if token.close_authority.is_some() {
        //     return Err(OneSolError::InvalidCloseAuthority.into());
        // }
        let obj = OneSolState {
            version: 1,
            nonce,
            token_program_id,
            token: *token_info.key,
            token_mint: token.mint,
        };
        OneSolState::pack(obj, &mut onesol_info.data.borrow_mut())?;
        Ok(())
    }

    /// Processes an [Swap](enum.Instruction.html).
    pub fn process_swap(
        program_id: &Pubkey,
        minimum_amount_out: u64,
        spl_token_swap_data: Option<SplTokenSwapData>,
        serum_dex_order_data: Option<SerumDexOrderData>,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        msg!("start process swap");
        if spl_token_swap_data.is_none() && serum_dex_order_data.is_none() {
            return Err(OneSolError::InvalidInput.into());
        }

        let account_info_iter = &mut accounts.iter();
        let protocol_account = next_account_info(account_info_iter)?;
        let protocol_authority = next_account_info(account_info_iter)?;
        // let user_transfer_authority_info = next_account_info(account_info_iter)?;
        let wallet_owner = next_account_info(account_info_iter)?;
        let protocol_token_acc_info = next_account_info(account_info_iter)?;
        let source_token_acc_info = next_account_info(account_info_iter)?;
        let destination_token_acc_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?;

        if protocol_account.owner != program_id {
            return Err(ProgramError::IncorrectProgramId);
        }
        let protocol_info = OneSolState::unpack(&protocol_account.data.borrow())?;

        if *protocol_authority.key
            != Self::authority_id(program_id, protocol_account.key, protocol_info.nonce)?
        {
            return Err(OneSolError::InvalidProgramAddress.into());
        }

        if *source_token_acc_info.key == protocol_info.token
            || *source_token_acc_info.key == protocol_info.token
        {
            return Err(OneSolError::IncorrectSwapAccount.into());
        }

        if *source_token_acc_info.key == *destination_token_acc_info.key {
            return Err(OneSolError::InvalidInput.into());
        }

        let token_program_id = *token_program_info.key;

        let source_token_account = unpack_token_account(source_token_acc_info, &token_program_id)?;
        msg!(
            "source_token_account amount: {}",
            source_token_account.amount
        );
        // this is middle destination token account
        let protocol_token_account =
            unpack_token_account(protocol_token_acc_info, &token_program_id)?;
        msg!(
            "protocol_token_account amount: {}",
            protocol_token_account.amount
            );
        let destination_token =
            unpack_token_account(destination_token_acc_info, &token_program_id)?;
        msg!(
            "destination_token amount: {}",
            destination_token.amount
        );
        if protocol_token_account.mint != destination_token.mint {
            return Err(OneSolError::InvalidInput.into());
        }

        let dest_account1 =
            spl_token::state::Account::unpack(&protocol_token_acc_info.data.borrow())?;
        let amount1 = dest_account1.amount;
        // msg!("account amount: {}", dest_account1.amount);

        if spl_token_swap_data.is_some() {
            let data = spl_token_swap_data.unwrap();
            let token_swap_amount_in = data.amount_in;
            let token_swap_minimum_amount_out = data.minimum_amount_out;
            if token_swap_amount_in > 0 {
                msg!(
                "swap onesolA -> onesolB using token-swap, amount_in: {}, minimum_amount_out: {}",
                token_swap_amount_in,
                token_swap_minimum_amount_out,
                );
                let mut accounts = vec![
                    token_program_info.clone(),
                    wallet_owner.clone(),
                    source_token_acc_info.clone(),
                    protocol_token_acc_info.clone(),
                ];
                let account_size = data.account_size;
                let dex_accounts = next_account_infos(account_info_iter, account_size)?;
                accounts.extend(dex_accounts.iter().cloned());
                spl_token_swap::process_token_swap_invoke_swap(
                    &accounts[..],
                    token_swap_amount_in,
                    token_swap_minimum_amount_out,
                )?;
                let temp_account =
                    spl_token::state::Account::unpack(&protocol_token_acc_info.data.borrow())?;
                msg!("token swap done, account amount: {}", temp_account.amount);
            }
        }

        if serum_dex_order_data.is_some() {
            let data = serum_dex_order_data.unwrap();
            let account_size = data.account_size;
            let source_token_account2 =
                unpack_token_account(source_token_acc_info, &token_program_id)?;
            msg!(
                "serum_dex trade, max_coin_qty: {}, max_pc_qty: {}, account_size: {}, source_token_account_amount: {}",
                data.max_coin_qty,
                data.max_native_pc_qty_including_fees,
                account_size,
                source_token_account2.amount,
            );
            if account_size < 11 {
                return Err(OneSolError::InvalidInput.into());
            }
            let dex_accounts = next_account_infos(account_info_iter, account_size)?;
            let dex_account_info_iter = &mut dex_accounts.iter();
            let market_acc_info = next_account_info(dex_account_info_iter)?;

            let open_orders_acc_info = next_account_info(dex_account_info_iter)?;
            // 这个 owner 跟上面可能会有重复
            // let open_orders_account_owner_acc_info = next_account_info(dex_account_info_iter)?;

            let request_queue_acc_info = next_account_info(dex_account_info_iter)?;
            let evnet_queue_acc_info = next_account_info(dex_account_info_iter)?;
            let market_bids_acc_info = next_account_info(dex_account_info_iter)?;
            let market_asks_acc_info = next_account_info(dex_account_info_iter)?;

            /*
            new_order:
                source -> coin_vault
            settle_funds:
                coin_vault  -> source
                pc_vault    -> pc_wallet
            */
            // it's spl_token::state::Account
            let coin_vault_acc_info = next_account_info(dex_account_info_iter)?;
            // it's spl_token::state::Account
            let pc_vault_acc_info = next_account_info(dex_account_info_iter)?;

            let vault_signer_acc_info = next_account_info(dex_account_info_iter)?;

            let rend_sysvar_acc_info = next_account_info(dex_account_info_iter)?;
            let serum_dex_program_info = next_account_info(dex_account_info_iter)?;

            let serum_dex_program_id = *serum_dex_program_info.key;

            let market_acc_clone = market_acc_info.clone();
            let market = serum_dex_order::load_market_state(&market_acc_clone)?;
            let coin_mint = Pubkey::new(transmute_one_to_bytes(&identity(market.coin_mint)));
            // // let pc_vault = Pubkey::new(transmute_one_to_bytes(&identity(market.pc_vault)));

            let side = if coin_mint == source_token_account.mint {
                serum_dex::matching::Side::Ask
            } else {
                serum_dex::matching::Side::Bid
            };

            msg!(
                "[SerumDex] side: {:?}, market: {}",
                side,
                market_acc_info.key
            );
            // msg!("[SerumDex] market, coin_vault: {:?}, pc_vault: {:?}", market.coin_vault, market.pc_vault);
            // market.check_coin_vault(vault: account_parser::TokenAccount)

            let new_order_accounts = vec![
                market_acc_info.clone(),
                open_orders_acc_info.clone(),
                request_queue_acc_info.clone(),
                evnet_queue_acc_info.clone(),
                market_bids_acc_info.clone(),
                market_asks_acc_info.clone(),
                source_token_acc_info.clone(),
                wallet_owner.clone(),
                coin_vault_acc_info.clone(),
                pc_vault_acc_info.clone(),
                token_program_info.clone(),
                rend_sysvar_acc_info.clone(),
            ];

            // let size =  / u64::from(data.limit_price);

            // msg!("[SerumDex] limit_price: {}, size: {}", data.limit_price, size);

            serum_dex_order::invoke_new_order(
                &new_order_accounts[..],
                &serum_dex_program_id,
                side,
                data.limit_price,
                data.max_coin_qty,
                data.client_order_id,
                data.self_trade_behavior,
                65535,
                data.max_native_pc_qty_including_fees,
            )?;

            msg!("[SerumDex] invoke settle funds");
            // TODO settle_funds
            serum_dex_order::invoke_settle_funds(
                market_acc_info.clone(),
                token_program_info.clone(),
                open_orders_acc_info.clone(),
                wallet_owner.clone(),
                coin_vault_acc_info.clone(),
                source_token_acc_info.clone(),
                pc_vault_acc_info.clone(),
                protocol_token_acc_info.clone(),
                vault_signer_acc_info.clone(),
                &serum_dex_program_id,
            )?;
            let temp_account =
                spl_token::state::Account::unpack(&protocol_token_acc_info.data.borrow())?;
            msg!(
                "serumdex order settle done, account amount: {}",
                temp_account.amount
            );
        }

        let dest_account =
            spl_token::state::Account::unpack(&protocol_token_acc_info.data.borrow())?;
        let result_amount = dest_account.amount - amount1;

        // TODO 计算手续费
        msg!(
            "onesol_destination amount: {}, should tranfer: {}, minimum: {}",
            dest_account.amount,
            result_amount,
            minimum_amount_out,
        );
        if result_amount < minimum_amount_out {
            return Err(OneSolError::ExceededSlippage.into());
        }
        // Transfer OnesolB -> AliceB
        msg!("transfer OneSolB -> AliceB");
        sol_log_compute_units();
        Self::token_transfer(
            protocol_account.key,
            token_program_info.clone(),
            protocol_token_acc_info.clone(),
            destination_token_acc_info.clone(),
            protocol_authority.clone(),
            protocol_info.nonce,
            result_amount,
        )
        .unwrap();

        Ok(())
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
}

impl PrintProgramError for OneSolError {
    fn print<E>(&self)
    where
        E: 'static + std::error::Error + DecodeError<E> + PrintProgramError + FromPrimitive,
    {
        match self {
            OneSolError::Unknown => msg!("Error: Unknown"),
            OneSolError::ExceededSlippage => msg!("Error: ExceededSlippage"),
            OneSolError::IncorrectSwapAccount => msg!("Error: IncorrectSwapAccount"),
            OneSolError::InvalidDelegate => msg!("Error: InvalidDelegate"),
            OneSolError::InvalidCloseAuthority => msg!("Error: InvalidCloseAuthority"),
            OneSolError::InvalidInstruction => msg!("Error: InvalidInstruction"),
            OneSolError::InvalidInput => msg!("Error: InvalidInput"),
            OneSolError::InvalidOwner => msg!("Error: InvalidOwner"),
            OneSolError::InvalidProgramAddress => msg!("Error: InvildProgramAddress"),
            OneSolError::ExpectedAccount => msg!("Error: ExpectedAccount"),
            OneSolError::IncorrectTokenProgramId => msg!("Error: IncorrectTokenProgramId"),
            OneSolError::ConversionFailure => msg!("Error: ConversionFailure"),
            OneSolError::ZeroTradingTokens => msg!("Error: ZeroTradingTokens"),
            OneSolError::InternalError => msg!("Error: InternalError"),
        }
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     // #[test]
//     // fn test_find_best_parts() {
//     //     let r = find_best_parts(10, 2);
//     //     assert_eq!(r, 8);
//     //     let r = find_best_parts(10, 8);
//     //     assert_eq!(r, 2);
//     //     let r = find_best_parts(10, 9);
//     //     assert_eq!(r, 2);
//     //     let r = find_best_parts(10, 1);
//     //     assert_eq!(r, 16);
//     // }
// }
