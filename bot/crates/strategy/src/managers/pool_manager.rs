use anyhow::{anyhow, Result};
use cfmms::{
    checkpoint::sync_pools_from_checkpoint,
    dex::{Dex, DexVariant},
    pool::Pool,
    sync::sync_pairs,
};
use colored::Colorize;
use dashmap::DashMap;
use ethers::{
    abi,
    providers::Middleware,
    types::{Address, BlockNumber, Diff, TraceType, Transaction, H160, H256, U256},
};
use log::info;
use std::{path::Path, str::FromStr, sync::Arc};

use crate::{constants::WETH_ADDRESS, startup_info_log};

pub(crate) struct PoolManager<M> {
    /// Provider
    provider: Arc<M>,
    /// Sandwichable pools
    pools: DashMap<Address, Pool>,
    /// Which dexes to monitor
    dexes: Vec<Dex>,
}

impl<M: Middleware + 'static> PoolManager<M> {
    /// Gets state of all pools
    pub async fn setup(&mut self) -> Result<()> {
        let checkpoint_path = ".cfmms-checkpoint.json";

        let checkpoint_exists = Path::new(checkpoint_path).exists();

        let pools = if checkpoint_exists {
            let (_, pools) =
                sync_pools_from_checkpoint(checkpoint_path, 100000, self.provider.clone()).await?;
            pools
        } else {
            sync_pairs(
                self.dexes.clone(),
                self.provider.clone(),
                Some(checkpoint_path),
            )
            .await?
        };

        for pool in pools {
            self.pools.insert(pool.address(), pool);
        }

        startup_info_log!("pools synced: {}", self.pools.len());

        Ok(())
    }

    /// Return a tx's touched pools
    // enhancement: record stable coin pairs to sandwich as well here
    pub async fn get_touched_sandwichable_pools(
        &self,
        victim_tx: &Transaction,
        latest_block: BlockNumber,
        provider: Arc<M>,
    ) -> Result<Vec<Pool>> {
        // get victim tx state diffs
        let state_diffs = provider
            .trace_call(victim_tx, vec![TraceType::StateDiff], Some(latest_block))
            .await?
            .state_diff
            .ok_or(anyhow!("not sandwichable, no state diffs produced"))?
            .0;

        // capture all addresses that have a state change and are also a `WETH` pool
        let touched_pools: Vec<Pool> = state_diffs
            .keys()
            .filter_map(|e| self.pools.get(e).map(|p| (*p.value()).clone()))
            .filter(|e| match e {
                Pool::UniswapV2(p) => vec![p.token_a, p.token_b].contains(&WETH_ADDRESS),
                Pool::UniswapV3(p) => vec![p.token_a, p.token_b].contains(&WETH_ADDRESS),
            })
            .collect();

        // nothing to sandwich
        if touched_pools.is_empty() {
            return Ok(vec![]);
        }

        // find trade direction
        let weth_state_diff = &state_diffs
            .get(&WETH_ADDRESS)
            .ok_or(anyhow!("Missing WETH state diffs"))?
            .storage;

        let mut sandwichable_pools = vec![];

        for pool in touched_pools {
            // find pool mapping location on WETH contract
            let storage_key = H256::from(ethers::utils::keccak256(abi::encode(&[
                abi::Token::Address(pool.address()),
                abi::Token::Uint(U256::from(3)), // WETH balanceOf mapping is at index 3
            ])));

            // in reality we also want to check stable coin pools
            if let Some(Diff::Changed(c)) = weth_state_diff.get(&storage_key) {
                let from = U256::from(c.from.to_fixed_bytes());
                let to = U256::from(c.to.to_fixed_bytes());

                // right now bot can only sandwich `weth->token` trades
                // enhancement: add support for `token->weth` trades (using longtail or flashswaps sandos)
                if to > from {
                    sandwichable_pools.push(pool);
                }
            }
        }

        Ok(sandwichable_pools)
    }

    pub fn new(provider: Arc<M>) -> Self {
        let dexes_data = [
            (
                // Uniswap v2
                "0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f",
                DexVariant::UniswapV2,
                10000835u64,
            ),
            (
                // Sushiswap
                "0xC0AEe478e3658e2610c5F7A4A2E1777cE9e4f2Ac",
                DexVariant::UniswapV2,
                10794229u64,
            ),
            (
                // Crypto.com swap
                "0x9DEB29c9a4c7A88a3C0257393b7f3335338D9A9D",
                DexVariant::UniswapV2,
                10828414u64,
            ),
            (
                // Convergence swap
                "0x4eef5746ED22A2fD368629C1852365bf5dcb79f1",
                DexVariant::UniswapV2,
                12385067u64,
            ),
            (
                // Pancakeswap
                "0x1097053Fd2ea711dad45caCcc45EfF7548fCB362",
                DexVariant::UniswapV2,
                15614590u64,
            ),
            (
                // ShibaSwap
                "0x115934131916C8b277DD010Ee02de363c09d037c",
                DexVariant::UniswapV2,
                12771526u64,
            ),
            (
                // Saitaswap
                "0x35113a300ca0D7621374890ABFEAC30E88f214b1",
                DexVariant::UniswapV2,
                15210780u64,
            ),
            (
                // Uniswap v3
                "0x1F98431c8aD98523631AE4a59f267346ea31F984",
                DexVariant::UniswapV3,
                12369621u64,
            ),
        ];

        let dexes = dexes_data
            .into_iter()
            .map(|(address, variant, number)| {
                Dex::new(H160::from_str(address).unwrap(), variant, number, Some(300))
            })
            .collect();

        Self {
            pools: DashMap::new(),
            provider,
            dexes,
        }
    }
}
