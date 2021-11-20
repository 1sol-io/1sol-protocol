import assert from "assert";
import BN, { min } from "bn.js";
import bs58 from "bs58";
import { Buffer } from "buffer";
import * as BufferLayout from "buffer-layout";
import {
  AccountInfo,
  Connection,
  Keypair,
  SystemInstruction,
  SystemProgram,
  SYSVAR_CLOCK_PUBKEY,
  TransactionSignature,
} from "@solana/web3.js";
import {
  SYSVAR_RENT_PUBKEY,
  Signer,
  AccountMeta,
  PublicKey,
  Transaction,
  TransactionInstruction,
  sendAndConfirmTransaction,
} from "@solana/web3.js";
import { Market } from "@project-serum/serum";
import { TokenSwapLayout } from "@solana/spl-token-swap";
import {
  MintInfo as TokenMint,
  MintLayout as TokenMintLayout,
  Token,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import {
  StableSwapLayout,
  SWAP_PROGRAM_ID as STABLE_SWAP_PROGRAM_ID,
} from "@saberhq/stableswap-sdk";

export const ONESOL_PROTOCOL_PROGRAM_ID: PublicKey = new PublicKey(
  "1SoLTvbiicqXZ3MJmnTL2WYXKLYpuxwHpa4yYrVQaMZ"
);
export const RAYDIUM_PROGRAM_ID: PublicKey = new PublicKey(
  '675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8'
);

/**
 * Layout for a public key
 */
export const publicKeyLayout = (property: string = "publicKey"): Object => {
  return BufferLayout.blob(32, property);
};

/**
 * Layout for a 64bit unsigned value
 */
export const uint64 = (property: string = "uint64"): Object => {
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
    assert(b.length < 8, "Numberu64 too large");

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
        .map((i) => `00${i.toString(16)}`.slice(-2))
        .join(""),
      16
    );
  }
}

export interface TokenMintInfo {
  pubkey: PublicKey;
  mintInfo: TokenMint;
}

export async function loadAccount(
  connection: Connection,
  address: PublicKey,
  programId: PublicKey
): Promise<Buffer> {
  const accountInfo = await connection.getAccountInfo(address);
  if (accountInfo === null) {
    throw new Error("Failed to find account");
  }

  if (!accountInfo.owner.equals(programId)) {
    throw new Error(`Invalid owner: ${JSON.stringify(accountInfo.owner.toBase58())}`);
  }

  return Buffer.from(accountInfo.data);
}

export const DexMarketInfoLayout = BufferLayout.struct([
  BufferLayout.u8("isInitialized"),
  BufferLayout.u8("status"),
  BufferLayout.u8("nonce"),
  publicKeyLayout("market"),
  publicKeyLayout("pcMint"),
  publicKeyLayout("coinMint"),
  publicKeyLayout("openOrders"),
  publicKeyLayout("dexProgramId"),
]);

export const SwapInfoLayout = BufferLayout.struct([
  BufferLayout.u8("isInitialized"),
  BufferLayout.u8("status"),
  uint64("tokenLatestAmount"),
  publicKeyLayout("owner"),
  BufferLayout.u32("tokenAccountOption"),
  publicKeyLayout("tokenAccount")
]);

export const RaydiumLiquidityStateLayout = BufferLayout.struct([
  uint64("status"),
  uint64("nonce"),
  uint64("maxOrder"),
  uint64("depth"),
  uint64("baseDecimal"),
  uint64("quoteDecimal"),
  uint64("state"),
  uint64("resetFlag"),
  uint64("minSize"),
  uint64("volMaxCutRatio"),
  uint64("amountWaveRatio"),
  uint64("baseLotSize"),
  uint64("quoteLotSize"),
  uint64("minPriceMultiplier"),
  uint64("maxPriceMultiplier"),
  uint64("systemDecimalValue"),
  uint64("minSeparateNumerator"),
  uint64("minSeparateDenominator"),
  uint64("tradeFeeNumerator"),
  uint64("tradeFeeDenominator"),
  uint64("pnlNumerator"),
  uint64("pnlDenominator"),
  uint64("swapFeeNumerator"),
  uint64("swapFeeDenominator"),
  uint64("baseNeedTakePnl"),
  uint64("quoteNeedTakePnl"),
  uint64("quoteTotalPnl"),
  uint64("baseTotalPnl"),
  BufferLayout.blob(16, "quoteTotalDeposited"),
  BufferLayout.blob(16, "baseTotalDeposited"),
  BufferLayout.blob(16, "swapBaseInAmount"),
  BufferLayout.blob(16, "swapQuoteOutAmount"),
  uint64("swapBase2QuoteFee"),
  BufferLayout.blob(16, "swapQuoteInAmount"),
  BufferLayout.blob(16, "swapBaseOutAmount"),
  uint64("swapQuote2BaseFee"),
  // amm vault
  publicKeyLayout("baseVault"),
  publicKeyLayout("quoteVault"),
  // mint
  publicKeyLayout("baseMint"),
  publicKeyLayout("quoteMint"),
  publicKeyLayout("lpMint"),
  // market
  publicKeyLayout("openOrders"),
  publicKeyLayout("marketId"),
  publicKeyLayout("marketProgramId"),
  publicKeyLayout("targetOrders"),
  publicKeyLayout("withdrawQueue"),
  publicKeyLayout("tempLpVault"),
  publicKeyLayout("owner"),
  publicKeyLayout("pnlOwner"),
]);

export interface SwapInfo {
  pubkey: PublicKey;
  programId: PublicKey;
  isInitialized: number;
  status: number;
  tokenLatestAmount: Numberu64;
  owner: PublicKey;
  tokenAccount: PublicKey | null;
}

export class TokenSwapInfo {
  constructor(
    private programId: PublicKey,
    private swapInfo: PublicKey,
    private authority: PublicKey,
    private tokenAccountA: PublicKey,
    private tokenAccountB: PublicKey,
    private mintA: PublicKey,
    private mintB: PublicKey,
    private poolMint: PublicKey,
    private feeAccount: PublicKey,
  ) {
    this.programId = programId;
    this.swapInfo = swapInfo;
    this.authority = authority;
    this.tokenAccountA = tokenAccountA;
    this.tokenAccountB = tokenAccountB;
    this.mintA = mintA;
    this.mintB = mintB;
    this.poolMint = poolMint;
    this.feeAccount = feeAccount;
  }

  toKeys(): Array<AccountMeta> {
    const keys = [
      { pubkey: this.swapInfo, isSigner: false, isWritable: false },
      { pubkey: this.authority, isSigner: false, isWritable: false },
      { pubkey: this.tokenAccountA, isSigner: false, isWritable: true },
      { pubkey: this.tokenAccountB, isSigner: false, isWritable: true },
      { pubkey: this.poolMint, isSigner: false, isWritable: true },
      { pubkey: this.feeAccount, isSigner: false, isWritable: true },
      { pubkey: this.programId, isSigner: false, isWritable: false },
    ];
    return keys;
  }
}

//
export class SerumDexMarketInfo {
  constructor(
    private market: PublicKey,
    private requestQueue: PublicKey,
    private eventQueue: PublicKey,
    private bids: PublicKey,
    private asks: PublicKey,
    private coinVault: PublicKey,
    private pcVault: PublicKey,
    private vaultSigner: PublicKey,
    private openOrders: PublicKey,
    private programId: PublicKey
  ) {
    this.market = market;
    this.requestQueue = requestQueue;
    this.eventQueue = eventQueue;
    this.bids = bids;
    this.asks = asks;
    this.coinVault = coinVault;
    this.pcVault = pcVault;
    this.vaultSigner = vaultSigner;
    this.openOrders = openOrders;
    this.programId = programId;
  }

  toKeys(): Array<AccountMeta> {
    const keys = [
      { pubkey: this.market, isSigner: false, isWritable: true },
      {
        pubkey: this.requestQueue,
        isSigner: false,
        isWritable: true,
      },
      {
        pubkey: this.eventQueue,
        isSigner: false,
        isWritable: true,
      },
      { pubkey: this.bids, isSigner: false, isWritable: true },
      { pubkey: this.asks, isSigner: false, isWritable: true },
      {
        pubkey: this.coinVault,
        isSigner: false,
        isWritable: true,
      },
      {
        pubkey: this.pcVault,
        isSigner: false,
        isWritable: true,
      },
      { pubkey: this.vaultSigner, isSigner: false, isWritable: false },
      { pubkey: this.openOrders, isSigner: false, isWritable: true },
      { pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false },
      { pubkey: this.programId, isSigner: false, isWritable: false },
    ];
    return keys;
  }
}

export class SaberStableSwapInfo {
  constructor(
    private programId: PublicKey,
    private swapInfo: PublicKey,
    private authority: PublicKey,
    private tokenAccountA: PublicKey,
    private mintA: PublicKey,
    private adminFeeAccountA: PublicKey,
    private tokenAccountB: PublicKey,
    private mintB: PublicKey,
    private adminFeeAccountB: PublicKey,
  ) {
    this.programId = programId;
    this.swapInfo = swapInfo;
    this.authority = authority;
    this.tokenAccountA = tokenAccountA;
    this.tokenAccountB = tokenAccountB;
    this.adminFeeAccountA = adminFeeAccountA;
    this.adminFeeAccountB = adminFeeAccountB;
  }

  toKeys(sourceMint: PublicKey): Array<AccountMeta> {
    const keys = [
      { pubkey: this.swapInfo, isSigner: false, isWritable: false },
      { pubkey: this.authority, isSigner: false, isWritable: false },
      { pubkey: this.tokenAccountA, isSigner: false, isWritable: true },
      { pubkey: this.tokenAccountB, isSigner: false, isWritable: true },
    ];

    if (sourceMint.equals(this.mintA)) {
      keys.push(
        { pubkey: this.adminFeeAccountB, isSigner: false, isWritable: true },
      );
    } else {
      keys.push(
        { pubkey: this.adminFeeAccountA, isSigner: false, isWritable: true },
      );
    }
    keys.push(
      { pubkey: SYSVAR_CLOCK_PUBKEY, isSigner: false, isWritable: false },
      { pubkey: this.programId, isSigner: false, isWritable: false },
    );
    return keys;
  }
}

export class RaydiumAmmInfo {
  constructor(
    private programId: PublicKey,
    private ammInfo: PublicKey,
    private authority: PublicKey,
    private ammOpenOrders: PublicKey,
    private ammTargetOrders: PublicKey,
    private poolCoinTokenAccount: PublicKey,
    private poolPcTokenAccount: PublicKey,
    private serumProgramId: PublicKey,
    private serumMarket: PublicKey,
    private serumBids: PublicKey,
    private serumAsks: PublicKey,
    private serumEventQueue: PublicKey,
    private serumCoinVaultAccount: PublicKey,
    private serumPcVaultAccount: PublicKey,
    private serumVaultSigner: PublicKey,
  ) {
    this.programId = programId;
    this.ammInfo = ammInfo;
    this.authority = authority;
    this.ammOpenOrders = ammOpenOrders;
    this.ammTargetOrders = ammTargetOrders;
    this.poolCoinTokenAccount = poolCoinTokenAccount;
    this.poolPcTokenAccount = poolPcTokenAccount;
    this.serumProgramId = serumProgramId;
    this.serumMarket = serumMarket;
    this.serumBids = serumBids;
    this.serumAsks = serumAsks;
    this.serumEventQueue = serumEventQueue;
    this.serumCoinVaultAccount = serumCoinVaultAccount;
    this.serumPcVaultAccount = serumPcVaultAccount;
    this.serumVaultSigner = serumVaultSigner;
  }

  toKeys(): Array<AccountMeta> {
    const keys = [
      { pubkey: this.ammInfo, isSigner: false, isWritable: true },
      { pubkey: this.authority, isSigner: false, isWritable: false },
      { pubkey: this.ammOpenOrders, isSigner: false, isWritable: true },
      { pubkey: this.ammTargetOrders, isSigner: false, isWritable: true },
      { pubkey: this.poolCoinTokenAccount, isSigner: false, isWritable: true },
      { pubkey: this.poolPcTokenAccount, isSigner: false, isWritable: true },
      { pubkey: this.serumProgramId, isSigner: false, isWritable: false },
      { pubkey: this.serumMarket, isSigner: false, isWritable: true },
      { pubkey: this.serumBids, isSigner: false, isWritable: true },
      { pubkey: this.serumAsks, isSigner: false, isWritable: true },
      { pubkey: this.serumEventQueue, isSigner: false, isWritable: true },
      { pubkey: this.serumCoinVaultAccount, isSigner: false, isWritable: true },
      { pubkey: this.serumPcVaultAccount, isSigner: false, isWritable: true },
      { pubkey: this.serumVaultSigner, isSigner: false, isWritable: false },
      { pubkey: this.programId, isSigner: false, isWritable: false },
    ];
    return keys;
  }
}

/**
 * A program to exchange tokens against a pool of liquidity
 */
export class OneSolProtocol {
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
    public programId: PublicKey,
    public tokenProgramId: PublicKey,
    public wallet: PublicKey
  ) {
    this.connection = connection;
    this.programId = programId;
    this.tokenProgramId = tokenProgramId;
    this.wallet = wallet;
  }

  /**
   * findOneSolProtocol instance
   * @param connection
   * @param walletAddress
   * @param pcMintKey
   * @param coinMintKey
   * @param wallet
   * @param programId
   * @returns
   */
  static async createOneSolProtocol({
    connection,
    wallet,
    programId = ONESOL_PROTOCOL_PROGRAM_ID,
  }: {
    connection: Connection;
    wallet: PublicKey;
    programId?: PublicKey;
  }): Promise<OneSolProtocol> {
    return new OneSolProtocol(connection, programId, TOKEN_PROGRAM_ID, wallet);
  }

  async findSwapInfo({
    wallet,
  }: {
    wallet: PublicKey,
  }): Promise<SwapInfo|null> {
    const [accountItem] = await this.connection.getProgramAccounts(this.programId, {
      filters: [
        {
          dataSize: SwapInfoLayout.span,
        },
        {
          memcmp: {
            offset: SwapInfoLayout.offsetOf('owner'),
            bytes: wallet.toBase58(),
          },
        },
      ],
    });

    if (!accountItem) {
      return null
    }
    const { pubkey, account } = accountItem;
    const decoded = SwapInfoLayout.decode(account.data);
    const tokenAccount = decoded.tokenAccountOption === 0 ? null : new PublicKey(decoded.tokenAccount);
    return {
      pubkey,
      programId: account.owner,
      isInitialized: decoded.isInitialized,
      status: decoded.status,
      tokenLatestAmount: decoded.tokenLatestAmount,
      owner: new PublicKey(decoded.owner),
      tokenAccount,
    }
  }

  async createSwapInfo(
    { instructions, signers, owner }: {
      owner: PublicKey;
      instructions: Array<TransactionInstruction>,
      signers: Array<Signer>,
    }): Promise<PublicKey> {

    const swapInfoAccount = Keypair.generate();

    const dataLayout = BufferLayout.struct([
      BufferLayout.u8("instruction"),
    ]);
    const dataMap: any = {
      instruction: 10, // Swap instruction
    };
    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode(dataMap, data);

    const keys = [
      { pubkey: swapInfoAccount.publicKey, isSigner: true, isWritable: true },
      { pubkey: owner, isSigner: true, isWritable: false },
    ];


    instructions.push(new TransactionInstruction({
      keys,
      programId: this.programId,
      data,
    }));
    signers.push(swapInfoAccount);
    return swapInfoAccount.publicKey;
  }

  async setupSwapInfo(
    { swapInfo, tokenAccount, instructions, signers }: {
      swapInfo: PublicKey,
      tokenAccount: PublicKey,
      instructions: Array<TransactionInstruction>,
      signers: Array<Signer>,
    }
  ) {
    const keys = [
      { pubkey: swapInfo, isSigner: false, isWritable: true },
      { pubkey: tokenAccount, isSigner: false, isWritable: true },
    ];
    const dataLayout = BufferLayout.struct([
      BufferLayout.u8("instruction"),
    ]);
    const dataMap: any = {
      instruction: 11,
    };
    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode(dataMap, data);

    instructions.push(new TransactionInstruction({
      keys,
      programId: this.programId,
      data,
    }));
  }

  async createSwapByTokenSwapInstruction(
    {
      fromTokenAccountKey,
      toTokenAccountKey,
      fromMintKey,
      toMintKey,
      userTransferAuthority,
      feeTokenAccount,
      amountIn,
      expectAmountOut,
      minimumAmountOut,
      splTokenSwapInfo,
    }: {
      fromTokenAccountKey: PublicKey;
      toTokenAccountKey: PublicKey;
      fromMintKey: PublicKey;
      toMintKey: PublicKey;
      userTransferAuthority: PublicKey;
      feeTokenAccount: PublicKey;
      amountIn: Numberu64;
      expectAmountOut: Numberu64;
      minimumAmountOut: Numberu64;
      splTokenSwapInfo: TokenSwapInfo;
    },
    instructions: Array<TransactionInstruction>,
    signers: Array<Signer>
  ): Promise<void> {
    instructions.push(
      await OneSolProtocol.makeSwapByTokenSwapInstruction({
        sourceTokenKey: fromTokenAccountKey,
        sourceMint: fromMintKey,
        destinationTokenKey: toTokenAccountKey,
        destinationMint: toMintKey,
        transferAuthority: userTransferAuthority,
        feeTokenAccount: feeTokenAccount,
        tokenProgramId: this.tokenProgramId,
        splTokenSwapInfo: splTokenSwapInfo,
        amountIn: amountIn,
        expectAmountOut: expectAmountOut,
        minimumAmountOut: minimumAmountOut,
        programId: this.programId,
      })
    );
  }

  static async makeSwapByTokenSwapInstruction({
    sourceTokenKey,
    sourceMint,
    destinationTokenKey,
    destinationMint,
    transferAuthority,
    tokenProgramId,
    feeTokenAccount,
    splTokenSwapInfo,
    amountIn,
    expectAmountOut,
    minimumAmountOut,
    programId = ONESOL_PROTOCOL_PROGRAM_ID,
  }: {
    sourceTokenKey: PublicKey;
    sourceMint: PublicKey;
    destinationTokenKey: PublicKey;
    destinationMint: PublicKey;
    transferAuthority: PublicKey;
    tokenProgramId: PublicKey;
    feeTokenAccount: PublicKey;
    splTokenSwapInfo: TokenSwapInfo;
    amountIn: Numberu64;
    expectAmountOut: Numberu64;
    minimumAmountOut: Numberu64;
    programId?: PublicKey;
  }): Promise<TransactionInstruction> {

    const dataLayout = BufferLayout.struct([
      BufferLayout.u8("instruction"),
      uint64("amountIn"),
      uint64("expectAmountOut"),
      uint64("minimumAmountOut"),
    ]);

    let dataMap: any = {
      instruction: 3, // Swap instruction
      amountIn: amountIn.toBuffer(),
      expectAmountOut: expectAmountOut.toBuffer(),
      minimumAmountOut: minimumAmountOut.toBuffer(),
    };

    const keys = [
      { pubkey: sourceTokenKey, isSigner: false, isWritable: true },
      { pubkey: destinationTokenKey, isSigner: false, isWritable: true },
      { pubkey: transferAuthority, isSigner: true, isWritable: false },
      { pubkey: tokenProgramId, isSigner: false, isWritable: false },
      { pubkey: feeTokenAccount, isSigner: false, isWritable: true }
    ];

    const swapKeys = splTokenSwapInfo.toKeys();
    keys.push(...swapKeys);

    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode(dataMap, data);

    return new TransactionInstruction({
      keys,
      programId: programId,
      data,
    });
  }

  async createSwapInByTokenSwapInstruction(
    {
      fromTokenAccountKey,
      toTokenAccountKey,
      fromMintKey,
      toMintKey,
      userTransferAuthority,
      swapInfo,
      amountIn,
      splTokenSwapInfo,
    }: {
      fromTokenAccountKey: PublicKey;
      toTokenAccountKey: PublicKey;
      fromMintKey: PublicKey;
      toMintKey: PublicKey;
      userTransferAuthority: PublicKey;
      swapInfo: PublicKey,
      amountIn: Numberu64;
      splTokenSwapInfo: TokenSwapInfo;
    },
    instructions: Array<TransactionInstruction>,
    signers: Array<Signer>
  ): Promise<void> {
    instructions.push(
      await OneSolProtocol.makeSwapInByTokenSwapInstruction({
        sourceTokenKey: fromTokenAccountKey,
        sourceMint: fromMintKey,
        destinationTokenKey: toTokenAccountKey,
        destinationMint: toMintKey,
        transferAuthority: userTransferAuthority,
        swapInfo: swapInfo,
        tokenProgramId: this.tokenProgramId,
        splTokenSwapInfo: splTokenSwapInfo,
        amountIn: amountIn,
        programId: this.programId,
      })
    );
  }

  static async makeSwapInByTokenSwapInstruction({
    sourceTokenKey,
    sourceMint,
    destinationTokenKey,
    destinationMint,
    transferAuthority,
    tokenProgramId,
    swapInfo,
    splTokenSwapInfo,
    amountIn,
    programId = ONESOL_PROTOCOL_PROGRAM_ID,
  }: {
    sourceTokenKey: PublicKey;
    sourceMint: PublicKey;
    destinationTokenKey: PublicKey;
    destinationMint: PublicKey;
    transferAuthority: PublicKey;
    tokenProgramId: PublicKey;
    swapInfo: PublicKey,
    splTokenSwapInfo: TokenSwapInfo;
    amountIn: Numberu64;
    programId?: PublicKey;
  }): Promise<TransactionInstruction> {

    const dataLayout = BufferLayout.struct([
      BufferLayout.u8("instruction"),
      uint64("amountIn"),
    ]);

    let dataMap: any = {
      instruction: 12, // Swap instruction
      amountIn: amountIn.toBuffer(),
    };

    const keys = [
      { pubkey: sourceTokenKey, isSigner: false, isWritable: true },
      { pubkey: destinationTokenKey, isSigner: false, isWritable: true },
      { pubkey: transferAuthority, isSigner: true, isWritable: false },
      { pubkey: swapInfo, isSigner: false, isWritable: true },
      { pubkey: tokenProgramId, isSigner: false, isWritable: false },
    ];

    const swapKeys = splTokenSwapInfo.toKeys();
    keys.push(...swapKeys);

    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode(dataMap, data);

    return new TransactionInstruction({
      keys,
      programId: programId,
      data,
    });
  }

  async createSwapOutByTokenSwapInstruction(
    {
      fromTokenAccountKey,
      toTokenAccountKey,
      fromMintKey,
      toMintKey,
      userTransferAuthority,
      feeTokenAccount,
      swapInfo,
      expectAmountOut,
      minimumAmountOut,
      splTokenSwapInfo,
    }: {
      fromTokenAccountKey: PublicKey;
      toTokenAccountKey: PublicKey;
      fromMintKey: PublicKey;
      toMintKey: PublicKey;
      userTransferAuthority: PublicKey;
      feeTokenAccount: PublicKey;
      swapInfo: PublicKey,
      expectAmountOut: Numberu64;
      minimumAmountOut: Numberu64;
      splTokenSwapInfo: TokenSwapInfo;
    },
    instructions: Array<TransactionInstruction>,
    signers: Array<Signer>
  ): Promise<void> {
    instructions.push(
      await OneSolProtocol.makeSwapOutByTokenSwapInstruction({
        sourceTokenKey: fromTokenAccountKey,
        sourceMint: fromMintKey,
        destinationTokenKey: toTokenAccountKey,
        destinationMint: toMintKey,
        transferAuthority: userTransferAuthority,
        feeTokenAccount: feeTokenAccount,
        swapInfo: swapInfo,
        tokenProgramId: this.tokenProgramId,
        splTokenSwapInfo: splTokenSwapInfo,
        expectAmountOut: expectAmountOut,
        minimumAmountOut: minimumAmountOut,
        programId: this.programId,
      })
    );
  }

  static async makeSwapOutByTokenSwapInstruction({
    sourceTokenKey,
    sourceMint,
    destinationTokenKey,
    destinationMint,
    transferAuthority,
    tokenProgramId,
    feeTokenAccount,
    splTokenSwapInfo,
    swapInfo,
    expectAmountOut,
    minimumAmountOut,
    programId = ONESOL_PROTOCOL_PROGRAM_ID,
  }: {
    sourceTokenKey: PublicKey;
    sourceMint: PublicKey;
    destinationTokenKey: PublicKey;
    destinationMint: PublicKey;
    transferAuthority: PublicKey;
    tokenProgramId: PublicKey;
    feeTokenAccount: PublicKey;
    splTokenSwapInfo: TokenSwapInfo;
    swapInfo: PublicKey,
    expectAmountOut: Numberu64;
    minimumAmountOut: Numberu64;
    programId?: PublicKey;
  }): Promise<TransactionInstruction> {

    const dataLayout = BufferLayout.struct([
      BufferLayout.u8("instruction"),
      uint64("expectAmountOut"),
      uint64("minimumAmountOut"),
    ]);

    let dataMap: any = {
      instruction: 13, // Swap instruction
      expectAmountOut: expectAmountOut.toBuffer(),
      minimumAmountOut: minimumAmountOut.toBuffer(),
    };

    const keys = [
      { pubkey: sourceTokenKey, isSigner: false, isWritable: true },
      { pubkey: destinationTokenKey, isSigner: false, isWritable: true },
      { pubkey: transferAuthority, isSigner: true, isWritable: false },
      { pubkey: swapInfo, isSigner: false, isWritable: true },
      { pubkey: tokenProgramId, isSigner: false, isWritable: false },
      { pubkey: feeTokenAccount, isSigner: false, isWritable: true }
    ];

    const swapKeys = splTokenSwapInfo.toKeys();
    keys.push(...swapKeys);

    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode(dataMap, data);

    return new TransactionInstruction({
      keys,
      programId: programId,
      data,
    });
  }


  async createSwapBySaberStableSwapInstruction(
    {
      fromTokenAccountKey,
      toTokenAccountKey,
      fromMintKey,
      toMintKey,
      userTransferAuthority,
      feeTokenAccount,
      amountIn,
      expectAmountOut,
      minimumAmountOut,
      stableSwapInfo,
    }: {
      fromTokenAccountKey: PublicKey;
      toTokenAccountKey: PublicKey;
      fromMintKey: PublicKey;
      toMintKey: PublicKey;
      userTransferAuthority: PublicKey;
      feeTokenAccount: PublicKey;
      amountIn: Numberu64;
      expectAmountOut: Numberu64;
      minimumAmountOut: Numberu64;
      stableSwapInfo: SaberStableSwapInfo;
    },
    instructions: Array<TransactionInstruction>,
    signers: Array<Signer>
  ): Promise<void> {
    instructions.push(
      await OneSolProtocol.makeSwapBySaberStableSwapInstruction({
        sourceTokenKey: fromTokenAccountKey,
        sourceMint: fromMintKey,
        destinationTokenKey: toTokenAccountKey,
        destinationMint: toMintKey,
        transferAuthority: userTransferAuthority,
        tokenProgramId: this.tokenProgramId,
        feeTokenAccount: feeTokenAccount,
        stableSwapInfo: stableSwapInfo,
        amountIn: amountIn,
        expectAmountOut: expectAmountOut,
        minimumAmountOut: minimumAmountOut,
        programId: this.programId,
      })
    );
  }

  static async makeSwapBySaberStableSwapInstruction({
    sourceTokenKey,
    sourceMint,
    destinationTokenKey,
    destinationMint,
    transferAuthority,
    tokenProgramId,
    feeTokenAccount,
    stableSwapInfo,
    amountIn,
    expectAmountOut,
    minimumAmountOut,
    programId = ONESOL_PROTOCOL_PROGRAM_ID,
  }: {
    sourceTokenKey: PublicKey;
    sourceMint: PublicKey;
    destinationTokenKey: PublicKey;
    destinationMint: PublicKey;
    transferAuthority: PublicKey;
    tokenProgramId: PublicKey;
    feeTokenAccount: PublicKey;
    stableSwapInfo: SaberStableSwapInfo;
    amountIn: Numberu64;
    expectAmountOut: Numberu64;
    minimumAmountOut: Numberu64;
    programId?: PublicKey
  }): Promise<TransactionInstruction> {
    const dataLayout = BufferLayout.struct([
      BufferLayout.u8("instruction"),
      uint64("amountIn"),
      uint64("expectAmountOut"),
      uint64("minimumAmountOut"),
    ]);

    let dataMap: any = {
      instruction: 6, // Swap instruction
      amountIn: amountIn.toBuffer(),
      expectAmountOut: expectAmountOut.toBuffer(),
      minimumAmountOut: minimumAmountOut.toBuffer(),
    };

    const keys = [
      { pubkey: sourceTokenKey, isSigner: false, isWritable: true },
      { pubkey: destinationTokenKey, isSigner: false, isWritable: true },
      { pubkey: transferAuthority, isSigner: true, isWritable: false },
      { pubkey: tokenProgramId, isSigner: false, isWritable: false },
      { pubkey: feeTokenAccount, isSigner: false, isWritable: true }
    ];
    const swapKeys = stableSwapInfo.toKeys(sourceMint);
    keys.push(...swapKeys);

    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode(dataMap, data);

    return new TransactionInstruction({
      keys,
      programId: programId,
      data,
    });
  }

  async createSwapInBySaberStableSwapInstruction(
    {
      fromTokenAccountKey,
      toTokenAccountKey,
      fromMintKey,
      toMintKey,
      userTransferAuthority,
      swapInfo,
      amountIn,
      stableSwapInfo,
    }: {
      fromTokenAccountKey: PublicKey;
      toTokenAccountKey: PublicKey;
      fromMintKey: PublicKey;
      toMintKey: PublicKey;
      userTransferAuthority: PublicKey;
      swapInfo: PublicKey;
      amountIn: Numberu64;
      stableSwapInfo: SaberStableSwapInfo;
    },
    instructions: Array<TransactionInstruction>,
    signers: Array<Signer>
  ): Promise<void> {
    instructions.push(
      await OneSolProtocol.makeSwapInBySaberStableSwapInstruction({
        sourceTokenKey: fromTokenAccountKey,
        sourceMint: fromMintKey,
        destinationTokenKey: toTokenAccountKey,
        destinationMint: toMintKey,
        transferAuthority: userTransferAuthority,
        tokenProgramId: this.tokenProgramId,
        swapInfo: swapInfo,
        stableSwapInfo: stableSwapInfo,
        amountIn: amountIn,
        programId: this.programId,
      })
    );
  }

  static async makeSwapInBySaberStableSwapInstruction({
    sourceTokenKey,
    sourceMint,
    destinationTokenKey,
    destinationMint,
    transferAuthority,
    tokenProgramId,
    swapInfo,
    stableSwapInfo,
    amountIn,
    programId = ONESOL_PROTOCOL_PROGRAM_ID,
  }: {
    sourceTokenKey: PublicKey;
    sourceMint: PublicKey;
    destinationTokenKey: PublicKey;
    destinationMint: PublicKey;
    transferAuthority: PublicKey;
    tokenProgramId: PublicKey;
    swapInfo: PublicKey;
    stableSwapInfo: SaberStableSwapInfo;
    amountIn: Numberu64;
    programId?: PublicKey
  }): Promise<TransactionInstruction> {
    const dataLayout = BufferLayout.struct([
      BufferLayout.u8("instruction"),
      uint64("amountIn"),
    ]);

    let dataMap: any = {
      instruction: 16, // Swap instruction
      amountIn: amountIn.toBuffer(),
    };

    const keys = [
      { pubkey: sourceTokenKey, isSigner: false, isWritable: true },
      { pubkey: destinationTokenKey, isSigner: false, isWritable: true },
      { pubkey: transferAuthority, isSigner: true, isWritable: false },
      { pubkey: swapInfo, isSigner: false, isWritable: true },
      { pubkey: tokenProgramId, isSigner: false, isWritable: false },
    ];
    const swapKeys = stableSwapInfo.toKeys(sourceMint);
    keys.push(...swapKeys);

    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode(dataMap, data);

    return new TransactionInstruction({
      keys,
      programId: programId,
      data,
    });
  }

  async createSwapOutBySaberStableSwapInstruction(
    {
      fromTokenAccountKey,
      toTokenAccountKey,
      fromMintKey,
      toMintKey,
      userTransferAuthority,
      feeTokenAccount,
      swapInfo,
      expectAmountOut,
      minimumAmountOut,
      stableSwapInfo,
    }: {
      fromTokenAccountKey: PublicKey;
      toTokenAccountKey: PublicKey;
      fromMintKey: PublicKey;
      toMintKey: PublicKey;
      userTransferAuthority: PublicKey;
      feeTokenAccount: PublicKey;
      swapInfo: PublicKey;
      expectAmountOut: Numberu64;
      minimumAmountOut: Numberu64;
      stableSwapInfo: SaberStableSwapInfo;
    },
    instructions: Array<TransactionInstruction>,
    signers: Array<Signer>
  ): Promise<void> {
    instructions.push(
      await OneSolProtocol.makeSwapOutBySaberStableSwapInstruction({
        sourceTokenKey: fromTokenAccountKey,
        sourceMint: fromMintKey,
        destinationTokenKey: toTokenAccountKey,
        destinationMint: toMintKey,
        transferAuthority: userTransferAuthority,
        tokenProgramId: this.tokenProgramId,
        swapInfo: swapInfo,
        feeTokenAccount: feeTokenAccount,
        stableSwapInfo: stableSwapInfo,
        expectAmountOut: expectAmountOut,
        minimumAmountOut: minimumAmountOut,
        programId: this.programId,
      })
    );
  }

  static async makeSwapOutBySaberStableSwapInstruction({
    sourceTokenKey,
    sourceMint,
    destinationTokenKey,
    destinationMint,
    transferAuthority,
    tokenProgramId,
    swapInfo,
    feeTokenAccount,
    stableSwapInfo,
    expectAmountOut,
    minimumAmountOut,
    programId = ONESOL_PROTOCOL_PROGRAM_ID,
  }: {
    sourceTokenKey: PublicKey;
    sourceMint: PublicKey;
    destinationTokenKey: PublicKey;
    destinationMint: PublicKey;
    transferAuthority: PublicKey;
    tokenProgramId: PublicKey;
    swapInfo: PublicKey;
    feeTokenAccount: PublicKey;
    stableSwapInfo: SaberStableSwapInfo;
    expectAmountOut: Numberu64;
    minimumAmountOut: Numberu64;
    programId?: PublicKey
  }): Promise<TransactionInstruction> {
    const dataLayout = BufferLayout.struct([
      BufferLayout.u8("instruction"),
      uint64("expectAmountOut"),
      uint64("minimumAmountOut"),
    ]);

    let dataMap: any = {
      instruction: 17, // Swap instruction
      expectAmountOut: expectAmountOut.toBuffer(),
      minimumAmountOut: minimumAmountOut.toBuffer(),
    };

    const keys = [
      { pubkey: sourceTokenKey, isSigner: false, isWritable: true },
      { pubkey: destinationTokenKey, isSigner: false, isWritable: true },
      { pubkey: transferAuthority, isSigner: true, isWritable: false },
      { pubkey: swapInfo, isSigner: false, isWritable: true },
      { pubkey: tokenProgramId, isSigner: false, isWritable: false },
      { pubkey: feeTokenAccount, isSigner: false, isWritable: true }
    ];
    const swapKeys = stableSwapInfo.toKeys(sourceMint);
    keys.push(...swapKeys);

    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode(dataMap, data);

    return new TransactionInstruction({
      keys,
      programId: programId,
      data,
    });
  }


  async createSwapByRaydiumSwapInstruction(
    {
      fromTokenAccountKey,
      toTokenAccountKey,
      fromMintKey,
      toMintKey,
      userTransferAuthority,
      feeTokenAccount,
      amountIn,
      expectAmountOut,
      minimumAmountOut,
      raydiumInfo,
    }: {
      fromTokenAccountKey: PublicKey;
      toTokenAccountKey: PublicKey;
      fromMintKey: PublicKey;
      toMintKey: PublicKey;
      userTransferAuthority: PublicKey;
      feeTokenAccount: PublicKey;
      amountIn: Numberu64;
      expectAmountOut: Numberu64;
      minimumAmountOut: Numberu64;
      raydiumInfo: RaydiumAmmInfo;
    },
    instructions: Array<TransactionInstruction>,
    signers: Array<Signer>
  ): Promise<void> {
    instructions.push(
      await OneSolProtocol.makeSwapByRaydiumSwapInstruction({
        sourceTokenKey: fromTokenAccountKey,
        sourceMint: fromMintKey,
        destinationTokenKey: toTokenAccountKey,
        destinationMint: toMintKey,
        transferAuthority: userTransferAuthority,
        tokenProgramId: this.tokenProgramId,
        feeTokenAccount: feeTokenAccount,
        raydiumInfo: raydiumInfo,
        amountIn: amountIn,
        expectAmountOut: expectAmountOut,
        minimumAmountOut: minimumAmountOut,
        programId: this.programId,
      })
    );
  }

  static async makeSwapByRaydiumSwapInstruction({
    sourceTokenKey,
    sourceMint,
    destinationTokenKey,
    destinationMint,
    transferAuthority,
    tokenProgramId,
    feeTokenAccount,
    raydiumInfo,
    amountIn,
    expectAmountOut,
    minimumAmountOut,
    programId = ONESOL_PROTOCOL_PROGRAM_ID,
  }: {
    sourceTokenKey: PublicKey;
    sourceMint: PublicKey;
    destinationTokenKey: PublicKey;
    destinationMint: PublicKey;
    transferAuthority: PublicKey;
    tokenProgramId: PublicKey;
    feeTokenAccount: PublicKey;
    raydiumInfo: RaydiumAmmInfo;
    amountIn: Numberu64;
    expectAmountOut: Numberu64;
    minimumAmountOut: Numberu64;
    programId?: PublicKey,
  }): Promise<TransactionInstruction> {
    const dataLayout = BufferLayout.struct([
      BufferLayout.u8("instruction"),
      uint64("amountIn"),
      uint64("expectAmountOut"),
      uint64("minimumAmountOut"),
    ]);

    let dataMap: any = {
      instruction: 9, // Swap instruction
      amountIn: amountIn.toBuffer(),
      expectAmountOut: expectAmountOut.toBuffer(),
      minimumAmountOut: minimumAmountOut.toBuffer(),
    };

    const keys = [
      { pubkey: sourceTokenKey, isSigner: false, isWritable: true },
      { pubkey: destinationTokenKey, isSigner: false, isWritable: true },
      { pubkey: transferAuthority, isSigner: true, isWritable: false },
      { pubkey: tokenProgramId, isSigner: false, isWritable: false },
      { pubkey: feeTokenAccount, isSigner: false, isWritable: true }
    ];
    const swapKeys = raydiumInfo.toKeys();
    keys.push(...swapKeys);

    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode(dataMap, data);

    return new TransactionInstruction({
      keys,
      programId: programId,
      data,
    });
  }

  async createSwapInByRaydiumSwapInstruction(
    {
      fromTokenAccountKey,
      toTokenAccountKey,
      fromMintKey,
      toMintKey,
      userTransferAuthority,
      swapInfo,
      amountIn,
      raydiumInfo,
    }: {
      fromTokenAccountKey: PublicKey;
      toTokenAccountKey: PublicKey;
      fromMintKey: PublicKey;
      toMintKey: PublicKey;
      userTransferAuthority: PublicKey;
      swapInfo: PublicKey;
      amountIn: Numberu64;
      raydiumInfo: RaydiumAmmInfo;
    },
    instructions: Array<TransactionInstruction>,
    signers: Array<Signer>
  ): Promise<void> {
    instructions.push(
      await OneSolProtocol.makeSwapInByRaydiumSwapInstruction({
        sourceTokenKey: fromTokenAccountKey,
        sourceMint: fromMintKey,
        destinationTokenKey: toTokenAccountKey,
        destinationMint: toMintKey,
        transferAuthority: userTransferAuthority,
        tokenProgramId: this.tokenProgramId,
        swapInfo: swapInfo,
        raydiumInfo: raydiumInfo,
        amountIn: amountIn,
        programId: this.programId,
      })
    );
  }

  static async makeSwapInByRaydiumSwapInstruction({
    sourceTokenKey,
    sourceMint,
    destinationTokenKey,
    destinationMint,
    transferAuthority,
    swapInfo,
    tokenProgramId,
    raydiumInfo,
    amountIn,
    programId = ONESOL_PROTOCOL_PROGRAM_ID,
  }: {
    sourceTokenKey: PublicKey;
    sourceMint: PublicKey;
    destinationTokenKey: PublicKey;
    destinationMint: PublicKey;
    transferAuthority: PublicKey;
    tokenProgramId: PublicKey;
    swapInfo: PublicKey;
    raydiumInfo: RaydiumAmmInfo;
    amountIn: Numberu64;
    programId?: PublicKey,
  }): Promise<TransactionInstruction> {
    const dataLayout = BufferLayout.struct([
      BufferLayout.u8("instruction"),
      uint64("amountIn"),
    ]);

    let dataMap: any = {
      instruction: 18, // Swap instruction
      amountIn: amountIn.toBuffer(),
    };

    const keys = [
      { pubkey: sourceTokenKey, isSigner: false, isWritable: true },
      { pubkey: destinationTokenKey, isSigner: false, isWritable: true },
      { pubkey: transferAuthority, isSigner: true, isWritable: false },
      { pubkey: swapInfo, isSigner: true, isWritable: true },
      { pubkey: tokenProgramId, isSigner: false, isWritable: false },
    ];
    const swapKeys = raydiumInfo.toKeys();
    keys.push(...swapKeys);

    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode(dataMap, data);

    return new TransactionInstruction({
      keys,
      programId: programId,
      data,
    });
  }


  async createSwapOutByRaydiumSwapInstruction(
    {
      fromTokenAccountKey,
      toTokenAccountKey,
      fromMintKey,
      toMintKey,
      userTransferAuthority,
      feeTokenAccount,
      swapInfo,
      expectAmountOut,
      minimumAmountOut,
      raydiumInfo,
    }: {
      fromTokenAccountKey: PublicKey;
      toTokenAccountKey: PublicKey;
      fromMintKey: PublicKey;
      toMintKey: PublicKey;
      userTransferAuthority: PublicKey;
      feeTokenAccount: PublicKey;
      swapInfo: PublicKey;
      expectAmountOut: Numberu64;
      minimumAmountOut: Numberu64;
      raydiumInfo: RaydiumAmmInfo;
    },
    instructions: Array<TransactionInstruction>,
    signers: Array<Signer>
  ): Promise<void> {
    instructions.push(
      await OneSolProtocol.makeSwapOutByRaydiumSwapInstruction({
        sourceTokenKey: fromTokenAccountKey,
        sourceMint: fromMintKey,
        destinationTokenKey: toTokenAccountKey,
        destinationMint: toMintKey,
        transferAuthority: userTransferAuthority,
        tokenProgramId: this.tokenProgramId,
        feeTokenAccount: feeTokenAccount,
        swapInfo: swapInfo,
        raydiumInfo: raydiumInfo,
        expectAmountOut: expectAmountOut,
        minimumAmountOut: minimumAmountOut,
        programId: this.programId,
      })
    );
  }

  static async makeSwapOutByRaydiumSwapInstruction({
    sourceTokenKey,
    sourceMint,
    destinationTokenKey,
    destinationMint,
    transferAuthority,
    tokenProgramId,
    feeTokenAccount,
    swapInfo,
    raydiumInfo,
    expectAmountOut,
    minimumAmountOut,
    programId = ONESOL_PROTOCOL_PROGRAM_ID,
  }: {
    sourceTokenKey: PublicKey;
    sourceMint: PublicKey;
    destinationTokenKey: PublicKey;
    destinationMint: PublicKey;
    transferAuthority: PublicKey;
    tokenProgramId: PublicKey;
    feeTokenAccount: PublicKey;
    swapInfo: PublicKey;
    raydiumInfo: RaydiumAmmInfo;
    expectAmountOut: Numberu64;
    minimumAmountOut: Numberu64;
    programId?: PublicKey,
  }): Promise<TransactionInstruction> {
    const dataLayout = BufferLayout.struct([
      BufferLayout.u8("instruction"),
      uint64("expectAmountOut"),
      uint64("minimumAmountOut"),
    ]);

    let dataMap: any = {
      instruction: 19, // Swap instruction
      expectAmountOut: expectAmountOut.toBuffer(),
      minimumAmountOut: minimumAmountOut.toBuffer(),
    };

    const keys = [
      { pubkey: sourceTokenKey, isSigner: false, isWritable: true },
      { pubkey: destinationTokenKey, isSigner: false, isWritable: true },
      { pubkey: transferAuthority, isSigner: true, isWritable: false },
      { pubkey: swapInfo, isSigner: true, isWritable: true },
      { pubkey: tokenProgramId, isSigner: false, isWritable: false },
      { pubkey: feeTokenAccount, isSigner: false, isWritable: true }
    ];
    const swapKeys = raydiumInfo.toKeys();
    keys.push(...swapKeys);

    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode(dataMap, data);

    return new TransactionInstruction({
      keys,
      programId: programId,
      data,
    });
  }


  async createSwapBySerumDexInstruction(
    {
      fromTokenAccountKey,
      toTokenAccountKey,
      fromMintKey,
      toMintKey,
      userTransferAuthority,
      feeTokenAccount,
      amountIn,
      expectAmountOut,
      minimumAmountOut,
      dexMarketInfo,
    }: {
      fromTokenAccountKey: PublicKey;
      toTokenAccountKey: PublicKey;
      fromMintKey: PublicKey;
      toMintKey: PublicKey;
      userTransferAuthority: PublicKey;
      feeTokenAccount: PublicKey,
      amountIn: Numberu64;
      expectAmountOut: Numberu64;
      minimumAmountOut: Numberu64;
      dexMarketInfo: SerumDexMarketInfo;
    },
    instructions: Array<TransactionInstruction>,
    signers: Array<Signer>
  ): Promise<void> {
    instructions.push(
      await OneSolProtocol.makeSwapBySerumDexInstruction({
        sourceTokenKey: fromTokenAccountKey,
        sourceMintKey: fromMintKey,
        destinationTokenKey: toTokenAccountKey,
        destinationMintKey: toMintKey,
        transferAuthority: userTransferAuthority,
        feeTokenAccount: feeTokenAccount,
        tokenProgramId: this.tokenProgramId,
        dexMarketInfo,
        amountIn: amountIn,
        expectAmountOut,
        minimumAmountOut,
        programId: this.programId,
      })
    );
  }

  static async makeSwapBySerumDexInstruction({
    sourceTokenKey,
    sourceMintKey,
    destinationTokenKey,
    destinationMintKey,
    feeTokenAccount,
    transferAuthority,
    tokenProgramId,
    dexMarketInfo,
    amountIn,
    expectAmountOut,
    minimumAmountOut,
    programId = ONESOL_PROTOCOL_PROGRAM_ID,
  }: {
    sourceTokenKey: PublicKey;
    sourceMintKey: PublicKey;
    destinationTokenKey: PublicKey;
    destinationMintKey: PublicKey;
    feeTokenAccount: PublicKey;
    transferAuthority: PublicKey;
    tokenProgramId: PublicKey;
    dexMarketInfo: SerumDexMarketInfo;
    amountIn: Numberu64;
    expectAmountOut: Numberu64;
    minimumAmountOut: Numberu64;
    programId?: PublicKey,
  }): Promise<TransactionInstruction> {
    const instructionStruct: any = [
      BufferLayout.u8("instruction"),
      uint64("amountIn"),
      uint64("expectAmountOut"),
      uint64("minimumAmountOut"),
    ];
    // console.log("side: " + side + ", exchangeRate: " + exchangeRate);
    let dataMap: any = {
      instruction: 4, // Swap instruction
      amountIn: amountIn.toBuffer(),
      expectAmountOut: expectAmountOut.toBuffer(),
      minimumAmountOut: minimumAmountOut.toBuffer(),
    };

    const keys = [
      { pubkey: sourceTokenKey, isSigner: false, isWritable: true },
      { pubkey: destinationTokenKey, isSigner: false, isWritable: true },
      { pubkey: transferAuthority, isSigner: true, isWritable: false },
      { pubkey: tokenProgramId, isSigner: false, isWritable: false },
      { pubkey: feeTokenAccount, isSigner: false, isWritable: true }
    ];
    const swapKeys = dexMarketInfo.toKeys();
    keys.push(...swapKeys);

    const dataLayout = BufferLayout.struct(instructionStruct);
    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode(dataMap, data);

    return new TransactionInstruction({
      keys,
      programId: programId,
      data,
    });
  }

  async createSwapInBySerumDexInstruction(
    {
      fromTokenAccountKey,
      toTokenAccountKey,
      fromMintKey,
      toMintKey,
      userTransferAuthority,
      swapInfo,
      amountIn,
      dexMarketInfo,
    }: {
      fromTokenAccountKey: PublicKey;
      toTokenAccountKey: PublicKey;
      fromMintKey: PublicKey;
      toMintKey: PublicKey;
      swapInfo: PublicKey,
      userTransferAuthority: PublicKey;
      amountIn: Numberu64;
      dexMarketInfo: SerumDexMarketInfo;
    },
    instructions: Array<TransactionInstruction>,
    signers: Array<Signer>
  ): Promise<void> {
    instructions.push(
      await OneSolProtocol.makeSwapInBySerumDexInstruction({
        sourceTokenKey: fromTokenAccountKey,
        sourceMintKey: fromMintKey,
        destinationTokenKey: toTokenAccountKey,
        destinationMintKey: toMintKey,
        transferAuthority: userTransferAuthority,
        swapInfo: swapInfo,
        tokenProgramId: this.tokenProgramId,
        dexMarketInfo,
        amountIn: amountIn,
        programId: this.programId,
      })
    );
  }

  static async makeSwapInBySerumDexInstruction({
    sourceTokenKey,
    sourceMintKey,
    destinationTokenKey,
    destinationMintKey,
    swapInfo,
    transferAuthority,
    tokenProgramId,
    dexMarketInfo,
    amountIn,
    programId = ONESOL_PROTOCOL_PROGRAM_ID,
  }: {
    sourceTokenKey: PublicKey;
    sourceMintKey: PublicKey;
    destinationTokenKey: PublicKey;
    destinationMintKey: PublicKey;
    swapInfo: PublicKey;
    transferAuthority: PublicKey;
    tokenProgramId: PublicKey;
    dexMarketInfo: SerumDexMarketInfo;
    amountIn: Numberu64;
    programId?: PublicKey,
  }): Promise<TransactionInstruction> {
    const instructionStruct: any = [
      BufferLayout.u8("instruction"),
      uint64("amountIn"),
    ];
    // console.log("side: " + side + ", exchangeRate: " + exchangeRate);
    let dataMap: any = {
      instruction: 14, // Swap instruction
      amountIn: amountIn.toBuffer(),
    };

    const keys = [
      { pubkey: sourceTokenKey, isSigner: false, isWritable: true },
      { pubkey: destinationTokenKey, isSigner: false, isWritable: true },
      { pubkey: transferAuthority, isSigner: true, isWritable: false },
      { pubkey: swapInfo, isSigner: true, isWritable: true },
      { pubkey: tokenProgramId, isSigner: false, isWritable: false },
    ];
    const swapKeys = dexMarketInfo.toKeys();
    keys.push(...swapKeys);

    const dataLayout = BufferLayout.struct(instructionStruct);
    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode(dataMap, data);

    return new TransactionInstruction({
      keys,
      programId: programId,
      data,
    });
  }

  async createSwapOutBySerumDexInstruction(
    {
      fromTokenAccountKey,
      toTokenAccountKey,
      fromMintKey,
      toMintKey,
      userTransferAuthority,
      swapInfo,
      feeTokenAccount,
      expectAmountOut,
      minimumAmountOut,
      dexMarketInfo,
    }: {
      fromTokenAccountKey: PublicKey;
      toTokenAccountKey: PublicKey;
      fromMintKey: PublicKey;
      toMintKey: PublicKey;
      userTransferAuthority: PublicKey;
      feeTokenAccount: PublicKey,
      swapInfo: PublicKey,
      expectAmountOut: Numberu64;
      minimumAmountOut: Numberu64;
      dexMarketInfo: SerumDexMarketInfo;
    },
    instructions: Array<TransactionInstruction>,
    signers: Array<Signer>
  ): Promise<void> {
    instructions.push(
      await OneSolProtocol.makeSwapOutBySerumDexInstruction({
        sourceTokenKey: fromTokenAccountKey,
        sourceMintKey: fromMintKey,
        destinationTokenKey: toTokenAccountKey,
        destinationMintKey: toMintKey,
        transferAuthority: userTransferAuthority,
        feeTokenAccount: feeTokenAccount,
        tokenProgramId: this.tokenProgramId,
        swapInfo: swapInfo,
        dexMarketInfo,
        expectAmountOut,
        minimumAmountOut,
        programId: this.programId,
      })
    );
  }

  static async makeSwapOutBySerumDexInstruction({
    sourceTokenKey,
    sourceMintKey,
    destinationTokenKey,
    destinationMintKey,
    feeTokenAccount,
    transferAuthority,
    swapInfo,
    tokenProgramId,
    dexMarketInfo,
    expectAmountOut,
    minimumAmountOut,
    programId = ONESOL_PROTOCOL_PROGRAM_ID,
  }: {
    sourceTokenKey: PublicKey;
    sourceMintKey: PublicKey;
    destinationTokenKey: PublicKey;
    destinationMintKey: PublicKey;
    feeTokenAccount: PublicKey;
    transferAuthority: PublicKey;
    tokenProgramId: PublicKey;
    swapInfo: PublicKey;
    dexMarketInfo: SerumDexMarketInfo;
    expectAmountOut: Numberu64;
    minimumAmountOut: Numberu64;
    programId?: PublicKey,
  }): Promise<TransactionInstruction> {
    const instructionStruct: any = [
      BufferLayout.u8("instruction"),
      uint64("expectAmountOut"),
      uint64("minimumAmountOut"),
    ];
    // console.log("side: " + side + ", exchangeRate: " + exchangeRate);
    let dataMap: any = {
      instruction: 15, // Swap instruction
      expectAmountOut: expectAmountOut.toBuffer(),
      minimumAmountOut: minimumAmountOut.toBuffer(),
    };

    const keys = [
      { pubkey: sourceTokenKey, isSigner: false, isWritable: true },
      { pubkey: destinationTokenKey, isSigner: false, isWritable: true },
      { pubkey: transferAuthority, isSigner: true, isWritable: false },
      { pubkey: swapInfo, isSigner: true, isWritable: true },
      { pubkey: tokenProgramId, isSigner: false, isWritable: false },
      { pubkey: feeTokenAccount, isSigner: false, isWritable: true }
    ];
    const swapKeys = dexMarketInfo.toKeys();
    keys.push(...swapKeys);

    const dataLayout = BufferLayout.struct(instructionStruct);
    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode(dataMap, data);

    return new TransactionInstruction({
      keys,
      programId: programId,
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
    commitment: "recent",
    preflightCommitment: "recent",
  });
}

export async function loadTokenSwapInfo(
  connection: Connection,
  address: PublicKey,
  programId: PublicKey,
  hostFeeAccount: PublicKey | null
): Promise<TokenSwapInfo> {
  const data = await loadAccount(connection, address, programId);
  const tokenSwapData = TokenSwapLayout.decode(data);

  if (!tokenSwapData.isInitialized) {
    throw new Error(`Invalid token swap state`);
  }

  const authority = await PublicKey.createProgramAddress(
    [address.toBuffer()].concat(Buffer.from([tokenSwapData.nonce])),
    programId
  );

  const poolMint = new PublicKey(tokenSwapData.tokenPool);
  const feeAccount = new PublicKey(tokenSwapData.feeAccount);
  const tokenAccountA = new PublicKey(tokenSwapData.tokenAccountA);
  const mintA = new PublicKey(tokenSwapData.mintA);
  const tokenAccountB = new PublicKey(tokenSwapData.tokenAccountB);
  const mintB = new PublicKey(tokenSwapData.mintB);

  return new TokenSwapInfo(
    programId,
    address,
    authority,
    tokenAccountA,
    tokenAccountB,
    mintA,
    mintB,
    poolMint,
    feeAccount,
  );
}

export async function loadSerumDexMarket(
  connection: Connection,
  pubkey: PublicKey,
  programId: PublicKey,
  extPubkey: PublicKey,
  extProgramId: PublicKey
): Promise<SerumDexMarketInfo> {
  const [dexMarketData, extDexMarketData] = await Promise.all([
    loadAccount(connection, pubkey, programId),
    loadAccount(connection, extPubkey, extProgramId),
  ]);

  const marketDecoded = Market.getLayout(programId).decode(dexMarketData);
  const extMarketDecoded = DexMarketInfoLayout.decode(extDexMarketData);

  const extMarket = new PublicKey(extMarketDecoded.market);
  if (!pubkey.equals(extMarket)) {
    throw new Error(
      `extMarket(${extMarket.toString()}) not equals pubkey(${pubkey.toString()})`
    );
  }

  // return new SerumDexMarketInfo(programId, market, openOrders.publicKey);
  const requestQueue = new PublicKey(marketDecoded.requestQueue);
  const eventQueue = new PublicKey(marketDecoded.eventQueue);
  const bids = new PublicKey(marketDecoded.bids);
  const asks = new PublicKey(marketDecoded.asks);
  const coinVault = new PublicKey(marketDecoded.baseVault);
  const pcVault = new PublicKey(marketDecoded.quoteVault);
  const vaultSignerNonce = marketDecoded.vaultSignerNonce;

  const vaultSigner = await PublicKey.createProgramAddress(
    [pubkey.toBuffer()].concat(vaultSignerNonce.toArrayLike(Buffer, "le", 8)),
    programId
  );

  const openOrders = new PublicKey(extMarketDecoded.openOrders);

  return new SerumDexMarketInfo(
    pubkey,
    requestQueue,
    eventQueue,
    bids,
    asks,
    coinVault,
    pcVault,
    vaultSigner,
    openOrders,
    programId
  );
}

export async function loadSaberStableSwap(
  {
    connection,
    address,
    programId = STABLE_SWAP_PROGRAM_ID,
  }: {
    connection: Connection;
    address: PublicKey,
    programId: PublicKey,
  }
): Promise<SaberStableSwapInfo> {

  const data = await loadAccount(connection, address, programId);
  const stableSwapData = StableSwapLayout.decode(data);

  if (!stableSwapData.isInitialized || stableSwapData.isPaused) {
    throw new Error(`Invalid token swap state`);
  }

  const authority = await PublicKey.createProgramAddress(
    [address.toBuffer()].concat(Buffer.from([stableSwapData.nonce])),
    programId
  );

  const tokenAccountA = new PublicKey(stableSwapData.tokenAccountA);
  const mintA = new PublicKey(stableSwapData.mintA);
  const adminFeeAccountA = new PublicKey(stableSwapData.adminFeeAccountA);
  const tokenAccountB = new PublicKey(stableSwapData.tokenAccountB);
  const mintB = new PublicKey(stableSwapData.mintB);
  const adminFeeAccountB = new PublicKey(stableSwapData.adminFeeAccountB);

  return new SaberStableSwapInfo(
    programId,
    address,
    authority,
    tokenAccountA,
    mintA,
    adminFeeAccountA,
    tokenAccountB,
    mintB,
    adminFeeAccountB
  );
}

export async function loadRaydiumAmmInfo(
  {
    connection,
    address,
    programId = RAYDIUM_PROGRAM_ID,
  }: {
    connection: Connection;
    address: PublicKey,
    programId?: PublicKey,
  }
): Promise<RaydiumAmmInfo> {
  const data = await loadAccount(connection, address, programId);
  const raydiumDecoded = RaydiumLiquidityStateLayout.decode(data);

  // this from raydium-sdk src/liquidity/liquidity.ts:getAuthority()
  const [authority, _] = await PublicKey.findProgramAddress(
    [Buffer.from([97, 109, 109, 32, 97, 117, 116, 104, 111, 114, 105, 116, 121])],
    programId
  );

  const serumMarketProgramId = new PublicKey(raydiumDecoded.marketProgramId);
  const serumMarket = new PublicKey(raydiumDecoded.marketId);
  const marketDecoded = Market.getLayout(serumMarketProgramId).decode(await loadAccount(connection, serumMarket, serumMarketProgramId));

  const vaultSignerNonce = marketDecoded.vaultSignerNonce;

  const vaultSigner = await PublicKey.createProgramAddress(
    [serumMarket.toBuffer()].concat(vaultSignerNonce.toArrayLike(Buffer, "le", 8)),
    serumMarketProgramId
  );

  return new RaydiumAmmInfo(
    programId,
    address,
    authority,
    new PublicKey(raydiumDecoded.openOrders),
    new PublicKey(raydiumDecoded.targetOrders),
    new PublicKey(raydiumDecoded.baseVault),
    new PublicKey(raydiumDecoded.quoteVault),
    serumMarketProgramId,
    serumMarket,
    new PublicKey(marketDecoded.bids),
    new PublicKey(marketDecoded.asks),
    new PublicKey(marketDecoded.eventQueue),
    new PublicKey(marketDecoded.baseVault),
    new PublicKey(marketDecoded.quoteVault),
    vaultSigner
  );
}
