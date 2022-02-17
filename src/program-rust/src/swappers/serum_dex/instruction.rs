use std::num::NonZeroU64;

use num_enum::{IntoPrimitive, TryFromPrimitive};
use solana_program::{
  instruction::{AccountMeta, Instruction},
  pubkey::Pubkey,
  sysvar::rent,
};

use crate::error::ProtocolError;

use super::matching::{OrderType, Side};

#[derive(PartialEq, Eq, Copy, Clone, Debug, TryFromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum SelfTradeBehavior {
  DecrementTake = 0,
  CancelProvide = 1,
  AbortTransaction = 2,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct NewOrderInstructionV3 {
  pub side: Side,
  pub limit_price: NonZeroU64,
  pub max_coin_qty: NonZeroU64,
  pub max_native_pc_qty_including_fees: NonZeroU64,
  pub self_trade_behavior: SelfTradeBehavior,
  pub order_type: OrderType,
  pub client_order_id: u64,
  pub limit: u16,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum MarketInstruction {
  /// 0. `[writable]` market
  /// 1. `[writable]` OpenOrders
  /// 2. `[signer]` the OpenOrders owner
  /// 3. `[writable]` coin vault
  /// 4. `[writable]` pc vault
  /// 5. `[writable]` coin wallet
  /// 6. `[writable]` pc wallet
  /// 7. `[]` vault signer
  /// 8. `[]` spl token program
  /// 9. `[writable]` (optional) referrer pc wallet
  SettleFunds,
  /// 0. `[writable]` the market
  /// 1. `[writable]` the OpenOrders account to use
  /// 2. `[writable]` the request queue
  /// 3. `[writable]` the event queue
  /// 4. `[writable]` bids
  /// 5. `[writable]` asks
  /// 6. `[writable]` the (coin or price currency) account paying for the order
  /// 7. `[signer]` owner of the OpenOrders account
  /// 8. `[writable]` coin vault
  /// 9. `[writable]` pc vault
  /// 10. `[]` spl token program
  /// 11. `[]` the rent sysvar
  /// 12. `[]` (optional) the (M)SRM account used for fee discounts
  NewOrderV3(NewOrderInstructionV3),
  /// 0. `[writable]` OpenOrders
  /// 1. `[signer]` the OpenOrders owner
  /// 2. `[writable]` the destination account to send rent exemption SOL to
  /// 3. `[]` market
  CloseOpenOrders,
  /// 0. `[writable]` OpenOrders
  /// 1. `[signer]` the OpenOrders owner
  /// 2. `[]` market
  /// 3. `[]`
  /// 4. `[signer]` open orders market authority (optional).
  InitOpenOrders,
}

impl MarketInstruction {
  pub fn pack(&self) -> Vec<u8> {
    let mut buf = vec![0u8];

    match &*self {
      Self::SettleFunds => {
        buf.extend_from_slice(&5u32.to_le_bytes());
      }
      Self::NewOrderV3(NewOrderInstructionV3 {
        side,
        limit_price,
        max_coin_qty,
        max_native_pc_qty_including_fees,
        self_trade_behavior,
        order_type,
        client_order_id,
        limit,
      }) => {
        buf.extend_from_slice(&10u32.to_le_bytes());
        let side_value: u8 = Side::into(*side);
        buf.extend_from_slice(&(side_value as u32).to_le_bytes());
        buf.extend_from_slice(&limit_price.get().to_le_bytes());
        buf.extend_from_slice(&max_coin_qty.get().to_le_bytes());
        buf.extend_from_slice(&max_native_pc_qty_including_fees.get().to_le_bytes());
        let self_trade_behavior_value: u8 = SelfTradeBehavior::into(*self_trade_behavior);
        buf.extend_from_slice(&(self_trade_behavior_value as u32).to_le_bytes());
        let order_type_value: u8 = OrderType::into(*order_type);
        buf.extend_from_slice(&(order_type_value as u32).to_le_bytes());
        buf.extend_from_slice(&client_order_id.to_le_bytes());
        buf.extend_from_slice(&limit.to_le_bytes());
      }
      Self::CloseOpenOrders => {
        buf.extend_from_slice(&14u32.to_le_bytes());
      }
      Self::InitOpenOrders => {
        buf.extend_from_slice(&15u32.to_le_bytes());
      }
    }
    buf
  }
}

#[allow(clippy::too_many_arguments)]
pub fn new_order(
  market: &Pubkey,
  open_orders_account: &Pubkey,
  request_queue: &Pubkey,
  event_queue: &Pubkey,
  market_bids: &Pubkey,
  market_asks: &Pubkey,
  order_payer: &Pubkey,
  open_orders_account_owner: &Pubkey,
  coin_vault: &Pubkey,
  pc_vault: &Pubkey,
  spl_token_program_id: &Pubkey,
  rent_sysvar_id: &Pubkey,
  srm_account_referral: Option<&Pubkey>,
  program_id: &Pubkey,
  side: Side,
  limit_price: NonZeroU64,
  max_coin_qty: NonZeroU64,
  order_type: OrderType,
  client_order_id: u64,
  self_trade_behavior: SelfTradeBehavior,
  limit: u16,
  max_native_pc_qty_including_fees: NonZeroU64,
) -> Result<Instruction, ProtocolError> {
  let data = MarketInstruction::NewOrderV3(NewOrderInstructionV3 {
    side,
    limit_price,
    max_coin_qty,
    order_type,
    client_order_id,
    self_trade_behavior,
    limit,
    max_native_pc_qty_including_fees,
  })
  .pack();
  let mut accounts = vec![
    AccountMeta::new(*market, false),
    AccountMeta::new(*open_orders_account, false),
    AccountMeta::new(*request_queue, false),
    AccountMeta::new(*event_queue, false),
    AccountMeta::new(*market_bids, false),
    AccountMeta::new(*market_asks, false),
    AccountMeta::new(*order_payer, false),
    AccountMeta::new_readonly(*open_orders_account_owner, true),
    AccountMeta::new(*coin_vault, false),
    AccountMeta::new(*pc_vault, false),
    AccountMeta::new_readonly(*spl_token_program_id, false),
    AccountMeta::new_readonly(*rent_sysvar_id, false),
  ];
  if let Some(key) = srm_account_referral {
    accounts.push(AccountMeta::new_readonly(*key, false))
  }
  Ok(Instruction {
    program_id: *program_id,
    data,
    accounts,
  })
}

#[allow(clippy::too_many_arguments)]
pub fn settle_funds(
  program_id: &Pubkey,
  market: &Pubkey,
  spl_token_program_id: &Pubkey,
  open_orders_account: &Pubkey,
  open_orders_account_owner: &Pubkey,
  coin_vault: &Pubkey,
  coin_wallet: &Pubkey,
  pc_vault: &Pubkey,
  pc_wallet: &Pubkey,
  referrer_pc_wallet: Option<&Pubkey>,
  vault_signer: &Pubkey,
) -> Result<Instruction, ProtocolError> {
  let data = MarketInstruction::SettleFunds.pack();
  let mut accounts: Vec<AccountMeta> = vec![
    AccountMeta::new(*market, false),
    AccountMeta::new(*open_orders_account, false),
    AccountMeta::new_readonly(*open_orders_account_owner, true),
    AccountMeta::new(*coin_vault, false),
    AccountMeta::new(*pc_vault, false),
    AccountMeta::new(*coin_wallet, false),
    AccountMeta::new(*pc_wallet, false),
    AccountMeta::new_readonly(*vault_signer, false),
    AccountMeta::new_readonly(*spl_token_program_id, false),
  ];
  if let Some(key) = referrer_pc_wallet {
    accounts.push(AccountMeta::new(*key, false))
  }
  Ok(Instruction {
    program_id: *program_id,
    data,
    accounts,
  })
}

#[allow(dead_code)]
pub fn close_open_orders(
  program_id: &Pubkey,
  open_orders: &Pubkey,
  owner: &Pubkey,
  destination: &Pubkey,
  market: &Pubkey,
) -> Result<Instruction, ProtocolError> {
  let data = MarketInstruction::CloseOpenOrders.pack();
  let accounts: Vec<AccountMeta> = vec![
    AccountMeta::new(*open_orders, false),
    AccountMeta::new_readonly(*owner, true),
    AccountMeta::new(*destination, false),
    AccountMeta::new_readonly(*market, false),
  ];
  Ok(Instruction {
    program_id: *program_id,
    data,
    accounts,
  })
}

#[allow(dead_code)]
pub fn init_open_orders(
  program_id: &Pubkey,
  open_orders: &Pubkey,
  owner: &Pubkey,
  market: &Pubkey,
  market_authority: Option<&Pubkey>,
) -> Result<Instruction, ProtocolError> {
  let data = MarketInstruction::InitOpenOrders.pack();
  let mut accounts: Vec<AccountMeta> = vec![
    AccountMeta::new(*open_orders, false),
    AccountMeta::new_readonly(*owner, true),
    AccountMeta::new_readonly(*market, false),
    AccountMeta::new_readonly(rent::ID, false),
  ];
  if let Some(market_authority) = market_authority {
    accounts.push(AccountMeta::new_readonly(*market_authority, true));
  }
  Ok(Instruction {
    program_id: *program_id,
    data,
    accounts,
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  pub fn test_pack_market_instruction_new_order() {
    let mi = MarketInstruction::NewOrderV3(NewOrderInstructionV3 {
      side: Side::Ask,
      limit_price: NonZeroU64::new(100).unwrap(),
      max_coin_qty: NonZeroU64::new(20).unwrap(),
      max_native_pc_qty_including_fees: NonZeroU64::new(109).unwrap(),
      self_trade_behavior: SelfTradeBehavior::AbortTransaction,
      order_type: OrderType::PostOnly,
      client_order_id: 33,
      limit: 65535,
    });

    let mi2 = serum_dex::instruction::MarketInstruction::NewOrderV3(
      serum_dex::instruction::NewOrderInstructionV3 {
        side: serum_dex::matching::Side::Ask,
        limit_price: NonZeroU64::new(100).unwrap(),
        max_coin_qty: NonZeroU64::new(20).unwrap(),
        max_native_pc_qty_including_fees: NonZeroU64::new(109).unwrap(),
        self_trade_behavior: serum_dex::instruction::SelfTradeBehavior::AbortTransaction,
        order_type: serum_dex::matching::OrderType::PostOnly,
        client_order_id: 33,
        limit: 65535,
      },
    );

    assert!(mi.pack() == mi2.pack());

    let mi = MarketInstruction::NewOrderV3(NewOrderInstructionV3 {
      side: Side::Bid,
      limit_price: NonZeroU64::new(100).unwrap(),
      max_coin_qty: NonZeroU64::new(20).unwrap(),
      max_native_pc_qty_including_fees: NonZeroU64::new(109).unwrap(),
      self_trade_behavior: SelfTradeBehavior::DecrementTake,
      order_type: OrderType::ImmediateOrCancel,
      client_order_id: 33,
      limit: 65535,
    });

    let mi2 = serum_dex::instruction::MarketInstruction::NewOrderV3(
      serum_dex::instruction::NewOrderInstructionV3 {
        side: serum_dex::matching::Side::Bid,
        limit_price: NonZeroU64::new(100).unwrap(),
        max_coin_qty: NonZeroU64::new(20).unwrap(),
        max_native_pc_qty_including_fees: NonZeroU64::new(109).unwrap(),
        self_trade_behavior: serum_dex::instruction::SelfTradeBehavior::DecrementTake,
        order_type: serum_dex::matching::OrderType::ImmediateOrCancel,
        client_order_id: 33,
        limit: 65535,
      },
    );

    assert!(mi.pack() == mi2.pack());
  }
}
