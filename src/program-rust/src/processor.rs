//! Program state processor

#[cfg(feature = "production")]
use crate::constraints::OWNER_KEY;
use crate::{
  account_parser::{
    validate_authority_pubkey, AmmInfoArgs, RaydiumSwapArgs, SerumDexArgs, SerumDexMarket,
    SplTokenProgram, SplTokenSwapArgs, StableSwapArgs, TokenAccount, TokenAccountAndMint,
    TokenMint, UserArgs,
  },
  error::ProtocolError,
  instruction::{
    ExchangeStep, ExchangerType, Initialize, OneSolInstruction, SwapInstruction,
    SwapTwoStepsInstruction,
  },
  state::{AccountFlag, AmmInfo, DexMarketInfo, OutputData},
  swappers::{raydium_swap, serum_dex_order, spl_token_swap},
};
use arrayref::{array_ref, array_refs};
// use safe_transmute::to_bytes::transmute_one_to_bytes;
use serum_dex::matching::Side as DexSide;
use solana_program::{
  account_info::{next_account_info, AccountInfo},
  entrypoint::ProgramResult,
  log::sol_log_compute_units,
  msg,
  program::{invoke, invoke_signed},
  program_error::ProgramError,
  program_pack::Pack,
  pubkey::Pubkey,
  rent::Rent,
  sysvar::{self, Sysvar},
};
use std::num::NonZeroU64;
// use std::convert::identity;
/// Program state handler.
pub struct Processor {}

impl Processor {
  /// Processes an [Instruction](enum.Instruction.html).
  pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]) -> ProgramResult {
    let instruction = OneSolInstruction::unpack(input)?;
    match instruction {
      OneSolInstruction::InitializeAmmInfo(data) => {
        msg!("Instruction: Initialize AmmInfo");
        Self::process_initialize_amm_info(program_id, &data, accounts)
      }
      OneSolInstruction::InitDexMarketOpenOrders(data) => {
        msg!("Instruction: Initialize Dex Market Open Orders");
        Self::process_initialize_dex_mark_open_orders(program_id, &data, accounts)
      }
      OneSolInstruction::UpdateDexMarketOpenOrders => {
        Self::process_update_dex_mark_open_orders(program_id, accounts)
      }
      OneSolInstruction::SwapFees => Self::process_swap_fees(program_id, accounts),
      OneSolInstruction::SwapSplTokenSwap(data) => {
        msg!("Instruction: Swap TokenSwap");
        Self::process_single_step_swap(program_id, &data, accounts, ExchangerType::SplTokenSwap)
      }
      OneSolInstruction::SwapSerumDex(data) => {
        msg!("Instruction: Swap SerumDex");
        Self::process_single_step_swap(program_id, &data, accounts, ExchangerType::SerumDex)
      }
      OneSolInstruction::SwapStableSwap(data) => {
        msg!("Instruction: Swap StableSwap");
        Self::process_single_step_swap(program_id, &data, accounts, ExchangerType::StableSwap)
      }
      OneSolInstruction::SwapRaydiumSwap(data) => {
        msg!("Instruction: Swap RaydiumSwap");
        Self::process_single_step_swap(program_id, &data, accounts, ExchangerType::RaydiumSwap)
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

    // check onesol_amm_info_acc
    if *onesol_amm_info_acc.owner != *program_id {
      return Err(ProtocolError::InvalidAmmInfoAccount.into());
    }
    let rent = Rent::get()?;
    if !rent.is_exempt(
      onesol_amm_info_acc.lamports(),
      onesol_amm_info_acc.data_len(),
    ) {
      return Err(ProtocolError::NotRentExempt.into());
    }
    validate_authority_pubkey(
      authority_info.key,
      program_id,
      onesol_amm_info_acc.key,
      data.nonce,
    )?;

    #[cfg(feature = "production")]
    if *owner_account_info.key.to_string() != OWNER_KEY.to_string() {
      return Err(ProtocolError::InvalidOwnerKey.into());
    }

    let token_a = TokenAccountAndMint::new(
      TokenAccount::new(token_a_vault_info)?,
      TokenMint::new(token_a_mint_info)?,
    )?;
    token_a.account.check_owner(authority_info.key, true)?;

    #[cfg(feature = "production")]
    match token_a.account.delegate()? {
      Some(delegate) => {
        if delegate.to_string() != OWNER_KEY.to_string() {
          return Err(ProtocolError::InvalidTokenAccountDelegate.into());
        }
      }
      None => {}
    }

    let token_b = TokenAccountAndMint::new(
      TokenAccount::new(token_b_vault_info)?,
      TokenMint::new(token_b_mint_info)?,
    )?;
    token_b.account.check_owner(authority_info.key, true)?;

    #[cfg(feature = "production")]
    match token_b.account.delegate()? {
      Some(delegate) => {
        if delegate.to_string() != OWNER_KEY.to_string() {
          return Err(ProtocolError::InvalidTokenAccountDelegate.into());
        }
      }
      None => {}
    }

    let spl_token_program = SplTokenProgram::new(spl_token_program_info)?;

    let is_initialized = match AmmInfo::unpack(&onesol_amm_info_acc.data.borrow()) {
      Ok(amm_info) => amm_info
        .flags()
        .map(|x| x.contains(AccountFlag::Initialized))
        .unwrap_or(false),
      Err(_) => false,
    };
    if is_initialized {
      return Err(ProtocolError::InvalidAccountFlags.into());
    }
    let amm_info = AmmInfo {
      account_flags: (AccountFlag::Initialized | AccountFlag::AmmInfo).bits(),
      nonce: data.nonce,
      owner: *owner_account_info.key,
      token_program_id: *spl_token_program.inner().key,
      token_a_vault: *token_a.account.inner().key,
      token_a_mint: *token_a.mint.inner().key,
      token_b_vault: *token_b.account.inner().key,
      token_b_mint: *token_b.mint.inner().key,
      output_data: OutputData::new(),
    };
    AmmInfo::pack(amm_info, &mut onesol_amm_info_acc.data.borrow_mut())?;
    Ok(())
  }

  /// process initialize dex market open orders
  pub fn process_initialize_dex_mark_open_orders(
    program_id: &Pubkey,
    data: &Initialize,
    accounts: &[AccountInfo],
  ) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let amm_info_acc_info = next_account_info(account_info_iter)?;
    let authority_info = next_account_info(account_info_iter)?;
    let protocol_market_acc_info = next_account_info(account_info_iter)?;
    let dex_market_acc_info = next_account_info(account_info_iter)?;
    let dex_open_orders_info = next_account_info(account_info_iter)?;
    let rent_acc_info = next_account_info(account_info_iter)?;
    let dex_program_id_info = next_account_info(account_info_iter)?;

    let dex_program_id = *dex_program_id_info.key;

    // check onesol_market_acc_info
    if *protocol_market_acc_info.owner != *program_id {
      return Err(ProtocolError::InvalidProgramAddress.into());
    }
    let rent = Rent::get()?;
    if !rent.is_exempt(
      protocol_market_acc_info.lamports(),
      protocol_market_acc_info.data_len(),
    ) {
      return Err(ProtocolError::NotRentExempt.into());
    }

    // check amm_info_acc_info
    if *amm_info_acc_info.owner != *program_id {
      return Err(ProtocolError::InvalidAmmInfoAccount.into());
    }
    validate_authority_pubkey(
      authority_info.key,
      program_id,
      amm_info_acc_info.key,
      data.nonce,
    )?;

    let amm_info = AmmInfo::unpack(&amm_info_acc_info.data.borrow())
      .map_err(|_| ProtocolError::InvalidAccountData)?;
    if amm_info.nonce != data.nonce {
      return Err(ProtocolError::InvalidNonce.into());
    }

    if *dex_open_orders_info.owner != dex_program_id {
      return Err(ProtocolError::InvalidProgramAddress.into());
    }
    let market = SerumDexMarket::new(dex_market_acc_info)?;

    let market_coin_mint = market.coin_mint()?;
    let market_pc_mint = market.pc_mint()?;

    if amm_info.token_a_mint == amm_info.token_b_mint {
      return Err(ProtocolError::InvalidTokenMint.into());
    }
    if market_pc_mint != amm_info.token_a_mint && market_pc_mint != amm_info.token_b_mint {
      return Err(ProtocolError::InvalidTokenMint.into());
    }
    if market_coin_mint != amm_info.token_a_mint && market_coin_mint != amm_info.token_b_mint {
      return Err(ProtocolError::InvalidTokenMint.into());
    }
    if *market.inner().owner != dex_program_id {
      return Err(ProtocolError::InvalidProgramAddress.into());
    }

    if !sysvar::rent::check_id(rent_acc_info.key) {
      return Err(ProtocolError::InvalidRentAccount.into());
    }

    let is_initialized = match DexMarketInfo::unpack(&protocol_market_acc_info.data.borrow()) {
      Ok(dex_market_info) => dex_market_info
        .flags()
        .map(|x| x.contains(AccountFlag::Initialized))
        .unwrap_or(false),
      Err(_) => false,
    };

    if is_initialized {
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

    let obj = DexMarketInfo {
      account_flags: (AccountFlag::Initialized | AccountFlag::DexMarketInfo).bits(),
      amm_info: *amm_info_acc_info.key,
      dex_program_id: dex_program_id,
      market: *dex_market_acc_info.key,
      pc_mint: market_pc_mint,
      coin_mint: market_coin_mint,
      open_orders: *dex_open_orders_info.key,
    };
    DexMarketInfo::pack(obj, &mut protocol_market_acc_info.data.borrow_mut())?;
    Ok(())
  }

  /// process update DexMarketInfo OpenOrders
  pub fn process_update_dex_mark_open_orders(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
  ) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let authority_info = next_account_info(account_info_iter)?;
    let amm_info_acc = next_account_info(account_info_iter)?;
    let protocol_market_info_acc = next_account_info(account_info_iter)?;
    let dex_market_acc_info = next_account_info(account_info_iter)?;
    let dex_open_orders_info = next_account_info(account_info_iter)?;
    let rent_acc_info = next_account_info(account_info_iter)?;
    let dex_program_id_info = next_account_info(account_info_iter)?;

    let dex_program_id = *dex_program_id_info.key;

    if *amm_info_acc.owner != *program_id {
      return Err(ProtocolError::InvalidAmmInfoAccount.into());
    }
    if *protocol_market_info_acc.owner != *program_id {
      return Err(ProtocolError::InvalidDexMarketInfoAccount.into());
    }
    if !protocol_market_info_acc.is_writable {
      return Err(ProtocolError::ReadonlyAccount.into());
    }
    let amm_info = AmmInfo::unpack(&amm_info_acc.data.borrow())
      .map_err(|_| ProtocolError::InvalidAmmInfoAccount)?;
    if amm_info.account_flags != (AccountFlag::Initialized | AccountFlag::AmmInfo).bits() {
      return Err(ProtocolError::InvalidAccountFlags.into());
    }

    let protocol_market_info = DexMarketInfo::unpack(&protocol_market_info_acc.data.borrow())
      .map_err(|e| {
        msg!("DexMarketInfo::unpack err: {}", e);
        ProtocolError::InvalidDexMarketInfoAccount
      })?;
    if protocol_market_info.account_flags
      != (AccountFlag::Initialized | AccountFlag::DexMarketInfo).bits()
    {
      return Err(ProtocolError::InvalidAccountFlags.into());
    }
    if protocol_market_info.amm_info != *amm_info_acc.key {
      msg!("dex_market_info.amm_info != amm_info_acc.key");
      return Err(ProtocolError::InvalidDexMarketInfoAccount.into());
    }

    let market = SerumDexMarket::new(dex_market_acc_info)?;

    if protocol_market_info.market != *dex_market_acc_info.key {
      msg!("protocol_market_info.market != dex_market_acc_info.key");
      return Err(ProtocolError::InvalidDexMarketInfoAccount.into());
    }

    let market_coin_mint = market.coin_mint()?;
    let market_pc_mint = market.pc_mint()?;

    if amm_info.token_a_mint == amm_info.token_b_mint {
      return Err(ProtocolError::InvalidTokenMint.into());
    }
    if market_pc_mint != amm_info.token_a_mint && market_pc_mint != amm_info.token_b_mint {
      return Err(ProtocolError::InvalidTokenMint.into());
    }
    if market_coin_mint != amm_info.token_a_mint && market_coin_mint != amm_info.token_b_mint {
      return Err(ProtocolError::InvalidTokenMint.into());
    }
    if *market.inner().owner != dex_program_id {
      return Err(ProtocolError::InvalidProgramAddress.into());
    }
    if !sysvar::rent::check_id(rent_acc_info.key) {
      return Err(ProtocolError::InvalidRentAccount.into());
    }

    serum_dex_order::invoke_init_open_orders(
      amm_info_acc.key,
      &dex_program_id,
      dex_open_orders_info,
      authority_info,
      dex_market_acc_info,
      rent_acc_info,
      amm_info.nonce,
    )?;

    let mut obj = protocol_market_info;
    obj.open_orders = *dex_open_orders_info.key;

    DexMarketInfo::pack(obj, &mut protocol_market_info_acc.data.borrow_mut())?;
    Ok(())
  }

  /// Process swap fees instruction
  pub fn process_swap_fees(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    if accounts.len() < 7 {
      return Err(ProtocolError::InvalidAccountsLength.into());
    }
    let fixed_accounts = array_ref![accounts, 0, 7];
    let (
      amm_info_accounts,
      &[ref spl_token_program_acc, ref token_a_destination, ref token_b_destination],
    ) = array_refs![fixed_accounts, 4, 3];

    let amm_info_args = AmmInfoArgs::with_parsed_args(program_id, amm_info_accounts)?;
    let spl_token_program = SplTokenProgram::new(spl_token_program_acc)?;

    let owner_key = amm_info_args.amm_info.owner;
    amm_info_args.token_a_account.check_writable()?;
    amm_info_args.token_b_account.check_writable()?;

    let token_a_dest = TokenAccount::new(token_a_destination)?;
    token_a_dest.check_owner(&owner_key, true)?;
    token_a_dest.check_writable()?;
    if token_a_dest.mint()? != amm_info_args.token_a_account.mint()? {
      msg!(
        "token_a_dest.mint: {}, amm_info.token_a_mint: {}",
        token_a_dest.mint()?,
        amm_info_args.token_a_account.mint()?
      );
      return Err(ProtocolError::InvalidTokenAccount.into());
    }

    let token_b_dest = TokenAccount::new(token_b_destination)?;
    token_b_dest.check_owner(&owner_key, true)?;
    token_b_dest.check_writable()?;
    if token_b_dest.mint()? != amm_info_args.token_b_account.mint()? {
      msg!(
        "token_b_dest.mint: {}, amm_info.token_b_mint: {}",
        token_b_dest.mint()?,
        amm_info_args.token_b_account.mint()?
      );
      return Err(ProtocolError::InvalidTokenAccount.into());
    }

    let balance_a = amm_info_args.token_a_account.balance()?;
    let balance_b = amm_info_args.token_b_account.balance()?;

    if balance_a > 0 {
      Self::token_transfer(
        amm_info_args.amm_info_acc_info.key,
        spl_token_program.inner(),
        amm_info_args.token_a_account.inner(),
        token_a_dest.inner(),
        amm_info_args.authority_acc_info,
        amm_info_args.nonce(),
        balance_a,
      )?;
    }
    if balance_b > 0 {
      Self::token_transfer(
        amm_info_args.amm_info_acc_info.key,
        spl_token_program.inner(),
        amm_info_args.token_b_account.inner(),
        token_b_dest.inner(),
        amm_info_args.authority_acc_info,
        amm_info_args.nonce(),
        balance_b,
      )?;
    }
    Ok(())
  }

  pub fn process_single_step_swap(
    program_id: &Pubkey,
    data: &SwapInstruction,
    accounts: &[AccountInfo],
    exchanger: ExchangerType,
  ) -> ProgramResult {
    if accounts.len() < exchanger.min_accounts() {
      return Err(ProtocolError::InvalidAccountsLength.into());
    }
    let (fixed_accounts, other_accounts) = array_refs![accounts, 8; ..;];

    let (user_accounts, &[ref spl_token_program_acc], amm_info_accounts) =
      array_refs![fixed_accounts, 3, 1, 4];

    let user_args = UserArgs::with_parsed_args(user_accounts)?;

    let spl_token_program = SplTokenProgram::new(spl_token_program_acc)?;
    let amm_info_args = AmmInfoArgs::with_parsed_args(program_id, amm_info_accounts)?;

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

    Self::token_transfer(
      amm_info_args.amm_info_acc_info.key,
      spl_token_program.inner(),
      user_args.token_source_account.inner(),
      amm_source_token_acc.inner(),
      user_args.source_account_owner.inner(),
      amm_info_args.nonce(),
      data.amount_in.get(),
    )?;

    // transfer from user token_source_account to amm_source_token_account

    let from_amount_before = amm_source_token_acc.balance()?;
    let to_amount_before = amm_destination_token_acc.balance()?;
    msg!(
      "from_amount_before: {}, to_amount_before: {}, amount_in: {}",
      from_amount_before,
      to_amount_before,
      data.amount_in
    );

    let base_bytes = amm_info_args.amm_info_acc_info.key.to_bytes();
    let authority_signature_seeds = [&base_bytes[..32], &[amm_info_args.nonce()]];
    let signers = &[&authority_signature_seeds[..]];

    match exchanger {
      ExchangerType::SplTokenSwap => Self::process_step_tokenswap(
        program_id,
        data,
        amm_source_token_acc,
        amm_destination_token_acc,
        amm_info_args.authority_acc_info,
        Some(signers),
        &spl_token_program,
        other_accounts,
      ),
      ExchangerType::StableSwap => Self::process_step_stableswap(
        program_id,
        data,
        amm_source_token_acc,
        amm_destination_token_acc,
        amm_info_args.authority_acc_info,
        Some(signers),
        &spl_token_program,
        other_accounts,
      ),
      ExchangerType::RaydiumSwap => Self::process_step_raydium(
        program_id,
        data,
        amm_source_token_acc,
        amm_destination_token_acc,
        amm_info_args.authority_acc_info,
        Some(signers),
        &spl_token_program,
        other_accounts,
      ),
      ExchangerType::SerumDex => Self::process_step_serumdex(
        program_id,
        data,
        amm_source_token_acc,
        amm_destination_token_acc,
        amm_info_args.authority_acc_info,
        Some(signers),
        &spl_token_program,
        other_accounts,
      ),
    }?;

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

    amm_info_args
      .record(
        &user_source_token_mint,
        &user_destination_token_mint,
        from_amount_changed,
        to_amount,
        fee,
      )
      .ok();

    // Transfer OnesolB -> AliceB
    msg!("transfer to user destination, amount: {}", to_amount);
    sol_log_compute_units();
    Self::token_transfer(
      amm_info_args.amm_info_acc_info.key,
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

  pub fn process_swap_two_steps(
    program_id: &Pubkey,
    data: &SwapTwoStepsInstruction,
    accounts: &[AccountInfo],
  ) -> ProgramResult {
    msg!("start process swap: two_steps");
    let account_counts = 4 + data.step1.accounts_count + 4 + data.step2.accounts_count;
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
    let (step1_accounts, step2_accounts) = other_accounts.split_at(data.step1.accounts_count);
    // let mut steps = &[(data.step1, step1_accounts), (data.step2, step2_accounts)];
    // step1

    let (step2_amm_info_args, step2_other_account) = {
      let (amm_info_accounts, step_other_accounts) = step2_accounts.split_at(4);
      let amm_info_args = AmmInfoArgs::with_parsed_args(program_id, amm_info_accounts)?;
      (amm_info_args, step_other_accounts)
    };

    let (step2_source_token_account, step2_destination_token_account) =
      if user_args.token_destination_account.mint()?
        == step2_amm_info_args.token_b_account.mint()?
      {
        (
          step2_amm_info_args.token_a_account,
          step2_amm_info_args.token_b_account,
        )
      } else {
        (
          step2_amm_info_args.token_b_account,
          step2_amm_info_args.token_a_account,
        )
      };

    let from_amount_before = user_args.token_source_account.balance()?;
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
      SwapInstruction {
        amount_in: data.amount_in,
        expect_amount_out: NonZeroU64::new(1).unwrap(),
        minimum_amount_out: NonZeroU64::new(1).unwrap(),
        use_full: false,
      },
      &user_args.token_source_account,
      &step2_source_token_account,
      user_args.source_account_owner.inner(),
      None,
      &spl_token_program,
      &step1_accounts,
    )?;
    let step2_amount_in = step2_source_token_account
      .balance()?
      .checked_sub(step1_to_amount_before)
      .ok_or(ProtocolError::Unreachable)?;
    msg!("step2 amount_in: {}", step2_amount_in);

    // step2

    let base_bytes = step2_amm_info_args.amm_info_acc_info.key.to_bytes();
    let authority_signature_seeds = [&base_bytes[..32], &[step2_amm_info_args.nonce()]];
    let signers = &[&authority_signature_seeds[..]];

    Self::process_step_auto(
      program_id,
      &data.step2,
      SwapInstruction {
        amount_in: NonZeroU64::new(step2_amount_in).unwrap(),
        expect_amount_out: data.expect_amount_out,
        minimum_amount_out: data.minimum_amount_out,
        use_full: false,
      },
      &step2_source_token_account,
      &step2_destination_token_account,
      &step2_amm_info_args.authority_acc_info,
      Some(signers),
      &spl_token_program,
      &step2_other_account,
    )?;

    let from_amount_after = user_args.token_source_account.balance()?;
    let to_amount_after = step2_destination_token_account.balance()?;
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
    step2_amm_info_args
      .record(
        &step2_source_token_account.mint()?,
        &step2_destination_token_account.mint()?,
        from_amount_changed,
        to_amount,
        fee,
      )
      .ok();

    msg!("transfer to user destination, amount: {}", to_amount);
    Self::token_transfer(
      step2_amm_info_args.amm_info_acc_info.key,
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
    authority: &'a AccountInfo<'b>,
    signers_seed: Option<&[&[&[u8]]]>,
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
          authority,
          signers_seed,
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
          authority,
          signers_seed,
          spl_token_program,
          accounts,
        )?;
      }
      ExchangerType::StableSwap => {
        Self::process_step_stableswap(
          program_id,
          &data,
          source_token_account,
          destination_token_account,
          authority,
          signers_seed,
          spl_token_program,
          accounts,
        )?;
      }
      ExchangerType::RaydiumSwap => {
        Self::process_step_stableswap(
          program_id,
          &data,
          source_token_account,
          destination_token_account,
          authority,
          signers_seed,
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
    data: &SwapInstruction,
    source_token_account: &TokenAccount<'a, 'b>,
    destination_token_account: &TokenAccount<'a, 'b>,
    authority: &'a AccountInfo<'b>,
    signers_seed: Option<&[&[&[u8]]]>,
    spl_token_program: &SplTokenProgram<'a, 'b>,
    accounts: &'a [AccountInfo<'b>],
  ) -> ProgramResult {
    msg!(
      "swap using token-swap, amount_in: {}, minimum_amount_out: {}, expect_amount_out: {}, use_full: {}",
      data.amount_in,
      data.minimum_amount_out,
      data.expect_amount_out,
      data.use_full,
    );

    let spl_token_swap_args = SplTokenSwapArgs::with_parsed_args(accounts)?;
    let token_swap_amount_in = Self::get_amount_in(
      data.amount_in.get(),
      source_token_account.balance()?,
      data.use_full,
    );
    let token_swap_minimum_amount_out = data.minimum_amount_out;

    let source_token_mint = source_token_account.mint()?;
    let destination_token_mint = destination_token_account.mint()?;

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
      authority.clone(),
      // amm_info_args.authority_acc_info.clone(),
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
    swap_accounts.push(spl_token_swap_args.program.clone());

    let instruction_data = spl_token_swap::Swap {
      amount_in: token_swap_amount_in,
      minimum_amount_out: token_swap_minimum_amount_out.get(),
    };
    let instruction = spl_token_swap::spl_token_swap_instruction(
      spl_token_swap_args.program.key,
      spl_token_program.inner().key,
      spl_token_swap_args.swap_info.inner().key,
      spl_token_swap_args.authority_acc_info.key,
      authority.key,
      source_token_account.inner().key,
      pool_source_token_acc.inner().key,
      pool_destination_token_acc.inner().key,
      destination_token_account.inner().key,
      spl_token_swap_args.pool_mint.inner().key,
      spl_token_swap_args.fee_account.inner().key,
      host_fee_account_key,
      instruction_data,
    )?;
    msg!("invoke spl-token-swap swap");

    match signers_seed {
      Some(signers) => {
        invoke_signed(&instruction, &swap_accounts[..], signers)?;
      }
      None => {
        if !authority.is_signer {
          return Err(ProtocolError::InvalidAuthority.into());
        }
        invoke(&instruction, &swap_accounts)?;
      }
    }
    Ok(())
  }

  #[allow(clippy::too_many_arguments, unused_variables)]
  fn process_step_serumdex<'a, 'b: 'a>(
    program_id: &Pubkey,
    data: &SwapInstruction,
    source_token_account: &TokenAccount<'a, 'b>,
    destination_token_account: &TokenAccount<'a, 'b>,
    authority: &'a AccountInfo<'b>,
    signers_seed: Option<&[&[&[u8]]]>,
    spl_token_program: &SplTokenProgram<'a, 'b>,
    accounts: &'a [AccountInfo<'b>],
  ) -> ProgramResult {
    let dex_args = SerumDexArgs::with_parsed_args(accounts)?;

    let source_token_mint = source_token_account.mint()?;
    let destination_token_mint = destination_token_account.mint()?;

    let amount_in = Self::get_amount_in(
      data.amount_in.get(),
      source_token_account.balance()?,
      data.use_full,
    );

    let side = dex_args.find_side(&source_token_mint)?;

    let (pc_wallet_account, coin_wallet_account) = match side {
      DexSide::Bid => (source_token_account, destination_token_account),
      DexSide::Ask => (destination_token_account, source_token_account),
    };

    let orderbook = serum_dex_order::OrderbookClient {
      market: serum_dex_order::MarketAccounts {
        market: dex_args.market.inner(),
        open_orders: dex_args.open_orders.inner(),
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
      authority: authority,
      pc_wallet: pc_wallet_account.inner(),
      dex_program: dex_args.program_acc,
      token_program: spl_token_program.inner(),
      rent: dex_args.rent_sysvar_acc,
      signers_seed: signers_seed,
    };
    // orderbook.cancel_order(side)?;
    match side {
      DexSide::Bid => orderbook.buy(amount_in, None)?,
      DexSide::Ask => orderbook.sell(amount_in, None)?,
    }
    orderbook.settle(None)?;
    Ok(())
  }

  /// Step swap in spl-token-swap
  #[allow(clippy::too_many_arguments, unused_variables)]
  fn process_step_stableswap<'a, 'b: 'a>(
    program_id: &Pubkey,
    data: &SwapInstruction,
    source_token_account: &TokenAccount<'a, 'b>,
    destination_token_account: &TokenAccount<'a, 'b>,
    authority: &'a AccountInfo<'b>,
    signers_seed: Option<&[&[&[u8]]]>,
    spl_token_program: &SplTokenProgram<'a, 'b>,
    accounts: &'a [AccountInfo<'b>],
  ) -> ProgramResult {
    sol_log_compute_units();

    let swap_args = StableSwapArgs::with_parsed_args(accounts)?;
    let amount_in = Self::get_amount_in(
      data.amount_in.get(),
      source_token_account.balance()?,
      data.use_full,
    );
    let swap_minimum_amount_out = data.minimum_amount_out;

    msg!(
      "swap using stable-swap, amount_in: {}, minimum_amount_out: {}, expect_amount_out: {}",
      amount_in,
      data.minimum_amount_out,
      data.expect_amount_out,
    );

    let source_token_mint = source_token_account.mint()?;
    let destination_token_mint = destination_token_account.mint()?;

    let (swap_source_token_acc, swap_destination_token_acc) =
      swap_args.find_token_pair(&source_token_mint)?;

    if swap_source_token_acc.mint()? != source_token_mint {
      return Err(ProtocolError::InvalidTokenMint.into());
    }
    if swap_destination_token_acc.mint()? != destination_token_mint {
      return Err(ProtocolError::InvalidTokenMint.into());
    }

    let swap_accounts = vec![
      swap_args.swap_info.inner().clone(),
      swap_args.authority_acc.clone(),
      authority.clone(),
      source_token_account.inner().clone(),
      swap_source_token_acc.inner().clone(),
      swap_destination_token_acc.inner().clone(),
      destination_token_account.inner().clone(),
      swap_args.admin_fee_acc.clone(),
      spl_token_program.inner().clone(),
      swap_args.clock_sysvar_acc.inner().clone(),
      swap_args.program_acc.clone(),
    ];

    let instruction = stable_swap_client::instruction::swap(
      spl_token_program.inner().key,
      swap_args.swap_info.inner().key,
      swap_args.authority_acc.key,
      authority.key,
      source_token_account.inner().key,
      swap_source_token_acc.inner().key,
      swap_destination_token_acc.inner().key,
      destination_token_account.inner().key,
      swap_args.admin_fee_acc.key,
      amount_in,
      swap_minimum_amount_out.get(),
    )?;

    msg!("invoke saber-stableswap swap");

    sol_log_compute_units();
    match signers_seed {
      Some(signers) => {
        invoke_signed(&instruction, &swap_accounts[..], signers)?;
      }
      None => {
        if !authority.is_signer {
          return Err(ProtocolError::InvalidAuthority.into());
        }
        invoke(&instruction, &swap_accounts)?;
      }
    }
    sol_log_compute_units();
    Ok(())
  }

  /// Step swap in spl-token-swap
  #[allow(clippy::too_many_arguments, unused_variables)]
  fn process_step_raydium<'a, 'b: 'a>(
    program_id: &Pubkey,
    data: &SwapInstruction,
    source_token_account: &TokenAccount<'a, 'b>,
    destination_token_account: &TokenAccount<'a, 'b>,
    authority: &'a AccountInfo<'b>,
    signers_seed: Option<&[&[&[u8]]]>,
    spl_token_program: &SplTokenProgram<'a, 'b>,
    accounts: &'a [AccountInfo<'b>],
  ) -> ProgramResult {
    sol_log_compute_units();

    let swap_args = RaydiumSwapArgs::with_parsed_args(accounts)?;
    let amount_in = Self::get_amount_in(
      data.amount_in.get(),
      source_token_account.balance()?,
      data.use_full,
    );
    let swap_minimum_amount_out = data.minimum_amount_out;

    msg!(
      "swap using raydium, amount_in: {}, minimum_amount_out: {}, expect_amount_out: {}",
      amount_in,
      data.minimum_amount_out,
      data.expect_amount_out,
    );

    let source_token_mint = source_token_account.mint()?;
    let destination_token_mint = destination_token_account.mint()?;

    // let (swap_source_token_acc, swap_destination_token_acc) =
    //   swap_args.find_token_pair(&source_token_mint)?;

    // if swap_source_token_acc.mint()? != source_token_mint {
    //   return Err(ProtocolError::InvalidTokenMint.into());
    // }
    // if swap_destination_token_acc.mint()? != destination_token_mint {
    //   return Err(ProtocolError::InvalidTokenMint.into());
    // }

    let swap_accounts = vec![
      swap_args.program_id.clone(),
      spl_token_program.inner().clone(),
      swap_args.amm_info.inner().clone(),
      swap_args.authority.clone(),
      swap_args.open_orders.inner().clone(),
      swap_args.target_orders.clone(),
      swap_args.pool_token_coin.inner().clone(),
      swap_args.pool_token_pc.inner().clone(),
      swap_args.serum_dex_program_id.clone(),
      swap_args.serum_market.inner().clone(),
      swap_args.bids.clone(),
      swap_args.asks.clone(),
      swap_args.event_q.clone(),
      swap_args.coin_vault.inner().clone(),
      swap_args.pc_vault.inner().clone(),
      swap_args.vault_signer.clone(),
      source_token_account.inner().clone(),
      destination_token_account.inner().clone(),
    ];

    let instruction = raydium_swap::swap_base_in(
      swap_args.program_id.key,
      swap_args.amm_info.pubkey(),
      swap_args.authority.key,
      swap_args.open_orders.pubkey(),
      swap_args.target_orders.key,
      swap_args.pool_token_coin.pubkey(),
      swap_args.pool_token_pc.pubkey(),
      swap_args.serum_dex_program_id.key,
      swap_args.serum_market.pubkey(),
      swap_args.bids.key,
      swap_args.asks.key,
      swap_args.event_q.key,
      swap_args.coin_vault.pubkey(),
      swap_args.pc_vault.pubkey(),
      swap_args.vault_signer.key,
      source_token_account.pubkey(),
      destination_token_account.pubkey(),
      authority.key,
      amount_in,
      swap_minimum_amount_out.get(),
    )?;

    msg!("invoke raydium swap_base_in");

    match signers_seed {
      Some(signers) => {
        invoke_signed(&instruction, &swap_accounts[..], signers)?;
      }
      None => {
        if !authority.is_signer {
          return Err(ProtocolError::InvalidAuthority.into());
        }
        invoke(&instruction, &swap_accounts)?;
      }
    }
    Ok(())
  }

  fn get_amount_in(amount_in: u64, source_token_balance: u64, use_full: bool) -> u64 {
    if use_full {
      source_token_balance
    } else if source_token_balance < amount_in {
      source_token_balance
    } else {
      amount_in
    }
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
