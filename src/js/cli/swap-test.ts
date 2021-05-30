import {
    Account,
    Connection,
    Keypair,
    PublicKey,
    SystemProgram,
    Transaction,
} from '@solana/web3.js';
import {AccountLayout, Token, TOKEN_PROGRAM_ID} from '@solana/spl-token';
import {TokenSwap, TokenSwapLayout} from '@solana/spl-token-swap';

import {OneSolProtocol, ONESOL_PROTOCOL_PROGRAM_ID, TokenSwapInfo} from '../src';
import {newAccountWithLamports} from '../src/util/new-account-with-lamports';
import {url} from '../src/util/url';
import {sleep} from '../src/util/sleep';
import {loadAccount} from '../src/util/account';

export const TOKEN_SWAP_PROGRAM_ID: PublicKey = new PublicKey(
  // 'SwaPpA9LAaLfeLi3a68M4DjnLqgtticKg6CnyNwgAC8',
  'BgGyXsZxLbug3f4q7W5d4EtsqkQjH1M9pJxUSGQzVGyf',
);


let payer: Account;
// owner of the user accounts
let owner: Account;

const onesolOwner = Keypair.generate();

let tokenSwapAccount: Account;
let onesolProtocolAccount: Account;

// Tokens swapped
let tokenPool: Token;
let mintA: Token;
let mintB: Token;
let tokenAccountA: PublicKey;
let tokenAccountB: PublicKey;

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
const SWAP_AMOUNT_OUT = SWAP_PROGRAM_OWNER_FEE_ADDRESS ? 90000 : 90000;
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

  connection = new Connection(url, 'recent');
  const version = await connection.getVersion();

  console.log('Connection to cluster established:', url, version);
  return connection;
}

export async function prepareTokenSwap(): Promise<void> {
    const connection = await getConnection();
    payer = await newAccountWithLamports(connection, 1000000000);
    owner = await newAccountWithLamports(connection, 1000000000);
    tokenSwapAccount = new Account();

    // authority of the token and accounts
    let authority: PublicKey;
    // nonce used to generate the authority public key
    let nonce: number; 
    [authority, nonce] = await PublicKey.findProgramAddress(
      [tokenSwapAccount.publicKey.toBuffer()],
      TOKEN_SWAP_PROGRAM_ID,
    );

    console.log('creating pool mint');
    tokenPool = await Token.createMint(
      connection,
      payer,
      authority,
      null,
      2,
      TOKEN_PROGRAM_ID,
    );
  
    console.log('creating pool account');
    const tokenAccountPool = await tokenPool.createAccount(owner.publicKey);
    const ownerKey = SWAP_PROGRAM_OWNER_FEE_ADDRESS || owner.publicKey.toString();
    const feeAccount = await tokenPool.createAccount(new PublicKey(ownerKey));
  
    console.log('creating token A');
    mintA = await Token.createMint(
      connection,
      payer,
      owner.publicKey,
      null,
      2,
      TOKEN_PROGRAM_ID,
    );
  
    console.log('creating token A account');
    tokenAccountA = await mintA.createAccount(authority);
    console.log('token A account address: ' + tokenAccountA.toString());
    console.log('minting token A to swap');
    await mintA.mintTo(tokenAccountA, owner, [], currentSwapTokenA);
  
    console.log('creating token B');
    mintB = await Token.createMint(
      connection,
      payer,
      owner.publicKey,
      null,
      2,
      TOKEN_PROGRAM_ID,
    );
  
    console.log('creating token B account');
    tokenAccountB = await mintB.createAccount(authority);
    console.log('token B account address: ' + tokenAccountB.toString());
    console.log('minting token B to swap');
    await mintB.mintTo(tokenAccountB, owner, [], currentSwapTokenB);
  
    console.log('creating token swap');
    const swapPayer = await newAccountWithLamports(connection, 10000000000);
    let tokenSwap = await TokenSwap.createTokenSwap(
      connection,
      swapPayer,
      tokenSwapAccount,
      authority,
      tokenAccountA,
      tokenAccountB,
      tokenPool.publicKey,
      mintA.publicKey,
      mintB.publicKey,
      feeAccount,
      tokenAccountPool,
      TOKEN_SWAP_PROGRAM_ID,
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

    console.log('token swap created');

}

export async function createOneSolProtocol(): Promise<void> {
  const connection = await getConnection();
  console.log('creating onesolProtocol');

  // await connection.requestAirdrop(onesolOwner.publicKey, 10000);
  onesolProtocolAccount = new Account();

  let [authority, nonce] = await PublicKey.findProgramAddress(
    [onesolProtocolAccount.publicKey.toBuffer()],
    ONESOL_PROTOCOL_PROGRAM_ID,
  );
  // let payer = Keypair.fromSecretKey

  console.log('creating middle TokenB account');
  let tokenBAccount = await mintB.createAccount(authority);
  console.log('middle tokenB account address: ' + tokenBAccount.toString());
  await mintB.mintTo(tokenAccountB, owner, [], 0);

  console.log("create OneSolProtocol account");
  let onesolProtocol = await OneSolProtocol.createOneSolProtocol(
    connection,
    onesolProtocolAccount,
    tokenBAccount,
    TOKEN_PROGRAM_ID,
    authority,
    nonce,
    payer,
    ONESOL_PROTOCOL_PROGRAM_ID, 
  );
  console.log("OneSolProtocol account created");
  let fetchedAccount = await OneSolProtocol.loadOneSolProtocol(
    connection,
    onesolProtocolAccount.publicKey, 
    ONESOL_PROTOCOL_PROGRAM_ID,
    payer,
  );
  assert(fetchedAccount.tokenProgramId.equals(TOKEN_PROGRAM_ID));
  assert(fetchedAccount.tokenAccountKey.equals(tokenBAccount));
  assert(fetchedAccount.authority.equals(authority));
  assert(fetchedAccount.nonce == nonce);
  console.log("authority: " + fetchedAccount.authority.toString());
  console.log("nonce: " + nonce);

};

export async function swap(): Promise<void> {
  const connection = await getConnection();

  const alice = await newAccountWithLamports(connection, 1000000000);
  // TokenSwap.loadTokenSwap(connection, tokenSwap.)

  // const onesolPro = new Account();
  // let onesolProtocolAuthority, nonce
  // [onesolProtocolAuthority, nonce] = await PublicKey.findProgramAddress(
  //   [onesolPro.publicKey.toBuffer()],
  //     ONESOL_PROTOCOL_PROGRAM_ID,
  // );
  // TODO create tokenAccount

  let poolAccount = SWAP_PROGRAM_OWNER_FEE_ADDRESS
  ? await tokenPool.createAccount(owner.publicKey)
  : null;

  console.log("load TokenSwapInfo");
  const tokenSwapInfo = await loadTokenSwapInfo(
    connection,
    tokenSwapAccount.publicKey,
    TOKEN_SWAP_PROGRAM_ID,
    poolAccount,
  )

  console.log("load OneSolProtocolInfo");
  let onesolProtocol = await OneSolProtocol.loadOneSolProtocol(
    connection,
    onesolProtocolAccount.publicKey, 
    ONESOL_PROTOCOL_PROGRAM_ID,
    alice,
  );


  const userTransferAuthority = new Account();

  console.log('Creating Alice TokenA account');
  let userAccountA = await mintA.createAccount(alice.publicKey);
  await mintA.mintTo(userAccountA, owner, [], SWAP_AMOUNT_IN);
  await mintA.approve(
    userAccountA,
    userTransferAuthority.publicKey,
    alice,
    [],
    SWAP_AMOUNT_IN,
  );

  console.log('Alice TokenA Account: ' + userAccountA.toString())

  console.log('Creating Alice TokenB account');
  let userAccountB = await mintB.createAccount(alice.publicKey);
  console.log('Alice TokenB Account: ' + userAccountB.toString())

  let info;
  info = await mintA.getAccountInfo(userAccountA);
  console.log("userA:" + info.amount.toNumber());
  info = await mintB.getAccountInfo(userAccountB);
  console.log("userB:" + info.amount.toNumber());
  console.log("authority: " + onesolProtocol.authority.toString());
  console.log('Swapping');

// tokenAccountA,
//     tokenAccountB,

  await onesolProtocol.swap(
    userTransferAuthority,
    userAccountA,
    userAccountB,
    SWAP_AMOUNT_IN,
    SWAP_AMOUNT_OUT,
    tokenSwapInfo,
    null,
  );

  await sleep(500);
  console.log("swap done.")

  // let info;
  info = await mintA.getAccountInfo(userAccountA);
  console.log("user TokenA amount:" + info.amount.toNumber());
  // assert(info.amount.toNumber() == 0);

  info = await mintB.getAccountInfo(userAccountB);
  console.log("user TokenB amount:" + info.amount.toNumber());
  // assert(info.amount.toNumber() == SWAP_AMOUNT_OUT);

  info = await mintB.getAccountInfo(onesolProtocol.tokenAccountKey);
  console.log("protocol Token amount:" + info.amount.toNumber());
  // assert(info.amount.toNumber() == SWAP_AMOUNT_OUT);

  info = await mintA.getAccountInfo(tokenAccountA);
  console.log("tokenA:" + info.amount.toNumber());
  // assert(info.amount.toNumber() == currentSwapTokenA + SWAP_AMOUNT_IN);
  // currentSwapTokenA += SWAP_AMOUNT_IN;

  info = await mintB.getAccountInfo(tokenAccountB);
  console.log("tokenB:" + info.amount.toNumber());
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