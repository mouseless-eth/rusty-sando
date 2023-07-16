use anyhow::Result;
use cfmms::pool::Pool;
use ethers::{
    providers::Middleware,
    types::{AccountDiff, Address, Transaction, U256},
};
use foundry_evm::executor::fork::{BlockchainDb, BlockchainDbMeta, SharedBackend};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use crate::managers::block_manager::BlockInfo;

use super::lil_router::find_optimal_input;

// Ideally we want to create db from call_trace `stateDiffs`
fn new_db(_state_diffs: &BTreeMap<Address, AccountDiff>) -> BlockchainDb {
    let meta = BlockchainDbMeta {
        cfg_env: Default::default(),
        block_env: Default::default(),
        hosts: BTreeSet::from(["".to_string()]),
    };

    BlockchainDb::new(meta, None)
}
