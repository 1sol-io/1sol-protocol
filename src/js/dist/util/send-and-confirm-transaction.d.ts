import type { Account, Connection, Transaction, TransactionSignature } from '@solana/web3.js';
export declare function sendAndConfirmTransaction(title: string, connection: Connection, transaction: Transaction, ...signers: Array<Account>): Promise<TransactionSignature>;
