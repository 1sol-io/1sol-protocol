import assert from 'assert';
import BN from 'bn.js';
import {Buffer} from 'buffer';
import * as BufferLayout from 'buffer-layout';
import type {Connection, TransactionSignature} from '@solana/web3.js';
import {
  SYSVAR_RENT_PUBKEY,
  Account,
  Keypair,
  Signer,
  AccountMeta,
  PublicKey,
  SystemProgram,
  Transaction,
  TransactionInstruction,
  sendAndConfirmTransaction,
} from '@solana/web3.js';
import {
  Market,
  OpenOrders,
} from '@project-serum/serum';
import {
  TokenSwapLayout
} from '@solana/spl-token-swap';

export const ONESOL_PROTOCOL_PROGRAM_ID: PublicKey = new PublicKey(
  '26XgL6X46AHxcMkfDNfnfQHrqZGzYEcTLj9SmAV5dLrV',
);

/**
 * Layout for a public key
 */
export const publicKeyLayout = (property: string = 'publicKey'): Object => {
  return BufferLayout.blob(32, property);
};

/**
 * Layout for a 64bit unsigned value
 */
export const uint64 = (property: string = 'uint64'): Object => {
  return BufferLayout.blob(8, property);
};

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


export async function loadAccount(
  connection: Connection,
  address: PublicKey,
  programId: PublicKey,
): Promise<Buffer> {
  const accountInfo = await connection.getAccountInfo(address);
  if (accountInfo === null) {
    throw new Error('Failed to find account');
  }

  if (!accountInfo.owner.equals(programId)) {
    throw new Error(`Invalid owner: ${JSON.stringify(accountInfo.owner)}`);
  }

  return Buffer.from(accountInfo.data);
}

export const OneSolProtocolLayout = BufferLayout.struct([
  BufferLayout.u8('version'),
  BufferLayout.u8('nonce'),
  publicKeyLayout('tokenProgramId'),
  publicKeyLayout('tokenAccount'),
  publicKeyLayout('mint'),
]);

export class TokenSwapInfo {
  constructor(
    public amountIn: Numberu64,
    public miniumAmountOut: Numberu64,
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

//
// *. CoinQty = maxBaseQuantity
// *. PcQty = maxQuoteQuantity
export class SerumDexMarketInfo {
  constructor(
    public programId: PublicKey,
    public market: Market,
    public limitPrice: Numberu64,
    public maxCoinQty: Numberu64,
    public maxPcQty: Numberu64,
    public clientId: Numberu64,
    public openOrderAccountKey?: PublicKey,
  ) {
    this.programId = programId;
    this.market = market;
    this.limitPrice = limitPrice;
    this.maxCoinQty = maxCoinQty;
    this.maxPcQty = maxPcQty;
    this.clientId = clientId;

    this.openOrderAccountKey = openOrderAccountKey;
  }

  public static create(market: Market, price: number, size: number, clientId: Numberu64): SerumDexMarketInfo {
    let limitPrice = market.priceNumberToLots(price);
    let maxBaseQuantity = market.baseSizeNumberToLots(size);
    let maxQuoteQuantity = new BN(market.decoded.quoteLotSize.toNumber()).mul(
      maxBaseQuantity.mul(limitPrice),
    );
    console.log("[SerumDexMarketInfo] maxQuoteQuantity: " + maxQuoteQuantity);
    console.log("[SerumDexMarketInfo] maxBaseQuantity: " + maxBaseQuantity);
    console.log("[SerumDexMarketInfo] limitPrice: " + limitPrice);
    return new SerumDexMarketInfo(
      market.programId,
      market,
      new Numberu64(limitPrice.toNumber()),
      new Numberu64(maxBaseQuantity.toNumber()),
      new Numberu64(maxQuoteQuantity.toNumber()),
      clientId,
    );
  }

  public side(sourceMint: PublicKey): number {
    if (this.market.baseMintAddress == sourceMint) {
      return 0;
    } 
    return 1;
  }

  dataLayout(): Array<any> {
    return [
      BufferLayout.u8('serumDexFlag'),
      BufferLayout.u8('serumDexAccountsSize'),
      BufferLayout.u8('serumDexSide'),
      uint64('serumDexPrice'),
      uint64('serumDexMaxCoinQty'),
      uint64('serumDexMaxPcQty'),
      uint64('serumDexClientId'),
    ];
  }

  dataMap(sourceMint: PublicKey) {
    return {
      serumDexFlag: 1,
      serumDexAccountsSize: 11,
      serumDexSide: this.side(sourceMint),
      serumDexPrice: this.limitPrice.toBuffer(),
      serumDexMaxCoinQty: this.maxCoinQty.toBuffer(),
      serumDexMaxPcQty: this.maxPcQty.toBuffer(),
      serumDexClientId: this.clientId.toBuffer(),
    }; 
  }

  async toKeys(): Promise<Array<AccountMeta>>{
    const vaultSigner = await PublicKey.createProgramAddress(
      [
        this.market.address.toBuffer(),
        this.market.decoded.vaultSignerNonce.toArrayLike(Buffer, 'le', 8),
      ],
      this.programId,
    );
    const keys = [
      {pubkey: this.market.publicKey, isSigner: false, isWritable: true},
      {pubkey: this.openOrderAccountKey, isSigner: false, isWritable: false},
      {pubkey: this.market.decoded.requestQueue, isSigner: false, isWritable: true},
      {pubkey: this.market.decoded.eventQueue, isSigner: false, isWritable: true},
      {pubkey: this.market.bidsAddress, isSigner: false, isWritable: true},
      {pubkey: this.market.asksAddress, isSigner: false, isWritable: true},
      {pubkey: this.market.decoded.baseVault, isSigner: false, isWritable: true},
      {pubkey: this.market.decoded.quoteVault, isSigner: false, isWritable: true},
      {pubkey: vaultSigner, isSigner: false, isWritable: false},
      {pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false},
      {pubkey: this.programId, isSigner: false, isWritable: false},
    ];
    return keys;
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
    public protocolInfo: PublicKey,
    public protocolProgramId: PublicKey,
    public tokenProgramId: PublicKey,
    public tokenAccountKey: PublicKey,
    public authority: PublicKey,
    public nonce: number,
    public wallet: PublicKey,
  ) {
    this.connection = connection;
    this.protocolInfo = protocolInfo;
    this.protocolProgramId = protocolProgramId;
    this.tokenProgramId = tokenProgramId;
    this.tokenAccountKey = tokenAccountKey;
    this.nonce = nonce;
    this.wallet = wallet;
  }

  static async loadOneSolProtocol(
    connection: Connection,
    address: PublicKey,
    programId: PublicKey,
    wallet: PublicKey,
  ): Promise<OneSolProtocol> {
    const data = await loadAccount(connection, address, programId);
    const onesolProtocolData = OneSolProtocolLayout.decode(data);
    if (onesolProtocolData.version != 1) {
      throw new Error(`Invalid OneSolProtocol data`);
    }
    const [authority] = await PublicKey.findProgramAddress(
      [address.toBuffer()],
      programId,
    )

    return new OneSolProtocol(
      connection,
      address,
      programId,
      new PublicKey(onesolProtocolData.tokenProgramId),
      new PublicKey(onesolProtocolData.tokenAccount),
      authority,
      onesolProtocolData.nonce,
      wallet,
    )

  }

  /**
   * Create a new OneSol Swap
   *
   * @param connection The connection to use
   * @param protocolAccountInfo The onesolProtocol account pubkey to use
   * @param tokenAccountKey The onesolProtocol token account pubkey to use
   * @param tokenProgramId The program ID of the token program
   * @param authority the authority over the swap and accounts
   * @param nonce The nonce used to generate the authority
   * @param payer the payer
   * @param protocolProgramID The  program ID of the onesolProtocol program
   * @return Token object for the newly minted token, Public key of the account holding the total supply of new tokens
   */
  static async createOneSolProtocol(
    connection: Connection,
    protocolAccountInfo: Account,
    tokenAccountKey: PublicKey,
    tokenProgramId: PublicKey,
    authority: PublicKey,
    nonce: number, 
    payer: Keypair,
    protocolProgramId: PublicKey,
  ): Promise<OneSolProtocol> {
    // let transaction;
    const onesolSwap = new OneSolProtocol(
      connection,
      protocolAccountInfo.publicKey,
      protocolProgramId,
      tokenProgramId,
      tokenAccountKey,
      authority,
      nonce,
      payer.publicKey,
    );

    // Allocate memory for the account
    const balanceNeeded = await OneSolProtocol.getMinBalanceRentForExemptTokenSwap(
      connection,
    );
    // console.log("balanceNeeded: " + balanceNeeded);
    // console.log("create onesolProgram account.");
    let transaction = new Transaction();
    transaction.add(
      SystemProgram.createAccount({
        fromPubkey: payer.publicKey,
        newAccountPubkey: protocolAccountInfo.publicKey,
        lamports: balanceNeeded,
        space: OneSolProtocolLayout.span,
        programId: protocolProgramId,
      }),
    );
    const instruction = OneSolProtocol.createInitSwapInstruction(
      protocolAccountInfo,
      authority,
      tokenAccountKey,
      protocolProgramId,
      tokenProgramId,
      nonce,
    );

    transaction.add(instruction);
    await realSendAndConfirmTransaction(
      'createAccount and InitializeSwap',
      connection,
      transaction,
      payer,
      protocolAccountInfo,
    );
    return onesolSwap;
  }

  /**
   Get the minimum balance for the token swap account to be rent exempt
  
   @return Number of lamports required
  **/
  static async getMinBalanceRentForExemptTokenSwap(
    connection: Connection,
  ): Promise<number> {
    return await connection.getMinimumBalanceForRentExemption(
      OneSolProtocolLayout.span,
    );
  }

  static createInitSwapInstruction(
    onesolProtocolAccount: Account,
    authority: PublicKey,
    tokenAccount: PublicKey,
    protocolProgramId: PublicKey,
    tokenProgramId: PublicKey,
    nonce: number,
  ): TransactionInstruction {
    const keys = [
      {pubkey: onesolProtocolAccount.publicKey, isSigner: false, isWritable: true},
      {pubkey: authority, isSigner: false, isWritable: false},
      {pubkey: tokenAccount, isSigner: false, isWritable: false},
      {pubkey: tokenProgramId, isSigner: false, isWritable: false},
    ];
    const commandDataLayout = BufferLayout.struct([
      BufferLayout.u8('instruction'),
      BufferLayout.u8('nonce'),
    ]);
    let data = Buffer.alloc(1024);
    {
      const encodeLength = commandDataLayout.encode({
        instruction: 0,
        nonce,
      }, data);
      data = data.slice(0, encodeLength);
    }
    return new TransactionInstruction({
      keys,
      programId: protocolProgramId,
      data,
    })
  }

  /**
   * Swap token A for token B
   *
   * @param userSource User's source token account
   * @param userDestination User's destination token account
   * @param amountIn Amount to transfer from source account
   * @param minimumAmountOut Minimum amount of tokens the user will receive
   * @param TokenSwapInfo  nullable
   * @param SerumDexMarketInfo nullable
   */
  async swap(
    userSource: PublicKey,
    sourceMint: PublicKey,
    userDestination: PublicKey,
    minimumAmountOut: number | Numberu64,
    splTokenSwapInfo: TokenSwapInfo | null,
    serumDexTradeInfo: SerumDexMarketInfo | null,
    payer: Keypair,
  ): Promise<TransactionSignature> {
    if (splTokenSwapInfo === null && serumDexTradeInfo === null) {
      throw new Error('One of splTokenSwapInfo and serumDexInfo is must not be null');
    }
    let transaction = new Transaction();
    const signers: Array<Signer> = [];
    let ins = await this.createSwapInstruction(
      userSource,
      sourceMint,
      userDestination,
      minimumAmountOut,
      splTokenSwapInfo,
      serumDexTradeInfo,
      signers,
    )
    transaction.add(
      ins,
    )
    signers.push(payer);
    // console.log("signers length: " + signers.length);
    return await realSendAndConfirmTransaction(
      'swap',
      this.connection, 
      transaction,
      ...signers,
    )
  }


  async createSwapInstruction(
    userSource: PublicKey,
    sourceMint: PublicKey,
    userDestination: PublicKey,
    minimumAmountOut: number | Numberu64,
    splTokenSwapInfo: TokenSwapInfo | null,
    serumDexTradeInfo: SerumDexMarketInfo | null,
    signers: Array<Signer>,
  ): Promise<TransactionInstruction> {
    if (splTokenSwapInfo === null && serumDexTradeInfo === null) {
      throw new Error('One of splTokenSwapInfo and serumDexInfo is must not be null');
    }
    let transaction = new Transaction();
    if (serumDexTradeInfo !== null) {
      let market = serumDexTradeInfo.market;
      let orders = await serumDexTradeInfo.market.findOpenOrdersAccountsForOwner(
        this.connection, this.wallet
      );
      console.log("orders length: " + orders.length);
      if (orders.length === 0) {
        let openOrderAccount = new Account(); 
        transaction.add(await OpenOrders.makeCreateAccountTransaction(
          this.connection,
          market.address,
          this.wallet,
          openOrderAccount.publicKey,
          market.programId
        ));
        // console.log("makeCreateAccountTransaction.market: " + market.address);
        // console.log("makeCreateAccountTransaction.payer: " + this.payer.publicKey);
        // console.log("makeCreateAccountTransaction.openOrderAccount: " + openOrderAccount.publicKey);
        // console.log("makeCreateAccountTransaction.programId: " + market.programId);
        serumDexTradeInfo.openOrderAccountKey = openOrderAccount.publicKey;
        signers.push(openOrderAccount);
      }
      // let openOrderAccount = serumDexTradeInfo.data.openOrdersAccount;
    }
    return await OneSolProtocol.swapInstruction(
        this.protocolInfo,
        this.wallet,
        this.authority,
        this.tokenAccountKey,
        userSource,
        sourceMint,
        userDestination,
        this.tokenProgramId,
        splTokenSwapInfo,
        serumDexTradeInfo,
        this.protocolProgramId,
        minimumAmountOut,
      )
  }

  static async swapInstruction(
    protocolAccount: PublicKey,
    owner: PublicKey,
    authority: PublicKey,
    protocolToken: PublicKey,
    userSource: PublicKey,
    sourceMint: PublicKey,
    userDestination: PublicKey,
    tokenProgramId: PublicKey,
    splTokenSwapInfo: TokenSwapInfo | null,
    serumDexInfo: SerumDexMarketInfo | null,
    protocolProgramId: PublicKey,
    minimumAmountOut: number | Numberu64,
  ): Promise<TransactionInstruction> {

    const bflStruct: any = [
      BufferLayout.u8('instruction'),
      uint64('minimumAmountOut'),
    ];
    // let dataMap: any = {};
    let dataMap: any = {
      instruction: 1, // Swap instruction
      minimumAmountOut: new Numberu64(minimumAmountOut).toBuffer(),
    };

    const keys = [
      {pubkey: protocolAccount, isSigner: false, isWritable: false},
      {pubkey: authority, isSigner: false, isWritable: false},
      {pubkey: owner, isSigner: true, isWritable: false},
      {pubkey: protocolToken, isSigner: false, isWritable: true},
      {pubkey: userSource, isSigner: false, isWritable: true},
      {pubkey: userDestination, isSigner: false, isWritable: true},
      {pubkey: tokenProgramId, isSigner: false, isWritable: false},
    ];

    if (splTokenSwapInfo !== null) {
      const swapKeys = splTokenSwapInfo.toKeys();
      keys.push(...swapKeys);
      bflStruct.push(
        BufferLayout.u8('splTokenSwapFlag'),
        BufferLayout.u8('splTokenSwapAccountsSize'),
        uint64('splTokenSwapAmountIn'),
        uint64('splTokenSwapMinimumAmountOut'),
      );
      dataMap = {
        ...dataMap,
        splTokenSwapFlag: 1,
        splTokenSwapAccountsSize: swapKeys.length,
        splTokenSwapAmountIn: splTokenSwapInfo.amountIn.toBuffer(),
        splTokenSwapMinimumAmountOut: splTokenSwapInfo.miniumAmountOut.toBuffer(),
      };
      
    } else {
      bflStruct.push(
        BufferLayout.u8('splTokenSwapFlag'),
      ); 
      dataMap = {
        ...dataMap,
        splTokenSwapFlag: 0,
      }
    }
    if (serumDexInfo !== null) {
      const swapKeys = await serumDexInfo.toKeys();
      keys.push(...swapKeys);
      bflStruct.push(...serumDexInfo.dataLayout());
      dataMap = {
        ...dataMap,
        ...serumDexInfo.dataMap(sourceMint),
      };
    } else {
      bflStruct.push(
        BufferLayout.u8('serumDexFlag'),
      ); 
      dataMap = {
        ...dataMap,
        serumDexFlag: 0,
      } 
    }
    const dataLayout = BufferLayout.struct(bflStruct);
    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode(dataMap, data);

    return new TransactionInstruction({
      keys,
      programId: protocolProgramId,
      data,
    });
  }
}

export function realSendAndConfirmTransaction(
  title: string,
  connection: Connection,
  transaction: Transaction,
  ...signers: Array<Signer>
): Promise<TransactionSignature> {
  return sendAndConfirmTransaction(connection, transaction, signers, {
    skipPreflight: false,
    commitment: 'recent',
    preflightCommitment: 'recent',
  });
}

export function deserializeAccount(info: any) {
  const data = OneSolProtocolLayout.decode(info.account.data);

  const details = {
    pubkey: info.pubkey,
    account: {
      ...info.account,
    },
    info: data,
  };

  return details;
}

export async function loadTokenSwapInfo(
  connection: Connection,
  address: PublicKey,
  programId: PublicKey,
  amountIn: Numberu64,
  minimumAmountOut: Numberu64,
  hostFeeAccount: PublicKey | null,
): Promise<TokenSwapInfo> {
  const data = await loadAccount(connection, address, programId);
  const tokenSwapData = TokenSwapLayout.decode(data);

  if (!tokenSwapData.isInitialized) {
    throw new Error(`Invalid token swap state`);
  }

  const [authority] = await PublicKey.findProgramAddress(
    [address.toBuffer()],
    programId,
  );

  const poolToken = new PublicKey(tokenSwapData.tokenPool);
  const feeAccount = new PublicKey(tokenSwapData.feeAccount);
  const tokenAccountA = new PublicKey(tokenSwapData.tokenAccountA);
  const tokenAccountB = new PublicKey(tokenSwapData.tokenAccountB);

  return new TokenSwapInfo(
    amountIn,
    minimumAmountOut,
    programId,
    address,
    authority,
    tokenAccountA,
    tokenAccountB,
    poolToken,
    feeAccount,
    hostFeeAccount
  );
}

export async function getOneSolProtocol(
  onesolProtocolData: any,
  connection: Connection,
  address: PublicKey,
  programId: PublicKey,
  wallet: PublicKey 
): Promise<OneSolProtocol> {
  if (onesolProtocolData.version !== 1) {
    throw new Error(`Invalid OneSolProtocol data`);
  }

  const [authority] = await PublicKey.findProgramAddress(
    [address.toBuffer()],
    programId,
  )

  return new OneSolProtocol(
    connection,
    address,
    programId,
    new PublicKey(onesolProtocolData.tokenProgramId),
    new PublicKey(onesolProtocolData.tokenAccount),
    authority,
    onesolProtocolData.nonce,
    wallet,
  )
}
