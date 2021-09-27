//! State transition types
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_program::{
  program_error::ProgramError,
  program_pack::{IsInitialized, Pack, Sealed},
  pubkey::Pubkey,
};

/// Program states.
#[repr(C)]
#[derive(PartialEq, Debug, Clone, Copy)]
pub struct OneSolState {
  /// Initialized state.
  pub version: u8,

  /// Nonce used in program address.
  pub nonce: u8,

  /// Program ID of the tokens
  pub token_program_id: Pubkey,

  /// Token Account
  pub token: Pubkey,

  /// Mint information for token
  pub token_mint: Pubkey,
}

impl OneSolState {}

impl IsInitialized for OneSolState {
  fn is_initialized(&self) -> bool {
    self.version == 1
  }
}

impl Sealed for OneSolState {}
impl Pack for OneSolState {
  const LEN: usize = 98;

  fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
    let src = array_ref![src, 0, 98];
    let (version, nonce, token_program_id, token, token_mint) = array_refs![src, 1, 1, 32, 32, 32];
    Ok(OneSolState {
      version: version[0],
      nonce: nonce[0],
      token_program_id: Pubkey::new_from_array(*token_program_id),
      token: Pubkey::new_from_array(*token),
      token_mint: Pubkey::new_from_array(*token_mint),
    })
  }

  fn pack_into_slice(&self, output: &mut [u8]) {
    let output = array_mut_ref![output, 0, 98];
    let (version_dst, nonce_dst, token_program_id, token, token_mint) =
      mut_array_refs![output, 1, 1, 32, 32, 32];
    version_dst[0] = self.version;
    nonce_dst[0] = self.nonce;
    token_program_id.copy_from_slice(self.token_program_id.as_ref());
    token.copy_from_slice(self.token.as_ref());
    token_mint.copy_from_slice(self.token_mint.as_ref());
  }
}

#[cfg(test)]
mod test {
  use super::*;

  const TEST_VERSION: u8 = 1;
  const TEST_NONCE: u8 = 255;
  const TEST_TOKEN_PROGRAM_ID: Pubkey = Pubkey::new_from_array([1u8; 32]);
  const TEST_TOKEN: Pubkey = Pubkey::new_from_array([2u8; 32]);
  const TEST_TOKEN_MINT: Pubkey = Pubkey::new_from_array([5u8; 32]);

  #[test]
  pub fn test_onesol_state_pack() {
    let test_state = OneSolState {
      version: TEST_VERSION,
      nonce: TEST_NONCE,
      token_program_id: TEST_TOKEN_PROGRAM_ID,
      token: TEST_TOKEN,
      token_mint: TEST_TOKEN_MINT,
    };
    let mut packed = [0u8; OneSolState::LEN];
    OneSolState::pack_into_slice(&test_state, &mut packed);
    let unpacked = OneSolState::unpack(&packed).unwrap();
    assert_eq!(test_state, unpacked);

    let mut packed = vec![1u8, TEST_NONCE];
    packed.extend_from_slice(&TEST_TOKEN_PROGRAM_ID.to_bytes());
    packed.extend_from_slice(&TEST_TOKEN.to_bytes());
    packed.extend_from_slice(&TEST_TOKEN_MINT.to_bytes());
    let unpacked = OneSolState::unpack(&packed).unwrap();
    assert_eq!(test_state, unpacked);
  }
}
