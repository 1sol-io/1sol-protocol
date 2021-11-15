use solana_program::pubkey::Pubkey;
#[cfg(feature = "production")]
use std::env;

pub const OWNER_KEY: Option<Pubkey> = {
  #[cfg(feature = "production")]
  {
    Some(Pubkey::from_str(env!("PROTOCOL_OWNER_FEE_ADDRESS")))
  }
  #[cfg(not(feature = "production"))]
  {
    None
  }
};
