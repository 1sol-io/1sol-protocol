import assert from "assert";
import BN, { min } from "bn.js";
import bs58 from "bs58";
import { Buffer } from "buffer";
import * as BufferLayout from "buffer-layout";
import type {
  AccountInfo,
  Connection,
  TransactionSignature,
} from "@solana/web3.js";
import {
  SYSVAR_RENT_PUBKEY,
  Keypair,
  Signer,
  AccountMeta,
  PublicKey,
  SystemProgram,
  Transaction,
  TransactionInstruction,
  sendAndConfirmTransaction,
} from "@solana/web3.js";
import { Market, OpenOrders } from "@project-serum/serum";
import { TokenSwapLayout } from "@solana/spl-token-swap";
import {
  MintInfo as TokenMint,
  MintLayout as TokenMintLayout,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";

export const ONESOL_PROTOCOL_PROGRAM_ID: PublicKey = new PublicKey(
  "9Bj8zgNWT6UaNcXMgzMFrnH5Z13nQ6vFkRNxP743zZyr"
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
    throw new Error(`Invalid owner: ${JSON.stringify(accountInfo.owner)}`);
  }

  return Buffer.from(accountInfo.data);
}

export const AmmInfoLayout = BufferLayout.struct([
  BufferLayout.u16("accountFlags"),
  BufferLayout.u8("nonce"),
  publicKeyLayout("owner"),
  publicKeyLayout("tokenProgramId"),
  publicKeyLayout("tokenAccountA"),
  publicKeyLayout("tokenMintA"),
  publicKeyLayout("tokenAccountB"),
  publicKeyLayout("tokenMintB"),
  BufferLayout.struct(
    [
      BufferLayout.blob(128, "TokenAInAmount"),
      BufferLayout.blob(128, "TokenBOutAmount"),
      uint64("tokenA2BFee"),
      BufferLayout.blob(128, "TokenAInAmount"),
      BufferLayout.blob(128, "TokenBOutAmount"),
      uint64("tokenB2AFee"),
    ],
    "outputData"
  ),
]);

export const DexMarketInfoLayout = BufferLayout.struct([
  BufferLayout.u16("accountFlags"),
  publicKeyLayout("ammInfo"),
  publicKeyLayout("market"),
  publicKeyLayout("pcMint"),
  publicKeyLayout("coinMint"),
  publicKeyLayout("openOrders"),
  publicKeyLayout("dexProgramId"),
]);

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
    private hostFeeAccount: PublicKey | null
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
    this.hostFeeAccount = hostFeeAccount;
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
    if (this.hostFeeAccount) {
      keys.push({
        pubkey: this.hostFeeAccount,
        isSigner: false,
        isWritable: true,
      });
    }
    return keys;
  }

  includeHostFeeAccount(): number {
    if (this.hostFeeAccount !== null) {
      return 1;
    } else {
      return 0;
    }
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
      { pubkey: this.openOrders, isSigner: false, isWritable: false },
      { pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false },
      { pubkey: this.programId, isSigner: false, isWritable: false },
    ];
    return keys;
  }
}

export class AmmInfo {
  constructor(
    public pubkey: PublicKey,
    public programId: PublicKey,
    protected authority: PublicKey,
    private decoded: any
  ) {
    this.pubkey = pubkey;
    this.programId = programId;
    this.authority = authority;
    this.decoded = decoded;
  }

  public tokenAccountA(): PublicKey {
    return new PublicKey(this.decoded.tokenAccountA);
  }

  public tokenAccountB(): PublicKey {
    return new PublicKey(this.decoded.tokenAccountB);
  }

  public tokenMintA(): PublicKey {
    return new PublicKey(this.decoded.tokenMintA);
  }

  public tokenMintB(): PublicKey {
    return new PublicKey(this.decoded.tokenMintB);
  }

  public toKeys() {
    return [
      { pubkey: this.pubkey, isSigner: false, isWritable: true },
      { pubkey: this.authority, isSigner: false, isWritable: false },
      { pubkey: this.tokenAccountA(), isSigner: false, isWritable: true },
      { pubkey: this.tokenAccountB(), isSigner: false, isWritable: true },
    ];
  }

  public static async from({
    pubkey,
    account,
  }: {
    pubkey: PublicKey;
    account: AccountInfo<Buffer>;
  }): Promise<AmmInfo> {
    const data = AmmInfoLayout.decode(account.data);

    const authority = await PublicKey.createProgramAddress(
      [pubkey.toBuffer()].concat(Buffer.from(data.nonce)),
      account.owner
    );
    return new AmmInfo(pubkey, account.owner, authority, data);
  }

  public static async loadAmmInfo(
    connection: Connection,
    ammInfoKey: PublicKey
  ): Promise<AmmInfo> {
    const account = await connection.getAccountInfo(ammInfoKey);
    if (!account) {
      throw new Error(`AmmInfo ${ammInfoKey.toBase58()} not found`);
    }
    return await AmmInfo.from({ pubkey: ammInfoKey, account });
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

  public static async loadAllAmmInfos(
    connection: Connection,
    programId?: PublicKey
  ): Promise<Array<AmmInfo>> {
    programId = programId ? programId : ONESOL_PROTOCOL_PROGRAM_ID;

    let programAccounts = await connection.getProgramAccounts(programId, {
      encoding: "base64",
      filters: [
        {
          memcmp: {
            offset: 0,
            bytes: bs58.encode(Buffer.from(Uint8Array.of(18))),
          },
        },
      ],
    });
    const ammInfoArray = new Array<AmmInfo>();
    programAccounts.forEach(async ({ pubkey, account }) => {
      ammInfoArray.push(await AmmInfo.from({ pubkey, account }));
    });
    return ammInfoArray;
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
    programId,
  }: {
    connection: Connection;
    wallet: PublicKey;
    programId?: PublicKey;
  }): Promise<OneSolProtocol> {
    programId = programId ? programId : ONESOL_PROTOCOL_PROGRAM_ID;
    return new OneSolProtocol(connection, programId, TOKEN_PROGRAM_ID, wallet);
  }

  protected static async findOneSolAmmInfoAccounts(
    connection: Connection,
    pcMint: PublicKey,
    coinMint: PublicKey,
    programId: PublicKey
  ) {
    return await connection.getProgramAccounts(programId, {
      encoding: "base64",
      filters: [
        {
          memcmp: {
            offset: AmmInfoLayout.offset("aTokenMint"),
            bytes: pcMint.toBase58(),
          },
        },
        {
          memcmp: {
            offset: AmmInfoLayout.offset("bTokenMint"),
            bytes: coinMint.toBase58(),
          },
        },
      ],
    });
  }

  async createSwapByTokenSwapInstruction(
    {
      fromTokenAccountKey,
      toTokenAccountKey,
      fromMintKey,
      toMintKey,
      userTransferAuthority,
      ammInfo,
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
      ammInfo: AmmInfo;
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
        ammInfo: ammInfo,
        sourceTokenKey: fromTokenAccountKey,
        sourceMint: fromMintKey,
        destinationTokenKey: toTokenAccountKey,
        destinationMint: toMintKey,
        transferAuthority: userTransferAuthority,
        tokenProgramId: this.tokenProgramId,
        splTokenSwapInfo: splTokenSwapInfo,
        amountIn: amountIn,
        expectAmountOut: expectAmountOut,
        minimumAmountOut: minimumAmountOut,
      })
    );
  }

  static async makeSwapByTokenSwapInstruction({
    ammInfo,
    sourceTokenKey,
    sourceMint,
    destinationTokenKey,
    destinationMint,
    transferAuthority,
    tokenProgramId,
    splTokenSwapInfo,
    amountIn,
    expectAmountOut,
    minimumAmountOut,
  }: {
    ammInfo: AmmInfo;
    sourceTokenKey: PublicKey;
    sourceMint: PublicKey;
    destinationTokenKey: PublicKey;
    destinationMint: PublicKey;
    transferAuthority: PublicKey;
    tokenProgramId: PublicKey;
    splTokenSwapInfo: TokenSwapInfo;
    amountIn: Numberu64;
    expectAmountOut: Numberu64;
    minimumAmountOut: Numberu64;
  }): Promise<TransactionInstruction> {
    if (
      !(
        (sourceMint.equals(ammInfo.tokenMintA()) &&
          destinationMint.equals(ammInfo.tokenMintB())) ||
        (sourceMint.equals(ammInfo.tokenMintB()) &&
          destinationMint.equals(ammInfo.tokenMintA()))
      )
    ) {
      throw new Error(`ammInfo(${ammInfo.pubkey}) error`);
    }
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
    ];
    keys.push(...ammInfo.toKeys());
    const swapKeys = splTokenSwapInfo.toKeys();
    keys.push(...swapKeys);

    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode(dataMap, data);

    return new TransactionInstruction({
      keys,
      programId: ammInfo.programId,
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
      ammInfo,
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
      ammInfo: AmmInfo;
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
        ammInfo: ammInfo,
        sourceTokenKey: fromTokenAccountKey,
        sourceMintKey: fromMintKey,
        destinationTokenKey: toTokenAccountKey,
        destinationMintKey: toMintKey,
        transferAuthority: userTransferAuthority,
        tokenProgramId: this.tokenProgramId,
        dexMarketInfo,
        amountIn: amountIn,
        expectAmountOut,
        minimumAmountOut,
      })
    );
  }

  static async makeSwapBySerumDexInstruction({
    ammInfo,
    sourceTokenKey,
    sourceMintKey,
    destinationTokenKey,
    destinationMintKey,
    transferAuthority,
    tokenProgramId,
    dexMarketInfo,
    amountIn,
    expectAmountOut,
    minimumAmountOut,
  }: {
    ammInfo: AmmInfo;
    sourceTokenKey: PublicKey;
    sourceMintKey: PublicKey;
    destinationTokenKey: PublicKey;
    destinationMintKey: PublicKey;
    transferAuthority: PublicKey;
    tokenProgramId: PublicKey;
    dexMarketInfo: SerumDexMarketInfo;
    amountIn: Numberu64;
    expectAmountOut: Numberu64;
    minimumAmountOut: Numberu64;
  }): Promise<TransactionInstruction> {
    if (
      !(
        (sourceMintKey.equals(ammInfo.tokenMintA()) &&
          destinationMintKey.equals(ammInfo.tokenMintB())) ||
        (sourceMintKey.equals(ammInfo.tokenMintB()) &&
          destinationMintKey.equals(ammInfo.tokenMintA()))
      )
    ) {
      throw new Error(`ammInfo(${ammInfo.pubkey}) error`);
    }
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
    ];
    keys.push(...ammInfo.toKeys());
    const swapKeys = dexMarketInfo.toKeys();
    keys.push(...swapKeys);

    const dataLayout = BufferLayout.struct(instructionStruct);
    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode(dataMap, data);

    return new TransactionInstruction({
      keys,
      programId: ammInfo.programId,
      data,
    });
  }

  async createSwapTwoStepsInstruction(
    {
      fromTokenAccountKey,
      toTokenAccountKey,
      fromMintKey,
      toMintKey,
      userTransferAuthority,
      amountIn,
      expectAmountOut,
      minimumAmountOut,
      step1,
      step2,
    }: {
      fromTokenAccountKey: PublicKey;
      toTokenAccountKey: PublicKey;
      fromMintKey: PublicKey;
      toMintKey: PublicKey;
      userTransferAuthority: PublicKey;
      amountIn: Numberu64;
      expectAmountOut: Numberu64;
      minimumAmountOut: Numberu64;
      step1: {
        ammInfo: AmmInfo;
        stepInfo: TokenSwapInfo | SerumDexMarketInfo;
      };
      step2: {
        ammInfo: AmmInfo;
        stepInfo: TokenSwapInfo | SerumDexMarketInfo;
      };
    },
    instructions: Array<TransactionInstruction>,
    signers: Array<Signer>
  ): Promise<void> {
    instructions.push(
      await OneSolProtocol.makeSwapTwoStepsInstruction({
        sourceTokenKey: fromTokenAccountKey,
        sourceMint: fromMintKey,
        destinationTokenKey: toTokenAccountKey,
        destinationMint: toMintKey,
        transferAuthority: userTransferAuthority,
        tokenProgramId: this.tokenProgramId,
        step1,
        step2,
        amountIn: amountIn,
        expectAmountOut,
        minimumAmountOut,
        programId: this.programId,
      })
    );
  }

  static async makeSwapTwoStepsInstruction({
    sourceTokenKey,
    sourceMint,
    destinationTokenKey,
    destinationMint,
    transferAuthority,
    tokenProgramId,
    step1,
    step2,
    amountIn,
    expectAmountOut,
    minimumAmountOut,
    programId,
  }: {
    sourceTokenKey: PublicKey;
    sourceMint: PublicKey;
    destinationTokenKey: PublicKey;
    destinationMint: PublicKey;
    transferAuthority: PublicKey;
    tokenProgramId: PublicKey;
    step1: {
      ammInfo: AmmInfo;
      stepInfo: TokenSwapInfo | SerumDexMarketInfo;
    };
    step2: {
      ammInfo: AmmInfo;
      stepInfo: TokenSwapInfo | SerumDexMarketInfo;
    };
    amountIn: Numberu64;
    expectAmountOut: Numberu64;
    minimumAmountOut: Numberu64;
    programId: PublicKey;
  }): Promise<TransactionInstruction> {
    if (
      !(
        (sourceMint.equals(step1.ammInfo.tokenMintA()) &&
          destinationMint.equals(step2.ammInfo.tokenMintB())) ||
        (sourceMint.equals(step1.ammInfo.tokenMintB()) &&
          destinationMint.equals(step2.ammInfo.tokenMintA()))
      )
    ) {
      throw new Error(`ammInfo error`);
    }
    const dataLayout = BufferLayout.struct([
      BufferLayout.u8("instruction"),
      uint64("amountIn"),
      uint64("expectAmountOut"),
      uint64("minimumAmountOut"),
    ]);

    let dataMap: any = {
      instruction: 5, // Swap instruction
      amountIn: amountIn.toBuffer(),
      expectAmountOut: expectAmountOut.toBuffer(),
      minimumAmountOut: minimumAmountOut.toBuffer(),
    };

    const keys = [
      { pubkey: sourceTokenKey, isSigner: false, isWritable: true },
      { pubkey: destinationTokenKey, isSigner: false, isWritable: true },
      { pubkey: transferAuthority, isSigner: true, isWritable: false },
      { pubkey: tokenProgramId, isSigner: false, isWritable: false },
    ];

    [step1, step2].forEach(({ ammInfo, stepInfo }) => {
      keys.push(...ammInfo.toKeys());
      if (stepInfo instanceof TokenSwapInfo) {
        const swapKeys = stepInfo.toKeys();
        keys.push(...swapKeys);
      } else if (stepInfo instanceof SerumDexMarketInfo) {
        const swapKeys = stepInfo.toKeys();
        keys.push(...swapKeys);
      }
    });

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
    [address.toBuffer()].concat(Buffer.from(tokenSwapData.bumpSeed)),
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
    hostFeeAccount
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

  const openOrders = extMarketDecoded.openOrders;

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
