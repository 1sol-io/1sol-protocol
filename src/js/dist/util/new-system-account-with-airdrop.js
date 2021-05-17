"use strict";
// @flow
Object.defineProperty(exports, "__esModule", { value: true });
exports.newSystemAccountWithAirdrop = void 0;
const web3_js_1 = require("@solana/web3.js");
/**
 * Create a new system account and airdrop it some lamports
 *
 * @private
 */
async function newSystemAccountWithAirdrop(connection, lamports = 1) {
    const account = new web3_js_1.Account();
    await connection.requestAirdrop(account.publicKey, lamports);
    return account;
}
exports.newSystemAccountWithAirdrop = newSystemAccountWithAirdrop;
