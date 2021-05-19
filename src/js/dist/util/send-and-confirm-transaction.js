"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.sendAndConfirmTransaction = void 0;
const web3_js_1 = require("@solana/web3.js");
function sendAndConfirmTransaction(title, connection, transaction, ...signers) {
    return web3_js_1.sendAndConfirmTransaction(connection, transaction, signers, {
        skipPreflight: false,
        commitment: 'recent',
        preflightCommitment: 'recent',
    });
}
exports.sendAndConfirmTransaction = sendAndConfirmTransaction;
