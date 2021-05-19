import assert from 'assert';
import BN from 'bn.js';
import {Buffer} from 'buffer';
import * as BufferLayout from 'buffer-layout';
import type {Connection, TransactionSignature} from '@solana/web3.js';
import {TokenSwapLayout} from '@solana/spl-token-swap';
import {
  Account,
  PublicKey,
  SystemProgram,
  Transaction,
  TransactionInstruction,
} from '@solana/web3.js';

import * as Layout from './layout';
import {sendAndConfirmTransaction} from './util/send-and-confirm-transaction';
import {loadAccount} from './util/account';

export const ONESOL_PROTOCOL_PROGRAM_ID: PublicKey = new PublicKey(
  // 'SwaPpA9LAaLfeLi3a68M4DjnLqgtticKg6CnyNwgAC8',
  // 'GSKD4BfZBFzCtGzZ7qEgPgr4UgkxiCK3bgTV9PQFRMab',
  '4GnKgDtXinfwxXJvsaZRUVShtd27pNRXTgLZJqmoVA45',
);

/**
 * Some amount of tokens
 */
export class Numberu64 extends BN {
  /**
   * Convert to Buffer representation
   */
  toBuffer(): Buffer {
    const a = super.toArray().reverse();
    const b = Buffer.from(a);
    if (b.length === 8) {
      return b;
    }
    assert(b.length < 8, 'Numberu64 too large');

    const zeroPad = Buffer.alloc(8);
    b.copy(zeroPad);
    return zeroPad;
  }

  /**
   * Construct a Numberu64 from Buffer representation
   */
  static fromBuffer(buffer: Buffer): Numberu64 {
    assert(buffer.length === 8, `Invalid buffer length: ${buffer.length}`);
    return new Numberu64(
      [...buffer]
        .reverse()
        .map(i => `00${i.toString(16)}`.slice(-2))
        .join(''),
      16,
    );
  }
}

export const OneSolProtocolLayout = BufferLayout.struct([
  BufferLayout.u8('version'),
  BufferLayout.u8('isInitialized'),
  BufferLayout.u8('nonce'),
  Layout.publicKey('tokenProgramId'),
  Layout.publicKey('tokenAccountA'),
  Layout.publicKey('tokenAccountB'),
  Layout.publicKey('tokenPool'),
  Layout.publicKey('feeAccount'),
]);

// export const CurveType = Object.freeze({
//   ConstantProduct: 0, // Constant product curve, Uniswap-style
//   ConstantPrice: 1, // Constant price curve, always X amount of A token for 1 B token, where X is defined at init
//   Offset: 3, // Offset curve, like Uniswap, but with an additional offset on the token B side
// });

/**
 * A program to exchange tokens against a pool of liquidity
 */
export class OneSolProtocol{
  /**
   * Create a Token object attached to the specific token
   *
   * @param connection The connection to use
   * @param onesolProtocol The onesol protocol account
   * @param tokenSwap The token swap account
   * @param swapProgramId The program ID of the token-swap program
   * @param tokenProgramId The program ID of the token program
   * @param poolToken The pool token
   * @param authority The authority over the swap and accounts
   * @param tokenAccountA The token swap's Token A account
   * @param tokenAccountB The token swap's Token B account
   * @param payer Pays for the transaction
   */
  constructor(
    private connection: Connection,
    public onesolProtocol: PublicKey,
    public tokenSwap: PublicKey,
    public protocolProgramId: PublicKey,
    public tokenSwapProgramId: PublicKey,
    public tokenProgramId: PublicKey,
    public poolToken: PublicKey,
    public feeAccount: PublicKey,
    public onesolAuthority: PublicKey,
    public tokenSwapAuthority: PublicKey,
    public tokenAccountA: PublicKey,
    public tokenAccountB: PublicKey,
    public payer: Account,
  ) {
    this.connection = connection;
    this.onesolProtocol = onesolProtocol
    this.tokenSwap = tokenSwap
    this.protocolProgramId = protocolProgramId;
    this.tokenSwapProgramId = tokenSwapProgramId;
    this.tokenProgramId = tokenProgramId;
    this.poolToken = poolToken;
    this.feeAccount = feeAccount;
    this.onesolAuthority = onesolAuthority;
    this.tokenSwapAuthority = tokenSwapAuthority;
    this.tokenAccountA = tokenAccountA;
    this.tokenAccountB = tokenAccountB;
    this.payer = payer;
  }

  /**
   * Get the minimum balance for the token swap account to be rent exempt
   *
   * @return Number of lamports required
   */
  static async getMinBalanceRentForExemptTokenSwap(
    connection: Connection,
  ): Promise<number> {
    return await connection.getMinimumBalanceForRentExemption(
      OneSolProtocolLayout.span,
    );
  }

  /**
   * Create a new OneSol Swap
   *
   * @param connection The connection to use
   * @param payer Pays for the transaction
   * @param tokenSwapAccount The token swap account
   * @param authority The authority over the swap and accounts
   * @param nonce The nonce used to generate the authority
   * @param tokenAccountA: The token swap's Token A account
   * @param tokenAccountB: The token swap's Token B account
   * @param poolToken The pool token
   * @param tokenAccountPool The token swap's pool token account
   * @param tokenProgramId The program ID of the token program
   * @param swapProgramId The program ID of the token-swap program
   * @param feeNumerator Numerator of the fee ratio
   * @param feeDenominator Denominator of the fee ratio
   * @return Token object for the newly minted token, Public key of the account holding the total supply of new tokens
   */
  static async createOneSolProtocol(
    connection: Connection,
    payer: Account,
    onesolProtocolAccount: Account,
    tokenSwapAccount: Account,
    onesolAuthority: PublicKey,
    tokenSwapAuthority: PublicKey,
    tokenAccountA: PublicKey,
    tokenAccountB: PublicKey,
    poolToken: PublicKey,
    feeAccount: PublicKey,
    protocolProgramId: PublicKey,
    tokenSwapProgramId: PublicKey,
    tokenProgramId: PublicKey,

  ): Promise<OneSolProtocol> {
    let transaction;
    const onesolSwap = new OneSolProtocol(
      connection,
      onesolProtocolAccount.publicKey,
      tokenSwapAccount.publicKey,
      protocolProgramId,
      tokenSwapProgramId,
      tokenProgramId,
      poolToken,
      feeAccount,
      onesolAuthority,
      tokenSwapAuthority,
      tokenAccountA,
      tokenAccountB,
      payer,
    );

    // Allocate memory for the account
    const balanceNeeded = await OneSolProtocol.getMinBalanceRentForExemptTokenSwap(
      connection,
    );
    console.log("balanceNeeded: " + balanceNeeded);
    console.log("create onesolProgram account.");
    transaction = new Transaction();
    transaction.add(
      SystemProgram.createAccount({
        fromPubkey: payer.publicKey,
        newAccountPubkey: onesolProtocolAccount.publicKey,
        lamports: balanceNeeded,
        space: OneSolProtocolLayout.span,
        programId: protocolProgramId,
      }),
    );


    // transaction.add(instruction);
    await sendAndConfirmTransaction(
      'createAccount and InitializeSwap',
      connection,
      transaction,
      payer,
      onesolProtocolAccount,
    );
    return onesolSwap;
  }

  /**
   * Swap token A for token B
   *
   * @param userSource User's source token account
   * @param poolSource Pool's source token account
   * @param poolDestination Pool's destination token account
   * @param userDestination User's destination token account
   * @param hostFeeAccount Host account to gather fees
   * @param userTransferAuthority Account delegated to transfer user's tokens
   * @param amountIn Amount to transfer from source account
   * @param minimumAmountOut Minimum amount of tokens the user will receive
   */
  async swap(
    userSource: PublicKey,
    onesolSource: PublicKey,
    poolSource: PublicKey,
    poolDestination: PublicKey,
    onesolDestination: PublicKey,
    userDestination: PublicKey,
    hostFeeAccount: PublicKey | null,
    userTransferAuthority: Account,
    amountIn: number | Numberu64,
    minimumAmountOut: number | Numberu64,
  ): Promise<TransactionSignature> {
    return await sendAndConfirmTransaction(
      'swap',
      this.connection,
      new Transaction().add(
        OneSolProtocol.swapInstruction(
          this.onesolProtocol,
          this.tokenSwap,
          this.onesolAuthority,
          this.tokenSwapAuthority,
          userTransferAuthority.publicKey,
          userSource,
          onesolSource,
          poolSource,
          poolDestination,
          onesolDestination,
          userDestination,
          this.poolToken,
          this.feeAccount,
          hostFeeAccount,
          this.protocolProgramId,
          this.tokenProgramId,
          this.tokenSwapProgramId,
          amountIn,
          minimumAmountOut,
        ),
      ),
      this.payer,
      userTransferAuthority
    );
  }

  static swapInstruction(
    onesolProtocol: PublicKey,
    tokenSwap: PublicKey,
    onesolAuthority: PublicKey,
    tokenSwapAuthority: PublicKey,
    userTransferAuthority: PublicKey,
    userSource: PublicKey,
    onesolSource: PublicKey,
    poolSource: PublicKey,
    poolDestination: PublicKey,
    onesolDestination: PublicKey,
    userDestination: PublicKey,
    poolMint: PublicKey,
    feeAccount: PublicKey,
    hostFeeAccount: PublicKey | null,
    protocolProgramId: PublicKey,
    tokenProgramId: PublicKey,
    tokenSwapProgramId: PublicKey,
    amountIn: number | Numberu64,
    minimumAmountOut: number | Numberu64,
  ): TransactionInstruction {
    const dataLayout = BufferLayout.struct([
      BufferLayout.u8('instruction'),
      Layout.uint64('amountIn'),
      Layout.uint64('minimumAmountOut'),
    ]);

    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode(
      {
        instruction: 1, // Swap instruction
        amountIn: new Numberu64(amountIn).toBuffer(),
        minimumAmountOut: new Numberu64(minimumAmountOut).toBuffer(),
      },
      data,
    );

    const keys = [
      {pubkey: onesolProtocol, isSigner: false, isWritable: false},
      {pubkey: tokenSwap, isSigner: false, isWritable: false},
      {pubkey: onesolAuthority, isSigner: false, isWritable: false},
      {pubkey: tokenSwapAuthority, isSigner: false, isWritable: false},
      {pubkey: userTransferAuthority, isSigner: true, isWritable: false},
      {pubkey: userSource, isSigner: false, isWritable: true},
      {pubkey: onesolSource, isSigner: false, isWritable: true},
      {pubkey: poolSource, isSigner: false, isWritable: true},
      {pubkey: poolDestination, isSigner: false, isWritable: true},
      {pubkey: onesolDestination, isSigner: false, isWritable: true},
      {pubkey: userDestination, isSigner: false, isWritable: true},
      {pubkey: poolMint, isSigner: false, isWritable: true},
      {pubkey: feeAccount, isSigner: false, isWritable: true},
      {pubkey: tokenProgramId, isSigner: false, isWritable: false},
      {pubkey: tokenSwapProgramId, isSigner: false, isWritable: false},
    ];
    if (hostFeeAccount !== null) {
      keys.push({pubkey: hostFeeAccount, isSigner: false, isWritable: true});
    }
    return new TransactionInstruction({
      keys,
      programId: protocolProgramId,
      data,
    });
  }
}
