# 1sol contract

Recently, there have been increasing number of projects focused on infrastructure building including DEXs, lending and DeFi platforms on Solana. The emergence of DEXs and building out aggregators will bring a lot of benefits for DeFi on the Solana ecosystem. In the field of DeFi, there have been more and more transactions that want to get onto to Solana via cross-chain, and currently we don't have a convenient way to do so.

So we decided to create 1Sol.

## Goals

 - Go-To Trading Portal: Be the go-to trading portal for trading on Solana.
 - One-Stop Aggregation Service: Integrate all kinds of DeFi and DEX.
 - Fool-Proof Operation: Provides the average user with information on prices, slippage and costs of all DEX on the web. Users can choose for themselves the one path that best suits them to trade.

## How to develop
1. `yarn install`
2. `yarn cluster:localhost`
3. deploy your dev token-swap program
4. modify all token-swap program id to your id
5. deploy onesol-protocol program
6. modify onesol-protocol program id in src/js/src/index.ts
6. run `yarn start` to test

### Program id
```
Dm4bCGrSQ4rKLn2f6SJ46BbHdaexFMjyrKy25qxAmZ7P
```

## TODO

 - Link with Solana's official swap.
 - Link with Serum swap.
 - Link with [dexlab](https://www.dexlab.space/).
 - Link with Falcomp.
