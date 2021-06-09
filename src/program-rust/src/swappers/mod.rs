//! mod spl token

pub mod spl_token_swap;
pub mod token_swap;

pub use token_swap::Swapper;

pub fn linear_interpolation(value: u64, parts: u64) -> Vec<u64> {
    let mut rets = vec![0; parts as usize];
    for i in 0..parts {
        rets[i as usize] = value * (i + 1) / parts;
    }
    rets
}
