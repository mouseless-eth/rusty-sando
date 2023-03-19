use std::sync::Arc;

use crate::utils;
use ethers::prelude::*;

#[derive(Debug, Clone, Default)]
pub struct BlockInfo {
    pub number: U64,
    pub timestamp: U256,
    pub base_fee: U256,
}

impl BlockInfo {
    // Create a new `BlockInfo` instance
    pub fn new(number: U64, timestamp: U256, base_fee: U256) -> Self {
        Self {
            number,
            timestamp,
            base_fee,
        }
    }

    // Find the next block ahead of `prev_block`
    pub fn find_next_block_info(prev_block: Block<TxHash>) -> Self {
        let number = prev_block.number.unwrap_or_default() + 1;
        let timestamp = prev_block.timestamp + 12;
        let base_fee = utils::calculate_next_block_base_fee(prev_block);

        Self {
            number,
            timestamp,
            base_fee,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct BlockOracle {
    pub latest_block: BlockInfo,
    pub next_block: BlockInfo,
}

impl BlockOracle {
    // Create new latest block oracle
    pub async fn new(client: &Arc<Provider<Ws>>) -> Result<Self, ProviderError> {
        let latest_block = match client.get_block(BlockNumber::Latest).await {
            Ok(b) => b,
            Err(e) => return Err(e),
        };

        let lb = if let Some(b) = latest_block {
            b
        } else {
            return Err(ProviderError::CustomError("Block not found".to_string()));
        };

        // latets block info
        let number = lb.number.unwrap();
        let timestamp = lb.timestamp;
        let base_fee = lb.base_fee_per_gas.unwrap_or_default();

        let latest_block = BlockInfo::new(number, timestamp, base_fee);

        // next block info
        let number = number + 1;
        let timestamp = timestamp + 12;
        let base_fee = utils::calculate_next_block_base_fee(lb);

        let next_block = BlockInfo::new(number, timestamp, base_fee);

        Ok(BlockOracle {
            latest_block,
            next_block,
        })
    }

    // Updates block's number
    pub fn update_block_number(&mut self, block_number: U64) {
        self.latest_block.number = block_number;
        self.next_block.number = block_number + 1;
    }

    // Updates block's timestamp
    pub fn update_block_timestamp(&mut self, timestamp: U256) {
        self.latest_block.timestamp = timestamp;
        self.next_block.timestamp = timestamp + 12;
    }

    // Updates block's base fee
    pub fn update_base_fee(&mut self, latest_block: Block<TxHash>) {
        self.latest_block.base_fee = latest_block.base_fee_per_gas.unwrap_or_default();
        self.next_block.base_fee = utils::calculate_next_block_base_fee(latest_block);
    }
}
