//! Program state processor

use crate::{
  error::OneSolError,
  instruction::{Initialize, OneSolInstruction, SwapSerumDex, SwapTokenSwap},
  state::OneSolState,
  swappers::{serum_dex_order, spl_token_swap},
  util::unpack_token_account,
};
use num_traits::FromPrimitive;
// use safe_transmute::to_bytes::transmute_one_to_bytes;
use serum_dex::matching::Side as DexSide;
use solana_program::{
  account_info::{next_account_info, AccountInfo},
  decode_error::DecodeError,
  entrypoint::ProgramResult,
  log::sol_log_compute_units,
  msg,
  program::{invoke, invoke_signed},
  program_error::{PrintProgramError, ProgramError},
  program_pack::Pack,
  pubkey::Pubkey,
};
// use std::convert::identity;

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
      OneSolInstruction::SwapTokenSwap(data) => {
        msg!("Instruction: Swap TokenSwap");
        Self::process_swap_tokenswap(program_id, &data, accounts)
      }
      OneSolInstruction::SwapSerumDex(data) => {
        msg!("Instruction: Swap SerumDex");
        Self::process_swap_serumdex(program_id, &data, accounts)
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
  pub fn process_swap_tokenswap(
    program_id: &Pubkey,
    data: &SwapTokenSwap,
    accounts: &[AccountInfo],
  ) -> ProgramResult {
    msg!("start process swap: tokenswap");

    let account_info_iter = &mut accounts.iter();
    let protocol_account = next_account_info(account_info_iter)?;
    let protocol_authority = next_account_info(account_info_iter)?;
    let protocol_token_acc_info = next_account_info(account_info_iter)?;
    let source_token_acc_info = next_account_info(account_info_iter)?;
    let destination_token_acc_info = next_account_info(account_info_iter)?;
    let token_program_info = next_account_info(account_info_iter)?;

    let user_transfer_authority_info = next_account_info(account_info_iter)?;
    let swap_info = next_account_info(account_info_iter)?;
    let swap_authority_info = next_account_info(account_info_iter)?;
    let swap_source_token_acc_info = next_account_info(account_info_iter)?;
    let swap_destination_token_acc_info = next_account_info(account_info_iter)?;
    let pool_mint_info = next_account_info(account_info_iter)?;
    let pool_fee_account_info = next_account_info(account_info_iter)?;
    let token_swap_program_info = next_account_info(account_info_iter)?;
    let host_fee_account_info = next_account_info(account_info_iter);

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
    let protocol_token_account = unpack_token_account(protocol_token_acc_info, &token_program_id)?;
    msg!(
      "protocol_token_account amount: {}",
      protocol_token_account.amount
    );
    let destination_token_account =
      unpack_token_account(destination_token_acc_info, &token_program_id)?;
    msg!(
      "destination_token amount: {}",
      destination_token_account.amount
    );
    if protocol_token_account.mint != destination_token_account.mint {
      return Err(OneSolError::InvalidInput.into());
    }

    if protocol_token_account.owner == source_token_account.owner
      || protocol_token_account.owner == destination_token_account.owner
    {
      return Err(OneSolError::InvalidOwner.into());
    }
    let swap_source_token_account =
      unpack_token_account(swap_source_token_acc_info, &token_program_id)?;

    let (pool_source_token_acc_info, pool_destination_token_acc_info) =
      if source_token_account.mint == swap_source_token_account.mint {
        (swap_source_token_acc_info, swap_destination_token_acc_info)
      } else {
        (swap_destination_token_acc_info, swap_source_token_acc_info)
      };

    let token_swap_amount_in = data.amount_in;
    let token_swap_minimum_amount_out = data.minimum_amount_out;
    msg!(
      "swap source -> onesolDest using token-swap, amount_in: {}, minimum_amount_out: {}",
      token_swap_amount_in,
      token_swap_minimum_amount_out,
    );

    let mut swap_accounts = vec![
      swap_info.clone(),
      swap_authority_info.clone(),
      user_transfer_authority_info.clone(),
      source_token_acc_info.clone(),
      pool_source_token_acc_info.clone(),
      pool_destination_token_acc_info.clone(),
      protocol_token_acc_info.clone(),
      pool_mint_info.clone(),
      pool_fee_account_info.clone(),
    ];

    let host_fee_account_key = if host_fee_account_info.is_ok() {
      let account = host_fee_account_info?;
      swap_accounts.push(account.clone());
      Some(account.key)
    } else {
      None
    };

    let instruction = spl_token_swap::Swap {
      amount_in: token_swap_amount_in.get(),
      minimum_amount_out: token_swap_minimum_amount_out.get(),
    };
    let instruction = spl_token_swap::spl_token_swap_instruction(
      token_swap_program_info.key,
      token_program_info.key,
      swap_info.key,
      swap_authority_info.key,
      user_transfer_authority_info.key,
      source_token_acc_info.key,
      pool_source_token_acc_info.key,
      pool_destination_token_acc_info.key,
      protocol_token_acc_info.key,
      pool_mint_info.key,
      pool_fee_account_info.key,
      host_fee_account_key,
      instruction,
    )?;
    let temp_account = unpack_token_account(protocol_token_acc_info, &token_program_id)?;
    let amount1 = temp_account.amount;

    msg!(
      "invoke token-swap swap start, account amount: {}",
      temp_account.amount
    );
    invoke(&instruction, &swap_accounts[..])?;
    msg!(
      "invoke token-swap swap done, account amount: {}",
      temp_account.amount
    );

    let dest_account = spl_token::state::Account::unpack(&protocol_token_acc_info.data.borrow())?;
    let to_amount_include_fee = dest_account.amount - amount1;

    msg!(
      "onesol_destination amount: {}, result_with_fee: {}, expect: {}, minimum: {}",
      dest_account.amount,
      to_amount_include_fee,
      data.expect_amount_out,
      token_swap_minimum_amount_out,
    );
    if to_amount_include_fee < token_swap_minimum_amount_out.get() {
      return Err(OneSolError::ExceededSlippage.into());
    }
    let fee = to_amount_include_fee
      .checked_sub(data.expect_amount_out.get())
      .map(|v| v.checked_mul(25).unwrap().checked_div(100).unwrap_or(0))
      .unwrap_or(0);
    let to_amount = to_amount_include_fee.checked_sub(fee).unwrap();

    // Transfer OnesolB -> AliceB
    msg!("[token_swap] transfer to user destination, {}", to_amount);
    sol_log_compute_units();
    Self::token_transfer(
      protocol_account.key,
      token_program_info.clone(),
      protocol_token_acc_info.clone(),
      destination_token_acc_info.clone(),
      protocol_authority.clone(),
      protocol_info.nonce,
      to_amount,
    )?;
    // TODO close native_wrap account
    Ok(())
  }

  /// process_swap through serum-dex
  pub fn process_swap_serumdex(
    program_id: &Pubkey,
    data: &SwapSerumDex,
    accounts: &[AccountInfo],
  ) -> ProgramResult {
    msg!("start process swap: serumdex");

    let account_info_iter = &mut accounts.iter();
    let protocol_account = next_account_info(account_info_iter)?;
    let protocol_authority = next_account_info(account_info_iter)?;

    // this as pc_wallet
    let protocol_token_acc_info = next_account_info(account_info_iter)?;
    // this as coin_wallet
    let source_token_acc_info = next_account_info(account_info_iter)?;
    let destination_token_acc_info = next_account_info(account_info_iter)?;
    let token_program_info = next_account_info(account_info_iter)?;

    let token_program_id = *token_program_info.key;

    let source_token_account = unpack_token_account(source_token_acc_info, &token_program_id)?;
    msg!(
      "source_token_account amount: {}",
      source_token_account.amount
    );

    let market_account_info = next_account_info(account_info_iter)?;
    let request_queue_account_info = next_account_info(account_info_iter)?;
    let event_queue_account_info = next_account_info(account_info_iter)?;
    let bids_account_info = next_account_info(account_info_iter)?;
    let asks_account_info = next_account_info(account_info_iter)?;
    let coin_vault_account_info = next_account_info(account_info_iter)?;
    let pc_vault_account_info = next_account_info(account_info_iter)?;
    let vault_signer_account_info = next_account_info(account_info_iter)?;
    let open_orders_account_info = next_account_info(account_info_iter)?;
    let open_order_owner_account_info = next_account_info(account_info_iter)?;
    let rent_sysvar_account_info = next_account_info(account_info_iter)?;
    let dex_program_account_info = next_account_info(account_info_iter)?;

    if protocol_account.owner != program_id {
      return Err(ProgramError::IncorrectProgramId);
    }
    let protocol_info = OneSolState::unpack(&protocol_account.data.borrow())?;

    let (pc_wallet_account, coin_wallet_account) = match data.side {
      DexSide::Bid => (source_token_acc_info, protocol_token_acc_info),
      DexSide::Ask => (protocol_token_acc_info, source_token_acc_info),
    };

    let orderbook = serum_dex_order::OrderbookClient {
      market: serum_dex_order::MarketAccounts {
        market: market_account_info.clone(),
        open_orders: open_orders_account_info.clone(),
        request_queue: request_queue_account_info.clone(),
        event_queue: event_queue_account_info.clone(),
        bids: bids_account_info.clone(),
        asks: asks_account_info.clone(),
        order_payer_token_account: source_token_acc_info.clone(),
        coin_vault: coin_vault_account_info.clone(),
        pc_vault: pc_vault_account_info.clone(),
        vault_signer: vault_signer_account_info.clone(),
        coin_wallet: coin_wallet_account.clone(),
      },
      authority: open_order_owner_account_info.clone(),
      pc_wallet: pc_wallet_account.clone(),
      dex_program: dex_program_account_info.clone(),
      token_program: token_program_info.clone(),
      rent: rent_sysvar_account_info.clone(),
    };

    // FIXME maybe should run unpack token account
    let from_amount_before = unpack_token_account(source_token_acc_info, &token_program_id)?.amount;
    let to_amount_before = unpack_token_account(protocol_token_acc_info, &token_program_id)?.amount;
    msg!("from_amount_before: {}", from_amount_before);
    msg!("to_amount_before: {}", to_amount_before);
    msg!("amount_in: {}", data.amount_in);

    match data.side {
      DexSide::Bid => orderbook.buy(data.amount_in.get(), None)?,
      DexSide::Ask => orderbook.sell(data.amount_in.get(), None)?,
    }
    orderbook.settle(None)?;

    let from_amount_after = unpack_token_account(source_token_acc_info, &token_program_id)?.amount;
    let to_amount_after = unpack_token_account(protocol_token_acc_info, &token_program_id)?.amount;
    msg!("from_amount_after: {}", from_amount_after);
    msg!("to_amount_after: {}", to_amount_after);

    let from_amount = from_amount_before.checked_sub(from_amount_after).unwrap();
    let to_amount_include_fee = to_amount_after.checked_sub(to_amount_before).unwrap();
    msg!("from_amount: {}", from_amount);

    if to_amount_include_fee == 0 {
      return Err(OneSolError::DexSwapError.into());
    }

    // let min_expected_amount = u128::from(from_amount)
    //   .checked_mul(min_exchange_rate.rate.into())
    //   .unwrap()
    //   .checked_mul(
    //     10u128
    //       .checked_pow(min_exchange_rate.quote_decimals.into())
    //       .unwrap(),
    //   )
    //   .unwrap();
    // If there is spill (i.e. quote tokens *not* fully consumed for
    // the buy side of a transitive swap), then credit those tokens marked
    // at the executed exchange rate to create an "effective" to_amount.
    // let effective_to_amount = u128::from(to_amount)
    //   .checked_mul(
    //     10u128
    //       .checked_pow(data.from_decimals.into())
    //       .unwrap(),
    //   )
    //   .unwrap();
    msg!(
      "to_amount_include_fee: {:?}, min_expect_amount: {:?}, expect_amount: {:?}",
      to_amount_include_fee,
      data.minimum_amount_out,
      data.expect_amount_out,
    );
    if to_amount_include_fee < data.minimum_amount_out.get() {
      return Err(OneSolError::ExceededSlippage.into());
    }
    // TODO 计算手续费
    let fee = to_amount_include_fee
      .checked_sub(data.expect_amount_out.get())
      .map(|v| v.checked_mul(25).unwrap().checked_div(100).unwrap_or(0))
      .unwrap_or(0);
    let to_amount = to_amount_include_fee.checked_sub(fee).unwrap();

    msg!(
      "[serum_dex] transfer to user destination, amount: {}",
      to_amount
    );
    sol_log_compute_units();
    Self::token_transfer(
      protocol_account.key,
      token_program_info.clone(),
      protocol_token_acc_info.clone(),
      destination_token_acc_info.clone(),
      protocol_authority.clone(),
      protocol_info.nonce,
      to_amount,
    )?;

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
      OneSolError::InvalidProgramAddress => msg!("Error: InvalidProgramAddress"),
      OneSolError::ExpectedAccount => msg!("Error: ExpectedAccount"),
      OneSolError::IncorrectTokenProgramId => msg!("Error: IncorrectTokenProgramId"),
      OneSolError::ConversionFailure => msg!("Error: ConversionFailure"),
      OneSolError::ZeroTradingTokens => msg!("Error: ZeroTradingTokens"),
      OneSolError::InternalError => msg!("Error: InternalError"),
      OneSolError::DexInstructionError => msg!("Error: DexInstructionError"),
      OneSolError::DexInvokeError => msg!("Error: DexInvokeError"),
      OneSolError::DexSwapError => msg!("Error: DexSwapError"),
      OneSolError::InvalidExpectAmountOut => msg!("Error: InvalidExpectAmountOut"),
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
