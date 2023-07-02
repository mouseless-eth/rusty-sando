use anyhow::Result;
use cfmms::pool::Pool;
use ethers::{
    providers::Middleware,
    signers::LocalWallet,
    types::{AccountDiff, Address, Transaction, U256},
};
use foundry_evm::executor::fork::{BlockchainDb, BlockchainDbMeta, SharedBackend};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use crate::managers::block_manager::BlockInfo;

use super::minimal_router::braindance::find_optimal_input;

#[allow(unused_mut)]
pub async fn create_optimal_sandwich<M: Middleware + 'static>(
    meats: Vec<Transaction>,
    target_pool: Pool,
    target_block: BlockInfo,
    mut weth_inventory: U256,
    searcher: Address,
    sando_address: Address,
    provider: Arc<M>,
) -> Result<U256> {
    let shared_backend = SharedBackend::spawn_backend_thread(
        provider.clone(),
        new_db(&Default::default()), /* default because not accounting for this atm */
        Some((target_block.number - 1).into()),
    );

    #[cfg(feature = "debug")]
    {
        weth_inventory = (*crate::constants::WETH_FUND_AMT).into(); // Set a new value only when the debug feature is active
    }

    let optimal = find_optimal_input(
        meats,
        target_pool,
        target_block,
        weth_inventory,
        shared_backend,
    )
    .await?;

    Ok(optimal)
}

// Ideally we want to create db from call_trace `stateDiffs`
fn new_db(_state_diffs: &BTreeMap<Address, AccountDiff>) -> BlockchainDb {
    let meta = BlockchainDbMeta {
        cfg_env: Default::default(),
        block_env: Default::default(),
        hosts: BTreeSet::from(["".to_string()]),
    };

    BlockchainDb::new(meta, None)
}
