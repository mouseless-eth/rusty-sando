use anyhow::{anyhow, Result};
use ethers::{providers::Middleware, types::BlockNumber};
use log::info;
use std::sync::Arc;

use colored::Colorize;

use crate::{startup_info_log, types::BlockInfo};

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
