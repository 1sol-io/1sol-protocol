import {
    Account,
    Connection,
    Keypair,
    PublicKey,
    SystemProgram,
    Transaction,
    TransactionInstruction,
} from '@solana/web3.js';
import {Token, TOKEN_PROGRAM_ID} from '@solana/spl-token';
import {TokenSwap, TokenSwapLayout} from '@solana/spl-token-swap';
import {
  DexInstructions,
  Market,
  MARKET_STATE_LAYOUT_V2,
  OpenOrders
} from '@project-serum/serum';

import {
  OneSolProtocol,
  TokenSwapInfo,
  loadAccount,
  realSendAndConfirmTransaction,
  SerumDexMarketInfo,
  Numberu64
} from '../src/onesol-protocol';
import {newAccountWithLamports} from './util/new-account-with-lamports';
import {envConfig} from './util/url';
import {sleep} from './util/sleep';
import { BN } from 'bn.js';

const tokenSwapProgramPubKey: PublicKey = new PublicKey(
  envConfig.splTokenSwapProgramId,
);
const serumDexProgramPubKey: PublicKey = new PublicKey(
  envConfig.serumDexProgramId,
);
const onesolProtocolProgramId: PublicKey = new PublicKey(
  envConfig.onesolProtocolProgramId,
);

let basePayer: Keypair;
// // owner of the user accounts
let baseOwner: Keypair;

const onesolOwner = Keypair.generate();

let tokenSwapAccount: Keypair;
let onesolProtocolAccount: Keypair;

// Tokens swapped
let tokenMintPool: Token;
let tokenMintA: Token;
let tokenMintB: Token;

let swapTokenAAccount: PublicKey;
let swapTokenBAccount: PublicKey;

let serumTokenAAccount: PublicKey;
let serumTokenBAccount: PublicKey;

let serumMarket: Keypair;
let serumDefaultMarketOpenOrder: Keypair;

// Hard-coded fee address, for testing production mode
const SWAP_PROGRAM_OWNER_FEE_ADDRESS =
  process.env.SWAP_PROGRAM_OWNER_FEE_ADDRESS;

// Pool fees
const TRADING_FEE_NUMERATOR = 25;
const TRADING_FEE_DENOMINATOR = 10000;
const OWNER_TRADING_FEE_NUMERATOR = 5;
const OWNER_TRADING_FEE_DENOMINATOR = 10000;
const OWNER_WITHDRAW_FEE_NUMERATOR = SWAP_PROGRAM_OWNER_FEE_ADDRESS ? 0 : 1;
const OWNER_WITHDRAW_FEE_DENOMINATOR = SWAP_PROGRAM_OWNER_FEE_ADDRESS ? 0 : 6;
const HOST_FEE_NUMERATOR = 20;
const HOST_FEE_DENOMINATOR = 100;

// curve type used to calculate swaps and deposits
const CURVE_TYPE = 0;

// Initial amount in each swap token
let currentSwapTokenA = 1000000;
let currentSwapTokenB = 1000000;
let currentFeeAmount = 0;

// Swap instruction constants
// Because there is no withdraw fee in the production version, these numbers
// need to get slightly tweaked in the two cases.
const SWAP_AMOUNT_IN = 100000;
// const SWAP_AMOUNT_OUT = SWAP_PROGRAM_OWNER_FEE_ADDRESS ? 90661 : 90674;
const SWAP_AMOUNT_OUT = SWAP_PROGRAM_OWNER_FEE_ADDRESS ? 90600: 90600;
const SWAP_FEE = SWAP_PROGRAM_OWNER_FEE_ADDRESS ? 22273 : 22276;
const HOST_SWAP_FEE = SWAP_PROGRAM_OWNER_FEE_ADDRESS
  ? Math.floor((SWAP_FEE * HOST_FEE_NUMERATOR) / HOST_FEE_DENOMINATOR)
  : 0;
const OWNER_SWAP_FEE = SWAP_FEE - HOST_SWAP_FEE;

// Pool token amount minted on init
const DEFAULT_POOL_TOKEN_AMOUNT = 1000000000;
// Pool token amount to withdraw / deposit
const POOL_TOKEN_AMOUNT = 10000000;

function assert(condition: boolean, message?: string) {
    if (!condition) {
      console.log(Error().stack + ':token-test.js');
      throw message || 'Assertion failed';
    }
  }

let connection: Connection;
async function getConnection(): Promise<Connection> {
  if (connection) return connection;

  connection = new Connection(envConfig.url, 'recent');
  const version = await connection.getVersion();

  console.log('Connection to cluster established:', envConfig.url, version);
  return connection;
}

export async function prepareAccounts(): Promise<void> {
  const connection = await getConnection();
  basePayer = await newAccountWithLamports(connection, 1000000000);
  baseOwner = await newAccountWithLamports(connection, 1000000000);

  console.log('Creating TokenMintA');
  tokenMintA = await Token.createMint(
    connection,
    new Account(basePayer.secretKey),
    baseOwner.publicKey,
    null,
    2,
    TOKEN_PROGRAM_ID,
  );
  console.log('TokenMintA: ' + tokenMintA.publicKey.toString())

  console.log('Creating TokenMintB');
  tokenMintB = await Token.createMint(
    connection,
    new Account(basePayer.secretKey),
    baseOwner.publicKey,
    null,
    2,
    TOKEN_PROGRAM_ID,
  );
  console.log('TokenMintB: ' + tokenMintB.publicKey.toString())

}

export async function prepareTokenSwap(): Promise<void> {
  const connection = await getConnection(); 

  tokenSwapAccount = Keypair.generate();
  // authority of the token and accounts
  let [authority, nonce] = await PublicKey.findProgramAddress(
    [tokenSwapAccount.publicKey.toBuffer()],
    tokenSwapProgramPubKey,
  );

  console.log('[TokenSwap] Creating TokenMintPool');
  tokenMintPool = await Token.createMint(
    connection,
    new Account(basePayer.secretKey),
    authority,
    null,
    2,
    TOKEN_PROGRAM_ID,
  );

  console.log('[TokenSwap] Creating TokenPool account');
  const tokenAccountPool = await tokenMintPool.createAccount(baseOwner.publicKey);
  const feeAccount = await tokenMintPool.createAccount(baseOwner.publicKey);

  console.log('[TokenSwap] Creating TokenA account');
  swapTokenAAccount = await tokenMintA.createAccount(authority);
  await tokenMintA.mintTo(swapTokenAAccount, new Account(baseOwner.secretKey), [], currentSwapTokenA);
  console.log('[TokenSwap] TokenA account address: ' + swapTokenAAccount.toString());


  console.log('[TokenSwap] Creating TokenB account');
  swapTokenBAccount = await tokenMintB.createAccount(authority);
  await tokenMintB.mintTo(swapTokenBAccount, new Account(baseOwner.secretKey), [], currentSwapTokenB);
  console.log('[TokenSwap] TokenB account address: ' + swapTokenBAccount.toString());

  console.log('[TokenSwap] creating token swap');
  const swapPayer = await newAccountWithLamports(connection, 10000000000);
  let tokenSwap = await TokenSwap.createTokenSwap(
    connection,
    new Account(swapPayer.secretKey),
    new Account(tokenSwapAccount.secretKey),
    authority,
    swapTokenAAccount,
    swapTokenBAccount,
    tokenMintPool.publicKey,
    tokenMintA.publicKey,
    tokenMintB.publicKey,
    feeAccount,
    tokenAccountPool,
    tokenSwapProgramPubKey,
    TOKEN_PROGRAM_ID,
    nonce,
    TRADING_FEE_NUMERATOR,
    TRADING_FEE_DENOMINATOR,
    OWNER_TRADING_FEE_NUMERATOR,
    OWNER_TRADING_FEE_DENOMINATOR,
    OWNER_WITHDRAW_FEE_NUMERATOR,
    OWNER_WITHDRAW_FEE_DENOMINATOR,
    HOST_FEE_NUMERATOR,
    HOST_FEE_DENOMINATOR,
    CURVE_TYPE,
  );

  console.log('[TokenSwap] token swap created');

}

export async function prepareSerumDex(): Promise<void>{
  const connection = await getConnection();

  const serumDexPayer = await newAccountWithLamports(connection, 90000000000);
  await connection.requestAirdrop(serumDexPayer.publicKey, 90000000000);
  serumMarket = Keypair.generate();

  console.log("[SerumDex] serum_dex program: " + serumDexProgramPubKey);
  console.log("[SerumDex] serum_market publickey: " + serumMarket.publicKey);
  // authority of the token and accounts

  async function getVaultOwnerAndNonce(): Promise<[PublicKey, any]> {
    const nonce = new BN(0);
    while (true) {
      try {
        const vaultOwner = await PublicKey.createProgramAddress(
          [serumMarket.publicKey.toBuffer(), nonce.toArrayLike(Buffer, 'le', 8)],
          serumDexProgramPubKey,
        );
        return [vaultOwner, nonce];
      } catch (e) {
        nonce.iaddn(1);
      }
    }
  }

  const [vaultOwner, vaultSignerNonce] = await getVaultOwnerAndNonce();

  console.log("[SerumDex] authority: " + vaultOwner);
  console.log("[SerumDex] nonce: " + vaultSignerNonce);

  console.log('[SerumDex] Creating TokenA account');
  serumTokenAAccount = await tokenMintA.createAccount(vaultOwner);
  await tokenMintA.mintTo(serumTokenAAccount, new Account(baseOwner.secretKey), [], currentSwapTokenA);
  console.log('[SerumDex] TokenA account address: ' + serumTokenAAccount.toString());


  console.log('[SerumDex] Creating TokenB account');
  serumTokenBAccount = await tokenMintB.createAccount(vaultOwner);
  await tokenMintB.mintTo(serumTokenBAccount, new Account(baseOwner.secretKey), [], currentSwapTokenB);
  console.log('[SerumDex] TokenB account address: ' + serumTokenBAccount.toString());

 
  let requestQueue = Keypair.generate();
  let eventQueue = Keypair.generate();
  let bids = Keypair.generate();
  let asks = Keypair.generate();

  let transactions = new Transaction();
  
  transactions.add(await createDexAccount(connection, serumDexPayer.publicKey, serumMarket.publicKey, MARKET_STATE_LAYOUT_V2.span));
  transactions.add(await createDexAccount(connection, serumDexPayer.publicKey, requestQueue.publicKey, 5120 + 12));
  transactions.add(await createDexAccount(connection, serumDexPayer.publicKey, eventQueue.publicKey, 262144 + 12));
  transactions.add(await createDexAccount(connection, serumDexPayer.publicKey, bids.publicKey, 65536 + 12));
  transactions.add(await createDexAccount(connection, serumDexPayer.publicKey, asks.publicKey, 65536 + 12));

  // await realSendAndConfirmTransaction("prepare accounts", connection, transactions, serumDexPayer);

  // let m = Market.getLayout()
  // initial market    
  // let transaction2 = new Transaction();

  let initializeInstruction = DexInstructions.initializeMarket({
    market: serumMarket.publicKey,
    requestQueue: requestQueue.publicKey,
    eventQueue: eventQueue.publicKey,
    bids: bids.publicKey,
    asks: asks.publicKey,
    baseVault: serumTokenAAccount,
    quoteVault: serumTokenBAccount,
    baseMint: tokenMintA.publicKey,
    quoteMint: tokenMintB.publicKey,
    baseLotSize: new BN(1),
    quoteLotSize: new BN(1),
    feeRateBps: 1,
    vaultSignerNonce: vaultSignerNonce,
    quoteDustThreshold: new BN(5),
    programId: serumDexProgramPubKey,
  });
  transactions.add(initializeInstruction);
  let l = await realSendAndConfirmTransaction("setup_market", connection, transactions, serumDexPayer, serumMarket, requestQueue, eventQueue, bids, asks);
  // console.log('[SerumDex] l: ' + l)
  console.log('[SerumDex] SetupMarket: ' + serumMarket.publicKey);

  let market = await Market.load(connection, serumMarket.publicKey, {
  }, serumDexProgramPubKey);

  console.log("[SerumDex] Load Market: " + market.address);

  console.log('[SerumDex] Creating Order TokenB account');
  let serumOrderTokenA = await tokenMintA.createAccount(serumDexPayer.publicKey);
  await tokenMintA.mintTo(serumOrderTokenA, new Account(baseOwner.secretKey), [], currentSwapTokenB);
  console.log('[SerumDex] Order TokenB account address: ' + serumOrderTokenA.toString());

  serumDefaultMarketOpenOrder = Keypair.generate();

  console.log("[SerumDex] PlaceOrder: " + serumDefaultMarketOpenOrder.publicKey);
  let result = await market.placeOrder(connection, {
    owner: new Account(serumDexPayer.secretKey),
    payer: serumOrderTokenA,
    side: "sell",
    price: 1,
    size: 10000,
    clientId: new BN(12306),
    openOrdersAccount: new Account(serumDefaultMarketOpenOrder.secretKey),
    selfTradeBehavior: 'cancelProvide',
    feeDiscountPubkey: serumDexPayer.publicKey
  });
  console.log("[SerumDEX] placeOrder: " + result);

  let tmpOrder = await OpenOrders.load(connection, serumDefaultMarketOpenOrder.publicKey, serumDexProgramPubKey);
  console.log("[SerumDEX] openOrder, address: " + tmpOrder.address);
  console.log("[SerumDEX] openOrder, base: " + tmpOrder.baseTokenTotal);
  console.log("[SerumDEX] openOrder, quote: " + tmpOrder.quoteTokenTotal);
}

async function createDexAccount(
  connection: Connection,
  fromPubkey: PublicKey,
  newAccountPubkey: PublicKey,
  space: number
): Promise<TransactionInstruction> {
  let lamports = await connection.getMinimumBalanceForRentExemption(space);
  return SystemProgram.createAccount({
    fromPubkey: fromPubkey,
    newAccountPubkey: newAccountPubkey,
    lamports: lamports,
    space: space,
    programId: serumDexProgramPubKey,
  });
}

export async function createOneSolProtocol(): Promise<void> {
  const connection = await getConnection();
  console.log('[OnesolProtocol] creating onesolProtocol');

  // await connection.requestAirdrop(onesolOwner.publicKey, 10000);
  onesolProtocolAccount = Keypair.generate();

  let [authority, nonce] = await PublicKey.findProgramAddress(
    [onesolProtocolAccount.publicKey.toBuffer()],
    onesolProtocolProgramId,
  );
  // let payer = Keypair.fromSecretKey

  console.log('[OnesolProtocol] creating middle TokenA account');
  let tokenAAccount = await tokenMintA.createAccount(authority);
  // await tokenMintB.mintTo(tokenBAccount, baseOwner, [], 0);
  // console.log('middle tokenB account address: ' + tokenBAccount.toString());

  console.log("[OnesolProtocol] create OneSolProtocol account");
  let onesolProtocol = await OneSolProtocol.createOneSolProtocol(
    connection,
    onesolProtocolAccount,
    tokenAAccount,
    TOKEN_PROGRAM_ID,
    authority,
    nonce,
    Keypair.fromSecretKey(basePayer.secretKey),
    onesolProtocolProgramId, 
  );
  console.log("[OnesolProtocol] OneSolProtocol account created");
  let fetchedAccount = await OneSolProtocol.loadOneSolProtocol(
    connection,
    onesolProtocolAccount.publicKey, 
    onesolProtocolProgramId,
    basePayer.publicKey,
  );
  assert(fetchedAccount.tokenProgramId.equals(TOKEN_PROGRAM_ID));
  assert(fetchedAccount.tokenAccountKey.equals(tokenAAccount));
  assert(fetchedAccount.authority.equals(authority));
  assert(fetchedAccount.nonce == nonce);
  console.log("[OnesolProtocol] authority: " + fetchedAccount.authority.toString());
  console.log("[OnesolProtocol] nonce: " + nonce);

};

export async function swap(): Promise<void> {
  const connection = await getConnection();

  const alice = await newAccountWithLamports(connection, 1000000000);
  // const userTransferAuthority = new Account();

  console.log("[OnesolProtocol] load TokenSwapInfo");
  const tokenSwapInfo = await loadTokenSwapInfo(
    connection,
    tokenSwapAccount.publicKey,
    tokenSwapProgramPubKey,
    new Numberu64(49000),
    new Numberu64(40000),
    null,
  )

  console.log("[OnesolProtocol] load OneSolProtocolInfo");
  let onesolProtocol = await OneSolProtocol.loadOneSolProtocol(
    connection,
    onesolProtocolAccount.publicKey, 
    onesolProtocolProgramId,
    alice.publicKey,
  );


  console.log('[OnesolProtocol] Creating Alice TokenA account');
  let userAccountA = await tokenMintA.createAccount(alice.publicKey);
  // await tokenMintA.mintTo(userAccountA, new Account(baseOwner.secretKey), [], SWAP_AMOUNT_IN);

  // await tokenMintA.approve(
  //   userAccountA,
  //   userTransferAuthority.publicKey,
  //   alice,
  //   [],
  //   SWAP_AMOUNT_IN,
  // );

  console.log('[OnesolProtocol] Alice TokenA Account: ' + userAccountA.toString())

  console.log('[OnesolProtocol] Creating Alice TokenB account');
  let userAccountB = await tokenMintB.createAccount(alice.publicKey);
  await tokenMintB.mintTo(userAccountB, new Account(baseOwner.secretKey), [], SWAP_AMOUNT_IN);
  console.log('[OnesolProtocol] Alice TokenB Account: ' + userAccountB.toString())

  let userTokenAccountAInfo = await tokenMintA.getAccountInfo(userAccountA);
  console.log("[OnesolProtocol] user TokenA amount:" + userTokenAccountAInfo.amount.toNumber());
  let userTokenAccountBInfo = await tokenMintB.getAccountInfo(userAccountB);
  console.log("[OnesolProtocol] user TokenB amount:" + userTokenAccountBInfo.amount.toNumber());

  console.log("[OnesolProtocol] authority: " + onesolProtocol.authority.toString());
  console.log('[OnesolProtocol] Swapping');

  console.log("[OnesolProtocol] load serumMarket")
  let market = await Market.load(connection, serumMarket.publicKey, {}, serumDexProgramPubKey)


  // let openOrder = await OpenOrders.load(connection, serumDefaultMarketOpenOrder.publicKey, serumDexProgramPubKey);
  console.log("[OnesolProtocol][SerumDex] market: " + market.address +
   ", baseLotSize: " + market.decoded.baseLotSize +
   ", quoteLoteSize: " + market.decoded.quoteLotSize +
   ", quoteDustThreshold:" + market.decoded.quoteDustThreshold
  );

  // let size = 500;
  // let price = 1;
  // let serumTradeInfo = SerumDexMarketInfo.create(market, price, size, new Numberu64(65536));
  // let limitPrice = market.priceNumberToLots(price);
  // let maxBaseQuantity = market.baseSizeNumberToLots(size);
  // let maxQuoteQuantity = new BN(market.decoded.quoteLotSize.toNumber()).mul(
  //   market.baseSizeNumberToLots(size).mul(market.priceNumberToLots(price)),
  // );
  
  let serumTradeInfo = new SerumDexMarketInfo(
    serumDexProgramPubKey,
    market,
    new Numberu64(1),
    new Numberu64(51000),
    new Numberu64(51000),
    new Numberu64(65535),
  );

  // await onesolProtocol.swap(
  //   userAccountA,
  //   tokenMintA.publicKey,
  //   userAccountB,
  //   SWAP_AMOUNT_OUT,
  //   tokenSwapInfo,
  //   serumTradeInfo,
  //   Keypair.fromSecretKey(basePayer.secretKey),
  // );
  await onesolProtocol.swap(
    userAccountB,
    tokenMintB.publicKey,
    userAccountA,
    SWAP_AMOUNT_OUT,
    tokenSwapInfo,
    serumTradeInfo,
    alice,
  );

  await sleep(500);
  console.log("swap done.")

  // let info;
  let info = await tokenMintA.getAccountInfo(userAccountA);
  console.log("user TokenA amount:" + info.amount.toNumber());
  // assert(info.amount.toNumber() == 0);

  info = await tokenMintB.getAccountInfo(userAccountB);
  console.log("user TokenB amount:" + info.amount.toNumber());
  // assert(info.amount.toNumber() == SWAP_AMOUNT_OUT);

  info = await tokenMintA.getAccountInfo(onesolProtocol.tokenAccountKey);
  console.log("protocol Token amount:" + info.amount.toNumber());
  // assert(info.amount.toNumber() == SWAP_AMOUNT_OUT);

  info = await tokenMintA.getAccountInfo(swapTokenAAccount);
  console.log("swapTokenA amount:" + info.amount.toNumber());
  // assert(info.amount.toNumber() == currentSwapTokenA + SWAP_AMOUNT_IN);
  // currentSwapTokenA += SWAP_AMOUNT_IN;

  info = await tokenMintB.getAccountInfo(swapTokenBAccount);
  console.log("swapTokenB amount:" + info.amount.toNumber());
  // assert(info.amount.toNumber() == currentSwapTokenB - SWAP_AMOUNT_OUT);
  // currentSwapTokenB -= SWAP_AMOUNT_OUT;

  // info = await tokenPool.getAccountInfo(tokenAccountPool);
  // assert(
  //   info.amount.toNumber() == DEFAULT_POOL_TOKEN_AMOUNT - POOL_TOKEN_AMOUNT,
  // );

  // info = await tokenPool.getAccountInfo(feeAccount);
  // assert(info.amount.toNumber() == currentFeeAmount + OWNER_SWAP_FEE);

  // if (poolAccount != null) {
  //   info = await tokenPool.getAccountInfo(poolAccount);
  //   assert(info.amount.toNumber() == HOST_SWAP_FEE);
  // }
}

export async function loadTokenSwapInfo(
  connection: Connection,
  address: PublicKey,
  programId: PublicKey,
  amountIn: Numberu64,
  minimumAmountOut: Numberu64,
  hostFeeAccount: PublicKey | null,
): Promise<TokenSwapInfo> {
  const data = await loadAccount(connection, address, programId);
  const tokenSwapData = TokenSwapLayout.decode(data);
  if (!tokenSwapData.isInitialized) {
    throw new Error(`Invalid token swap state`);
  }

  const [authority] = await PublicKey.findProgramAddress(
    [address.toBuffer()],
    programId,
  );

  const poolToken = new PublicKey(tokenSwapData.tokenPool);
  const feeAccount = new PublicKey(tokenSwapData.feeAccount);
  const tokenAccountA = new PublicKey(tokenSwapData.tokenAccountA);
  const tokenAccountB = new PublicKey(tokenSwapData.tokenAccountB);
  const mintA = new PublicKey(tokenSwapData.mintA);
  const mintB = new PublicKey(tokenSwapData.mintB);
  const tokenProgramId = new PublicKey(tokenSwapData.tokenProgramId);

  const curveType = tokenSwapData.curveType;

  return new TokenSwapInfo(
    amountIn,
    minimumAmountOut,
    programId,
    address,
    authority,
    tokenAccountA,
    tokenAccountB,
    poolToken,
    feeAccount,
    hostFeeAccount
  );
}
