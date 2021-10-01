//! mod spl token

pub mod serum_dex_order;
pub mod spl_token_swap;

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum SwapperType {
  Test,
  SplTokenSwap,
  SerumDex,
}

