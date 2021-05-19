/// <reference types="node" />
import BN from 'bn.js';
import type { Connection, TransactionSignature } from '@solana/web3.js';
import { Account, PublicKey, TransactionInstruction } from '@solana/web3.js';
export declare const TOKEN_SWAP_PROGRAM_ID: PublicKey;
/**
 * Some amount of tokens
 */
export declare class Numberu64 extends BN {
    /**
     * Convert to Buffer representation
     */
    toBuffer(): Buffer;
    /**
     * Construct a Numberu64 from Buffer representation
     */
    static fromBuffer(buffer: Buffer): Numberu64;
}
export declare const TokenSwapLayout: any;
export declare const CurveType: Readonly<{
    ConstantProduct: number;
    ConstantPrice: number;
    Offset: number;
}>;
/**
 * A program to exchange tokens against a pool of liquidity
 */
export declare class TokenSwap {
    private connection;
    tokenSwap: PublicKey;
    swapProgramId: PublicKey;
    tokenProgramId: PublicKey;
    poolToken: PublicKey;
    feeAccount: PublicKey;
    authority: PublicKey;
    tokenAccountA: PublicKey;
    tokenAccountB: PublicKey;
    mintA: PublicKey;
    mintB: PublicKey;
    tradeFeeNumerator: Numberu64;
    tradeFeeDenominator: Numberu64;
    ownerTradeFeeNumerator: Numberu64;
    ownerTradeFeeDenominator: Numberu64;
    ownerWithdrawFeeNumerator: Numberu64;
    ownerWithdrawFeeDenominator: Numberu64;
    hostFeeNumerator: Numberu64;
    hostFeeDenominator: Numberu64;
    curveType: number;
    payer: Account;
    /**
     * Create a Token object attached to the specific token
     *
     * @param connection The connection to use
     * @param tokenSwap The token swap account
     * @param swapProgramId The program ID of the token-swap program
     * @param tokenProgramId The program ID of the token program
     * @param poolToken The pool token
     * @param authority The authority over the swap and accounts
     * @param tokenAccountA The token swap's Token A account
     * @param tokenAccountB The token swap's Token B account
     * @param mintA The mint of Token A
     * @param mintB The mint of Token B
     * @param tradeFeeNumerator The trade fee numerator
     * @param tradeFeeDenominator The trade fee denominator
     * @param ownerTradeFeeNumerator The owner trade fee numerator
     * @param ownerTradeFeeDenominator The owner trade fee denominator
     * @param ownerWithdrawFeeNumerator The owner withdraw fee numerator
     * @param ownerWithdrawFeeDenominator The owner withdraw fee denominator
     * @param hostFeeNumerator The host fee numerator
     * @param hostFeeDenominator The host fee denominator
     * @param curveType The curve type
     * @param payer Pays for the transaction
     */
    constructor(connection: Connection, tokenSwap: PublicKey, swapProgramId: PublicKey, tokenProgramId: PublicKey, poolToken: PublicKey, feeAccount: PublicKey, authority: PublicKey, tokenAccountA: PublicKey, tokenAccountB: PublicKey, mintA: PublicKey, mintB: PublicKey, tradeFeeNumerator: Numberu64, tradeFeeDenominator: Numberu64, ownerTradeFeeNumerator: Numberu64, ownerTradeFeeDenominator: Numberu64, ownerWithdrawFeeNumerator: Numberu64, ownerWithdrawFeeDenominator: Numberu64, hostFeeNumerator: Numberu64, hostFeeDenominator: Numberu64, curveType: number, payer: Account);
    /**
     * Get the minimum balance for the token swap account to be rent exempt
     *
     * @return Number of lamports required
     */
    static getMinBalanceRentForExemptTokenSwap(connection: Connection): Promise<number>;
    static createInitSwapInstruction(tokenSwapAccount: Account, authority: PublicKey, tokenAccountA: PublicKey, tokenAccountB: PublicKey, tokenPool: PublicKey, feeAccount: PublicKey, tokenAccountPool: PublicKey, tokenProgramId: PublicKey, swapProgramId: PublicKey, nonce: number, tradeFeeNumerator: number, tradeFeeDenominator: number, ownerTradeFeeNumerator: number, ownerTradeFeeDenominator: number, ownerWithdrawFeeNumerator: number, ownerWithdrawFeeDenominator: number, hostFeeNumerator: number, hostFeeDenominator: number, curveType: number): TransactionInstruction;
    static loadTokenSwap(connection: Connection, address: PublicKey, programId: PublicKey, payer: Account): Promise<TokenSwap>;
    /**
     * Create a new Token Swap
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
    static createTokenSwap(connection: Connection, payer: Account, tokenSwapAccount: Account, authority: PublicKey, tokenAccountA: PublicKey, tokenAccountB: PublicKey, poolToken: PublicKey, mintA: PublicKey, mintB: PublicKey, feeAccount: PublicKey, tokenAccountPool: PublicKey, swapProgramId: PublicKey, tokenProgramId: PublicKey, nonce: number, tradeFeeNumerator: number, tradeFeeDenominator: number, ownerTradeFeeNumerator: number, ownerTradeFeeDenominator: number, ownerWithdrawFeeNumerator: number, ownerWithdrawFeeDenominator: number, hostFeeNumerator: number, hostFeeDenominator: number, curveType: number): Promise<TokenSwap>;
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
    swap(userSource: PublicKey, poolSource: PublicKey, poolDestination: PublicKey, userDestination: PublicKey, hostFeeAccount: PublicKey | null, userTransferAuthority: Account, amountIn: number | Numberu64, minimumAmountOut: number | Numberu64): Promise<TransactionSignature>;
    static swapInstruction(tokenSwap: PublicKey, authority: PublicKey, userTransferAuthority: PublicKey, userSource: PublicKey, poolSource: PublicKey, poolDestination: PublicKey, userDestination: PublicKey, poolMint: PublicKey, feeAccount: PublicKey, hostFeeAccount: PublicKey | null, swapProgramId: PublicKey, tokenProgramId: PublicKey, amountIn: number | Numberu64, minimumAmountOut: number | Numberu64): TransactionInstruction;
    /**
     * Deposit tokens into the pool
     * @param userAccountA User account for token A
     * @param userAccountB User account for token B
     * @param poolAccount User account for pool token
     * @param userTransferAuthority Account delegated to transfer user's tokens
     * @param poolTokenAmount Amount of pool tokens to mint
     * @param maximumTokenA The maximum amount of token A to deposit
     * @param maximumTokenB The maximum amount of token B to deposit
     */
    depositAllTokenTypes(userAccountA: PublicKey, userAccountB: PublicKey, poolAccount: PublicKey, userTransferAuthority: Account, poolTokenAmount: number | Numberu64, maximumTokenA: number | Numberu64, maximumTokenB: number | Numberu64): Promise<TransactionSignature>;
    static depositAllTokenTypesInstruction(tokenSwap: PublicKey, authority: PublicKey, userTransferAuthority: PublicKey, sourceA: PublicKey, sourceB: PublicKey, intoA: PublicKey, intoB: PublicKey, poolToken: PublicKey, poolAccount: PublicKey, swapProgramId: PublicKey, tokenProgramId: PublicKey, poolTokenAmount: number | Numberu64, maximumTokenA: number | Numberu64, maximumTokenB: number | Numberu64): TransactionInstruction;
    /**
     * Withdraw tokens from the pool
     *
     * @param userAccountA User account for token A
     * @param userAccountB User account for token B
     * @param poolAccount User account for pool token
     * @param userTransferAuthority Account delegated to transfer user's tokens
     * @param poolTokenAmount Amount of pool tokens to burn
     * @param minimumTokenA The minimum amount of token A to withdraw
     * @param minimumTokenB The minimum amount of token B to withdraw
     */
    withdrawAllTokenTypes(userAccountA: PublicKey, userAccountB: PublicKey, poolAccount: PublicKey, userTransferAuthority: Account, poolTokenAmount: number | Numberu64, minimumTokenA: number | Numberu64, minimumTokenB: number | Numberu64): Promise<TransactionSignature>;
    static withdrawAllTokenTypesInstruction(tokenSwap: PublicKey, authority: PublicKey, userTransferAuthority: PublicKey, poolMint: PublicKey, feeAccount: PublicKey, sourcePoolAccount: PublicKey, fromA: PublicKey, fromB: PublicKey, userAccountA: PublicKey, userAccountB: PublicKey, swapProgramId: PublicKey, tokenProgramId: PublicKey, poolTokenAmount: number | Numberu64, minimumTokenA: number | Numberu64, minimumTokenB: number | Numberu64): TransactionInstruction;
    /**
     * Deposit one side of tokens into the pool
     * @param userAccount User account to deposit token A or B
     * @param poolAccount User account to receive pool tokens
     * @param userTransferAuthority Account delegated to transfer user's tokens
     * @param sourceTokenAmount The amount of token A or B to deposit
     * @param minimumPoolTokenAmount Minimum amount of pool tokens to mint
     */
    depositSingleTokenTypeExactAmountIn(userAccount: PublicKey, poolAccount: PublicKey, userTransferAuthority: Account, sourceTokenAmount: number | Numberu64, minimumPoolTokenAmount: number | Numberu64): Promise<TransactionSignature>;
    static depositSingleTokenTypeExactAmountInInstruction(tokenSwap: PublicKey, authority: PublicKey, userTransferAuthority: PublicKey, source: PublicKey, intoA: PublicKey, intoB: PublicKey, poolToken: PublicKey, poolAccount: PublicKey, swapProgramId: PublicKey, tokenProgramId: PublicKey, sourceTokenAmount: number | Numberu64, minimumPoolTokenAmount: number | Numberu64): TransactionInstruction;
    /**
     * Withdraw tokens from the pool
     *
     * @param userAccount User account to receive token A or B
     * @param poolAccount User account to burn pool token
     * @param userTransferAuthority Account delegated to transfer user's tokens
     * @param destinationTokenAmount The amount of token A or B to withdraw
     * @param maximumPoolTokenAmount Maximum amount of pool tokens to burn
     */
    withdrawSingleTokenTypeExactAmountOut(userAccount: PublicKey, poolAccount: PublicKey, userTransferAuthority: Account, destinationTokenAmount: number | Numberu64, maximumPoolTokenAmount: number | Numberu64): Promise<TransactionSignature>;
    static withdrawSingleTokenTypeExactAmountOutInstruction(tokenSwap: PublicKey, authority: PublicKey, userTransferAuthority: PublicKey, poolMint: PublicKey, feeAccount: PublicKey, sourcePoolAccount: PublicKey, fromA: PublicKey, fromB: PublicKey, userAccount: PublicKey, swapProgramId: PublicKey, tokenProgramId: PublicKey, destinationTokenAmount: number | Numberu64, maximumPoolTokenAmount: number | Numberu64): TransactionInstruction;
}
