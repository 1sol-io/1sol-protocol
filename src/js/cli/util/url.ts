// To connect to a public cluster, set `export LIVE=1` in your
// environment. By default, `LIVE=1` will connect to the devnet cluster.

import {clusterApiUrl, Cluster} from '@solana/web3.js';
import dotenv from 'dotenv';

function chooseCluster(): Cluster | undefined {
  dotenv.config();
  console.log("live:" + process.env.LIVE)
  if (!process.env.LIVE) return;
  switch (process.env.CLUSTER) {
    case 'devnet':
    case 'testnet':
    case 'mainnet-beta': {
      return process.env.CLUSTER;
    }
  }
  throw 'Unknown cluster "' + process.env.CLUSTER + '", check the .env file';
}

function getTokenSwapProgramId(): string {
  dotenv.config();
  return process.env.TOKEN_SWAP_PROGRAM_ID || "SwaPpA9LAaLfeLi3a68M4DjnLqgtticKg6CnyNwgAC8";
}

export const cluster = chooseCluster();

export const url =
  process.env.RPC_URL ||
  (process.env.LIVE ? clusterApiUrl(cluster, false) : 'http://localhost:8899');

export const urlTls =
  process.env.RPC_URL ||
  (process.env.LIVE ? clusterApiUrl(cluster, true) : 'http://localhost:8899');

export const tokenSwapProgramId = getTokenSwapProgramId();
