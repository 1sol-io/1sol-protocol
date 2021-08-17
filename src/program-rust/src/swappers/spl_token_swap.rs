use solana_program::{
    account_info::{next_account_info, AccountInfo},
    instruction::{AccountMeta, Instruction},
    program::invoke,
    program_error::ProgramError,
    pubkey::Pubkey,
};
use crate::util::unpack_token_account;
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
        }
        buf
    }
}

pub fn process_token_swap_invoke_swap(
    accounts: &[AccountInfo],
    amount_in: u64,
    minimum_amount_out: u64,
) -> Result<(), ProgramError> {
    let instruction = Swap {
        amount_in: amount_in,
        minimum_amount_out: minimum_amount_out,
    };

    let account_iters = &mut accounts.iter();
    let token_program_info = next_account_info(account_iters)?;
    let user_transfer_authority_info = next_account_info(account_iters)?;
    let source_token_acc_info = next_account_info(account_iters)?;
    let destination_token_acc_info = next_account_info(account_iters)?;
    let swap_info = next_account_info(account_iters)?;
    let swap_authority_info = next_account_info(account_iters)?;
    let swap_temp_source_token_acc_info = next_account_info(account_iters)?;
    let swap_temp_destination_token_acc_info = next_account_info(account_iters)?;
    let pool_mint_info = next_account_info(account_iters)?;
    let pool_fee_account_info = next_account_info(account_iters)?;
    let token_swap_program_info = next_account_info(account_iters)?;
    let host_fee_account_info = next_account_info(account_iters);

    let token_program_id = *token_program_info.key;
    let source_token_acc = unpack_token_account(source_token_acc_info, &token_program_id)?;
    let swap_source_acc = unpack_token_account(swap_temp_source_token_acc_info, &token_program_id)?;

    let (pool_source_token_acc_info, pool_destination_token_acc_info) = if source_token_acc.mint == swap_source_acc.mint {
        (swap_temp_source_token_acc_info, swap_temp_destination_token_acc_info)
    } else {
        (swap_temp_destination_token_acc_info, swap_temp_source_token_acc_info)
    };

    let mut accounts = vec![
        token_swap_program_info.clone(),
        token_program_info.clone(),
        swap_info.clone(),
        swap_authority_info.clone(),
        user_transfer_authority_info.clone(),
        source_token_acc_info.clone(),
        pool_source_token_acc_info.clone(),
        pool_destination_token_acc_info.clone(),
        destination_token_acc_info.clone(),
        pool_mint_info.clone(),
        pool_fee_account_info.clone(),
    ];
    let host_fee_account_key = if host_fee_account_info.is_ok() {
        let account = host_fee_account_info?;
        accounts.push(account.clone());
        Some(account.key)
    } else {
        None
    };

    let ix = spl_token_swap_instruction(
        token_swap_program_info.key,
        token_program_info.key,
        swap_info.key,
        swap_authority_info.key,
        user_transfer_authority_info.key,
        source_token_acc_info.key,
        pool_source_token_acc_info.key,
        pool_destination_token_acc_info.key,
        destination_token_acc_info.key,
        pool_mint_info.key,
        pool_fee_account_info.key,
        host_fee_account_key,
        instruction,
    )?;
    // invoke token-swap
    invoke(&ix, &accounts[..])
}

/// Creates a 'swap' instruction.
pub fn spl_token_swap_instruction(
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
