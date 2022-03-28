use crate::error::ProtocolError;
//use serum_dex::{instruction, matching::Side, state::MarketState};
use solana_program::{
  account_info::AccountInfo,
  entrypoint::ProgramResult,
  program::{invoke, invoke_signed},
  pubkey::Pubkey,
};
use std::num::NonZeroU64;

use super::{
  instruction::{self, init_open_orders, SelfTradeBehavior},
  matching::{OrderType, Side},
  state::MarketState,
};

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
pub struct MarketAccounts<'a, 'info: 'a> {
  pub market: &'a AccountInfo<'info>,
  pub open_orders: &'a AccountInfo<'info>,
  pub request_queue: &'a AccountInfo<'info>,
  pub event_queue: &'a AccountInfo<'info>,
  pub bids: &'a AccountInfo<'info>,
  pub asks: &'a AccountInfo<'info>,
  pub order_payer_authority: &'a AccountInfo<'info>,
  // Also known as the "base" currency. For a given A/B market,
  // this is the vault for the A mint.
  pub coin_vault: &'a AccountInfo<'info>,
  // Also known as the "quote" currency. For a given A/B market,
  // this is the vault for the B mint.
  pub pc_vault: &'a AccountInfo<'info>,
  // PDA owner of the DEX's token accounts for base + quote currencies.
  pub vault_signer: &'a AccountInfo<'info>,
  // User wallets.
  pub coin_wallet: &'a AccountInfo<'info>,
}

#[derive(Clone)]
pub struct OrderbookClient<'a, 'info: 'a> {
  pub market: MarketAccounts<'a, 'info>,
  pub open_order_authority: &'a AccountInfo<'info>,
  pub pc_wallet: &'a AccountInfo<'info>,
  pub dex_program: &'a AccountInfo<'info>,
  pub token_program: &'a AccountInfo<'info>,
  pub rent: &'a AccountInfo<'info>,
}

impl<'a, 'info: 'a> OrderbookClient<'a, 'info> {
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
      let market = MarketState::unpack_from_slice(&self.market.market.try_borrow_data()?)?;
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
      self.market.order_payer_authority.clone(),
      self.open_order_authority.clone(),
      self.market.coin_vault.clone(),
      self.market.pc_vault.clone(),
      self.token_program.clone(),
      self.rent.clone(),
    ];
    if let Some(account) = srm_msrm_discount.clone() {
      accounts.push(account)
    };
    let srm_msrm_discount_key = match srm_msrm_discount {
      Some(acc) => {
        accounts.push(acc.clone());
        Some(acc.key)
      }
      None => None,
    };
    accounts.push(self.dex_program.clone());

    let instruction = instruction::new_order(
      self.market.market.key,
      self.market.open_orders.key,
      self.market.request_queue.key,
      self.market.event_queue.key,
      self.market.bids.key,
      self.market.asks.key,
      self.market.order_payer_authority.key,
      self.open_order_authority.key,
      self.market.coin_vault.key,
      self.market.pc_vault.key,
      self.token_program.key,
      self.rent.key,
      srm_msrm_discount_key,
      self.dex_program.key,
      side,
      NonZeroU64::new(limit_price).unwrap(),
      NonZeroU64::new(max_coin_qty).unwrap(),
      OrderType::ImmediateOrCancel,
      client_order_id,
      SelfTradeBehavior::DecrementTake,
      limit,
      NonZeroU64::new(max_native_pc_qty).unwrap(),
    )
    .map_err(|_| ProtocolError::InvalidDelegate)?;

    invoke(&instruction, &accounts[..])?;
    Ok(())
  }

  pub fn settle(&self, referral: Option<AccountInfo<'info>>) -> ProgramResult {
    let mut accounts = vec![
      self.market.market.clone(),
      self.market.open_orders.clone(),
      self.open_order_authority.clone(),
      self.market.coin_vault.clone(),
      self.market.pc_vault.clone(),
      self.market.coin_wallet.clone(),
      self.pc_wallet.clone(),
      self.market.vault_signer.clone(),
      self.token_program.clone(),
      self.dex_program.clone(),
    ];
    let referral_key = match referral {
      Some(referral_acc) => {
        accounts.push(referral_acc.clone());
        Some(referral_acc.key)
      }
      None => None,
    };
    let instruction = instruction::settle_funds(
      self.dex_program.key,
      self.market.market.key,
      self.token_program.key,
      self.market.open_orders.key,
      self.open_order_authority.key,
      self.market.coin_vault.key,
      self.market.coin_wallet.key,
      self.market.pc_vault.key,
      self.pc_wallet.key,
      referral_key,
      self.market.vault_signer.key,
    )?;
    invoke(&instruction, &accounts[..])?;
    Ok(())
  }
}

// Returns the amount of lots for the base currency of a trade with `size`.
fn coin_lots(market: &MarketState, size: u64) -> u64 {
  size.checked_div(market.coin_lot_size).unwrap()
}

#[allow(dead_code)]
pub fn invoke_init_open_orders<'a>(
  base_seed: &[u8],
  program_id: &Pubkey,
  open_orders: &AccountInfo<'a>,
  authority: &AccountInfo<'a>,
  market: &AccountInfo<'a>,
  rent: &AccountInfo<'a>,
  nonce: u8,
) -> Result<(), ProtocolError> {
  let authority_signature_seeds = [base_seed, &[nonce]];
  let signers = &[&authority_signature_seeds[..]];

  let ix = init_open_orders(program_id, open_orders.key, authority.key, market.key, None)
    .map_err(|_| ProtocolError::InitOpenOrdersInstructionError)?;
  invoke_signed(
    &ix,
    &[
      open_orders.clone(),
      authority.clone(),
      market.clone(),
      rent.clone(),
    ],
    signers,
  )
  .map_err(|_| ProtocolError::InvokeError)
}
