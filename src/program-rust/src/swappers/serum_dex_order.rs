use crate::error::OneSolError;
use arrayref::array_refs;
use bytemuck::{cast_slice, from_bytes};
use serum_dex::{matching::Side, state::MarketState};
use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, program::invoke};
use std::num::NonZeroU64;

// An exchange rate for swapping *from* one token *to* another.
#[derive(Clone, Debug, PartialEq, Eq, Copy)]
pub struct ExchangeRate {
  // The amount of *to* tokens one should receive for a single *from token.
  // This number must be in native *to* units with the same amount of decimals
  // as the *to* mint.
  pub rate: u64,
  // Number of decimals of the *from* token's mint.
  pub from_decimals: u8,
  // Number of decimals of the *to* token's mint.
  // For a direct swap, this should be zero.
  pub quote_decimals: u8,
  // True if *all* of the *from* currency sold should be used when calculating
  // the executed exchange rate.
  //
  // To perform a transitive swap, one sells on one market and buys on
  // another, where both markets are quoted in the same currency. Now suppose
  // one swaps A for B across A/USDC and B/USDC. Further suppose the first
  // leg swaps the entire *from* amount A for USDC, and then only half of
  // the USDC is used to swap for B on the second leg. How should we calculate
  // the exchange rate?
  //
  // If strict is true, then the exchange rate will be calculated as a direct
  // function of the A tokens lost and B tokens gained, ignoring the surplus
  // in USDC received. If strict is false, an effective exchange rate will be
  // used. I.e. the surplus in USDC will be marked at the exchange rate from
  // the second leg of the swap and that amount will be added to the
  // *to* mint received before calculating the swap's exchange rate.
  //
  // Transitive swaps only. For direct swaps, this field is ignored.
  pub strict: bool,
}

// Market accounts are the accounts used to place orders against the dex minus
// common accounts, i.e., program ids, sysvars, and the `pc_wallet`.
#[derive(Clone)]
pub struct MarketAccounts<'info> {
  pub market: AccountInfo<'info>,
  pub open_orders: AccountInfo<'info>,
  pub request_queue: AccountInfo<'info>,
  pub event_queue: AccountInfo<'info>,
  pub bids: AccountInfo<'info>,
  pub asks: AccountInfo<'info>,
  // The `spl_token::Account` that funds will be taken from, i.e., transferred
  // from the user into the market's vault.
  //
  // For bids, this is the base currency. For asks, the quote.
  pub order_payer_token_account: AccountInfo<'info>,
  // Also known as the "base" currency. For a given A/B market,
  // this is the vault for the A mint.
  pub coin_vault: AccountInfo<'info>,
  // Also known as the "quote" currency. For a given A/B market,
  // this is the vault for the B mint.
  pub pc_vault: AccountInfo<'info>,
  // PDA owner of the DEX's token accounts for base + quote currencies.
  pub vault_signer: AccountInfo<'info>,
  // User wallets.
  pub coin_wallet: AccountInfo<'info>,
}

#[derive(Clone)]
pub struct OrderbookClient<'info> {
  pub market: MarketAccounts<'info>,
  pub authority: AccountInfo<'info>,
  pub pc_wallet: AccountInfo<'info>,
  pub dex_program: AccountInfo<'info>,
  pub token_program: AccountInfo<'info>,
  pub rent: AccountInfo<'info>,
}

impl<'info> OrderbookClient<'info> {
  // Executes the sell order portion of the swap, purchasing as much of the
  // quote currency as possible for the given `base_amount`.
  //
  // `base_amount` is the "native" amount of the base currency, i.e., token
  // amount including decimals.
  pub fn sell(
    &self,
    base_amount: u64,
    srm_msrm_discount: Option<AccountInfo<'info>>,
  ) -> ProgramResult {
    let limit_price = 1;
    let max_coin_qty = {
      // The loaded market must be dropped before CPI.
      let market = MarketState::load(&self.market.market, self.dex_program.key)?;
      coin_lots(&market, base_amount)
    };
    let max_native_pc_qty = u64::MAX;
    self.order_cpi(
      limit_price,
      max_coin_qty,
      max_native_pc_qty,
      Side::Ask,
      srm_msrm_discount,
    )
  }

  // Executes the buy order portion of the swap, purchasing as much of the
  // base currency as possible, for the given `quote_amount`.
  //
  // `quote_amount` is the "native" amount of the quote currency, i.e., token
  // amount including decimals.
  pub fn buy(
    &self,
    quote_amount: u64,
    srm_msrm_discount: Option<AccountInfo<'info>>,
  ) -> ProgramResult {
    let limit_price = u64::MAX;
    let max_coin_qty = u64::MAX;
    let max_native_pc_qty = quote_amount;
    self.order_cpi(
      limit_price,
      max_coin_qty,
      max_native_pc_qty,
      Side::Bid,
      srm_msrm_discount,
    )
  }

  // Executes a new order on the serum dex via CPI.
  //
  // * `limit_price` - the limit order price in lot units.
  // * `max_coin_qty`- the max number of the base currency lot units.
  // * `max_native_pc_qty` - the max number of quote currency in native token
  //                         units (includes decimals).
  // * `side` - bid or ask, i.e. the type of order.
  // * `referral` - referral account, earning a fee.
  fn order_cpi(
    &self,
    limit_price: u64,
    max_coin_qty: u64,
    max_native_pc_qty: u64,
    side: Side,
    srm_msrm_discount: Option<AccountInfo<'info>>,
  ) -> ProgramResult {
    // Client order id is only used for cancels. Not used here so hardcode.
    let client_order_id = 0;
    // Limit is the dex's custom compute budge parameter, setting an upper
    // bound on the number of matching cycles the program can perform
    // before giving up and posting the remaining unmatched order.
    let limit = 65535;

    // let srm_msrm_discount_key = match srm_msrm_discount {
    //   Some(srm_msrm_discount) => Some(srm_msrm_discount.key),
    //   None => None,
    // };
    // let mut ctx = CpiContext::new(self.dex_program.clone(), self.clone().into());
    // if let Some(srm_msrm_discount) = srm_msrm_discount {
    //     ctx = ctx.with_remaining_accounts(vec![srm_msrm_discount]);
    // }
    let mut accounts = vec![
      self.market.market.clone(),
      self.market.open_orders.clone(),
      self.market.request_queue.clone(),
      self.market.event_queue.clone(),
      self.market.bids.clone(),
      self.market.asks.clone(),
      self.market.order_payer_token_account.clone(),
      self.authority.clone(),
      self.market.coin_vault.clone(),
      self.market.pc_vault.clone(),
      self.token_program.clone(),
      self.rent.clone(),
    ];
    match srm_msrm_discount.clone() {
      Some(account) => accounts.push(account),
      None => {}
    };
    let srm_msrm_discount_key = match srm_msrm_discount {
      Some(acc) => {
        accounts.push(acc.clone());
        Some(acc.key)
      }
      None => None,
    };
    let instruction = serum_dex::instruction::new_order(
      self.market.market.key,
      self.market.open_orders.key,
      self.market.request_queue.key,
      self.market.event_queue.key,
      self.market.bids.key,
      self.market.asks.key,
      self.market.order_payer_token_account.key,
      self.authority.key,
      self.market.coin_vault.key,
      self.market.pc_vault.key,
      self.token_program.key,
      self.rent.key,
      srm_msrm_discount_key,
      self.dex_program.key,
      side,
      NonZeroU64::new(limit_price).unwrap(),
      NonZeroU64::new(max_coin_qty).unwrap(),
      serum_dex::matching::OrderType::ImmediateOrCancel,
      client_order_id,
      serum_dex::instruction::SelfTradeBehavior::DecrementTake,
      limit,
      NonZeroU64::new(max_native_pc_qty).unwrap(),
    )
    .map_err(|_| OneSolError::InvalidDelegate)?;

    invoke(&instruction, &accounts[..])
  }

  pub fn settle(&self, referral: Option<AccountInfo<'info>>) -> ProgramResult {
    let mut accounts = vec![
      self.market.market.clone(),
      self.market.open_orders.clone(),
      self.authority.clone(),
      self.market.coin_vault.clone(),
      self.market.pc_vault.clone(),
      self.market.coin_wallet.clone(),
      self.pc_wallet.clone(),
      self.market.vault_signer.clone(),
      self.token_program.clone(),
    ];
    let referral_key = match referral {
      Some(referral_acc) => {
        accounts.push(referral_acc.clone());
        Some(referral_acc.key)
      }
      None => None,
    };
    let instruction = serum_dex::instruction::settle_funds(
      self.dex_program.key,
      self.market.market.key,
      self.token_program.key,
      self.market.open_orders.key,
      self.authority.key,
      self.market.coin_vault.key,
      self.market.coin_wallet.key,
      self.market.pc_vault.key,
      self.pc_wallet.key,
      referral_key,
      self.market.vault_signer.key,
    )?;
    invoke(&instruction, &accounts[..])
  }
}

// Returns the amount of lots for the base currency of a trade with `size`.
fn coin_lots(market: &MarketState, size: u64) -> u64 {
  size.checked_div(market.coin_lot_size).unwrap()
}

// pub fn invoke_new_order(
//   accounts: &[AccountInfo],
//   program_id: &Pubkey,
//   side: Side,
//   limit_price: NonZeroU64,
//   max_coin_qty: NonZeroU64,
//   max_native_pc_qty_including_fees: NonZeroU64,
// ) -> ProgramResult {
//   let account_iters = &mut accounts.iter();
//   let market_account = next_account_info(account_iters)?;
//   let open_orders_account = next_account_info(account_iters)?;
//   let request_queue = next_account_info(account_iters)?;
//   let event_queue = next_account_info(account_iters)?;
//   let market_bids = next_account_info(account_iters)?;
//   let market_asks = next_account_info(account_iters)?;
//   let order_payer = next_account_info(account_iters)?;
//   let open_orders_account_owner = next_account_info(account_iters)?;
//   let coin_vault = next_account_info(account_iters)?;
//   let pc_vault = next_account_info(account_iters)?;
//   let spl_token_program_id = next_account_info(account_iters)?;
//   let rend_sysvar_id = next_account_info(account_iters)?;

//   let account_infos = vec![
//     market_account.clone(),
//     open_orders_account.clone(),
//     request_queue.clone(),
//     event_queue.clone(),
//     market_bids.clone(),
//     market_asks.clone(),
//     order_payer.clone(),
//     open_orders_account_owner.clone(),
//     coin_vault.clone(),
//     pc_vault.clone(),
//     spl_token_program_id.clone(),
//     rend_sysvar_id.clone(),
//   ];

//   let tx = new_order_instruction(
//     market_account.key,
//     open_orders_account.key,
//     request_queue.key,
//     event_queue.key,
//     market_bids.key,
//     market_asks.key,
//     order_payer.key,
//     open_orders_account_owner.key,
//     coin_vault.key,
//     pc_vault.key,
//     spl_token_program_id.key,
//     rend_sysvar_id.key,
//     None,
//     program_id,
//     side,
//     limit_price,
//     max_coin_qty,
//     OrderType::ImmediateOrCancel,
//     client_order_id,
//     self_trade_behavior,
//     limit,
//     max_native_pc_qty_including_fees,
//   )?;
//   invoke(&tx, &account_infos[..])
// }

// pub fn invoke_settle_funds<'a>(
//   market_account: AccountInfo<'a>,
//   spl_token_program_account: AccountInfo<'a>,
//   open_orders_account: AccountInfo<'a>,
//   open_orders_account_owner: AccountInfo<'a>,
//   coin_vault_account: AccountInfo<'a>,
//   coin_wallet_account: AccountInfo<'a>,
//   pc_vault_account: AccountInfo<'a>,
//   pc_wallet_account: AccountInfo<'a>,
//   vault_signer_account: AccountInfo<'a>,
//   program_id: &Pubkey,
// ) -> ProgramResult {
//   msg!("[SerumDex] settle funds tx");
//   let tx = serum_dex_settle_funds(
//     program_id,
//     market_account.key,
//     spl_token_program_account.key,
//     open_orders_account.key,
//     open_orders_account_owner.key,
//     coin_vault_account.key,
//     coin_wallet_account.key,
//     pc_vault_account.key,
//     pc_wallet_account.key,
//     Some(pc_wallet_account.key),
//     vault_signer_account.key,
//   )?;
//   msg!("[SerumDex] settle funds accounts");
//   let account_infos = vec![
//     market_account.clone(),
//     open_orders_account.clone(),
//     open_orders_account_owner.clone(),
//     coin_vault_account.clone(),
//     pc_vault_account.clone(),
//     coin_wallet_account.clone(),
//     pc_wallet_account.clone(),
//     vault_signer_account.clone(),
//     spl_token_program_account.clone(),
//     pc_wallet_account.clone(),
//   ];
//   msg!("[SerumDex] settle fund invoke");
//   invoke(&tx, &account_infos[..])
// }

// pub fn invoke_cancel_order(
//   accounts: &[AccountInfo],
//   program_id: &Pubkey,
//   client_order_id: u64,
// ) -> ProgramResult {
//   msg!("[SerumDex] cancel order");
//   let account_iters = &mut accounts.iter();
//   let market_acc = next_account_info(account_iters)?;
//   let market_bids_acc = next_account_info(account_iters)?;
//   let market_asks_acc = next_account_info(account_iters)?;
//   let open_orders_acc = next_account_info(account_iters)?;
//   let open_orders_acc_owner = next_account_info(account_iters)?;
//   let event_queue_acc = next_account_info(account_iters)?;

//   let tx = serum_dex_cancel_order(
//     program_id,
//     market_acc.key,
//     market_bids_acc.key,
//     market_asks_acc.key,
//     open_orders_acc.key,
//     open_orders_acc_owner.key,
//     event_queue_acc.key,
//     client_order_id,
//   )?;

//   let account_infos = vec![
//     market_acc.clone(),
//     market_bids_acc.clone(),
//     market_asks_acc.clone(),
//     open_orders_acc.clone(),
//     open_orders_acc_owner.clone(),
//     event_queue_acc.clone(),
//   ];

//   invoke(&tx, &account_infos[..])
// }

#[allow(dead_code)]
pub const ACCOUNT_HEAD_PADDING: &[u8; 5] = b"serum";
#[allow(dead_code)]
pub const ACCOUNT_TAIL_PADDING: &[u8; 7] = b"padding";

#[allow(dead_code)]
pub fn load_market_state(market_acc: &AccountInfo) -> Result<MarketState, OneSolError> {
  let account_data = market_acc.data.borrow();
  if account_data.len() < 12 {
    return Err(OneSolError::InvalidInput);
  }
  let (head, data, tail) = array_refs![&account_data, 5; ..; 7];
  if head != ACCOUNT_HEAD_PADDING {
    return Err(OneSolError::InvalidInput);
  }
  if tail != ACCOUNT_TAIL_PADDING {
    return Err(OneSolError::InvalidInput.into());
  }
  let market_state: &MarketState = from_bytes(cast_slice(data));

  Ok(*market_state)
}

// fn check_account_padding(data: &[u8]) -> Result<&[[u8; 8]], OneSolError> {
//     if data.len() < 12 {
//         return Err(OneSolError::InvalidInput);
//     }
//     let (head, data, tail) = array_refs![data, 5; ..; 7];
//     if head != ACCOUNT_HEAD_PADDING {
//         return Err(OneSolError::InvalidInput);
//     }
//     if tail != ACCOUNT_TAIL_PADDING {
//         return Err(OneSolError::InvalidInput.into());
//     }
//     Ok(try_cast_slice(data).map_err(|_| OneSolError::InvalidInput)?)
// }
