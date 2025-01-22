# Raydium Balancer Fetcher

This project is a simple implementation to get SOL/WSOL balance and SOL LP position in SOL-USDC.1bp liquidity pool in Raydium mainnet.

Note: 
- Only work for Solana mainnet as all program Ids are hardcoded in the code. 
- Just for learning purpose, not for production use.

## How to Run


Before running the project, you need Rust installed. Following the instructions on the [Rust official website](https://www.rust-lang.org/tools/install).

Get the wallet address in Solana mainnet, and execute the following command:

```shell
cargo run 53zSj4G935ZY2a5x2UnGAiJXSuXXmGHaLph2zhAUvYpg
```

The output will be like:

```shell
SOL Balance/Position Summary for address: 53zSj4G935ZY2a5x2UnGAiJXSuXXmGHaLph2zhAUvYpg
- SOL: 0.013955593
- WSOL: 0
- SOL Unified (SOL + WSOL): 0.013955593
- SOL in SOL-USDC.1bp LP Position: 178.603037773
```
