use anyhow::{anyhow, Result};
use artemis_core::collectors::block_collector::NewBlock;
use colored::Colorize;
use ethers::{
    providers::Middleware,
    types::{Block, BlockNumber, H256, U256, U64},
};
use log::info;
use std::sync::Arc;

use crate::startup_info_log;

pub struct BlockManager {
    latest_block: BlockInfo,
    next_block: BlockInfo,
}

impl BlockManager {
    pub fn new() -> Self {
        Self {
            latest_block: BlockInfo::default(),
            next_block: BlockInfo::default(),
        }
    }

    pub async fn setup<M: Middleware + 'static>(&mut self, provider: Arc<M>) -> Result<()> {
        let latest_block = provider
            .get_block(BlockNumber::Latest)
            .await
            .map_err(|_| anyhow!("Failed to get current block"))?
            .ok_or(anyhow!("Failed to get current block"))?;

        let latest_block: BlockInfo = latest_block.try_into()?;
        self.update_block_info(latest_block);

        startup_info_log!("latest block synced: {}", latest_block.number);
        Ok(())
    }

    /// Return info for the next block
    pub fn get_next_block(&self) -> BlockInfo {
        self.next_block
    }

    /// Return info for the next block
    pub fn get_latest_block(&self) -> BlockInfo {
        self.latest_block
    }

    /// Updates internal state with the latest mined block and next block
    pub fn update_block_info<T: Into<BlockInfo>>(&mut self, latest_block: T) {
        let latest_block: BlockInfo = latest_block.into();
        self.latest_block = latest_block;
        self.next_block = latest_block.get_next_block();
    }
}

#[derive(Default, Clone, Copy)]
pub struct BlockInfo {
    pub number: U64,
    pub base_fee_per_gas: U256,
    pub timestamp: U256,
    // These are optional because we don't know these values for `next_block`
    pub gas_used: Option<U256>,
    pub gas_limit: Option<U256>,
}

impl BlockInfo {
    /// Returns block info for next block
    pub fn get_next_block(&self) -> BlockInfo {
        BlockInfo {
            number: self.number + 1,
            base_fee_per_gas: calculate_next_block_base_fee(&self),
            timestamp: self.timestamp + 12,
            gas_used: None,
            gas_limit: None,
        }
    }
}

impl TryFrom<Block<H256>> for BlockInfo {
    type Error = anyhow::Error;

    fn try_from(value: Block<H256>) -> std::result::Result<Self, Self::Error> {
        Ok(BlockInfo {
            number: value.number.ok_or(anyhow!(
                "could not parse block.number when setting up `block_manager`"
            ))?,
            gas_used: Some(value.gas_used),
            gas_limit: Some(value.gas_limit),
            base_fee_per_gas: value.base_fee_per_gas.ok_or(anyhow!(
                "could not parse base fee when setting up `block_manager`"
            ))?,
            timestamp: value.timestamp,
        })
    }
}

impl From<NewBlock> for BlockInfo {
    fn from(value: NewBlock) -> Self {
        Self {
            number: value.number,
            base_fee_per_gas: value.base_fee_per_gas,
            timestamp: value.timestamp,
            gas_used: Some(value.gas_used),
            gas_limit: Some(value.gas_limit),
        }
    }
}

/// Calculate the next block base fee
// based on math provided here: https://ethereum.stackexchange.com/questions/107173/how-is-the-base-fee-per-gas-computed-for-a-new-block
fn calculate_next_block_base_fee(block: &BlockInfo) -> U256 {
    // Get the block base fee per gas
    let current_base_fee_per_gas = block.base_fee_per_gas;

    let current_gas_used = block
        .gas_used
        .expect("can't calculate base fee from unmined block \"next_block\"");

    let current_gas_target = block
        .gas_limit
        .expect("can't calculate base fee from unmined block \"next_block\"")
        / 2;

    if current_gas_used == current_gas_target {
        current_base_fee_per_gas
    } else if current_gas_used > current_gas_target {
        let gas_used_delta = current_gas_used - current_gas_target;
        let base_fee_per_gas_delta =
            current_base_fee_per_gas * gas_used_delta / current_gas_target / 8;

        return current_base_fee_per_gas + base_fee_per_gas_delta;
    } else {
        let gas_used_delta = current_gas_target - current_gas_used;
        let base_fee_per_gas_delta =
            current_base_fee_per_gas * gas_used_delta / current_gas_target / 8;

        return current_base_fee_per_gas - base_fee_per_gas_delta;
    }
}
