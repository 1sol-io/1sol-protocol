use crate::error::OneSolError;
use arrayref::array_refs;
use bytemuck::{cast_slice, from_bytes};
use serum_dex::{
    instruction::{
        new_order as serum_dex_new_order,
        settle_funds as serum_dex_settle_funds,
        // consume_events as serum_dex_consume_events,
        SelfTradeBehavior,
    },
    matching::{OrderType, Side},
    state::MarketState,
};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program::invoke,
    pubkey::Pubkey,
};
use std::num::NonZeroU64;

pub fn invoke_new_order<'a>(
    accounts: &[AccountInfo],
    program_id: &Pubkey,
    side: Side,
    limit_price: NonZeroU64,
    max_coin_qty: NonZeroU64,
    client_order_id: u64,
    self_trade_behavior: SelfTradeBehavior,
    limit: u16,
    max_native_pc_qty_including_fees: NonZeroU64,
) -> ProgramResult {
    let account_iters = &mut accounts.iter();
    let market_account = next_account_info(account_iters)?;
    let open_orders_account = next_account_info(account_iters)?;
    let request_queue = next_account_info(account_iters)?;
    let event_queue = next_account_info(account_iters)?;
    let market_bids = next_account_info(account_iters)?;
    let market_asks = next_account_info(account_iters)?;
    let order_payer = next_account_info(account_iters)?;
    let open_orders_account_owner = next_account_info(account_iters)?;
    let coin_vault = next_account_info(account_iters)?;
    let pc_vault = next_account_info(account_iters)?;
    let spl_token_program_id = next_account_info(account_iters)?;
    let rend_sysvar_id = next_account_info(account_iters)?;

    let account_infos = vec![
        market_account.clone(),
        open_orders_account.clone(),
        request_queue.clone(),
        event_queue.clone(),
        market_bids.clone(),
        market_asks.clone(),
        order_payer.clone(),
        open_orders_account_owner.clone(),
        coin_vault.clone(),
        pc_vault.clone(),
        spl_token_program_id.clone(),
        rend_sysvar_id.clone(),
    ];

    let tx = serum_dex_new_order(
        market_account.key,
        open_orders_account.key,
        request_queue.key,
        event_queue.key,
        market_bids.key,
        market_asks.key,
        order_payer.key,
        open_orders_account_owner.key,
        coin_vault.key,
        pc_vault.key,
        spl_token_program_id.key,
        rend_sysvar_id.key,
        None,
        program_id,
        side,
        limit_price,
        max_coin_qty,
        OrderType::ImmediateOrCancel,
        client_order_id,
        self_trade_behavior,
        limit,
        max_native_pc_qty_including_fees,
    )?;
    invoke(&tx, &account_infos[..])
}

pub fn invoke_settle_funds<'a>(
    market_account: AccountInfo<'a>,
    spl_token_program_account: AccountInfo<'a>,
    open_orders_account: AccountInfo<'a>,
    open_orders_account_owner: AccountInfo<'a>,
    coin_vault_account: AccountInfo<'a>,
    coin_wallet_account: AccountInfo<'a>,
    pc_vault_account: AccountInfo<'a>,
    pc_wallet_account: AccountInfo<'a>,
    vault_signer_account: AccountInfo<'a>,
    program_id: &Pubkey,
) -> ProgramResult {
    solana_program::msg!("[SerumDex] settle funds tx");
    let tx = serum_dex_settle_funds(
        program_id,
        market_account.key,
        spl_token_program_account.key,
        open_orders_account.key,
        open_orders_account_owner.key,
        coin_vault_account.key,
        coin_wallet_account.key,
        pc_vault_account.key,
        pc_wallet_account.key,
        Some(pc_wallet_account.key),
        vault_signer_account.key,
    )?;
    solana_program::msg!("[SerumDex] settle funds accounts");
    let account_infos = vec![
        market_account.clone(),
        open_orders_account.clone(),
        open_orders_account_owner.clone(),
        coin_vault_account.clone(),
        pc_vault_account.clone(),
        coin_wallet_account.clone(),
        pc_wallet_account.clone(),
        vault_signer_account.clone(),
        spl_token_program_account.clone(),
        pc_wallet_account.clone(),
    ];
    solana_program::msg!("[SerumDex] settle fund invoke");
    invoke(&tx, &account_infos[..])
}

pub const ACCOUNT_HEAD_PADDING: &[u8; 5] = b"serum";
pub const ACCOUNT_TAIL_PADDING: &[u8; 7] = b"padding";

pub fn load_market_state(market_acc: &AccountInfo) -> Result<MarketState, OneSolError> {
    let account_data = market_acc.data.borrow();
    if account_data.len() < 12 {
        return Err(OneSolError::InvalidInput);
    }
    let (head, data, tail) = array_refs![&account_data, 5; ..; 7];
    if head != ACCOUNT_HEAD_PADDING {
        return Err(OneSolError::InvalidInput);
    }
    if tail != ACCOUNT_TAIL_PADDING {
        return Err(OneSolError::InvalidInput.into());
    }
    let market_state: &MarketState = from_bytes(cast_slice(data));

    Ok(*market_state)
}

// fn check_account_padding(data: &[u8]) -> Result<&[[u8; 8]], OneSolError> {
//     if data.len() < 12 {
//         return Err(OneSolError::InvalidInput);
//     }
//     let (head, data, tail) = array_refs![data, 5; ..; 7];
//     if head != ACCOUNT_HEAD_PADDING {
//         return Err(OneSolError::InvalidInput);
//     }
//     if tail != ACCOUNT_TAIL_PADDING {
//         return Err(OneSolError::InvalidInput.into());
//     }
//     Ok(try_cast_slice(data).map_err(|_| OneSolError::InvalidInput)?)
// }

// fn invoke_consume_events() -> ProgramResult {
// 	serum_dex_consume_events(
// 		// program_id: &Pubkey,
// 		// open_orders_accounts: Vec<&Pubkey>,
// 		// market: &Pubkey,
// 		// event_queue: &Pubkey,
// 		// coin_fee_receivable_account: &Pubkey,
// 		// pc_fee_receivable_account: &Pubkey,
// 		// limit: u16,
// 	)
// 	Ok(())
// }
