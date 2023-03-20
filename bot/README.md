# Bot ![license](https://img.shields.io/badge/License-MIT-green.svg?label=license)

Bot logic relies heavily on REVM simulations to detect sandwichable transactions. The simulations are done by injecting a modified router contract called [`BrainDance.sol`](https://github.com/mouseless-eth/rusty-sando/blob/master/contract/src/BrainDance.sol) into a new EVM instance. Once injected, a concurrent binary search is performed to find a optimal input amount that results in the highest revenue. After sandwich calculations, the bot performs a [salmonella](https://github.com/Defi-Cartel/salmonella) check. If the sandwich is salmonella free, the bot then calculates gas bribes and sends bundle if profitable. 

Performing EVM simulations in this way allows the bot to detect sandwichable opportunities against any tx that introduces slippage. 

## Logic Breakdown
- At startup, index all pools from a specific factory by parsing the `PairCreated` event. And get all token dust stored on sando addy.
- Read and decode tx from mempool.
- Send tx to [`trace_CallMany`](https://openethereum.github.io/JSONRPC-trace-module#trace_callmany) to obtain `stateDiff`. (could modify to use any other rpc that returns stateDiff)
- Check if `statediff` contains keys that correspond to indexed pool addresses.
- Construct a new EVM database instance from `stateDiff`, used for local simulations.
- For each pool that tx touches:
  - Find the optimal amount in for a sandwich attack by performing a concurrent binary search.
  - Check for salmonella by checking if tx uses unconventional opcodes.
- If profitable after gas calculations, send bundle to relays. 
- Store sandwich opportunity in backlog for multi meat sandwich calculations.

## Usage

1. This repo requires you to run an [Erigon](https://github.com/ledgerwatch/erigon) archive node. The bot relies on the `newPendingTransactionsWithBody` subscription endpoint and `trace_callMany` rpc which are Erigon specific methods. Node needs to be synced in archive mode to index all pools. 

2. [Install Rust](https://www.rust-lang.org/tools/install) if you haven't already. 

3. Fill in searcher address in Huff contract and deploy either straight onchain or via create2 using a [metamorphic](https://github.com/0age/metamorphic) like factory.
> If you are using create2, you can easily mine for an address containing 7 zero bytes, saving 84 gas of calldata everytime the contract address is used as an argument. [read more](https://medium.com/coinmonks/deploy-an-efficient-address-contract-a-walkthrough-cb4be4ffbc70).

4. Copy `.env.example` into `.env` and fill out values.

```console
cp .env.example .env
```

```
RPC_URL_WSS=ws://localhost:8545
SEARCHER_PRIVATE_KEY=0000000000000000000000000000000000000000000000000000000000000001
FLASHBOTS_AUTH_KEY=0000000000000000000000000000000000000000000000000000000000000002
SANDWICH_CONTRACT=0xaAaAaAaaAaAaAaaAaAAAAAAAAaaaAaAaAaaAaaAa
V2_ALERT_DISCORD_WEBHOOK=...
V3_ALERT_DISCORD_WEBHOOK=...
POISON_ALERT_DISCORD_WEBHOOK=...
SANDWICH_INCEPTION_BLOCK=... // block that sandwich contract was deployed in
```

5. Before running backtests get the runtime bytecode of the contract and set it to [`get_test_sandwich_code`](https://github.com/mouseless-eth/rusty-sando/blob/5ddeb4bbf703420de3cd5bc2b0d6885fce4cb0a4/bot/src/utils/constants.rs#L26) in constants.rs.

```console
huffc --bin-runtime contract/src/sandwich.huff
```

5. Run the tests

```
cargo test --release -- --nocapture
```

6. Create a binary executable

```
cargo run --bin rusty-sando --release
```
> **Note**
> with the `--release` flag, the rust compiler will compile with optimizations. These optimizations are important because they speed up REVM simulations 10x. 
>
> **Warning**
>
> **By taking this codebase into production, you are doing so at your own risk under the MIT license.** Although heavily tested, I cannot gurantee that it is bug free. I prefer this codebase to be used as a case study of what MEV could look like using Rust and Huff. 

### Blueprint

```
src
├── lib.rs
├── main.rs
├── abi - Holds contract abis 
│   └── ...
├── cfmm - Holds logic to index pools
│   └── ...
├── forked_db
│   ├── ...
│   ├── fork_db.rs - Local EVM instance for simulations
│   ├── fork_factory.rs - Creates `fork_db` instances and maintains connection with `global_backend`
│   └── global_backend.rs - Makes and caches rpc calls for missing state
├── runner
│   ├── mod.rs - Main runtime logic lives here
│   ├── bundle_sender.rs - Wrapper to submit bundles
│   ├── oracles.rs - Create execution environments for oracles
│   └── state.rs - Holds information about bot state
├── simulate
│   ...
│   ├── inspectors
│   │   ├── access_list.rs - Locally create access list for sandwich txs
│   │   └── is_sando_safu.rs - Salmonella checker
│   └── make_sandwich.rs - Optimal sandwich calculations and sanity checks
├── types - Common types used throughout codebase
└── utils
    ├── ...
    └── tx_builder - Logic to encode transactions
        └── ...
```

### Oracles
There are three important oracles running on their own thread:

- **NextBlockOracle**: Every new block, update `latestBlock` and `nextBlock` block number, timestamp, and basefee. 
- **UpdatePoolOracle**: Every 50 blocks, add any new pools created. 
- **MegaSandwichOracle**: Every 10.5 seconds after the latest block, search sandwich backlog to detect for multi meat sandwiches. 


## Improvements

This repo explores only basic and simple multi V2 and V3 sandwiches, however sandwiches come in many flavours and require some modifications to the codebase to capture them:

- Stable coin pair sandwiches.
- Sandwiches involving pairs that have a transfer limit, an [example](https://eigenphi.io/mev/ethereum/tx/0xe7c1e7d96e63d31f937af48b61d534e32ed9cfdbef066f45d49b967caeea8eed). Transfer limit can be found using a method similiar to [Fej:Leuros's implementation](https://twitter.com/FejLeuros/status/1633379306750767106).
- Multi meat sandwiches that target more than one pool. example: [frontrun](https://etherscan.io/tx/0xa39d28624f6d18a3bd5f5289a70fdc2779782f9a2e2c36dddd95cf882a15da45), [meat1](https://etherscan.io/tx/0xd027b771da68544279262439fd3f1cdef6a438ab6219b510c73c033b4e377296), [meat2](https://etherscan.io/tx/0x288da393cb7c937b8fe29ce0013992063d252372da869e31c6aad689f8b1aaf3), [backrun](https://etherscan.io/tx/0xcf22f2a3c9c67d56282e77e60c09929e0451336a9ed38f037fd484ea29e3cd41).
- Token -> Weth sandwiches by using a 'flashswap' between two pools. Normally we can only sandwich Weth -> Token swaps as the bot has Weth inventory, however you can use another pool's reserves as inventory to sandwich swaps in the other direction. [example](https://eigenphi.io/mev/ethereum/tx/0x502b66ce1a8b71098decc3585c651745c1af55de19e8f29ec6fff4ed2fcd1589).
- Flashloan sandwiches for larger value swaps.
- Sandwiches that include a users token approval tx + swap tx in one bundle. 
- Sandwiches that include a users pending tx/s + swap tx in one bundle if swap tx nonce is higher than pending txs. 
