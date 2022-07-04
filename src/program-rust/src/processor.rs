//! Program state processor

use std::cmp;

use crate::{
  constraints::OWNER_KEY,
  error::ProtocolError,
  exchanger::{
    aldrin, crema, cropper, raydium,
    serum_dex::{self, matching::Side as DexSide},
    spl_token_swap, stable_swap,
  },
  instruction::{
    ExchangerType, ProtocolInstruction, SwapInInstruction, SwapInstruction, SwapOutInstruction,
    SwapOutSlimInstruction,
  },
  parser::{
    aldrin::AldrinPoolArgs,
    base::{SplTokenProgram, SwapInfoArgs, TokenAccount, UserArgs},
    crema::CremaSwapV1Args,
    cropper::CropperArgs,
    raydium::{RaydiumSwapArgs, RaydiumSwapArgs2},
    serum_dex::SerumDexArgs,
    spl_token_swap::SplTokenSwapArgs,
    stable_swap::StableSwapArgs,
  },
  spl_token,
  state::{Status, SwapInfo},
};
use arrayref::array_refs;
use solana_program::{
  account_info::AccountInfo,
  entrypoint::ProgramResult,
  log::sol_log_compute_units,
  msg,
  program::{invoke, invoke_signed},
  program_error::ProgramError,
  program_memory::{sol_memcmp, sol_memset},
  program_option::COption,
  program_pack::Pack,
  pubkey::{Pubkey, PUBKEY_BYTES},
  rent::Rent,
  sysvar::Sysvar,
};
/// Program state handler.
pub struct Processor {}

impl Processor {
  /// Processes an [Instruction](enum.Instruction.html).
  pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]) -> ProgramResult {
    let instruction = ProtocolInstruction::unpack(input)?;
    match instruction {
      ProtocolInstruction::SwapSplTokenSwap(data) => {
        msg!("Instruction: Swap TokenSwap");
        Self::process_single_step_swap(program_id, &data, accounts, ExchangerType::SplTokenSwap)
      }
      ProtocolInstruction::SwapSerumDex(data) => {
        msg!("Instruction: Swap SerumDex");
        Self::process_single_step_swap(program_id, &data, accounts, ExchangerType::SerumDex)
      }
      ProtocolInstruction::SwapStableSwap(data) => {
        msg!("Instruction: Swap StableSwap");
        Self::process_single_step_swap(program_id, &data, accounts, ExchangerType::StableSwap)
      }
      ProtocolInstruction::SwapRaydiumSwap(data) => {
        msg!("Instruction: Swap RaydiumSwap");
        Self::process_single_step_swap(program_id, &data, accounts, ExchangerType::RaydiumSwap)
      }
      ProtocolInstruction::InitializeSwapInfo => {
        msg!("Instruction: InitializeSwapInfo");
        Self::process_initialize_swap_info(program_id, accounts)
      }
      ProtocolInstruction::SetupSwapInfo => {
        msg!("Instruction: SetupSwapInfo");
        Self::process_setup_swap_info(program_id, accounts)
      }
      ProtocolInstruction::CloseSwapInfo => {
        msg!("Instruction: CloseSwapInfo");
        Self::process_close_swap_info(program_id, accounts)
      }
      ProtocolInstruction::SwapSplTokenSwapIn(data) => {
        msg!("Instruction: Swap SplTokenSwap In");
        Self::process_single_step_swap_in(program_id, &data, accounts, ExchangerType::SplTokenSwap)
      }
      ProtocolInstruction::SwapSplTokenSwapOut(data) => {
        msg!("Instruction: Swap SplTokenSwap Out");
        Self::process_single_step_swap_out(program_id, &data, accounts, ExchangerType::SplTokenSwap)
      }
      ProtocolInstruction::SwapSerumDexIn(data) => {
        msg!("Instruction: Swap SplTokenSwap In");
        Self::process_single_step_swap_in(program_id, &data, accounts, ExchangerType::SerumDex)
      }
      ProtocolInstruction::SwapSerumDexOut(data) => {
        msg!("Instruction: Swap SplTokenSwap Out");
        Self::process_single_step_swap_out(program_id, &data, accounts, ExchangerType::SerumDex)
      }
      ProtocolInstruction::SwapStableSwapIn(data) => {
        msg!("Instruction: Swap SplTokenSwap In");
        Self::process_single_step_swap_in(program_id, &data, accounts, ExchangerType::StableSwap)
      }
      ProtocolInstruction::SwapStableSwapOut(data) => {
        msg!("Instruction: Swap SplTokenSwap Out");
        Self::process_single_step_swap_out(program_id, &data, accounts, ExchangerType::StableSwap)
      }
      ProtocolInstruction::SwapRaydiumIn(data) => {
        msg!("Instruction: Swap SplTokenSwap In");
        Self::process_single_step_swap_in(program_id, &data, accounts, ExchangerType::RaydiumSwap)
      }
      ProtocolInstruction::SwapRaydiumOut(data) => {
        msg!("Instruction: Swap SplTokenSwap Out");
        Self::process_single_step_swap_out(program_id, &data, accounts, ExchangerType::RaydiumSwap)
      }
      ProtocolInstruction::SwapRaydiumIn2(data) => Self::process_single_step_swap_in(
        program_id,
        &data,
        accounts,
        ExchangerType::RaydiumSwapSlim,
      ),
      ProtocolInstruction::SwapRaydiumOut2(data) => Self::process_single_step_swap_out_slim(
        program_id,
        &data,
        accounts,
        ExchangerType::RaydiumSwapSlim,
      ),
      ProtocolInstruction::SwapCremaFinance(data) => {
        Self::process_single_step_swap(program_id, &data, accounts, ExchangerType::CremaFinance)
      }
      ProtocolInstruction::SwapCremaFinanceIn(data) => {
        Self::process_single_step_swap_in(program_id, &data, accounts, ExchangerType::CremaFinance)
      }
      ProtocolInstruction::SwapCremaFinanceOut(data) => {
        Self::process_single_step_swap_out(program_id, &data, accounts, ExchangerType::CremaFinance)
      }
      ProtocolInstruction::SwapAldrinExchange(data) => {
        Self::process_single_step_swap(program_id, &data, accounts, ExchangerType::AldrinExchange)
      }
      ProtocolInstruction::SwapAldrinExchangeIn(data) => Self::process_single_step_swap_in(
        program_id,
        &data,
        accounts,
        ExchangerType::AldrinExchange,
      ),
      ProtocolInstruction::SwapAldrinExchangeOut(data) => Self::process_single_step_swap_out(
        program_id,
        &data,
        accounts,
        ExchangerType::AldrinExchange,
      ),
      ProtocolInstruction::SwapCropperFinance(data) => {
        Self::process_single_step_swap(program_id, &data, accounts, ExchangerType::CropperFinance)
      }
      ProtocolInstruction::SwapCropperFinanceIn(data) => Self::process_single_step_swap_in(
        program_id,
        &data,
        accounts,
        ExchangerType::CropperFinance,
      ),
      ProtocolInstruction::SwapCropperFinanceOut(data) => Self::process_single_step_swap_out(
        program_id,
        &data,
        accounts,
        ExchangerType::CropperFinance,
      ),
    }
  }

  /// Checks two pubkeys for equality in a computationally cheap way using
  /// `sol_memcmp`
  pub fn cmp_pubkeys(a: &Pubkey, b: &Pubkey) -> bool {
    sol_memcmp(a.as_ref(), b.as_ref(), PUBKEY_BYTES) == 0
  }

  pub fn process_initialize_swap_info(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
  ) -> ProgramResult {
    if accounts.len() < 2 {
      return Err(ProtocolError::InvalidAccountsLength.into());
    }
    #[allow(clippy::ptr_offset_with_cast)]
    let (&[ref swap_info_account, ref user_account], _) = array_refs![accounts, 2;..;];
    // check onesol_market_acc_info
    if *swap_info_account.owner != *program_id {
      return Err(ProtocolError::InvalidProgramAddress.into());
    }
    let rent = Rent::get()?;
    if !rent.is_exempt(swap_info_account.lamports(), swap_info_account.data_len()) {
      return Err(ProtocolError::NotRentExempt.into());
    }
    if !swap_info_account.is_writable {
      return Err(ProtocolError::ReadonlyAccount.into());
    }
    if !user_account.is_signer {
      return Err(ProtocolError::InvalidSignerAccount.into());
    }
    if swap_info_account.data.borrow()[0] == 1 {
      return Err(ProtocolError::InvalidAccountFlags.into());
    }
    let swap_info = SwapInfo::new(user_account.key);
    SwapInfo::pack(swap_info, &mut swap_info_account.data.borrow_mut())?;
    Ok(())
  }

  pub fn process_setup_swap_info(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    if accounts.len() < 2 {
      return Err(ProtocolError::InvalidAccountsLength.into());
    }
    #[allow(clippy::ptr_offset_with_cast)]
    let (&[ref swap_info_account, ref token_account_info], _) = array_refs![accounts, 2;..;];
    if *swap_info_account.owner != *program_id {
      return Err(ProtocolError::InvalidProgramAddress.into());
    }
    let mut swap_info = SwapInfo::unpack(*swap_info_account.try_borrow_data()?)?;
    if Status::from_u8(swap_info.status)? != Status::SwapInfo {
      return Err(ProtocolError::InvalidAccountFlags.into());
    }
    let token_account = TokenAccount::new(token_account_info)?;
    token_account.check_owner(&swap_info.owner, true)?;
    swap_info.token_account = COption::Some(*token_account.pubkey());
    swap_info.token_latest_amount = 0;
    SwapInfo::pack(swap_info, &mut swap_info_account.data.borrow_mut())?;
    Ok(())
  }

  pub fn process_close_swap_info(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    if accounts.len() < 3 {
      return Err(ProtocolError::InvalidAccountsLength.into());
    }
    #[allow(clippy::ptr_offset_with_cast)]
    #[rustfmt::skip]
    let (&[
      ref swap_info_account,
      ref owner_account,
      ref destination_account,
    ], _) = array_refs![accounts, 3;..;];
    if !owner_account.is_signer {
      return Err(ProgramError::IllegalOwner);
    }
    if Self::cmp_pubkeys(swap_info_account.key, destination_account.key) {
      return Err(ProgramError::InvalidAccountData);
    }
    if !Self::cmp_pubkeys(swap_info_account.owner, program_id) {
      return Err(ProgramError::InvalidAccountData);
    }
    let swap_info = SwapInfo::unpack(&swap_info_account.data.borrow())?;
    if !Self::cmp_pubkeys(&swap_info.owner, owner_account.key) {
      return Err(ProtocolError::InvalidOwner.into());
    }
    let dest_starting_lamports = destination_account.lamports();
    **destination_account.lamports.borrow_mut() = dest_starting_lamports
      .checked_add(swap_info_account.lamports())
      .ok_or(ProtocolError::Overflow)?;
    **swap_info_account.lamports.borrow_mut() = 0;
    sol_memset(*swap_info_account.data.borrow_mut(), 0, SwapInfo::LEN);
    Ok(())
  }

  pub fn process_single_step_swap(
    program_id: &Pubkey,
    data: &SwapInstruction,
    accounts: &[AccountInfo],
    exchanger: ExchangerType,
  ) -> ProgramResult {
    if accounts.len() < 5 {
      return Err(ProtocolError::InvalidAccountsLength.into());
    }
    #[allow(clippy::ptr_offset_with_cast)]
    let (fixed_accounts, other_accounts) = array_refs![accounts, 5; ..;];

    let (user_accounts, &[ref spl_token_program_acc, ref fee_token_account_acc]) =
      array_refs![fixed_accounts, 3, 2];

    let user_args = UserArgs::with_parsed_args(user_accounts)?;
    let spl_token_program = SplTokenProgram::new(spl_token_program_acc)?;

    if !user_args.source_account_owner.is_signer {
      return Err(ProtocolError::InvalidSignerAccount.into());
    }
    user_args
      .token_source_account
      .check_owner(user_args.source_account_owner.key, false)?;

    let fee_token_account = TokenAccount::new(fee_token_account_acc)?;
    if fee_token_account.mint()? != user_args.token_destination_account.mint()? {
      return Err(ProtocolError::InvalidFeeTokenAccount.into());
    }
    if fee_token_account.owner()?.to_string() != *OWNER_KEY {
      return Err(ProtocolError::InvalidFeeTokenAccount.into());
    }

    if let Some(delegate) = fee_token_account.delegate()? {
      if delegate == *user_args.source_account_owner.key {
        return Err(ProtocolError::InvalidFeeTokenAccount.into());
      }
    }

    msg!(
      "source_token_account amount: {}",
      user_args.token_source_account.balance()?,
    );

    let from_amount_before = user_args.token_source_account.balance()?;
    let to_amount_before = user_args.token_destination_account.balance()?;
    msg!(
      "from_amount_before: {}, to_amount_before: {}, amount_in: {}",
      from_amount_before,
      to_amount_before,
      data.amount_in
    );

    match exchanger {
      ExchangerType::SplTokenSwap => Self::process_step_tokenswap(
        program_id,
        data.amount_in.get(),
        data.minimum_amount_out.get(),
        &user_args.token_source_account,
        &user_args.token_destination_account,
        user_args.source_account_owner,
        &spl_token_program,
        other_accounts,
      ),
      ExchangerType::StableSwap => Self::process_step_stableswap(
        program_id,
        data.amount_in.get(),
        data.minimum_amount_out.get(),
        &user_args.token_source_account,
        &user_args.token_destination_account,
        user_args.source_account_owner,
        &spl_token_program,
        other_accounts,
      ),
      ExchangerType::RaydiumSwap => Self::process_step_raydium(
        program_id,
        data.amount_in.get(),
        data.minimum_amount_out.get(),
        &user_args.token_source_account,
        &user_args.token_destination_account,
        user_args.source_account_owner,
        &spl_token_program,
        other_accounts,
      ),
      ExchangerType::RaydiumSwapSlim => Self::process_step_raydium_slim(
        program_id,
        data.amount_in.get(),
        data.minimum_amount_out.get(),
        &user_args.token_source_account,
        &user_args.token_destination_account,
        user_args.source_account_owner,
        &spl_token_program,
        other_accounts,
      ),
      ExchangerType::SerumDex => Self::process_step_serumdex(
        program_id,
        data.amount_in.get(),
        data.minimum_amount_out.get(),
        &user_args.token_source_account,
        &user_args.token_destination_account,
        user_args.source_account_owner,
        &spl_token_program,
        other_accounts,
      ),
      ExchangerType::CremaFinance => Self::process_step_crema_finance(
        program_id,
        data.amount_in.get(),
        data.minimum_amount_out.get(),
        &user_args.token_source_account,
        &user_args.token_destination_account,
        user_args.source_account_owner,
        &spl_token_program,
        other_accounts,
      ),
      ExchangerType::AldrinExchange => Self::process_step_aldrin_exchange(
        program_id,
        data.amount_in.get(),
        data.minimum_amount_out.get(),
        &user_args.token_source_account,
        &user_args.token_destination_account,
        user_args.source_account_owner,
        &spl_token_program,
        other_accounts,
      ),
      ExchangerType::CropperFinance => Self::process_step_cropper_finance(
        program_id,
        data.amount_in.get(),
        data.minimum_amount_out.get(),
        &user_args.token_source_account,
        &user_args.token_destination_account,
        user_args.source_account_owner,
        &spl_token_program,
        other_accounts,
      ),
    }?;
    let from_amount_after = user_args.token_source_account.balance()?;
    let to_amount_after = user_args.token_destination_account.balance()?;
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

    if fee > 0 {
      Self::token_transfer(
        spl_token_program.inner(),
        user_args.token_destination_account.inner(),
        fee_token_account.inner(),
        user_args.source_account_owner,
        fee,
      )?;
    }
    Ok(())
  }

  pub fn process_single_step_swap_in(
    program_id: &Pubkey,
    data: &SwapInInstruction,
    accounts: &[AccountInfo],
    exchanger: ExchangerType,
  ) -> ProgramResult {
    if accounts.len() < 5 {
      return Err(ProtocolError::InvalidAccountsLength.into());
    }
    #[allow(clippy::ptr_offset_with_cast)]
    let (fixed_accounts, other_accounts) = array_refs![accounts, 5; ..;];

    let (user_accounts, &[ref swap_info_account, ref spl_token_program_acc]) =
      array_refs![fixed_accounts, 3, 2];

    let user_args = UserArgs::with_parsed_args(user_accounts)?;
    let swap_info_args = SwapInfoArgs::with_parsed_args(swap_info_account, program_id)?;
    let spl_token_program = SplTokenProgram::new(spl_token_program_acc)?;

    if !user_args.source_account_owner.is_signer {
      return Err(ProtocolError::InvalidSignerAccount.into());
    }
    user_args
      .token_source_account
      .check_owner(user_args.source_account_owner.key, false)?;

    match swap_info_args.swap_info.token_account {
      COption::Some(k) => {
        if k != *user_args.token_destination_account.pubkey() {
          return Err(ProtocolError::InvalidTokenAccount.into());
        }
      }
      COption::None => {
        return Err(ProtocolError::InvalidTokenAccount.into());
      }
    };

    msg!(
      "source_token_account amount: {}",
      user_args.token_source_account.balance()?,
    );

    let from_amount_before = user_args.token_source_account.balance()?;
    let to_amount_before = user_args.token_destination_account.balance()?;
    msg!(
      "from_amount_before: {}, to_amount_before: {}, amount_in: {}",
      from_amount_before,
      to_amount_before,
      data.amount_in
    );

    match exchanger {
      ExchangerType::SplTokenSwap => Self::process_step_tokenswap(
        program_id,
        data.amount_in.get(),
        u64::MIN + 1,
        &user_args.token_source_account,
        &user_args.token_destination_account,
        user_args.source_account_owner,
        &spl_token_program,
        other_accounts,
      ),
      ExchangerType::StableSwap => Self::process_step_stableswap(
        program_id,
        data.amount_in.get(),
        u64::MIN + 1,
        &user_args.token_source_account,
        &user_args.token_destination_account,
        user_args.source_account_owner,
        &spl_token_program,
        other_accounts,
      ),
      ExchangerType::RaydiumSwap => Self::process_step_raydium(
        program_id,
        data.amount_in.get(),
        u64::MIN + 1,
        &user_args.token_source_account,
        &user_args.token_destination_account,
        user_args.source_account_owner,
        &spl_token_program,
        other_accounts,
      ),
      ExchangerType::RaydiumSwapSlim => Self::process_step_raydium_slim(
        program_id,
        data.amount_in.get(),
        u64::MIN + 1,
        &user_args.token_source_account,
        &user_args.token_destination_account,
        user_args.source_account_owner,
        &spl_token_program,
        other_accounts,
      ),
      ExchangerType::SerumDex => Self::process_step_serumdex(
        program_id,
        data.amount_in.get(),
        u64::MIN + 1,
        &user_args.token_source_account,
        &user_args.token_destination_account,
        user_args.source_account_owner,
        &spl_token_program,
        other_accounts,
      ),
      ExchangerType::CremaFinance => Self::process_step_crema_finance(
        program_id,
        data.amount_in.get(),
        u64::MIN + 1,
        &user_args.token_source_account,
        &user_args.token_destination_account,
        user_args.source_account_owner,
        &spl_token_program,
        other_accounts,
      ),
      ExchangerType::AldrinExchange => Self::process_step_aldrin_exchange(
        program_id,
        data.amount_in.get(),
        u64::MIN + 1,
        &user_args.token_source_account,
        &user_args.token_destination_account,
        user_args.source_account_owner,
        &spl_token_program,
        other_accounts,
      ),
      ExchangerType::CropperFinance => Self::process_step_cropper_finance(
        program_id,
        data.amount_in.get(),
        u64::MIN + 1,
        &user_args.token_source_account,
        &user_args.token_destination_account,
        user_args.source_account_owner,
        &spl_token_program,
        other_accounts,
      ),
    }?;

    let from_amount_after = user_args.token_source_account.balance()?;
    let to_amount_after = user_args.token_destination_account.balance()?;
    msg!(
      "from_amount_after: {}, to_amount_after: {}",
      from_amount_after,
      to_amount_after
    );

    let from_amount_changed = from_amount_before.checked_sub(from_amount_after).unwrap();
    let to_amount_include_fee = to_amount_after.checked_sub(to_amount_before).unwrap();
    msg!("from_amount changed: {}", from_amount_changed);
    msg!("result_with_fee: {}", to_amount_include_fee);

    let mut swap_info = swap_info_args.swap_info;
    swap_info.token_latest_amount = to_amount_include_fee;
    SwapInfo::pack(
      swap_info,
      &mut swap_info_args.swap_info_acc.data.borrow_mut(),
    )?;

    Ok(())
  }

  pub fn process_single_step_swap_out(
    program_id: &Pubkey,
    data: &SwapOutInstruction,
    accounts: &[AccountInfo],
    exchanger: ExchangerType,
  ) -> ProgramResult {
    if accounts.len() < 6 {
      return Err(ProtocolError::InvalidAccountsLength.into());
    }
    #[allow(clippy::ptr_offset_with_cast)]
    let (fixed_accounts, other_accounts) = array_refs![accounts, 6; ..;];

    let (
      user_accounts,
      &[ref swap_info_account, ref spl_token_program_acc, ref fee_token_account_acc],
    ) = array_refs![fixed_accounts, 3, 3];

    let user_args = UserArgs::with_parsed_args(user_accounts)?;
    let swap_info_args = SwapInfoArgs::with_parsed_args(swap_info_account, program_id)?;
    let spl_token_program = SplTokenProgram::new(spl_token_program_acc)?;

    if !user_args.source_account_owner.is_signer {
      return Err(ProtocolError::InvalidSignerAccount.into());
    }
    user_args
      .token_source_account
      .check_owner(user_args.source_account_owner.key, false)?;

    if !swap_info_args.swap_info_acc.is_writable {
      return Err(ProtocolError::ReadonlyAccount.into());
    }
    match swap_info_args.swap_info.token_account {
      COption::Some(k) => {
        if k != *user_args.token_source_account.pubkey() {
          return Err(ProtocolError::InvalidTokenAccount.into());
        }
      }
      COption::None => {
        return Err(ProtocolError::InvalidTokenAccount.into());
      }
    };

    msg!(
      "source_token_account amount: {}",
      user_args.token_source_account.balance()?,
    );

    let fee_token_account = TokenAccount::new(fee_token_account_acc)?;
    if fee_token_account.mint()? != user_args.token_destination_account.mint()? {
      return Err(ProtocolError::InvalidFeeTokenAccount.into());
    }
    if fee_token_account.owner()?.to_string() != *OWNER_KEY {
      return Err(ProtocolError::InvalidFeeTokenAccount.into());
    }

    if let Some(delegate) = fee_token_account.delegate()? {
      if delegate == *user_args.source_account_owner.key {
        return Err(ProtocolError::InvalidFeeTokenAccount.into());
      }
    }
    let from_amount_before = user_args.token_source_account.balance()?;
    let to_amount_before = user_args.token_destination_account.balance()?;

    let amount_in = swap_info_args.swap_info.token_latest_amount;
    let amount_out = data.minimum_amount_out.get();
    msg!(
      "from_amount_before: {}, to_amount_before: {}, amount_in: {}, expect_amount_out: {}, minimum_amount_out: {}",
      from_amount_before,
      to_amount_before,
      amount_in,
      data.expect_amount_out,
      data.minimum_amount_out,
    );

    match exchanger {
      ExchangerType::SplTokenSwap => Self::process_step_tokenswap(
        program_id,
        amount_in,
        amount_out,
        &user_args.token_source_account,
        &user_args.token_destination_account,
        user_args.source_account_owner,
        &spl_token_program,
        other_accounts,
      ),
      ExchangerType::StableSwap => Self::process_step_stableswap(
        program_id,
        amount_in,
        amount_out,
        &user_args.token_source_account,
        &user_args.token_destination_account,
        user_args.source_account_owner,
        &spl_token_program,
        other_accounts,
      ),
      ExchangerType::RaydiumSwap => Self::process_step_raydium(
        program_id,
        amount_in,
        amount_out,
        &user_args.token_source_account,
        &user_args.token_destination_account,
        user_args.source_account_owner,
        &spl_token_program,
        other_accounts,
      ),
      ExchangerType::RaydiumSwapSlim => Self::process_step_raydium_slim(
        program_id,
        amount_in,
        amount_out,
        &user_args.token_source_account,
        &user_args.token_destination_account,
        user_args.source_account_owner,
        &spl_token_program,
        other_accounts,
      ),
      ExchangerType::SerumDex => Self::process_step_serumdex(
        program_id,
        amount_in,
        amount_out,
        &user_args.token_source_account,
        &user_args.token_destination_account,
        user_args.source_account_owner,
        &spl_token_program,
        other_accounts,
      ),
      ExchangerType::CremaFinance => Self::process_step_crema_finance(
        program_id,
        amount_in,
        amount_out,
        &user_args.token_source_account,
        &user_args.token_destination_account,
        user_args.source_account_owner,
        &spl_token_program,
        other_accounts,
      ),
      ExchangerType::AldrinExchange => Self::process_step_aldrin_exchange(
        program_id,
        amount_in,
        amount_out,
        &user_args.token_source_account,
        &user_args.token_destination_account,
        user_args.source_account_owner,
        &spl_token_program,
        other_accounts,
      ),
      ExchangerType::CropperFinance => Self::process_step_cropper_finance(
        program_id,
        amount_in,
        amount_out,
        &user_args.token_source_account,
        &user_args.token_destination_account,
        user_args.source_account_owner,
        &spl_token_program,
        other_accounts,
      ),
    }?;

    let from_amount_after = user_args.token_source_account.balance()?;
    let to_amount_after = user_args.token_destination_account.balance()?;
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

    if fee > 0 {
      Self::token_transfer(
        spl_token_program.inner(),
        user_args.token_destination_account.inner(),
        fee_token_account.inner(),
        user_args.source_account_owner,
        fee,
      )?;
    }
    let mut swap_info = swap_info_args.swap_info;
    swap_info.token_latest_amount = to_amount_include_fee;
    swap_info.token_account = COption::None;

    SwapInfo::pack(
      swap_info,
      &mut swap_info_args.swap_info_acc.data.borrow_mut(),
    )?;
    Ok(())
  }

  pub fn process_single_step_swap_out_slim(
    program_id: &Pubkey,
    data: &SwapOutSlimInstruction,
    accounts: &[AccountInfo],
    exchanger: ExchangerType,
  ) -> ProgramResult {
    if accounts.len() < 6 {
      return Err(ProtocolError::InvalidAccountsLength.into());
    }
    #[allow(clippy::ptr_offset_with_cast)]
    let (fixed_accounts, other_accounts) = array_refs![accounts, 6; ..;];

    let (
      user_accounts,
      &[ref swap_info_account, ref spl_token_program_acc, ref fee_token_account_acc],
    ) = array_refs![fixed_accounts, 3, 3];

    let user_args = UserArgs::with_parsed_args(user_accounts)?;
    let swap_info_args = SwapInfoArgs::with_parsed_args(swap_info_account, program_id)?;
    let spl_token_program = SplTokenProgram::new(spl_token_program_acc)?;

    if !user_args.source_account_owner.is_signer {
      return Err(ProtocolError::InvalidSignerAccount.into());
    }
    user_args
      .token_source_account
      .check_owner(user_args.source_account_owner.key, false)?;

    if !swap_info_args.swap_info_acc.is_writable {
      return Err(ProtocolError::ReadonlyAccount.into());
    }
    match swap_info_args.swap_info.token_account {
      COption::Some(k) => {
        if k != *user_args.token_source_account.pubkey() {
          return Err(ProtocolError::InvalidTokenAccount.into());
        }
      }
      COption::None => {
        return Err(ProtocolError::InvalidTokenAccount.into());
      }
    };

    msg!(
      "source_token_account amount: {}",
      user_args.token_source_account.balance()?,
    );

    let fee_token_account = TokenAccount::new(fee_token_account_acc)?;
    if fee_token_account.mint()? != user_args.token_destination_account.mint()? {
      return Err(ProtocolError::InvalidFeeTokenAccount.into());
    }
    if fee_token_account.owner()?.to_string() != *OWNER_KEY {
      return Err(ProtocolError::InvalidFeeTokenAccount.into());
    }

    if let Some(delegate) = fee_token_account.delegate()? {
      if delegate == *user_args.source_account_owner.key {
        return Err(ProtocolError::InvalidFeeTokenAccount.into());
      }
    }
    let from_amount_before = user_args.token_source_account.balance()?;
    let to_amount_before = user_args.token_destination_account.balance()?;

    let amount_in = swap_info_args.swap_info.token_latest_amount;
    let amount_out = data.minimum_amount_out.get();
    msg!(
      "from_amount_before: {}, to_amount_before: {}, amount_in: {}, minimum_amount_out: {}",
      from_amount_before,
      to_amount_before,
      amount_in,
      data.minimum_amount_out,
    );

    match exchanger {
      ExchangerType::SplTokenSwap => Self::process_step_tokenswap(
        program_id,
        amount_in,
        amount_out,
        &user_args.token_source_account,
        &user_args.token_destination_account,
        user_args.source_account_owner,
        &spl_token_program,
        other_accounts,
      ),
      ExchangerType::StableSwap => Self::process_step_stableswap(
        program_id,
        amount_in,
        amount_out,
        &user_args.token_source_account,
        &user_args.token_destination_account,
        user_args.source_account_owner,
        &spl_token_program,
        other_accounts,
      ),
      ExchangerType::RaydiumSwap => Self::process_step_raydium(
        program_id,
        amount_in,
        amount_out,
        &user_args.token_source_account,
        &user_args.token_destination_account,
        user_args.source_account_owner,
        &spl_token_program,
        other_accounts,
      ),
      ExchangerType::RaydiumSwapSlim => Self::process_step_raydium_slim(
        program_id,
        amount_in,
        amount_out,
        &user_args.token_source_account,
        &user_args.token_destination_account,
        user_args.source_account_owner,
        &spl_token_program,
        other_accounts,
      ),
      ExchangerType::SerumDex => Self::process_step_serumdex(
        program_id,
        amount_in,
        amount_out,
        &user_args.token_source_account,
        &user_args.token_destination_account,
        user_args.source_account_owner,
        &spl_token_program,
        other_accounts,
      ),
      ExchangerType::CremaFinance => Self::process_step_crema_finance(
        program_id,
        amount_in,
        amount_out,
        &user_args.token_source_account,
        &user_args.token_destination_account,
        user_args.source_account_owner,
        &spl_token_program,
        other_accounts,
      ),
      ExchangerType::AldrinExchange => Self::process_step_aldrin_exchange(
        program_id,
        amount_in,
        amount_out,
        &user_args.token_source_account,
        &user_args.token_destination_account,
        user_args.source_account_owner,
        &spl_token_program,
        other_accounts,
      ),
      ExchangerType::CropperFinance => Self::process_step_cropper_finance(
        program_id,
        amount_in,
        amount_out,
        &user_args.token_source_account,
        &user_args.token_destination_account,
        user_args.source_account_owner,
        &spl_token_program,
        other_accounts,
      ),
    }?;

    let from_amount_after = user_args.token_source_account.balance()?;
    let to_amount_after = user_args.token_destination_account.balance()?;
    msg!(
      "from_amount_after: {}, to_amount_after: {}",
      from_amount_after,
      to_amount_after
    );

    let from_amount_changed = from_amount_before.checked_sub(from_amount_after).unwrap();
    let to_amount_include_fee = to_amount_after.checked_sub(to_amount_before).unwrap();
    msg!("from_amount changed: {}", from_amount_changed);
    msg!(
      "result_with_fee: {}, minimum: {}",
      to_amount_include_fee,
      data.minimum_amount_out,
    );
    if to_amount_include_fee == 0 {
      return Err(ProtocolError::DexSwapError.into());
    }

    if to_amount_include_fee < data.minimum_amount_out.get() {
      return Err(ProtocolError::ExceededSlippage.into());
    }

    let fee1 = to_amount_include_fee
      .checked_sub(data.minimum_amount_out.get())
      .map(|v| (v as u128).checked_div(4).unwrap_or(0) as u64)
      .unwrap_or(0);

    let fee2 = to_amount_include_fee.checked_div(10_000).unwrap_or(0);

    let fee = cmp::min(fee1, fee2);

    if fee > 0 {
      Self::token_transfer(
        spl_token_program.inner(),
        user_args.token_destination_account.inner(),
        fee_token_account.inner(),
        user_args.source_account_owner,
        fee,
      )?;
    }
    let mut swap_info = swap_info_args.swap_info;
    swap_info.token_latest_amount = to_amount_include_fee;
    swap_info.token_account = COption::None;

    SwapInfo::pack(
      swap_info,
      &mut swap_info_args.swap_info_acc.data.borrow_mut(),
    )?;
    Ok(())
  }

  /// Step swap in spl-token-swap
  #[allow(clippy::too_many_arguments, unused_variables)]
  fn process_step_tokenswap<'a, 'b: 'a>(
    program_id: &Pubkey,
    amount_in: u64,
    minimum_amount_out: u64,
    source_token_account: &TokenAccount<'a, 'b>,
    destination_token_account: &TokenAccount<'a, 'b>,
    source_account_authority: &'a AccountInfo<'b>,
    spl_token_program: &SplTokenProgram<'a, 'b>,
    accounts: &'a [AccountInfo<'b>],
  ) -> ProgramResult {
    msg!(
      "swap using token-swap, amount_in: {}, minimum_amount_out: {}",
      amount_in,
      minimum_amount_out,
    );

    let spl_token_swap_args = SplTokenSwapArgs::with_parsed_args(accounts)?;
    let token_swap_amount_in = Self::get_amount_in(amount_in, source_token_account.balance()?);

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
      source_account_authority.clone(),
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

    let instruction_data = spl_token_swap::instruction::Swap {
      amount_in: token_swap_amount_in,
      minimum_amount_out,
    };
    let instruction = spl_token_swap::instruction::swap(
      spl_token_swap_args.program.key,
      spl_token_program.inner().key,
      spl_token_swap_args.swap_info.inner().key,
      spl_token_swap_args.authority_acc_info.key,
      source_account_authority.key,
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
    invoke(&instruction, &swap_accounts)?;
    Ok(())
  }

  #[allow(clippy::too_many_arguments, unused_variables)]
  fn process_step_serumdex<'a, 'b: 'a>(
    program_id: &Pubkey,
    amount_in: u64,
    minimum_amount_out: u64,
    source_token_account: &TokenAccount<'a, 'b>,
    destination_token_account: &TokenAccount<'a, 'b>,
    source_account_authority: &'a AccountInfo<'b>,
    spl_token_program: &SplTokenProgram<'a, 'b>,
    accounts: &'a [AccountInfo<'b>],
  ) -> ProgramResult {
    let dex_args = SerumDexArgs::with_parsed_args(accounts)?;

    let amount_in = Self::get_amount_in(amount_in, source_token_account.balance()?);

    let side = dex_args.find_side(&source_token_account.mint()?)?;

    let (pc_wallet_account, coin_wallet_account) = match side {
      DexSide::Bid => (source_token_account, destination_token_account),
      DexSide::Ask => (destination_token_account, source_token_account),
    };

    let orderbook = serum_dex::order::OrderbookClient {
      market: serum_dex::order::MarketAccounts {
        market: dex_args.market.inner(),
        open_orders: dex_args.open_orders.inner(),
        request_queue: dex_args.request_queue_acc,
        event_queue: dex_args.event_queue_acc,
        bids: dex_args.bids_acc,
        asks: dex_args.asks_acc,
        order_payer_authority: source_token_account.inner(),
        coin_vault: dex_args.coin_vault_acc.inner(),
        pc_vault: dex_args.pc_vault_acc.inner(),
        vault_signer: dex_args.vault_signer_acc,
        coin_wallet: coin_wallet_account.inner(),
      },
      open_order_authority: source_account_authority,
      pc_wallet: pc_wallet_account.inner(),
      dex_program: dex_args.program_acc,
      token_program: spl_token_program.inner(),
      rent: dex_args.rent_sysvar_acc,
    };
    // orderbook.cancel_order(side)?;
    match side {
      DexSide::Bid => orderbook.buy(amount_in, None)?,
      DexSide::Ask => orderbook.sell(amount_in, None)?,
    }
    msg!("serum.settle");
    orderbook.settle(None)?;
    Ok(())
  }

  /// Step swap in spl-token-swap
  #[allow(clippy::too_many_arguments)]
  fn process_step_stableswap<'a, 'b: 'a>(
    _program_id: &Pubkey,
    amount_in: u64,
    minimum_amount_out: u64,
    source_token_account: &TokenAccount<'a, 'b>,
    destination_token_account: &TokenAccount<'a, 'b>,
    source_account_authority: &'a AccountInfo<'b>,
    spl_token_program: &SplTokenProgram<'a, 'b>,
    accounts: &'a [AccountInfo<'b>],
  ) -> ProgramResult {
    sol_log_compute_units();

    let swap_args = StableSwapArgs::with_parsed_args(accounts)?;
    let amount_in = Self::get_amount_in(amount_in, source_token_account.balance()?);

    msg!(
      "swap using stable-swap, amount_in: {}, minimum_amount_out: {}",
      amount_in,
      minimum_amount_out,
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
      source_account_authority.clone(),
      source_token_account.inner().clone(),
      swap_source_token_acc.inner().clone(),
      swap_destination_token_acc.inner().clone(),
      destination_token_account.inner().clone(),
      swap_args.admin_fee_acc.clone(),
      spl_token_program.inner().clone(),
      swap_args.program_acc.clone(),
    ];

    let instruction = stable_swap::instruction::swap(
      swap_args.program_acc.key,
      spl_token_program.inner().key,
      swap_args.swap_info.inner().key,
      swap_args.authority_acc.key,
      source_account_authority.key,
      source_token_account.inner().key,
      swap_source_token_acc.inner().key,
      swap_destination_token_acc.inner().key,
      destination_token_account.inner().key,
      swap_args.admin_fee_acc.key,
      amount_in,
      minimum_amount_out,
    )?;

    msg!("invoke saber-stableswap swap");

    sol_log_compute_units();
    invoke(&instruction, &swap_accounts)?;
    sol_log_compute_units();
    Ok(())
  }

  /// Step swap in spl-token-swap
  #[allow(clippy::too_many_arguments, unused_variables)]
  fn process_step_raydium<'a, 'b: 'a>(
    program_id: &Pubkey,
    amount_in: u64,
    minimum_amount_out: u64,
    source_token_account: &TokenAccount<'a, 'b>,
    destination_token_account: &TokenAccount<'a, 'b>,
    source_account_authority: &'a AccountInfo<'b>,
    spl_token_program: &SplTokenProgram<'a, 'b>,
    accounts: &'a [AccountInfo<'b>],
  ) -> ProgramResult {
    let swap_args = RaydiumSwapArgs::with_parsed_args(accounts)?;
    let amount_in = Self::get_amount_in(amount_in, source_token_account.balance()?);

    msg!(
      "swap using raydium, amount_in: {}, minimum_amount_out: {}",
      amount_in,
      minimum_amount_out,
    );

    let source_token_mint = source_token_account.mint()?;
    let destination_token_mint = destination_token_account.mint()?;

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
      source_account_authority.clone(),
    ];

    let instruction = raydium::instruction::swap(
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
      source_account_authority.key,
      amount_in,
      minimum_amount_out,
    )?;

    msg!("invoke raydium swap_base_in");
    invoke(&instruction, &swap_accounts)?;
    Ok(())
  }

  /// Step swap in spl-token-swap
  #[allow(clippy::too_many_arguments, unused_variables)]
  fn process_step_raydium_slim<'a, 'b: 'a>(
    program_id: &Pubkey,
    amount_in: u64,
    minimum_amount_out: u64,
    source_token_account: &TokenAccount<'a, 'b>,
    destination_token_account: &TokenAccount<'a, 'b>,
    source_account_authority: &'a AccountInfo<'b>,
    spl_token_program: &SplTokenProgram<'a, 'b>,
    accounts: &'a [AccountInfo<'b>],
  ) -> ProgramResult {
    let swap_args = RaydiumSwapArgs2::with_parsed_args(accounts)?;
    let amount_in = Self::get_amount_in(amount_in, source_token_account.balance()?);

    msg!("swap using raydium, amount_in: {}", amount_in,);

    let source_token_mint = source_token_account.mint()?;
    let destination_token_mint = destination_token_account.mint()?;

    let swap_accounts = vec![
      swap_args.program_id.clone(),
      spl_token_program.inner().clone(),
      swap_args.amm_info.inner().clone(),
      swap_args.authority.clone(),
      swap_args.open_orders.inner().clone(),
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
      source_account_authority.clone(),
    ];

    let instruction = raydium::instruction::swap_slim(
      swap_args.program_id.key,
      swap_args.amm_info.pubkey(),
      swap_args.authority.key,
      swap_args.open_orders.pubkey(),
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
      source_account_authority.key,
      amount_in,
      minimum_amount_out,
    )?;

    msg!("invoke raydium swap_base_in");
    invoke(&instruction, &swap_accounts)?;
    Ok(())
  }

  /// Step swap in spl-token-swap
  #[allow(clippy::too_many_arguments, unused_variables)]
  fn process_step_crema_finance<'a, 'b: 'a>(
    program_id: &Pubkey,
    amount_in: u64,
    minimum_amount_out: u64,
    source_token_account: &TokenAccount<'a, 'b>,
    destination_token_account: &TokenAccount<'a, 'b>,
    source_account_authority: &'a AccountInfo<'b>,
    spl_token_program: &SplTokenProgram<'a, 'b>,
    accounts: &'a [AccountInfo<'b>],
  ) -> ProgramResult {
    sol_log_compute_units();
    msg!("process_step crema-finance");

    let swap_args = CremaSwapV1Args::with_parsed_args(accounts)?;
    let amount_in = Self::get_amount_in(amount_in, source_token_account.balance()?);

    msg!(
      "swap using crema-finance, amount_in: {}, minimum_amount_out: {}",
      amount_in,
      minimum_amount_out,
    );

    let source_token_mint = source_token_account.mint()?;
    let destination_token_mint = destination_token_account.mint()?;

    let (pool_source_token_acc, pool_destination_token_acc) =
      swap_args.find_token_pair(&source_token_mint, &destination_token_mint)?;

    let swap_accounts = vec![
      swap_args.program_id.clone(),
      swap_args.swap_info.inner().clone(),
      swap_args.authority.clone(),
      source_account_authority.clone(),
      source_token_account.inner().clone(),
      destination_token_account.inner().clone(),
      pool_source_token_acc.inner().clone(),
      pool_destination_token_acc.inner().clone(),
      swap_args.tick_dst.clone(),
      spl_token_program.inner().clone(),
    ];

    let instruction = crema::instruction::swap_instruction(
      swap_args.program_id.key,
      swap_args.swap_info.inner().key,
      swap_args.authority.key,
      source_account_authority.key,
      source_token_account.inner().key,
      destination_token_account.inner().key,
      pool_source_token_acc.inner().key,
      pool_destination_token_acc.inner().key,
      swap_args.tick_dst.key,
      spl_token_program.inner().key,
      amount_in,
      minimum_amount_out,
    )?;

    msg!("invoke crema-finance swap");

    sol_log_compute_units();
    invoke(&instruction, &swap_accounts)?;
    sol_log_compute_units();
    Ok(())
  }

  /// Step swap in spl-token-swap
  #[allow(clippy::too_many_arguments, unused_variables)]
  fn process_step_aldrin_exchange<'a, 'b: 'a>(
    program_id: &Pubkey,
    amount_in: u64,
    minimum_amount_out: u64,
    source_token_account: &TokenAccount<'a, 'b>,
    destination_token_account: &TokenAccount<'a, 'b>,
    source_account_authority: &'a AccountInfo<'b>,
    spl_token_program: &SplTokenProgram<'a, 'b>,
    accounts: &'a [AccountInfo<'b>],
  ) -> ProgramResult {
    sol_log_compute_units();

    let swap_args = AldrinPoolArgs::with_parsed_args(accounts)?;
    let amount_in = Self::get_amount_in(amount_in, source_token_account.balance()?);

    msg!(
      "swap using aldrin-exchanger, amount_in: {}, minimum_amount_out: {}",
      amount_in,
      minimum_amount_out,
    );

    let source_token_mint = source_token_account.mint()?;
    let destination_token_mint = destination_token_account.mint()?;
    let pool_coin_mint = swap_args.pool_coin_vault.mint()?;
    let pool_pc_mint = swap_args.pool_pc_vault.mint()?;

    let side = swap_args.find_side(&source_token_mint)?;

    let (user_coin_token_acc, user_pc_token_acc) =
      if source_token_mint == pool_coin_mint && destination_token_mint == pool_pc_mint {
        (source_token_account, destination_token_account)
      } else if source_token_mint == pool_pc_mint && destination_token_mint == pool_coin_mint {
        (destination_token_account, source_token_account)
      } else {
        return Err(ProtocolError::InvalidTokenMint.into());
      };

    let swap_accounts = vec![
      swap_args.program_id.clone(),
      swap_args.pool_info.inner().clone(),
      swap_args.authority.clone(),
      swap_args.pool_mint.inner().clone(),
      swap_args.pool_coin_vault.inner().clone(),
      swap_args.pool_pc_vault.inner().clone(),
      swap_args.fee_account.clone(),
      swap_args.curve_key.clone(),
      user_coin_token_acc.inner().clone(),
      user_pc_token_acc.inner().clone(),
      source_account_authority.clone(),
      spl_token_program.inner().clone(),
    ];

    let instruction = aldrin::instruction::swap_instruction(
      swap_args.program_id.key,
      swap_args.pool_info.inner().key,
      swap_args.authority.key,
      swap_args.pool_mint.inner().key,
      swap_args.pool_coin_vault.inner().key,
      swap_args.pool_pc_vault.inner().key,
      swap_args.fee_account.key,
      swap_args.curve_key.key,
      user_coin_token_acc.inner().key,
      user_pc_token_acc.inner().key,
      source_account_authority.key,
      spl_token_program.inner().key,
      amount_in,
      minimum_amount_out,
      side,
    )?;

    msg!("invoke aldrin-exchanger swap");

    sol_log_compute_units();
    invoke(&instruction, &swap_accounts)?;
    sol_log_compute_units();
    Ok(())
  }

  /// Step swap in spl-token-swap
  #[allow(clippy::too_many_arguments, unused_variables)]
  fn process_step_cropper_finance<'a, 'b: 'a>(
    program_id: &Pubkey,
    amount_in: u64,
    minimum_amount_out: u64,
    source_token_account: &TokenAccount<'a, 'b>,
    destination_token_account: &TokenAccount<'a, 'b>,
    source_account_authority: &'a AccountInfo<'b>,
    spl_token_program: &SplTokenProgram<'a, 'b>,
    accounts: &'a [AccountInfo<'b>],
  ) -> ProgramResult {
    sol_log_compute_units();

    let swap_args = CropperArgs::with_parsed_args(accounts)?;
    let amount_in = Self::get_amount_in(amount_in, source_token_account.balance()?);

    msg!(
      "swap using cropper-finance, amount_in: {}, minimum_amount_out: {}",
      amount_in,
      minimum_amount_out,
    );
    let pool_token_a_mint = swap_args.swap_info.token_a_mint()?;
    let pool_token_b_mint = swap_args.swap_info.token_b_mint()?;
    let source_token_mint = source_token_account.mint()?;
    let destination_token_mint = destination_token_account.mint()?;

    if swap_args.fee_account.mint()? != source_token_mint {
      msg!(
        "cropper-finance.fee_account.mint is {}, expect {}",
        swap_args.fee_account.pubkey(),
        destination_token_mint
      );
    }

    let (pool_source_token_account, pool_destination_token_account) =
      if source_token_mint == pool_token_a_mint && destination_token_mint == pool_token_b_mint {
        (swap_args.token_a_account, swap_args.token_b_account)
      } else if source_token_mint == pool_token_b_mint
        && destination_token_mint == pool_token_a_mint
      {
        (swap_args.token_b_account, swap_args.token_a_account)
      } else {
        return Err(ProtocolError::InvalidTokenAccount.into());
      };

    let swap_accounts = vec![
      swap_args.program_id.clone(),
      swap_args.swap_info.inner().clone(),
      swap_args.authority.clone(),
      source_account_authority.clone(),
      swap_args.program_state.inner().clone(),
      source_token_account.inner().clone(),
      pool_source_token_account.inner().clone(),
      pool_destination_token_account.inner().clone(),
      destination_token_account.inner().clone(),
      swap_args.pool_mint.inner().clone(),
      swap_args.fee_account.inner().clone(),
      spl_token_program.inner().clone(),
    ];

    let instruction = cropper::instruction::swap_instruction(
      swap_args.program_id.key,
      spl_token_program.inner().key,
      swap_args.swap_info.inner().key,
      swap_args.authority.key,
      source_account_authority.key,
      swap_args.program_state.inner().key,
      source_token_account.inner().key,
      pool_source_token_account.inner().key,
      pool_destination_token_account.inner().key,
      destination_token_account.inner().key,
      swap_args.pool_mint.inner().key,
      swap_args.fee_account.inner().key,
      amount_in,
      minimum_amount_out,
    )?;

    msg!("invoke cropper-finance swap");

    sol_log_compute_units();
    invoke(&instruction, &swap_accounts)?;
    sol_log_compute_units();
    Ok(())
  }

  fn get_amount_in(amount_in: u64, source_token_balance: u64) -> u64 {
    if source_token_balance < amount_in {
      source_token_balance
    } else {
      amount_in
    }
  }

  // /// check token account authority
  // pub fn check_token_account_authority(
  //   token_account: &spl_token::state::Account,
  //   authority_info: &Pubkey,
  // ) -> Result<(), ProtocolError> {
  //   if !token_account
  //     .delegate
  //     .map(|d| d == *authority_info)
  //     .unwrap_or(false)
  //     || token_account.owner == *authority_info
  //   {
  //     return Err(ProtocolError::InvalidDelegate);
  //   }
  //   Ok(())
  // }

  /// Issue a spl_token `Transfer` instruction.
  pub fn token_transfer_signed<'a>(
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

  /// Issue a spl_token `Transfer` instruction.
  pub fn token_transfer<'a>(
    token_program: &AccountInfo<'a>,
    source: &AccountInfo<'a>,
    destination: &AccountInfo<'a>,
    authority: &AccountInfo<'a>,
    amount: u64,
  ) -> Result<(), ProgramError> {
    let ix = spl_token::instruction::transfer(
      token_program.key,
      source.key,
      destination.key,
      authority.key,
      &[],
      amount,
    )?;
    // invoke(&ix, &[source, destination, authority, token_program])
    invoke(
      &ix,
      &[
        source.clone(),
        destination.clone(),
        authority.clone(),
        token_program.clone(),
      ],
    )
  }
}
