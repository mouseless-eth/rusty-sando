pub mod huff_sando;
pub mod lil_router;
pub(crate) mod sandwich_finder;

use foundry_evm::{
    executor::fork::SharedBackend,
    revm::{db::CacheDB, primitives::U256 as rU256, EVM},
};

use crate::{constants::COINBASE, managers::block_manager::BlockInfo};

fn setup_block_state(evm: &mut EVM<CacheDB<SharedBackend>>, next_block: &BlockInfo) {
    evm.env.block.number = rU256::from(next_block.number.as_u64());
    evm.env.block.timestamp = next_block.timestamp.into();
    evm.env.block.basefee = next_block.base_fee_per_gas.into();
    // use something other than default
    evm.env.block.coinbase = *COINBASE;
}
