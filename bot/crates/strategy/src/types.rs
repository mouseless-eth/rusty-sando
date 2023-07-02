use anyhow::{anyhow, Result};
use artemis_core::{
    collectors::block_collector::NewBlock, executors::flashbots_executor::FlashbotsBundle,
};
use ethers::{
    providers::Middleware,
    types::{AccountDiff, Address, TraceType, Transaction, U64},
};
use std::{collections::BTreeMap, sync::Arc};

use crate::managers::block_manager::BlockInfo;

/// Holds information about a particular victim tx
pub struct VictimInfo {
    pub tx_args: Transaction,
    pub target_block: BlockInfo,
    pub state_diffs: Option<BTreeMap<Address, AccountDiff>>,
}

impl VictimInfo {
    /// Checks if victim tx can be included in target block
    pub fn can_include_in_target_block(&self) -> bool {
        self.tx_args.max_fee_per_gas.unwrap_or_default() < self.target_block.base_fee_per_gas
    }

    pub fn get_state_diffs(&self) -> &Option<BTreeMap<Address, AccountDiff>> {
        &self.state_diffs
    }

    /// Fetch state diffs for victim tx by doing a `trace_call`
    pub async fn fill_state_diffs<M: Middleware + 'static>(
        &mut self,
        provider: Arc<M>,
    ) -> Result<()> {
        let state_diffs = provider
            .trace_call(
                &self.tx_args,
                vec![TraceType::StateDiff],
                Some((self.target_block.number - 1).into()),
            )
            .await?
            .state_diff
            .ok_or(anyhow!("not sandwichable, no state diffs produced"))?
            .0;

        self.state_diffs = Some(state_diffs);

        Ok(())
    }

    pub fn new(tx: Transaction, target_block: BlockInfo) -> Self {
        Self {
            tx_args: tx,
            target_block,
            state_diffs: None,
        }
    }
}

/// Core Event enum for current strategy
#[derive(Debug, Clone)]
pub enum Event {
    NewBlock(NewBlock),
    NewTransaction(Transaction),
}

/// Core Action enum for current strategy
#[derive(Debug, Clone)]
pub enum Action {
    SubmitToFlashbots(FlashbotsBundle),
}

/// Configuration for variables needed for sandwiches
#[derive(Debug, Clone)]
pub struct StratConfig {
    pub sando_address: Address,
    pub sando_inception_block: U64,
}
