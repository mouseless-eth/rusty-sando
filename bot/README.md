# Rusty-Sando/Bot ![license](https://img.shields.io/badge/License-MIT-green.svg?label=license)

Bot logic relies heavily on REVM simulations to detect sandwichable transactions. The simulations are done by injecting a modified router contract called [`LilRouter.sol`](https://github.com/mouseless-eth/rusty-sando/blob/master/contract/src/LilRouter.sol) into a new EVM instance. Once injected, a concurrent binary search is performed to find an optimal input amount that results in the highest revenue. After sandwich calculations, the bot performs a [salmonella](https://github.com/Defi-Cartel/salmonella) check. If the sandwich is salmonella free, the bot then calculates gas bribes and sends the bundle to the fb relay. 

Performing EVM simulations in this way allows the bot to detect sandwichable opportunities against any tx that introduces slippage. 

## Logic Breakdown
- At startup, index all pools from a specific factory by parsing the `PairCreated` event. And fetch all token dust stored on sando addy.
- Read and decode tx from mempool.
- Send tx to [`trace_call`](https://openethereum.github.io/JSONRPC-trace-module#trace_call) to obtain `stateDiff`.
- Check if `statediff` contains keys that equal to indexed pool addresses.
- For each pool that tx touches:
  - Find the optimal amount in for a sandwich attack by performing a concurrent binary search.
  - Check for salmonella by checking if tx uses unconventional opcodes.
- If profitable after gas calculations, send the sando bundle to relays. 

## Usage

1. This repo requires you to run an [Erigon](https://github.com/ledgerwatch/erigon) archive node. The bot relies on the `newPendingTransactionsWithBody` subscription rpc endpoint which is a Erigon specific method. The node needs to be synced in archive mode to index all pools. 

2. [Install Rust](https://www.rust-lang.org/tools/install) if you haven't already. 

3. Fill in the searcher address in Huff contract and deploy either straight onchain or via create2 using a [metamorphic](https://github.com/0age/metamorphic) like factory.
> If you are using create2, you can easily mine for an address containing 7 zero bytes, saving 84 gas of calldata every time the contract address is used as an argument. [read more](https://medium.com/coinmonks/deploy-an-efficient-address-contract-a-walkthrough-cb4be4ffbc70).

4. Copy `.env.example` into `.env` and fill out values.

```console
cp .env.example .env
```

```
WSS_RPC=ws://localhost:8545
SEARCHER_PRIVATE_KEY=0000000000000000000000000000000000000000000000000000000000000001
FLASHBOTS_AUTH_KEY=0000000000000000000000000000000000000000000000000000000000000002
SANDWICH_CONTRACT=0xaAaAaAaaAaAaAaaAaAAAAAAAAaaaAaAaAaaAaaAa
SANDWICH_INCEPTION_BLOCK=...
```

5. Run the integration tests

```console
cargo test -p strategy --release --features debug
```

6. Run the bot in `debug mode`
Test bot's sandwich finding functionality without a deployed or funded Sando contract (no bundles will be sent)

```
cargo run --release --features debug
```

7. Running the bot

```console
cargo run --release
```
> **Warning**
>
> **By taking this codebase into production, you are doing so at your own risk under the MIT license.** Although heavily tested, I cannot gurantee that it is bug free. I prefer this codebase to be used as a case study of what MEV could look like using Rust and Huff. 

## Improvements

This repo explores only basic and simple multi V2 and V3 sandwiches, however, sandwiches come in many flavors and require some modifications to the codebase to capture them:

- Stable coin pair sandwiches.
- Sandwiches involving pairs that have a transfer limit, an [example](https://eigenphi.io/mev/ethereum/tx/0xe7c1e7d96e63d31f937af48b61d534e32ed9cfdbef066f45d49b967caeea8eed). Transfer limit can be found using a method similar to [Fej:Leuros's implementation](https://twitter.com/FejLeuros/status/1633379306750767106).
- Multi-meat sandwiches that target more than one pool. example: [frontrun](https://etherscan.io/tx/0xa39d28624f6d18a3bd5f5289a70fdc2779782f9a2e2c36dddd95cf882a15da45), [meat1](https://etherscan.io/tx/0xd027b771da68544279262439fd3f1cdef6a438ab6219b510c73c033b4e377296), [meat2](https://etherscan.io/tx/0x288da393cb7c937b8fe29ce0013992063d252372da869e31c6aad689f8b1aaf3), [backrun](https://etherscan.io/tx/0xcf22f2a3c9c67d56282e77e60c09929e0451336a9ed38f037fd484ea29e3cd41).
- Token -> Weth sandwiches by using a 'flashswap' between two pools. Normally we can only sandwich Weth -> Token swaps as the bot has Weth inventory, however you can use another pool's reserves as inventory to sandwich swaps in the other direction. [example](https://eigenphi.io/mev/ethereum/tx/0x502b66ce1a8b71098decc3585c651745c1af55de19e8f29ec6fff4ed2fcd1589).
- Longtail sandwiches to target TOKEN->(WETH or STABLE) swaps.
- Sandwiches that include a user's token approval tx + swap tx in one bundle. 
- Sandwiches that include a user's pending tx/s + swap tx in one bundle if swap tx nonce is higher than pending tx. 
