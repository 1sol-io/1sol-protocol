import {
    Account,
    clusterApiUrl,
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

let connection: Connection;
async function getConnection(): Promise<Connection> {
  if (connection) return connection;

  connection = new Connection(envConfig.url, 'recent');
  const version = await connection.getVersion();
  console.log('Connection to cluster established:', envConfig.url, version);
  return connection;
}

export async function loadAllAmmInfos() {
  const connection = new Connection(clusterApiUrl("devnet"), "recent");
  const version = await connection.getVersion();
  console.log('Connection to cluster established:', envConfig.url, version);

  const ammInfoArray = await OneSolProtocol.loadAllAmmInfos(connection);
  console.log(`ammInfoArray.count: ${ammInfoArray.length}`)
  ammInfoArray.forEach(ammInfo => {
    console.log(JSON.stringify({
      pubkey: ammInfo.pubkey.toBase58(),
      token_a_account: ammInfo.tokenAccountA().toBase58(),
      token_a_mint: ammInfo.tokenMintA().toBase58(),
      token_b_account: ammInfo.tokenAccountB().toBase58(),
      token_b_mint: ammInfo.tokenMintB().toBase58(),
      program_id: ammInfo.programId.toBase58(),
    }));
  });
}