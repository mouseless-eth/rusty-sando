use anyhow::Result;
use artemis_core::{collectors::block_collector::NewBlock, types::Strategy};
use async_trait::async_trait;
use colored::Colorize;
use ethers::{providers::Middleware, types::Transaction};
use log::{error, info};
use std::sync::Arc;

use crate::{
    log_error, log_info_cyan, log_new_block_info, log_not_sandwichable, log_sandwichable,
    managers::{
        block_manager::BlockManager, pool_manager::PoolManager,
        sando_state_manager::SandoStateManager,
    },
    simulator::sandwich_finder::create_optimal_sandwich,
    types::{Action, Event, StratConfig},
};

pub struct SandoBot<M> {
    /// Ethers client
    provider: Arc<M>,
    /// Keeps track of onchain pools
    pool_manager: PoolManager<M>,
    /// Block manager
    block_manager: BlockManager,
    /// Keeps track of weth inventory & token dust
    sando_state_manager: SandoStateManager,
}

impl<M: Middleware + 'static> SandoBot<M> {
    /// Create a new instance
    pub fn new(client: Arc<M>, config: StratConfig) -> Self {
        Self {
            pool_manager: PoolManager::new(client.clone()),
            provider: client,
            block_manager: BlockManager::new(),
            sando_state_manager: SandoStateManager::new(
                config.sando_address,
                config.searcher_signer,
                config.sando_inception_block,
            ),
        }
    }
}

#[async_trait]
impl<M: Middleware + 'static> Strategy<Event, Action> for SandoBot<M> {
    /// Setup by getting all pools to monitor for swaps
    async fn sync_state(&mut self) -> Result<()> {
        self.pool_manager.setup().await?;
        self.sando_state_manager
            .setup(self.provider.clone())
            .await?;
        self.block_manager.setup(self.provider.clone()).await?;
        Ok(())
    }

    /// Process incoming events
    async fn process_event(&mut self, event: Event) -> Option<Action> {
        match event {
            Event::NewBlock(block) => match self.process_new_block(block).await {
                Ok(_) => None,
                Err(e) => {
                    panic!("strategy is out of sync {}", e);
                }
            },
            Event::NewTransaction(tx) => self.process_new_tx(tx).await,
        }
    }
}

impl<M: Middleware + 'static> SandoBot<M> {
    /// Process new blocks as they come in
    async fn process_new_block(&mut self, event: NewBlock) -> Result<()> {
        log_new_block_info!(event);
        self.block_manager.update_block_info(event);
        Ok(())
    }

    /// Process new txs as they come in
    async fn process_new_tx(&mut self, tx: Transaction) -> Option<Action> {
        // setup variables for processing tx
        let next_block = self.block_manager.get_next_block();
        let latest_block = self.block_manager.get_latest_block();

        // ignore txs that we can't include in next block
        // enhancement: simulate all txs regardless, store result, and use result when tx can included
        if tx.max_fee_per_gas.unwrap_or_default() < next_block.base_fee_per_gas {
            log_info_cyan!("{:?} mf<nbf", tx.hash);
            return None;
        }

        // check if tx is a swap
        let touched_pools = self
            .pool_manager
            .get_touched_sandwichable_pools(&tx, latest_block.number.into(), self.provider.clone())
            .await
            .map_err(|e| {
                log_error!("Failed to get touched sandwichable pools: {}", e);
                e
            })
            .ok()?;

        // no touched pools = no sandwich opps
        if touched_pools.is_empty() {
            info!("{:?}", tx.hash);
            return None;
        }

        let weth_inventory = self.sando_state_manager.get_weth_inventory();

        // list of sandwiches that this victim tx produces
        let mut recipes = vec![];

        for pool in touched_pools {
            let optimal_sandwich = match create_optimal_sandwich(
                vec![tx.clone()],
                pool,
                next_block.clone(),
                weth_inventory,
                self.sando_state_manager.get_searcher(),
                self.sando_state_manager.get_sando_address(),
                self.provider.clone(),
            )
            .await
            {
                Ok(s) => {
                    log_sandwichable!("{:?} {:?}", tx.hash, s);
                    recipes.push(s)
                }
                Err(e) => log_not_sandwichable!("{:?} {:?}", tx.hash, e),
            };
        }

        None
    }
}