import assert from 'assert';
import BN from 'bn.js';
import {Buffer} from 'buffer';
import * as BufferLayout from 'buffer-layout';
import type {Connection, TransactionSignature} from '@solana/web3.js';
import {TokenSwap, TokenSwapLayout} from '@solana/spl-token-swap';
import {
  Account,
  AccountMeta,
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
  '26XgL6X46AHxcMkfDNfnfQHrqZGzYEcTLj9SmAV5dLrV',
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

export class TokenSwapInfo {
  constructor(
    private programId: PublicKey,
    private swapInfo: PublicKey,
    private authority: PublicKey,
    private poolSource: PublicKey,
    private poolDestination: PublicKey,
    private poolMint: PublicKey,
    private poolFeeAccount: PublicKey,
    private hostFeeAccount: PublicKey | null,
  ) {
    this.programId = programId;
    this.swapInfo = swapInfo;
    this.authority = authority;
    this.poolSource = poolSource;
    this.poolDestination = poolDestination;
    this.poolMint = poolMint;
    this.poolFeeAccount = poolFeeAccount;
    this.hostFeeAccount = hostFeeAccount;
  }

  toKeys(): Array<AccountMeta>{
    const keys = [
      {pubkey: this.swapInfo, isSigner: false, isWritable: false},
      {pubkey: this.authority, isSigner: false, isWritable: false},
      {pubkey: this.poolSource, isSigner: false, isWritable: true},
      {pubkey: this.poolDestination, isSigner: false, isWritable: true},
      {pubkey: this.poolMint, isSigner: false, isWritable: true},
      {pubkey: this.poolFeeAccount, isSigner: false, isWritable: true},
      {pubkey: this.programId, isSigner: false, isWritable: false},
    ];
    if (this.hostFeeAccount !== null) {
      keys.push({pubkey: this.hostFeeAccount, isSigner: false, isWritable: true});
    }
    return keys;
  }

  includeHostFeeAccount(): number {
    if (this.hostFeeAccount !== null) {
      return 1
    } else {
      return 0
    }
  }
}

/**
 * A program to exchange tokens against a pool of liquidity
 */
export class OneSolProtocol{
  /**
   * Create a Token object attached to the specific token
   *
   * @param connection The connection to use
   * @param protocolProgramID The program ID of the onesol-protocol program
   * @param swapProgramId The program ID of the token-swap program
   * @param tokenProgramId The program ID of the token program
   */
  constructor(
    private connection: Connection,
    public protocolProgramId: PublicKey,
    public tokenSwapProgramId: PublicKey,
    public tokenProgramId: PublicKey,
  ) {
    this.connection = connection;
    this.protocolProgramId = protocolProgramId;
    this.tokenSwapProgramId = tokenSwapProgramId;
    this.tokenProgramId = tokenProgramId;
  }

  /**
   * Create a new OneSol Swap
   *
   * @param connection The connection to use
   * @param protocolProgramID The  program ID of the onesolProtocol program
   * @param tokenProgramId The program ID of the token program
   * @param swapProgramId The program ID of the token-swap program
   * @return Token object for the newly minted token, Public key of the account holding the total supply of new tokens
   */
  static async createOneSolProtocol(
    connection: Connection,
    protocolProgramId: PublicKey,
    tokenSwapProgramId: PublicKey,
    tokenProgramId: PublicKey,
  ): Promise<OneSolProtocol> {
    // let transaction;
    const onesolSwap = new OneSolProtocol(
      connection,
      protocolProgramId,
      tokenSwapProgramId,
      tokenProgramId,
    );

    // // Allocate memory for the account
    // const balanceNeeded = await OneSolProtocol.getMinBalanceRentForExemptTokenSwap(
    //   connection,
    // );
    // console.log("balanceNeeded: " + balanceNeeded);
    // console.log("create onesolProgram account.");
    // transaction = new Transaction();
    // transaction.add(
    //   SystemProgram.createAccount({
    //     fromPubkey: payer.publicKey,
    //     newAccountPubkey: onesolProtocolAccount.publicKey,
    //     lamports: balanceNeeded,
    //     space: OneSolProtocolLayout.span,
    //     programId: protocolProgramId,
    //   }),
    // );


    // // transaction.add(instruction);
    // await sendAndConfirmTransaction(
    //   'createAccount and InitializeSwap',
    //   connection,
    //   transaction,
    //   payer,
    //   onesolProtocolAccount,
    // );
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
    payer: Account,
    middleOwner: PublicKey,
    userTransferAuthority: Account,
    userSource: PublicKey,
    middleSource: PublicKey,
    middleDestination: PublicKey,
    userDestination: PublicKey,
    amountIn: number | Numberu64,
    minimumAmountOut: number | Numberu64,
    nonce: number,
    tokenSwapInfo: TokenSwapInfo | null,
    tokenSwap1Info: TokenSwapInfo | null,
  ): Promise<TransactionSignature> {
    if (tokenSwapInfo === null && tokenSwap1Info === null) {
      throw new Error('tokenSwapInfo and tokenSwap1Info all null');
    }
    return await sendAndConfirmTransaction(
      'swap',
      this.connection,
      new Transaction().add(
        OneSolProtocol.swapInstruction(
          middleOwner,
          userTransferAuthority.publicKey,
          userSource,
          middleSource,
          middleDestination,
          userDestination,
          this.tokenProgramId,
          tokenSwapInfo,
          tokenSwap1Info,
          this.protocolProgramId,
          amountIn,
          minimumAmountOut,
          nonce,
        ),
      ),
      payer,
      userTransferAuthority
    );
  }

  static swapInstruction(
    middleOwnerInfo: PublicKey,
    userTransferAuthority: PublicKey,
    userSource: PublicKey,
    middleSource: PublicKey,
    middleDestination: PublicKey,
    userDestination: PublicKey,
    tokenProgramId: PublicKey,
    // token-swap key begin
    tokenSwapInfo: TokenSwapInfo | null,
    tokenSwap1Info: TokenSwapInfo | null,
    // tokenSwap: PublicKey,
    // tokenSwapAuthority: PublicKey,
    // poolSource: PublicKey,
    // poolDestination: PublicKey,
    // poolMint: PublicKey,
    // feeAccount: PublicKey,
    // tokenSwapProgramId: PublicKey,
    // hostFeeAccount: PublicKey | null,
    // token-swap key end
    protocolProgramId: PublicKey,
    amountIn: number | Numberu64,
    minimumAmountOut: number | Numberu64,
    nonce: number,
  ): TransactionInstruction {

    const dataLayout = BufferLayout.struct([
      BufferLayout.u8('instruction'),
      Layout.uint64('amountIn'),
      Layout.uint64('minimumAmountOut'),
      BufferLayout.u8('nonce'),
      BufferLayout.u8('dexesConfig'),
      BufferLayout.u8('tokenSwapFlag'),
      BufferLayout.u8('tokenSwapAccountsSize'),
      BufferLayout.u8('tokenSwap1Flag'),
      BufferLayout.u8('tokenSwap1AccountsSize'),
    ]);

    let tsKeys = Array<AccountMeta>();
    let tsFlag = 0;
    if (tokenSwapInfo !== null){
      tsFlag = 1;
      tsKeys = tokenSwapInfo.toKeys();
    };

    let ts1Keys = Array<AccountMeta>();
    let ts1Flag = 0;
    if (tokenSwap1Info !== null){
      ts1Flag = 1;
      ts1Keys = tokenSwap1Info.toKeys();
    };

    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode(
      {
        instruction: 1, // Swap instruction
        amountIn: new Numberu64(amountIn).toBuffer(),
        minimumAmountOut: new Numberu64(minimumAmountOut).toBuffer(),
        nonce: nonce,
        dexesConfig: 2,
        tokenSwapFlag: tsFlag,
        tokenSwapAccountsSize: tsKeys.length,
        tokenSwap1Flag: ts1Flag,
        tokenSwap1AccountsSize: tsKeys.length,
      },
      data,
    );

    const keys = [
      {pubkey: middleOwnerInfo, isSigner: false, isWritable: false},
      {pubkey: userTransferAuthority, isSigner: true, isWritable: false},
      {pubkey: userSource, isSigner: false, isWritable: true},
      {pubkey: middleSource, isSigner: false, isWritable: true},
      {pubkey: middleDestination, isSigner: false, isWritable: true},
      {pubkey: userDestination, isSigner: false, isWritable: true},
      {pubkey: tokenProgramId, isSigner: false, isWritable: false},

     
    ];
    for (var k of tsKeys) {
      keys.push(
        k,
      );
    };
    for (var k of ts1Keys) {
      keys.push(
        k,
      );
    };
    
    return new TransactionInstruction({
      keys,
      programId: protocolProgramId,
      data,
    });
  }
}
