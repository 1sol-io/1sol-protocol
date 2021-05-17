"use strict";
// To connect to a public cluster, set `export LIVE=1` in your
// environment. By default, `LIVE=1` will connect to the devnet cluster.
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.walletUrl = exports.urlTls = exports.url = exports.cluster = void 0;
const web3_js_1 = require("@solana/web3.js");
const dotenv_1 = __importDefault(require("dotenv"));
function chooseCluster() {
    dotenv_1.default.config();
    if (!process.env.LIVE)
        return;
    switch (process.env.CLUSTER) {
        case 'devnet':
        case 'testnet':
        case 'mainnet-beta': {
            return process.env.CLUSTER;
        }
    }
    throw 'Unknown cluster "' + process.env.CLUSTER + '", check the .env file';
}
exports.cluster = chooseCluster();
exports.url = process.env.RPC_URL ||
    (process.env.LIVE ? web3_js_1.clusterApiUrl(exports.cluster, false) : 'http://localhost:8899');
exports.urlTls = process.env.RPC_URL ||
    (process.env.LIVE ? web3_js_1.clusterApiUrl(exports.cluster, true) : 'http://localhost:8899');
exports.walletUrl = process.env.WALLET_URL || 'https://solana-example-webwallet.herokuapp.com/';
