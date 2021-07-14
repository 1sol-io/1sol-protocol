import assert from "assert";
import BN from "bn.js";
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
} from "@solana/spl-token";

export const ONESOL_PROTOCOL_PROGRAM_ID: PublicKey = new PublicKey(
  "HEQQHE6U6xp4aurpZFoBNguusLWs3cyyxV9A2qUA9cQo"
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

export const OneSolProtocolLayout = BufferLayout.struct([
  BufferLayout.u8("version"),
  BufferLayout.u8("nonce"),
  publicKeyLayout("tokenProgramId"),
  publicKeyLayout("tokenAccount"),
  publicKeyLayout("mint"),
]);

export class TokenSwapInfo {
  constructor(
    private programId: PublicKey,
    private swapInfo: PublicKey,
    private authority: PublicKey,
    private poolSource: PublicKey,
    private poolDestination: PublicKey,
    private poolMint: PublicKey,
    private poolFeeAccount: PublicKey,
    private hostFeeAccount: PublicKey | null
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

  toKeys(): Array<AccountMeta> {
    const keys = [
      { pubkey: this.swapInfo, isSigner: false, isWritable: false },
      { pubkey: this.authority, isSigner: false, isWritable: false },
      { pubkey: this.poolSource, isSigner: false, isWritable: true },
      { pubkey: this.poolDestination, isSigner: false, isWritable: true },
      { pubkey: this.poolMint, isSigner: false, isWritable: true },
      { pubkey: this.poolFeeAccount, isSigner: false, isWritable: true },
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
  constructor(public programId: PublicKey, public market: Market) {
    this.programId = programId;
    this.market = market;
  }

  public static create(market: Market): SerumDexMarketInfo {
    return new SerumDexMarketInfo(market.programId, market);
  }

  dataLayout(): Array<any> {
    return [
      uint64("amount_in"),
      BufferLayout.u8("side"),
      uint64("rate"),
      BufferLayout.u8("from_decimals"),
      BufferLayout.u8("to_decimals"),
      BufferLayout.u8("strict"),
    ];
  }

  // dataMap(amount_in: Numberu64, side: number, rate: Numberu64, from_decimals: number, to_decimals: number, strict: number): Buffer {
  //   return {
  //     amount_in: amount_in.toBuffer(),
  //     serumDexAccountsSize: 11,
  //     serumDexPrice: this.limitPrice.toBuffer(),
  //     serumDexMaxCoinQty: this.maxCoinQty.toBuffer(),
  //     serumDexMaxPcQty: this.maxPcQty.toBuffer(),
  //     serumDexClientId: this.clientId.toBuffer(),
  //   };
  // }

  async toKeys(
    openOrderAccountKey: PublicKey,
    openOrderOwnerKey: PublicKey
  ): Promise<Array<AccountMeta>> {
    const vaultSigner = await PublicKey.createProgramAddress(
      [
        this.market.address.toBuffer(),
        this.market.decoded.vaultSignerNonce.toArrayLike(Buffer, "le", 8),
      ],
      this.programId
    );
    const keys = [
      { pubkey: this.market.publicKey, isSigner: false, isWritable: true },
      {
        pubkey: this.market.decoded.requestQueue,
        isSigner: false,
        isWritable: true,
      },
      {
        pubkey: this.market.decoded.eventQueue,
        isSigner: false,
        isWritable: true,
      },
      { pubkey: this.market.bidsAddress, isSigner: false, isWritable: true },
      { pubkey: this.market.asksAddress, isSigner: false, isWritable: true },
      {
        pubkey: this.market.decoded.baseVault,
        isSigner: false,
        isWritable: true,
      },
      {
        pubkey: this.market.decoded.quoteVault,
        isSigner: false,
        isWritable: true,
      },
      { pubkey: vaultSigner, isSigner: false, isWritable: false },
      { pubkey: openOrderAccountKey, isSigner: false, isWritable: false },
      { pubkey: openOrderOwnerKey, isSigner: true, isWritable: false },
      { pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false },
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
    public protocolInfo: PublicKey,
    public protocolProgramId: PublicKey,
    public tokenProgramId: PublicKey,
    public tokenAccountKey: PublicKey,
    public authority: PublicKey,
    public nonce: number,
    public wallet: PublicKey
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
    wallet: PublicKey
  ): Promise<OneSolProtocol> {
    const data = await loadAccount(connection, address, programId);
    const onesolProtocolData = OneSolProtocolLayout.decode(data);
    if (onesolProtocolData.version != 1) {
      throw new Error(`Invalid OneSolProtocol data`);
    }
    const [authority] = await PublicKey.findProgramAddress(
      [address.toBuffer()],
      programId
    );

    return new OneSolProtocol(
      connection,
      address,
      programId,
      new PublicKey(onesolProtocolData.tokenProgramId),
      new PublicKey(onesolProtocolData.tokenAccount),
      authority,
      onesolProtocolData.nonce,
      wallet
    );
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
    protocolAccountInfo: Keypair,
    tokenAccountKey: PublicKey,
    tokenProgramId: PublicKey,
    authority: PublicKey,
    nonce: number,
    payer: Keypair,
    protocolProgramId: PublicKey
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
      payer.publicKey
    );

    // Allocate memory for the account
    const balanceNeeded =
      await OneSolProtocol.getMinBalanceRentForExemptTokenSwap(connection);
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
      })
    );
    const instruction = OneSolProtocol.createInitSwapInstruction(
      protocolAccountInfo,
      authority,
      tokenAccountKey,
      protocolProgramId,
      tokenProgramId,
      nonce
    );

    transaction.add(instruction);
    await realSendAndConfirmTransaction(
      "createAccount and InitializeSwap",
      connection,
      transaction,
      payer,
      protocolAccountInfo
    );
    return onesolSwap;
  }

  /**
   Get the minimum balance for the token swap account to be rent exempt
  
   @return Number of lamports required
  **/
  static async getMinBalanceRentForExemptTokenSwap(
    connection: Connection
  ): Promise<number> {
    return await connection.getMinimumBalanceForRentExemption(
      OneSolProtocolLayout.span
    );
  }

  static createInitSwapInstruction(
    onesolProtocolAccount: Keypair,
    authority: PublicKey,
    tokenAccount: PublicKey,
    protocolProgramId: PublicKey,
    tokenProgramId: PublicKey,
    nonce: number
  ): TransactionInstruction {
    const keys = [
      {
        pubkey: onesolProtocolAccount.publicKey,
        isSigner: false,
        isWritable: true,
      },
      { pubkey: authority, isSigner: false, isWritable: false },
      { pubkey: tokenAccount, isSigner: false, isWritable: false },
      { pubkey: tokenProgramId, isSigner: false, isWritable: false },
    ];
    const commandDataLayout = BufferLayout.struct([
      BufferLayout.u8("instruction"),
      BufferLayout.u8("nonce"),
    ]);
    let data = Buffer.alloc(1024);
    {
      const encodeLength = commandDataLayout.encode(
        {
          instruction: 0,
          nonce,
        },
        data
      );
      data = data.slice(0, encodeLength);
    }
    return new TransactionInstruction({
      keys,
      programId: protocolProgramId,
      data,
    });
  }

  async createSwapTokenSwapInstruction(
    fromTokenAccountKey: PublicKey,
    toTokenAccountKey: PublicKey,
    userTransferAuthority: Keypair,
    amountIn: number | Numberu64,
    minimumAmountOut: number | Numberu64,
    splTokenSwapInfo: TokenSwapInfo,
    instructions: Array<TransactionInstruction>,
    signers: Array<Signer>
  ): Promise<void> {
    instructions.push(
      await OneSolProtocol.makeSwapTokenSwapInstruction(
        this.protocolInfo,
        this.authority,
        this.tokenAccountKey,
        userTransferAuthority.publicKey,
        fromTokenAccountKey,
        toTokenAccountKey,
        this.tokenProgramId,
        splTokenSwapInfo,
        this.protocolProgramId,
        amountIn,
        minimumAmountOut
      )
    );
    signers.push(userTransferAuthority);
  }

  async createSwapSerumDexInstruction(
    fromTokenAccountKey: PublicKey,
    toTokenAccountKey: PublicKey,
    marketInfo: SerumDexMarketInfo,
    fromTokenMintInfo: TokenMintInfo,
    toTokenMintInfo: TokenMintInfo,
    amountIn: number | Numberu64,
    minimumAmountOut: number | Numberu64,
    instructions: Array<TransactionInstruction>,
    signers: Array<Signer>
  ): Promise<void> {
    // TODO open order
    const market = marketInfo.market;
    const openOrder = Keypair.generate();
    instructions.push(
      await OpenOrders.makeCreateAccountTransaction(
        this.connection,
        market.address,
        this.wallet,
        openOrder.publicKey,
        market.programId
      )
    );
    signers.push(openOrder);

    instructions.push(
      await OneSolProtocol.makeSwapSerumDexInstruction(
        this.protocolInfo,
        this.authority,
        this.tokenAccountKey,
        fromTokenAccountKey,
        toTokenAccountKey,
        this.tokenProgramId,
        openOrder.publicKey,
        this.wallet,
        marketInfo,
        this.protocolProgramId,
        fromTokenMintInfo,
        toTokenMintInfo,
        new Numberu64(amountIn),
        new Numberu64(minimumAmountOut)
      )
    );
  }

  static async makeSwapTokenSwapInstruction(
    protocolAccount: PublicKey,
    protocolAuthority: PublicKey,
    protocolToken: PublicKey,
    userTransferAuthority: PublicKey,
    userSource: PublicKey,
    userDestination: PublicKey,
    tokenProgramId: PublicKey,
    splTokenSwapInfo: TokenSwapInfo,
    protocolProgramId: PublicKey,
    amountIn: number | Numberu64,
    minimumAmountOut: number | Numberu64
  ): Promise<TransactionInstruction> {
    const bflStruct: any = [
      BufferLayout.u8("instruction"),
      uint64("amountIn"),
      uint64("minimumAmountOut"),
    ];
    let dataMap: any = {
      instruction: 1, // Swap instruction
      amountIn: new Numberu64(amountIn).toBuffer(),
      minimumAmountOut: new Numberu64(minimumAmountOut).toBuffer(),
    };

    const keys = [
      { pubkey: protocolAccount, isSigner: false, isWritable: false },
      { pubkey: protocolAuthority, isSigner: false, isWritable: false },
      { pubkey: protocolToken, isSigner: false, isWritable: true },
      { pubkey: userSource, isSigner: false, isWritable: true },
      { pubkey: userDestination, isSigner: false, isWritable: true },
      { pubkey: tokenProgramId, isSigner: false, isWritable: false },
      { pubkey: userTransferAuthority, isSigner: true, isWritable: false },
    ];
    const swapKeys = splTokenSwapInfo.toKeys();
    keys.push(...swapKeys);

    const dataLayout = BufferLayout.struct(bflStruct);
    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode(dataMap, data);

    return new TransactionInstruction({
      keys,
      programId: protocolProgramId,
      data,
    });
  }

  static async makeSwapSerumDexInstruction(
    protocolAccount: PublicKey,
    protocolAuthority: PublicKey,
    protocolToken: PublicKey,
    fromTokenAccountKey: PublicKey,
    toTokenAccountKey: PublicKey,
    tokenProgramId: PublicKey,
    openOrderAccountKey: PublicKey,
    openOrderOwnerKey: PublicKey,
    marketInfo: SerumDexMarketInfo,
    protocolProgramId: PublicKey,
    fromTokenMintInfo: TokenMintInfo,
    toTokenMintInfo: TokenMintInfo,
    amountIn: Numberu64,
    minimumAmountOut: Numberu64
    // side: "buy" | "sell",
    // exchangeRate: number | Numberu64
  ): Promise<TransactionInstruction> {
    if (
      !fromTokenMintInfo.pubkey.equals(marketInfo.market.baseMintAddress) &&
      !fromTokenMintInfo.pubkey.equals(marketInfo.market.quoteMintAddress)
    ) {
      throw new Error("aTokenMint must be baseMintAddress or quoteMintAddress");
    }
    if (
      !toTokenMintInfo.pubkey.equals(marketInfo.market.baseMintAddress) &&
      !toTokenMintInfo.pubkey.equals(marketInfo.market.quoteMintAddress)
    ) {
      throw new Error("bTokenMint must be baseMintAddress or quoteMintAddress");
    }
    const bflStruct: any = [
      BufferLayout.u8("instruction"),
      uint64("amount_in"),
      BufferLayout.u8("side"),
      uint64("rate"),
      BufferLayout.u8("from_decimals"),
      BufferLayout.u8("to_decimals"),
      BufferLayout.u8("strict"),
    ];
    const side = marketInfo.market.baseMintAddress.equals(
      fromTokenMintInfo.pubkey
    )
      ? 1
      : 0;
    const exchangeRate = minimumAmountOut.div(
      amountIn.div(new BN(10).pow(new BN(fromTokenMintInfo.mintInfo.decimals)))
    );
    console.log("side: " + side + ", exchangeRate: " + exchangeRate);
    let dataMap: any = {
      instruction: 2, // Swap instruction
      amount_in: new Numberu64(amountIn).toBuffer(),
      side: side,
      rate: new Numberu64(exchangeRate.toNumber()).toBuffer(),
      from_decimals: fromTokenMintInfo.mintInfo.decimals,
      to_decimals: toTokenMintInfo.mintInfo.decimals,
      strict: 1,
    };

    const keys = [
      { pubkey: protocolAccount, isSigner: false, isWritable: false },
      { pubkey: protocolAuthority, isSigner: false, isWritable: false },
      { pubkey: protocolToken, isSigner: false, isWritable: true },
      { pubkey: fromTokenAccountKey, isSigner: false, isWritable: true },
      { pubkey: toTokenAccountKey, isSigner: false, isWritable: true },
      { pubkey: tokenProgramId, isSigner: false, isWritable: false },
    ];
    const swapKeys = await marketInfo.toKeys(
      openOrderAccountKey,
      openOrderOwnerKey
    );
    keys.push(...swapKeys);

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
    commitment: "recent",
    preflightCommitment: "recent",
  });
}

export function deserializeAccount(info: {
  pubkey: PublicKey;
  account: AccountInfo<Buffer>;
}) {
  const data = OneSolProtocolLayout.decode(info.account.data);

  const details = {
    pubkey: info.pubkey,
    account: info.account,
    info: data,
  };
  return details;
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

  const [authority] = await PublicKey.findProgramAddress(
    [address.toBuffer()],
    programId
  );

  const poolToken = new PublicKey(tokenSwapData.tokenPool);
  const feeAccount = new PublicKey(tokenSwapData.feeAccount);
  const tokenAccountA = new PublicKey(tokenSwapData.tokenAccountA);
  const tokenAccountB = new PublicKey(tokenSwapData.tokenAccountB);

  return new TokenSwapInfo(
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

// export interface TokenMintInfo {
//   pubkey: PublicKey;
//   mintInfo: TokenMint;
// }

export async function loadSerumDexMarket(
  connection: Connection,
  address: PublicKey,
  programId: PublicKey
): Promise<SerumDexMarketInfo> {
  const market = await Market.load(connection, address, {}, programId);
  return new SerumDexMarketInfo(programId, market);
}

export async function findOneSolProtocol(
  connection: Connection,
  tokenMintPubkey: PublicKey,
  walletAddress: PublicKey,
  programId?: PublicKey
): Promise<OneSolProtocol> {
  const accounts = await connection.getProgramAccounts(
    programId ? programId : ONESOL_PROTOCOL_PROGRAM_ID,
    {
      encoding: "base64",
      filters: [
        {
          memcmp: {
            offset: OneSolProtocolLayout.offsetOf("mint"),
            bytes: tokenMintPubkey.toBase58(),
          },
        },
      ],
    }
  );
  const [account] = accounts;
  if (!account) {
    throw new Error(`Could not find OneSolProtocol account`);
  }
  return await getOneSolProtocol(
    OneSolProtocolLayout.decode(account.account.data),
    connection,
    account.pubkey,
    account.account.owner,
    walletAddress
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
    programId
  );

  return new OneSolProtocol(
    connection,
    address,
    programId,
    new PublicKey(onesolProtocolData.tokenProgramId),
    new PublicKey(onesolProtocolData.tokenAccount),
    authority,
    onesolProtocolData.nonce,
    wallet
  );
}
