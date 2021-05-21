//! TokenSwap
use solana_program::{
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    pubkey::Pubkey,
};
use std::mem::size_of;
/// Swap instruction data
#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub struct Swap {
    /// SOURCE amount to transfer, output to DESTINATION is based on the exchange rate
    pub amount_in: u64,
    /// Minimum amount of DESTINATION token to output, prevents excessive slippage
    pub minimum_amount_out: u64,
}

/// DepositAllTokenTypes instruction data
#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub struct DepositAllTokenTypes {
    /// Pool token amount to transfer. token_a and token_b amount are set by
    /// the current exchange rate and size of the pool
    pub pool_token_amount: u64,
    /// Maximum token A amount to deposit, prevents excessive slippage
    pub maximum_token_a_amount: u64,
    /// Maximum token B amount to deposit, prevents excessive slippage
    pub maximum_token_b_amount: u64,
}

/// WithdrawAllTokenTypes instruction data
#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub struct WithdrawAllTokenTypes {
    /// Amount of pool tokens to burn. User receives an output of token a
    /// and b based on the percentage of the pool tokens that are returned.
    pub pool_token_amount: u64,
    /// Minimum amount of token A to receive, prevents excessive slippage
    pub minimum_token_a_amount: u64,
    /// Minimum amount of token B to receive, prevents excessive slippage
    pub minimum_token_b_amount: u64,
}

/// Deposit one token type, exact amount in instruction data
#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub struct DepositSingleTokenTypeExactAmountIn {
    /// Token amount to deposit
    pub source_token_amount: u64,
    /// Pool token amount to receive in exchange. The amount is set by
    /// the current exchange rate and size of the pool
    pub minimum_pool_token_amount: u64,
}

/// WithdrawAllTokenTypes instruction data
#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub struct WithdrawSingleTokenTypeExactAmountOut {
    /// Amount of token A or B to receive
    pub destination_token_amount: u64,
    /// Maximum amount of pool tokens to burn. User receives an output of token A
    /// or B based on the percentage of the pool tokens that are returned.
    pub maximum_pool_token_amount: u64,
}

/// Instructions supported by the token swap program.
#[repr(C)]
#[derive(Debug, PartialEq)]
pub enum SwapInstruction {
    ///   Swap the tokens in the pool.
    ///
    ///   0. `[]` Token-swap
    ///   1. `[]` swap authority
    ///   2. `[]` user transfer authority
    ///   3. `[writable]` token_(A|B) SOURCE Account, amount is transferable by user transfer authority,
    ///   4. `[writable]` token_(A|B) Base Account to swap INTO.  Must be the SOURCE token.
    ///   5. `[writable]` token_(A|B) Base Account to swap FROM.  Must be the DESTINATION token.
    ///   6. `[writable]` token_(A|B) DESTINATION Account assigned to USER as the owner.
    ///   7. `[writable]` Pool token mint, to generate trading fees
    ///   8. `[writable]` Fee account, to receive trading fees
    ///   9. '[]` Token program id
    ///   10 `[optional, writable]` Host fee account to receive additional trading fees
    Swap(Swap),

    ///   Deposit both types of tokens into the pool.  The output is a "pool"
    ///   token representing ownership in the pool. Inputs are converted to
    ///   the current ratio.
    ///
    ///   0. `[]` Token-swap
    ///   1. `[]` swap authority
    ///   2. `[]` user transfer authority
    ///   3. `[writable]` token_a user transfer authority can transfer amount,
    ///   4. `[writable]` token_b user transfer authority can transfer amount,
    ///   5. `[writable]` token_a Base Account to deposit into.
    ///   6. `[writable]` token_b Base Account to deposit into.
    ///   7. `[writable]` Pool MINT account, swap authority is the owner.
    ///   8. `[writable]` Pool Account to deposit the generated tokens, user is the owner.
    ///   9. '[]` Token program id
    DepositAllTokenTypes(DepositAllTokenTypes),

    ///   Withdraw both types of tokens from the pool at the current ratio, given
    ///   pool tokens.  The pool tokens are burned in exchange for an equivalent
    ///   amount of token A and B.
    ///
    ///   0. `[]` Token-swap
    ///   1. `[]` swap authority
    ///   2. `[]` user transfer authority
    ///   3. `[writable]` Pool mint account, swap authority is the owner
    ///   4. `[writable]` SOURCE Pool account, amount is transferable by user transfer authority.
    ///   5. `[writable]` token_a Swap Account to withdraw FROM.
    ///   6. `[writable]` token_b Swap Account to withdraw FROM.
    ///   7. `[writable]` token_a user Account to credit.
    ///   8. `[writable]` token_b user Account to credit.
    ///   9. `[writable]` Fee account, to receive withdrawal fees
    ///   10 '[]` Token program id
    WithdrawAllTokenTypes(WithdrawAllTokenTypes),

    ///   Deposit one type of tokens into the pool.  The output is a "pool" token
    ///   representing ownership into the pool. Input token is converted as if
    ///   a swap and deposit all token types were performed.
    ///
    ///   0. `[]` Token-swap
    ///   1. `[]` swap authority
    ///   2. `[]` user transfer authority
    ///   3. `[writable]` token_(A|B) SOURCE Account, amount is transferable by user transfer authority,
    ///   4. `[writable]` token_a Swap Account, may deposit INTO.
    ///   5. `[writable]` token_b Swap Account, may deposit INTO.
    ///   6. `[writable]` Pool MINT account, swap authority is the owner.
    ///   7. `[writable]` Pool Account to deposit the generated tokens, user is the owner.
    ///   8. '[]` Token program id
    DepositSingleTokenTypeExactAmountIn(DepositSingleTokenTypeExactAmountIn),

    ///   Withdraw one token type from the pool at the current ratio given the
    ///   exact amount out expected.
    ///
    ///   0. `[]` Token-swap
    ///   1. `[]` swap authority
    ///   2. `[]` user transfer authority
    ///   3. `[writable]` Pool mint account, swap authority is the owner
    ///   4. `[writable]` SOURCE Pool account, amount is transferable by user transfer authority.
    ///   5. `[writable]` token_a Swap Account to potentially withdraw from.
    ///   6. `[writable]` token_b Swap Account to potentially withdraw from.
    ///   7. `[writable]` token_(A|B) User Account to credit
    ///   8. `[writable]` Fee account, to receive withdrawal fees
    ///   9. '[]` Token program id
    WithdrawSingleTokenTypeExactAmountOut(WithdrawSingleTokenTypeExactAmountOut),
}

impl SwapInstruction {
    /// Packs a [SwapInstruction](enum.SwapInstruction.html) into a byte buffer.
    pub fn pack(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(size_of::<Self>());
        match &*self {
            Self::Swap(Swap {
                amount_in,
                minimum_amount_out,
            }) => {
                buf.push(1);
                buf.extend_from_slice(&amount_in.to_le_bytes());
                buf.extend_from_slice(&minimum_amount_out.to_le_bytes());
            }
            Self::DepositAllTokenTypes(DepositAllTokenTypes {
                pool_token_amount,
                maximum_token_a_amount,
                maximum_token_b_amount,
            }) => {
                buf.push(2);
                buf.extend_from_slice(&pool_token_amount.to_le_bytes());
                buf.extend_from_slice(&maximum_token_a_amount.to_le_bytes());
                buf.extend_from_slice(&maximum_token_b_amount.to_le_bytes());
            }
            Self::WithdrawAllTokenTypes(WithdrawAllTokenTypes {
                pool_token_amount,
                minimum_token_a_amount,
                minimum_token_b_amount,
            }) => {
                buf.push(3);
                buf.extend_from_slice(&pool_token_amount.to_le_bytes());
                buf.extend_from_slice(&minimum_token_a_amount.to_le_bytes());
                buf.extend_from_slice(&minimum_token_b_amount.to_le_bytes());
            }
            Self::DepositSingleTokenTypeExactAmountIn(DepositSingleTokenTypeExactAmountIn {
                source_token_amount,
                minimum_pool_token_amount,
            }) => {
                buf.push(4);
                buf.extend_from_slice(&source_token_amount.to_le_bytes());
                buf.extend_from_slice(&minimum_pool_token_amount.to_le_bytes());
            }
            Self::WithdrawSingleTokenTypeExactAmountOut(
                WithdrawSingleTokenTypeExactAmountOut {
                    destination_token_amount,
                    maximum_pool_token_amount,
                },
            ) => {
                buf.push(5);
                buf.extend_from_slice(&destination_token_amount.to_le_bytes());
                buf.extend_from_slice(&maximum_pool_token_amount.to_le_bytes());
            }
        }
        buf
    }
}

/// create account for token_swap
pub fn create_account(swap_program_id: &Pubkey) -> (Pubkey, u8) {
    let swap_key = Pubkey::new_unique();
    Pubkey::find_program_address(&[&swap_key.to_bytes()[..]], swap_program_id)
}

/// Creates a 'swap' instruction.
pub fn swap(
    program_id: &Pubkey,
    token_program_id: &Pubkey,
    swap_pubkey: &Pubkey,
    authority_pubkey: &Pubkey,
    user_transfer_authority_pubkey: &Pubkey,
    source_pubkey: &Pubkey,
    swap_source_pubkey: &Pubkey,
    swap_destination_pubkey: &Pubkey,
    destination_pubkey: &Pubkey,
    pool_mint_pubkey: &Pubkey,
    pool_fee_pubkey: &Pubkey,
    host_fee_pubkey: Option<&Pubkey>,
    instruction: Swap,
) -> Result<Instruction, ProgramError> {
    let data = SwapInstruction::Swap(instruction).pack();

    let mut accounts = vec![
        AccountMeta::new_readonly(*swap_pubkey, false),
        AccountMeta::new_readonly(*authority_pubkey, false),
        AccountMeta::new_readonly(*user_transfer_authority_pubkey, true),
        AccountMeta::new(*source_pubkey, false),
        AccountMeta::new(*swap_source_pubkey, false),
        AccountMeta::new(*swap_destination_pubkey, false),
        AccountMeta::new(*destination_pubkey, false),
        AccountMeta::new(*pool_mint_pubkey, false),
        AccountMeta::new(*pool_fee_pubkey, false),
        AccountMeta::new_readonly(*token_program_id, false),
    ];
    if let Some(host_fee_pubkey) = host_fee_pubkey {
        accounts.push(AccountMeta::new(*host_fee_pubkey, false));
    }

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data,
    })
}
