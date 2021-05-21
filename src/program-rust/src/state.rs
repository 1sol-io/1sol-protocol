//! State transition types

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

/// Program states.
#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct OneSolProtocol {
    /// Initialized state.
    pub version: u8,

    /// Nonce used in program address.
    pub nonce: u8,

    /// Program ID of the tokens
    pub token_program_id: Pubkey,

}


impl OneSolProtocol {
    // /// Length serialized data
    // pub const LEN: usize = 179;

    /// Check if Pool already initialized
    pub fn is_initialized(&self) -> bool {
        self.version != UNINITIALIZED_VERSION
    }
}

mod test {
    #[cfg(test)]
    use super::*;

    #[test]
    pub fn test_protocol_pack_unpack() {
        let p = OneSolProtocol {
            version: 1,
            bump_seed: 2,
            token_program_id: Pubkey::new_unique(),
        };

        let packed = p.try_to_vec().unwrap();

        let unpacked = Pool::try_from_slice(packed.as_slice()).unwrap();

        assert_eq!(p, unpacked);
    }
}