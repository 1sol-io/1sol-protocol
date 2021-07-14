// To connect to a public cluster, set `export LIVE=1` in your
// environment. By default, `LIVE=1` will connect to the devnet cluster.

import {clusterApiUrl, Cluster} from '@solana/web3.js';
import {ONESOL_PROTOCOL_PROGRAM_ID} from '../../src/onesol-protocol';
import dotenv from 'dotenv';

function initEnvConfig(): EnvConfig {
  dotenv.config();
  let cluster: Cluster = "devnet";
  switch (process.env.CLUSTER) {
    case 'devnet':
    case 'testnet':
    case 'mainnet-beta': {
      cluster = process.env.CLUSTER;
    }
  }
  let url = process.env.RPC_URL || clusterApiUrl(cluster, false);
  let splTokenSwapProgramId = process.env.TOKEN_SWAP_PROGRAM_ID || "SwaPpA9LAaLfeLi3a68M4DjnLqgtticKg6CnyNwgAC8";
  let serumDexProgramId = process.env.SERUM_DEX_PROGRAM_ID || "DESVgJVGajEgKGXhb6XmqDHGz3VjdgP7rEVESBgxmroY";
  let onesolProtocolProgramId = process.env.ONESOL_PROTOCOL_PROGRAM_ID || ONESOL_PROTOCOL_PROGRAM_ID.toString();
  return {
    url: url,
    cluster: cluster,
    splTokenSwapProgramId: splTokenSwapProgramId,
    serumDexProgramId: serumDexProgramId, 
    onesolProtocolProgramId: onesolProtocolProgramId, 
  };
}

export const envConfig = initEnvConfig();
interface EnvConfig {
  url: string
  cluster: Cluster,
  splTokenSwapProgramId: string,
  serumDexProgramId: string,
  onesolProtocolProgramId: string,
}
