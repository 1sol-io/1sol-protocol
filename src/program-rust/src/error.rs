//! Error types

use num_derive::FromPrimitive;
use solana_program::{
  decode_error::DecodeError,
  msg,
  program_error::{PrintProgramError, ProgramError},
};
use thiserror::Error;

/// OneSolResult
pub type ProtocolResult<T = ()> = Result<T, ProtocolError>;

#[macro_export]
macro_rules! check_unreachable {
  () => {{
    Err(ProtocolError::Unreachable)
  }};
}

// pub(crate) use check_unreachable;

/// Errors that may be returned by the OneSol program.
#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum ProtocolError {
  /// Unknown error.
  #[error("Unknown error")]
  Unknown,
  /// Swap instruction exceeds desired slippage limit
  #[error("Swap instruction exceeds desired slippage limit")]
  ExceededSlippage,
  /// Address of the provided swap token account is incorrect.
  #[error("Address of the provided swap token account is incorrect")]
  IncorrectSwapAccount,
  /// Invalid instruction number passed in.
  #[error("Invalid instruction")]
  InvalidInstruction,

  /// The input token is invalid for swap.
  #[error("InvalidInput")]
  InvalidInput,

  /// The provided token account has a delegate.
  #[error("Token account has a delegate")]
  InvalidDelegate,

  /// The provided token account has a close authority.
  #[error("Token account has a close authority")]
  InvalidCloseAuthority,

  /// The owner of the input isn't set to the program address generated by the program.
  #[error("Input account owner is not the program address")]
  InvalidOwner,

  /// The program address provided doesn't match the value generated by the program.
  #[error("Invalid program address generated from nonce and key")]
  InvalidProgramAddress,

  /// The deserialized the account returned something besides State::Account.
  #[error("Deserialized account is not an SPL Token account")]
  ExpectedAccount,

  /// The provided token program does not match the token program expected by the swap
  #[error("The provided token program does not match the token program expected by the swap")]
  IncorrectTokenProgramId,

  /// ConversionFailure
  #[error("Conversion to u64 failed with an overflow or underflow")]
  ConversionFailure,

  /// Given pool token amount results in zero trading tokens
  #[error("Given pool token amount results in zero trading tokens")]
  ZeroTradingTokens,
  /// Internal error
  #[error("internal error")]
  InternalError,

  /// Dex Instruction Error
  #[error("dex market instruction error")]
  DexInstructionError,

  /// Dex Invoke Error
  #[error("dex market invoke error")]
  DexInvokeError,

  /// Dex Swap Error
  #[error("dex market swap error")]
  DexSwapError,

  /// Invalid expected amount tout
  #[error("invalid expect amount out")]
  InvalidExpectAmountOut,

  /// Invalid account flags
  #[error("invalid account flags")]
  InvalidAccountFlags,

  /// Invalid account flags
  #[error("account data borrow failed")]
  BorrowAccountDataError,

  /// Invalid Authority
  #[error("invalid authority")]
  InvalidAuthority,

  /// Invalid token account
  #[error("invalid token account")]
  InvalidTokenAccount,

  /// Invalid pc mint
  #[error("invalid Pc ")]
  InvalidPcMint,
  /// Invalid coin mint
  #[error("invalid Pc mint")]
  InvalidCoinMint,

  /// invalid token mint
  #[error("invalid token mint")]
  InvalidTokenMint,

  /// invalid pool mint
  #[error("invalid pool mint")]
  InvalidPoolMint,

  /// Init OpenOrders instruction error
  #[error("init open_orders instruction error")]
  InitOpenOrdersInstructionError,

  /// invoke error
  #[error("invoke error")]
  InvokeError,

  /// Invalid Nonce
  #[error("invalid nonce")]
  InvalidNonce,

  /// Invalid token program
  #[error("invalid token program")]
  InvalidTokenProgram,

  /// Invalid signer account
  #[error("invalid signer account")]
  InvalidSignerAccount,

  /// Invalid Account data
  #[error("invalid account data")]
  InvalidAccountData,

  /// Invalid Accounts length
  #[error("invalid accounts length")]
  InvalidAccountsLength,

  /// Unreachable
  #[error("unreachable")]
  Unreachable,

  /// readable account detect
  #[error("Readonly account")]
  ReadonlyAccount,

  /// Invalid source token balance
  #[error("invalid source balance")]
  InvalidSourceBalance,

  #[error("invalid spl-token-swap account")]
  InvalidSplTokenSwapInfoAccount,

  #[error("invalid serum-dex market account")]
  InvalidSerumDexMarketAccount,

  #[error("open_orders not found")]
  OpenOrdersNotFound,

  #[error("invalid open orders account data")]
  InvalidOpenOrdersAccountData,

  #[error("invalid open orders account")]
  InvalidOpenOrdersAccount,

  #[error("invalid stable-swap account")]
  InvalidStableSwapAccount,

  #[error("invalid stable-swap account state")]
  InvalidStableSwapAccountState,

  #[error("invalid clock account")]
  InvalidClockAccount,

  #[error("invalid rent account")]
  InvalidRentAccount,

  #[error("invalid amm-info account")]
  InvalidAmmInfoAccount,

  #[error("invalid dex-market-info account")]
  InvalidDexMarketInfoAccount,

  #[error("pack data failed")]
  PackDataFailed,

  #[error("not rent exempt")]
  NotRentExempt,

  #[error("invalid owner key")]
  InvalidOwnerKey,

  #[error("invalid token account delegate")]
  InvalidTokenAccountDelegate,

  #[error("invalid raydium amm-info account")]
  InvalidRaydiumAmmInfoAccount,

  #[error("invalid serum-dex program id")]
  InvalidSerumDexProgramId,

  #[error("invalid fee token account")]
  InvalidFeeTokenAccount,

  #[error("invalid crema swap account data")]
  InvalidCremaSwapAccountData,

  #[error("overflow")]
  Overflow,
}
impl From<ProtocolError> for ProgramError {
  fn from(e: ProtocolError) -> Self {
    ProgramError::Custom(e as u32)
  }
}
impl<T> DecodeError<T> for ProtocolError {
  fn type_of() -> &'static str {
    "OneSolError"
  }
}

impl PrintProgramError for ProtocolError {
  fn print<E>(&self)
  where
    E: 'static + std::error::Error + DecodeError<E> + PrintProgramError + num_traits::FromPrimitive,
  {
    match self {
      ProtocolError::Unknown => msg!("Error: Unknown"),
      ProtocolError::ExceededSlippage => msg!("Error: ExceededSlippage"),
      ProtocolError::IncorrectSwapAccount => msg!("Error: IncorrectSwapAccount"),
      ProtocolError::InvalidDelegate => msg!("Error: InvalidDelegate"),
      ProtocolError::InvalidCloseAuthority => msg!("Error: InvalidCloseAuthority"),
      ProtocolError::InvalidInstruction => msg!("Error: InvalidInstruction"),
      ProtocolError::InvalidInput => msg!("Error: InvalidInput"),
      ProtocolError::InvalidOwner => msg!("Error: InvalidOwner"),
      ProtocolError::InvalidProgramAddress => msg!("Error: InvalidProgramAddress"),
      ProtocolError::ExpectedAccount => msg!("Error: ExpectedAccount"),
      ProtocolError::IncorrectTokenProgramId => msg!("Error: IncorrectTokenProgramId"),
      ProtocolError::ConversionFailure => msg!("Error: ConversionFailure"),
      ProtocolError::ZeroTradingTokens => msg!("Error: ZeroTradingTokens"),
      ProtocolError::InternalError => msg!("Error: InternalError"),
      ProtocolError::DexInstructionError => msg!("Error: DexInstructionError"),
      ProtocolError::DexInvokeError => msg!("Error: DexInvokeError"),
      ProtocolError::DexSwapError => msg!("Error: DexSwapError"),
      ProtocolError::InvalidExpectAmountOut => msg!("Error: InvalidExpectAmountOut"),
      ProtocolError::InvalidAccountFlags => msg!("Error: InvalidAccountFlags"),
      ProtocolError::BorrowAccountDataError => msg!("Error: BorrowAccountDataError"),
      ProtocolError::InvalidAuthority => msg!("Error: InvalidAuthority"),
      ProtocolError::InvalidTokenAccount => msg!("Error: InvalidTokenAccount"),
      ProtocolError::InitOpenOrdersInstructionError => {
        msg!("Error: InitOpenOrdersInstructionError")
      }
      ProtocolError::InvokeError => msg!("Error: InvokeError"),
      ProtocolError::InvalidNonce => msg!("Error: InvalidNonce"),
      ProtocolError::InvalidTokenMint => msg!("Error: InvalidTokenMint"),
      ProtocolError::InvalidPcMint => msg!("Error: InvalidPcMint"),
      ProtocolError::InvalidCoinMint => msg!("Error: InvalidCoinMint"),
      ProtocolError::InvalidTokenProgram => msg!("Error: InvalidTokenProgram"),
      ProtocolError::InvalidSignerAccount => msg!("Error: InvalidSignerAccount"),
      ProtocolError::InvalidAccountData => msg!("Error: InvalidAccountData"),
      ProtocolError::InvalidAccountsLength => msg!("Error: InvalidAccountsLength"),
      ProtocolError::Unreachable => msg!("Error: Unreachable"),
      ProtocolError::ReadonlyAccount => msg!("Error: ReadonlyAccount"),
      ProtocolError::InvalidSourceBalance => msg!("Error: InvalidSourceBalance"),
      ProtocolError::InvalidSplTokenSwapInfoAccount => {
        msg!("Error: InvalidSplTokenSwapInfoAccount")
      }
      ProtocolError::InvalidSerumDexMarketAccount => msg!("Error: InvalidSerumDexMarketAccount"),
      ProtocolError::OpenOrdersNotFound => msg!("Error: OpenOrdersNotFound"),
      ProtocolError::InvalidOpenOrdersAccountData => {
        msg!("Error: InvalidOpenOrdersAccountData")
      }
      ProtocolError::InvalidStableSwapAccount => {
        msg!("Error: InvalidStableSwapAccount")
      }
      ProtocolError::InvalidStableSwapAccountState => {
        msg!("Error: InvalidStableSwapAccountState")
      }
      ProtocolError::InvalidClockAccount => {
        msg!("Error: InvalidClockAccount")
      }
      ProtocolError::InvalidRentAccount => {
        msg!("Error: InvalidRentAccount")
      }
      ProtocolError::InvalidAmmInfoAccount => {
        msg!("Error: InvalidAmmInfoAccount")
      }
      ProtocolError::PackDataFailed => {
        msg!("Error: PackDataFailed")
      }
      ProtocolError::NotRentExempt => {
        msg!("Error: NotRentExempt")
      }
      ProtocolError::InvalidDexMarketInfoAccount => {
        msg!("Error: InvalidDexMarketInfoAccount")
      }
      ProtocolError::InvalidOwnerKey => {
        msg!("Error: InvalidOwnerKey")
      }
      ProtocolError::InvalidTokenAccountDelegate => {
        msg!("Error: InvalidTokenAccountDelegate")
      }
      ProtocolError::InvalidRaydiumAmmInfoAccount => {
        msg!("Error: InvalidRaydiumAmmInfoAccount")
      }
      ProtocolError::InvalidSerumDexProgramId => {
        msg!("Error: InvalidSerumDexProgramId")
      }
      ProtocolError::InvalidFeeTokenAccount => {
        msg!("Error: InvalidFeeTokenAccount")
      }
      ProtocolError::InvalidOpenOrdersAccount => {
        msg!("Error: InvalidOpenOrdersAccount")
      }
      ProtocolError::InvalidCremaSwapAccountData => {
        msg!("Error: InvalidCremaSwapAccountData")
      }
      ProtocolError::InvalidPoolMint => {
        msg!("Error: InvalidPoolMint")
      }
      ProtocolError::Overflow => {
        msg!("Error: Overflow")
      }
    }
  }
}
