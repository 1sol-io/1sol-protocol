//! Program state processor

use crate::{
  account_parser::{
    validate_authority_pubkey, AmmInfoArgs, SerumDexArgs, SplTokenProgram, SplTokenSwapArgs,
    TokenAccount, UserArgs,TokenMint,
  },
  error::{ProtocolError},
  instruction::{
    ExchangeStep, ExchangerType, Initialize, OneSolInstruction, SwapInstruction,
    SwapTwoStepsInstruction,
  },
  state::{AccountFlag, AmmInfo, DexMarketInfo, OutputData},
  swappers::{serum_dex_order, spl_token_swap},
};
use arrayref::{array_refs};
// use safe_transmute::to_bytes::transmute_one_to_bytes;
use safe_transmute::to_bytes::transmute_to_bytes;
use serum_dex::matching::Side as DexSide;
use solana_program::{
  account_info::{next_account_info, AccountInfo},
  entrypoint::ProgramResult,
  log::sol_log_compute_units,
  msg,
  program::{invoke_signed},
  program_error::ProgramError,
  pubkey::Pubkey,
};
use std::{convert::{identity}, num::NonZeroU64};
// use std::convert::identity;

/// Program state handler.
pub struct Processor {}

impl Processor {
  /// Processes an [Instruction](enum.Instruction.html).
  pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]) -> ProgramResult {
    let instruction = OneSolInstruction::unpack(input)?;
    match instruction {
      OneSolInstruction::SwapSplTokenSwap(data) => {
        msg!("Instruction: Swap TokenSwap");
        Self::process_swap_spltokenswap(program_id, &data, accounts)
      }
      OneSolInstruction::SwapSerumDex(data) => {
        msg!("Instruction: Swap SerumDex");
        Self::process_swap_serumdex(program_id, &data, accounts)
      }
      OneSolInstruction::InitializeAmmInfo(data) => {
        msg!("Instruction: Initialize AmmInfo");
        Self::process_initialize_amm_info(program_id, &data, accounts)
      }
      OneSolInstruction::InitDexMarketOpenOrders(data) => {
        msg!("Instruction: Initialize Dex Market Open Orders");
        Self::process_initialize_dex_mark_open_orders(program_id, &data, accounts)
      }
      OneSolInstruction::SwapTwoSteps(data) => {
        msg!("Instruction: Swap Two Steps");
        Self::process_swap_two_steps(program_id, &data, accounts)
      }
    }
  }

  /// process initialize token pair
  pub fn process_initialize_amm_info(
    program_id: &Pubkey,
    data: &Initialize,
    accounts: &[AccountInfo],
  ) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let onesol_amm_info_acc = next_account_info(account_info_iter)?;
    let authority_info = next_account_info(account_info_iter)?;
    let owner_account_info = next_account_info(account_info_iter)?;
    let token_a_vault_info = next_account_info(account_info_iter)?;
    let token_a_mint_info = next_account_info(account_info_iter)?;
    let token_b_vault_info = next_account_info(account_info_iter)?;
    let token_b_mint_info = next_account_info(account_info_iter)?;
    let spl_token_program_info = next_account_info(account_info_iter)?;

    validate_authority_pubkey(
      authority_info.key,
      program_id,
      onesol_amm_info_acc.key,
      data.nonce,
    )?;

    let spl_token_program = SplTokenProgram::new(spl_token_program_info)?;

    let token_a_vault = TokenAccount::new(token_a_vault_info)?;
    let token_a_mint = TokenMint::new(token_a_mint_info)?;
    if token_a_vault.mint()? != *token_a_mint.inner().key {
      return Err(ProtocolError::InvalidTokenAccount.into());
    }
    token_a_vault.check_owner(authority_info.key)?;

    let token_b_vault = TokenAccount::new(token_b_vault_info)?;
    let token_b_mint = TokenMint::new(token_b_mint_info)?;
    if token_b_vault.mint()? != *token_b_mint.inner().key {
      return Err(ProtocolError::InvalidTokenAccount.into());
    }
    token_b_vault.check_owner(authority_info.key)?;

    if *token_a_mint.inner().key == *token_b_mint.inner().key {
      return Err(ProtocolError::InvalidTokenMint.into())
    }

    let mut amm_info = AmmInfo::load_mut(onesol_amm_info_acc, false)?;
    let amm_account_flags = amm_info.flags()?;
    if amm_account_flags.contains(AccountFlag::Initialized) {
      return Err(ProtocolError::InvalidAccountFlags.into());
    }
    amm_info.account_flags = (AccountFlag::Initialized | AccountFlag::AmmInfo).bits();
    amm_info.nonce = data.nonce;
    amm_info.owner = *owner_account_info.key;
    amm_info.token_program_id = *spl_token_program.inner().key;
    amm_info.token_a_vault = *token_a_vault.inner().key;
    amm_info.token_a_mint = *token_a_mint.inner().key;
    amm_info.token_b_vault = *token_b_vault.inner().key;
    amm_info.token_b_mint = *token_b_mint.inner().key;
    amm_info.output_data = OutputData::new();
    Ok(())
  }

  /// process initialize dex market open orders
  pub fn process_initialize_dex_mark_open_orders(
    program_id: &Pubkey,
    data: &Initialize,
    accounts: &[AccountInfo],
  ) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let onesol_market_acc_info = next_account_info(account_info_iter)?;
    let authority_info = next_account_info(account_info_iter)?;
    let amm_info_acc_info = next_account_info(account_info_iter)?;
    let dex_market_acc_info = next_account_info(account_info_iter)?;
    let dex_open_orders_info = next_account_info(account_info_iter)?;
    let rent_acc_info = next_account_info(account_info_iter)?;
    let dex_program_id_info = next_account_info(account_info_iter)?;

    let dex_program_id = *dex_program_id_info.key;

    validate_authority_pubkey(
      authority_info.key,
      program_id,
      amm_info_acc_info.key,
      data.nonce,
    )?;

    let amm_info = AmmInfo::load_mut(amm_info_acc_info, true)?;
    if amm_info.nonce != data.nonce {
      return Err(ProtocolError::InvalidNonce.into());
    }
    let market = serum_dex_order::load_market_state(dex_market_acc_info)?;

    let market_coin_mint = Pubkey::new(transmute_to_bytes(&identity(market.coin_mint)));
    let market_pc_mint = Pubkey::new(transmute_to_bytes(&identity(market.pc_mint)));

    if market_pc_mint != amm_info.token_a_mint && market_pc_mint != amm_info.token_b_mint {
      return Err(ProtocolError::InvalidTokenMint.into());
    }
    if market_coin_mint != amm_info.token_a_mint && market_coin_mint != amm_info.token_a_mint {
      return Err(ProtocolError::InvalidTokenMint.into());
    }
    if *dex_market_acc_info.owner != dex_program_id {
      return Err(ProtocolError::InvalidProgramAddress.into());
    }
    let mut dex_market_info = DexMarketInfo::load_mut(onesol_market_acc_info, false)?;
    let account_flags = dex_market_info.flags()?;
    if account_flags.contains(AccountFlag::Initialized) {
      return Err(ProtocolError::InvalidAccountFlags.into());
    }

    serum_dex_order::invoke_init_open_orders(
      amm_info_acc_info.key,
      &dex_program_id,
      dex_open_orders_info,
      authority_info,
      dex_market_acc_info,
      rent_acc_info,
      data.nonce,
    )?;

    dex_market_info.account_flags = (AccountFlag::Initialized | AccountFlag::DexMarketInfo).bits();
    dex_market_info.amm_info = *amm_info_acc_info.key;
    dex_market_info.dex_program_id = dex_program_id;
    dex_market_info.market = *dex_market_acc_info.key;
    dex_market_info.pc_mint = market_pc_mint;
    dex_market_info.coin_mint = market_coin_mint;
    dex_market_info.open_orders = *dex_open_orders_info.key;
    Ok(())
  }

  /// Processes an [Swap](enum.Instruction.html).
  pub fn process_swap_spltokenswap(
    program_id: &Pubkey,
    data: &SwapInstruction,
    accounts: &[AccountInfo],
  ) -> ProgramResult {
    const MIN_ACCOUNTS: usize = 15;
    if accounts.len() < MIN_ACCOUNTS {
      return Err(ProtocolError::InvalidAccountsLength.into());
    }
    let (fixed_accounts, other_accounts) = array_refs![accounts, 8; ..;];

    let (user_accounts, &[ref spl_token_program_acc], amm_info_accounts) =
      array_refs![fixed_accounts, 3, 1, 4];

    let user_args = UserArgs::with_parsed_args(user_accounts)?;

    let spl_token_program = SplTokenProgram::new(spl_token_program_acc)?;
    let amm_info_args = AmmInfoArgs::with_parsed_args(program_id, amm_info_accounts)?;

    // let spl_token_swap_args = SplTokenSwapArgs::with_parsed_args(other_accounts)?;

    msg!(
      "source_token_account amount: {}",
      user_args.token_source_account.balance()?,
    );

    let user_source_token_mint = user_args.token_source_account.mint()?;
    let user_destination_token_mint = user_args.token_destination_account.mint()?;

    let (amm_source_token_acc, amm_destination_token_acc) =
      amm_info_args.find_token_pair(&user_source_token_mint)?;

    if amm_source_token_acc.mint()? != user_source_token_mint {
      return Err(ProtocolError::InvalidTokenMint.into());
    }
    if amm_destination_token_acc.mint()? != user_destination_token_mint {
      return Err(ProtocolError::InvalidTokenMint.into());
    }

    // transfer from user token_source_account to amm_source_token_account
    Self::token_transfer(
      amm_info_args.amm_info_key,
      spl_token_program.inner(),
      user_args.token_source_account.inner(),
      amm_source_token_acc.inner(),
      user_args.source_account_owner.inner(),
      amm_info_args.nonce(),
      data.amount_in.get(),
    )?;

    let from_amount_before = amm_source_token_acc.balance()?;
    let to_amount_before = amm_destination_token_acc.balance()?;
    msg!(
      "from_amount_before: {}, to_amount_before: {}, amount_in: {}",
      from_amount_before,
      to_amount_before,
      data.amount_in
    );

    Self::process_step_tokenswap(
      program_id,
      // step,
      data,
      amm_source_token_acc,
      amm_destination_token_acc,
      &amm_info_args,
      &spl_token_program,
      other_accounts,
    )?;
    let from_amount_after = amm_source_token_acc.balance()?;
    let to_amount_after = amm_destination_token_acc.balance()?;
    msg!(
      "from_amount_after: {}, to_amount_after: {}",
      from_amount_after,
      to_amount_after
    );

    let from_amount_changed = from_amount_before.checked_sub(from_amount_after).unwrap();
    let to_amount_include_fee = to_amount_after.checked_sub(to_amount_before).unwrap();
    msg!("from_amount changed: {}", from_amount_changed);
    msg!(
      "result_with_fee: {}, expect: {}, minimum: {}",
      to_amount_include_fee,
      data.expect_amount_out,
      data.minimum_amount_out,
    );
    if to_amount_include_fee == 0 {
      return Err(ProtocolError::DexSwapError.into());
    }

    if to_amount_include_fee < data.minimum_amount_out.get() {
      return Err(ProtocolError::ExceededSlippage.into());
    }
    let fee = to_amount_include_fee
      .checked_sub(data.expect_amount_out.get())
      .map(|v| v.checked_mul(25).unwrap().checked_div(100).unwrap_or(0))
      .unwrap_or(0);
    let to_amount = to_amount_include_fee.checked_sub(fee).unwrap();

    // Transfer OnesolB -> AliceB
    msg!("transfer to user destination, amount: {}", to_amount);
    sol_log_compute_units();
    Self::token_transfer(
      amm_info_args.amm_info_key,
      spl_token_program.inner(),
      amm_destination_token_acc.inner(),
      user_args.token_destination_account.inner(),
      amm_info_args.authority_acc_info,
      amm_info_args.nonce(),
      to_amount,
    )?;
    // TODO close native_wrap account
    Ok(())
  }

  /// process_swap through serum-dex
  pub fn process_swap_serumdex(
    program_id: &Pubkey,
    data: &SwapInstruction,
    accounts: &[AccountInfo],
  ) -> ProgramResult {
    const MIN_ACCOUNTS: usize = 19;
    if accounts.len() < MIN_ACCOUNTS {
      return Err(ProtocolError::InvalidAccountsLength.into());
    }
    let (fixed_accounts, other_accounts) = array_refs![accounts, 8; ..;];

    let (user_accounts, &[ref spl_token_program_acc], amm_info_accounts) =
      array_refs![fixed_accounts, 3, 1, 4];

    let user_args = UserArgs::with_parsed_args(user_accounts)?;

    let spl_token_program = SplTokenProgram::new(spl_token_program_acc)?;
    let amm_info_args = AmmInfoArgs::with_parsed_args(program_id, amm_info_accounts)?;

    msg!(
      "source_token_account balance: {}",
      user_args.token_source_account.balance()?,
    );
    if user_args.token_source_account.balance()? < data.amount_in.get() {
      return Err(ProtocolError::InvalidSourceBalance.into());
    }

    let user_source_token_mint = user_args.token_source_account.mint()?;
    let user_destination_token_mint = user_args.token_destination_account.mint()?;

    let (amm_source_token_acc, amm_destination_token_acc) =
      amm_info_args.find_token_pair(&user_source_token_mint)?;

    let amm_source_token_mint = amm_source_token_acc.mint()?;
    let amm_destination_token_mint = amm_destination_token_acc.mint()?;

    if amm_source_token_mint != user_source_token_mint {
      return Err(ProtocolError::InvalidTokenMint.into());
    }
    if amm_destination_token_mint != user_destination_token_mint {
      return Err(ProtocolError::InvalidTokenMint.into());
    }
    // transfer from user token_source_account to amm_source_token_account
    Self::token_transfer(
      amm_info_args.amm_info_key,
      spl_token_program.inner(),
      user_args.token_source_account.inner(),
      amm_source_token_acc.inner(),
      user_args.source_account_owner.inner(),
      amm_info_args.nonce(),
      data.amount_in.get(),
    )?;

    let from_amount_before = amm_source_token_acc.balance()?;
    let to_amount_before = amm_destination_token_acc.balance()?;
    msg!(
      "from_amount_before: {}, to_amount_before: {}, amount_in: {}",
      from_amount_before,
      to_amount_before,
      data.amount_in
    );

    Self::process_step_serumdex(
      program_id,
      data,
      amm_source_token_acc,
      amm_destination_token_acc,
      &amm_info_args,
      &spl_token_program,
      other_accounts,
    )?;

    let from_amount_after = amm_source_token_acc.balance()?;
    let to_amount_after = amm_destination_token_acc.balance()?;
    msg!(
      "from_amount_after: {}, to_amount_after: {}",
      from_amount_after,
      to_amount_after
    );

    let from_amount_changed = from_amount_before.checked_sub(from_amount_after).unwrap();
    let to_amount_include_fee = to_amount_after.checked_sub(to_amount_before).unwrap();
    msg!("from_amount changed: {}", from_amount_changed);

    if to_amount_include_fee == 0 {
      return Err(ProtocolError::DexSwapError.into());
    }

    msg!(
      "to_amount_include_fee: {:?}, min_expect_amount: {:?}, expect_amount: {:?}",
      to_amount_include_fee,
      data.minimum_amount_out,
      data.expect_amount_out,
    );
    if to_amount_include_fee < data.minimum_amount_out.get() {
      return Err(ProtocolError::ExceededSlippage.into());
    }
    let fee = to_amount_include_fee
      .checked_sub(data.expect_amount_out.get())
      .map(|v| v.checked_mul(25).unwrap().checked_div(100).unwrap_or(0))
      .unwrap_or(0);
    let to_amount = to_amount_include_fee.checked_sub(fee).unwrap();

    msg!("transfer to user destination, amount: {}", to_amount);
    Self::token_transfer(
      amm_info_args.amm_info_key,
      spl_token_program.inner(),
      amm_destination_token_acc.inner(),
      user_args.token_destination_account.inner(),
      amm_info_args.authority_acc_info,
      amm_info_args.nonce(),
      to_amount,
    )?;

    Ok(())
  }

  pub fn process_swap_two_steps(
    program_id: &Pubkey,
    data: &SwapTwoStepsInstruction,
    accounts: &[AccountInfo],
  ) -> ProgramResult {
    msg!("start process swap: two_steps");
    let account_counts = 4 + 4 + data.step1.accounts_count + 4 + data.step2.accounts_count;
    if accounts.len() < account_counts {
      return Err(ProtocolError::InvalidAccountsLength.into());
    }
    let (fixed_accounts, other_accounts) = array_refs![accounts, 4;..; ];
    let (user_accounts, &[ref spl_token_program_acc]) = array_refs![fixed_accounts, 3, 1];
    let user_args = UserArgs::with_parsed_args(user_accounts)?;

    msg!(
      "source_token_account balance: {}",
      user_args.token_source_account.balance()?,
    );
    if user_args.token_source_account.balance()? < data.amount_in.get() {
      return Err(ProtocolError::InvalidSourceBalance.into());
    }

    let spl_token_program = SplTokenProgram::new(spl_token_program_acc)?;
    let (step1_accounts, step2_accounts) = other_accounts.split_at(4 + data.step1.accounts_count);
    // let mut steps = &[(data.step1, step1_accounts), (data.step2, step2_accounts)];
    let user_source_token_mint = user_args.token_source_account.mint()?;
    // step1

    let (step1_amm_info_args, step1_other_account) = {
      let (amm_info_accounts, step_other_accounts) = step1_accounts.split_at(4);
      let amm_info_args = AmmInfoArgs::with_parsed_args(program_id, amm_info_accounts)?;
      (amm_info_args, step_other_accounts)
    };

    let (step2_amm_info_args, step2_other_account) = {
      let (amm_info_accounts, step_other_accounts) = step2_accounts.split_at(4);
      let amm_info_args = AmmInfoArgs::with_parsed_args(program_id, amm_info_accounts)?;
      (amm_info_args, step_other_accounts)
    };

    let (step1_source_token_account, step1_temp_destination_token_account) =
      step1_amm_info_args.find_token_pair(&user_source_token_mint)?;
    let (step2_source_token_account, step2_destination_token_account) =
      step2_amm_info_args.find_token_pair(&step1_temp_destination_token_account.mint()?)?;

    Self::token_transfer(
      user_args.source_account_owner.pubkey(),
      spl_token_program.inner(),
      user_args.token_source_account.inner(),
      step1_source_token_account.inner(),
      user_args.source_account_owner.inner(),
      step1_amm_info_args.nonce(),
      data.amount_in.get(),
    )?;
    // transfer from user token_source_account to amm_source_token_account
    let from_amount_before = step1_source_token_account.balance()?;
    let to_amount_before = step2_destination_token_account.balance()?;
    let step1_to_amount_before = step2_source_token_account.balance()?;
    msg!(
      "from_amount_before: {}, to_amount_before: {}, amount_in: {}",
      from_amount_before,
      to_amount_before,
      data.amount_in
    );
    // step1
    Self::process_step_auto(
      program_id,
      &data.step1,
      SwapInstruction{
        amount_in: data.amount_in,
        expect_amount_out: NonZeroU64::new(1).unwrap(),
        minimum_amount_out: NonZeroU64::new(1).unwrap(),
      },
      step1_source_token_account,
      step2_source_token_account,
      &step1_amm_info_args,
      &spl_token_program,
      &step1_other_account
    )?;
    let step1_to_amount = step2_source_token_account.balance()?.checked_sub(step1_to_amount_before).ok_or(ProtocolError::Unreachable)?;
    msg!("step1 result: {}", step1_to_amount);

    // step2
    Self::process_step_auto(
      program_id,
      &data.step2,
      SwapInstruction{
        amount_in: NonZeroU64::new(step1_to_amount).unwrap(),
        expect_amount_out: data.expect_amount_out,
        minimum_amount_out: data.minimum_amount_out,
      },
      step2_source_token_account,
      step2_destination_token_account,
      &step2_amm_info_args,
      &spl_token_program, 
      &step2_other_account
    )?;

    let from_amount_after = step1_source_token_account.balance()?;
    let to_amount_after = step2_source_token_account.balance()?;
    msg!(
      "from_amount_after: {}, to_amount_after: {}",
      from_amount_after,
      to_amount_after
    );

    let from_amount_changed = from_amount_before.checked_sub(from_amount_after).unwrap();
    let to_amount_include_fee = to_amount_after.checked_sub(to_amount_before).unwrap();
    msg!("from_amount changed: {}", from_amount_changed);

    if to_amount_include_fee == 0 {
      return Err(ProtocolError::DexSwapError.into());
    }

    msg!(
      "to_amount_include_fee: {:?}, min_expect_amount: {:?}, expect_amount: {:?}",
      to_amount_include_fee,
      data.minimum_amount_out,
      data.expect_amount_out,
    );
    if to_amount_include_fee < data.minimum_amount_out.get() {
      return Err(ProtocolError::ExceededSlippage.into());
    }
    let fee = to_amount_include_fee
      .checked_sub(data.expect_amount_out.get())
      .map(|v| v.checked_mul(25).unwrap().checked_div(100).unwrap_or(0))
      .unwrap_or(0);
    let to_amount = to_amount_include_fee.checked_sub(fee).unwrap();

    msg!("transfer to user destination, amount: {}", to_amount);
    Self::token_transfer(
      step2_amm_info_args.amm_info_key,
      spl_token_program.inner(),
      step2_destination_token_account.inner(),
      user_args.token_destination_account.inner(),
      step2_amm_info_args.authority_acc_info,
      step2_amm_info_args.nonce(),
      to_amount,
    )?;

    Ok(())
  }

  fn process_step_auto<'a, 'b: 'a>(
    program_id: &Pubkey,
    step: &ExchangeStep,
    data: SwapInstruction,
    source_token_account: &TokenAccount<'a, 'b>,
    destination_token_account: &TokenAccount<'a, 'b>,
    amm_info_args: &AmmInfoArgs<'a, 'b>,
    spl_token_program: &SplTokenProgram<'a, 'b>,
    accounts: &'a [AccountInfo<'b>],
  ) -> ProgramResult {
    match step.exchanger_type {
      ExchangerType::SplTokenSwap => {
        Self::process_step_tokenswap(
          program_id,
          &data,
          source_token_account,
          destination_token_account,
          amm_info_args,
          spl_token_program,
          accounts,
        )?;
      }
      ExchangerType::SerumDex => {
        Self::process_step_serumdex(
          program_id,
          &data,
          source_token_account,
          destination_token_account,
          amm_info_args,
          spl_token_program,
          accounts,
        )?;
      }
    }
    Ok(())
  }

  /// Step swap in spl-token-swap
  #[allow(clippy::too_many_arguments, unused_variables)]
  fn process_step_tokenswap<'a, 'b: 'a>(
    program_id: &Pubkey,
    // step: &ExchangeStep,
    data: &SwapInstruction,
    source_token_account: &TokenAccount<'a, 'b>,
    destination_token_account: &TokenAccount<'a, 'b>,
    amm_info_args: &AmmInfoArgs<'a, 'b>,
    spl_token_program: &SplTokenProgram<'a, 'b>,
    accounts: &'a [AccountInfo<'b>],
  ) -> ProgramResult {
    let spl_token_swap_args = SplTokenSwapArgs::with_parsed_args(accounts)?;
    let token_swap_amount_in = data.amount_in;
    let token_swap_minimum_amount_out = data.minimum_amount_out;
    msg!(
      "swap using token-swap, amount_in: {}, minimum_amount_out: {}, expect_amount_out: {}",
      data.amount_in,
      data.minimum_amount_out,
      data.expect_amount_out,
    );

    let source_token_mint = source_token_account.mint()?;
    let destination_token_mint = destination_token_account.mint()?;

    if source_token_account.balance()? < data.amount_in.get() {
      return Err(ProtocolError::InvalidSourceBalance.into());
    }

    let (pool_source_token_acc, pool_destination_token_acc) =
      spl_token_swap_args.find_token_pair(&source_token_mint)?;

    if pool_source_token_acc.mint()? != source_token_mint {
      return Err(ProtocolError::InvalidTokenMint.into());
    }
    if pool_destination_token_acc.mint()? != destination_token_mint {
      return Err(ProtocolError::InvalidTokenMint.into());
    }

    let mut swap_accounts = vec![
      spl_token_swap_args.swap_info.inner().clone(),
      spl_token_swap_args.authority_acc_info.clone(),
      amm_info_args.authority_acc_info.clone(),
      source_token_account.inner().clone(),
      pool_source_token_acc.inner().clone(),
      pool_destination_token_acc.inner().clone(),
      destination_token_account.inner().clone(),
      spl_token_swap_args.pool_mint.inner().clone(),
      spl_token_swap_args.fee_account.inner().clone(),
    ];

    let host_fee_account_key = spl_token_swap_args.host_fee_account.map(|v| v.inner().key);

    if host_fee_account_key.is_some() {
      swap_accounts.push(
        spl_token_swap_args
          .host_fee_account
          .unwrap()
          .inner()
          .clone(),
      );
    }

    let instruction_data = spl_token_swap::Swap {
      amount_in: token_swap_amount_in.get(),
      minimum_amount_out: token_swap_minimum_amount_out.get(),
    };
    let instruction = spl_token_swap::spl_token_swap_instruction(
      spl_token_swap_args.program.key,
      spl_token_program.inner().key,
      spl_token_swap_args.swap_info.inner().key,
      spl_token_swap_args.authority_acc_info.key,
      amm_info_args.authority_acc_info.key,
      source_token_account.inner().key,
      pool_source_token_acc.inner().key,
      pool_destination_token_acc.inner().key,
      destination_token_account.inner().key,
      spl_token_swap_args.pool_mint.inner().key,
      spl_token_swap_args.fee_account.inner().key,
      host_fee_account_key,
      instruction_data,
    )?;
    let base_bytes = amm_info_args.amm_info_key.to_bytes();
    let authority_signature_seeds = [&base_bytes[..32], &[amm_info_args.nonce()]];
    let signers = &[&authority_signature_seeds[..]];
    msg!("invoke spl-token-swap swap");
    invoke_signed(&instruction, &swap_accounts[..], signers)?;
    Ok(())
  }

  #[allow(clippy::too_many_arguments, unused_variables)]
  fn process_step_serumdex<'a, 'b: 'a>(
    program_id: &Pubkey,
    // step: &ExchangeStep,
    data: &SwapInstruction,
    source_token_account: &TokenAccount<'a, 'b>,
    destination_token_account: &TokenAccount<'a, 'b>,
    amm_info_args: &AmmInfoArgs<'a, 'b>,
    spl_token_program: &SplTokenProgram<'a, 'b>,
    accounts: &'a [AccountInfo<'b>],
  ) -> ProgramResult {
    let dex_args = SerumDexArgs::with_parsed_args(accounts)?;

    let source_token_mint = source_token_account.mint()?;
    let destination_token_mint = destination_token_account.mint()?;

    let side = dex_args.find_side(&source_token_mint)?;

    let (pc_wallet_account, coin_wallet_account) = match side {
      DexSide::Bid => (source_token_account, destination_token_account),
      DexSide::Ask => (destination_token_account, source_token_account),
    };

    // TODO maybe check coin_wallet_account pc_wallet_account
    // TODO check open_order authority

    let orderbook = serum_dex_order::OrderbookClient {
      market: serum_dex_order::MarketAccounts {
        market: dex_args.market.inner(),
        open_orders: dex_args.open_order_acc,
        request_queue: dex_args.request_queue_acc,
        event_queue: dex_args.event_queue_acc,
        bids: dex_args.bids_acc,
        asks: dex_args.asks_acc,
        order_payer_token_account: source_token_account.inner(),
        coin_vault: dex_args.coin_vault_acc.inner(),
        pc_vault: dex_args.pc_vault_acc.inner(),
        vault_signer: dex_args.vault_signer_acc,
        coin_wallet: coin_wallet_account.inner(),
      },
      authority: amm_info_args.authority_acc_info,
      amm_info: amm_info_args.amm_info_key,
      pc_wallet: pc_wallet_account.inner(),
      dex_program: dex_args.program_acc,
      token_program: spl_token_program.inner(),
      rent: dex_args.rent_sysvar_acc,
      nonce: amm_info_args.nonce(),
    };
    match side {
      DexSide::Bid => orderbook.buy(data.amount_in.get(), None)?,
      DexSide::Ask => orderbook.sell(data.amount_in.get(), None)?,
    }
    orderbook.settle(None)?;
    Ok(())
  }

  /// check token account authority
  pub fn check_token_account_authority(
    token_account: &spl_token::state::Account,
    authority_info: &Pubkey,
  ) -> Result<(), ProtocolError> {
    if !token_account
      .delegate
      .map(|d| d == *authority_info)
      .unwrap_or(false)
      || token_account.owner == *authority_info
    {
      return Err(ProtocolError::InvalidDelegate);
    }
    Ok(())
  }

  /// Issue a spl_token `Transfer` instruction.
  pub fn token_transfer<'a>(
    base: &Pubkey,
    token_program: &AccountInfo<'a>,
    source: &AccountInfo<'a>,
    destination: &AccountInfo<'a>,
    authority: &AccountInfo<'a>,
    nonce: u8,
    amount: u64,
  ) -> Result<(), ProgramError> {
    let base_bytes = base.to_bytes();
    let authority_signature_seeds = [&base_bytes[..32], &[nonce]];
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
      &[
        source.clone(),
        destination.clone(),
        authority.clone(),
        token_program.clone(),
      ],
      signers,
    )
  }
}
