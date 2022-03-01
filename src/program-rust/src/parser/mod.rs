pub mod aldrin;
pub mod base;
pub mod crema;
pub mod cropper;
pub mod raydium;
pub mod serum_dex;
pub mod spl_token_swap;
pub mod stable_swap;

#[macro_export]
macro_rules! declare_validated_account_wrapper {
  ($WrapperT:ident, $validate:expr $(, $a:ident : $t:ty)*) => {
      #[derive(Copy, Clone)]
      pub struct $WrapperT<'a, 'b: 'a>(&'a AccountInfo<'b>);
      impl<'a, 'b: 'a> $WrapperT<'a, 'b> {
          #[allow(unused)]
          pub fn new(account: &'a AccountInfo<'b> $(,$a: $t)*) -> ProtocolResult<Self> {
              let validate_result: ProtocolResult = $validate(account $(,$a)*);
              validate_result?;
              Ok($WrapperT(account))
          }

          #[inline(always)]
          #[allow(unused)]
          pub fn inner(self) -> &'a AccountInfo<'b> {
              self.0
          }

          #[inline(always)]
          #[allow(unused)]
          pub fn pubkey(self) -> &'b Pubkey {
            self.0.key
          }

          #[inline(always)]
          #[allow(unused)]
          pub fn check_writable(self) -> ProtocolResult<()> {
            if !self.inner().is_writable {
              return Err(ProtocolError::ReadonlyAccount)
            }
            Ok(())
          }
      }
  }
}
