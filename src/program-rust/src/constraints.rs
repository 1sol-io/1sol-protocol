
#[cfg(feature = "production")]
use std::env;

#[cfg(feature = "production")]
pub const OWNER_KEY: &str = env!("PROTOCOL_OWNER_FEE_ADDRESS");
#[cfg(not(feature = "production"))]
pub const OWNER_KEY: &str = "change me";
