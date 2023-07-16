use artemis_core::{
    collectors::block_collector::NewBlock, executors::flashbots_executor::FlashbotsBundle,
};
use cfmms::pool::Pool;
use ethers::types::{Address, Transaction, U64};

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
    pub searcher_signer: Address,
}

/// Information on potential sandwichable opportunity
pub struct RawIngredients {
    /// Victim tx/s to be used in sandwich
    meats: Vec<Transaction>,
    /// Which token do start and end sandwich with
    start_end_token: Address,
    /// Which token do we hold for duration of sandwich
    intermediary_token: Address,
    /// Which pool are we targetting
    target_pool: Pool,
}

impl RawIngredients {
    pub fn new(
        meats: Vec<Transaction>,
        start_end_token: Address,
        intermediary_token: Address,
        target_pool: Pool,
    ) -> Self {
        Self {
            meats,
            start_end_token,
            intermediary_token,
            target_pool,
        }
    }
}
