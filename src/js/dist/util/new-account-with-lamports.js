"use strict";
// @flow
Object.defineProperty(exports, "__esModule", { value: true });
exports.newAccountWithLamports = void 0;
const web3_js_1 = require("@solana/web3.js");
const sleep_1 = require("./sleep");
async function newAccountWithLamports(connection, lamports = 1000000) {
    const account = new web3_js_1.Account();
    let retries = 30;
    await connection.requestAirdrop(account.publicKey, lamports);
    for (;;) {
        await sleep_1.sleep(500);
        if (lamports == (await connection.getBalance(account.publicKey))) {
            return account;
        }
        if (--retries <= 0) {
            break;
        }
    }
    throw new Error(`Airdrop of ${lamports} failed`);
}
exports.newAccountWithLamports = newAccountWithLamports;
