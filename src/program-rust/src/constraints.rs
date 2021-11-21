#[cfg(feature = "production")]
use std::env;

#[cfg(feature = "production")]
pub const OWNER_KEY: &str = env!("PROTOCOL_OWNER_FEE_ADDRESS");
#[cfg(not(feature = "production"))]
pub const OWNER_KEY: &str = "change me";

// pub const BASE_SEED: [u8; 32] = [
//   49, 97, 50, 98, 51, 99, 52, 100, 111, 110, 101, 115, 111, 108, 95, 97, 117, 116, 104, 111, 114,
//   105, 116, 121, 119, 54, 120, 55, 121, 56, 122, 57,
// ];
