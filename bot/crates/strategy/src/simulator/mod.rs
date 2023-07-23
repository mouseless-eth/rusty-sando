pub mod huff_sando;
pub(crate) mod lil_router;
pub(crate) mod salmonella_inspector;

use foundry_evm::{
    executor::fork::SharedBackend,
    revm::{db::CacheDB, primitives::U256 as rU256, EVM},
};

use crate::{
    constants::{COINBASE, ONE_ETHER_IN_WEI},
    types::BlockInfo,
};

fn setup_block_state(evm: &mut EVM<CacheDB<SharedBackend>>, next_block: &BlockInfo) {
    evm.env.block.number = rU256::from(next_block.number.as_u64());
    evm.env.block.timestamp = next_block.timestamp.into();
    evm.env.block.basefee = next_block.base_fee_per_gas.into();
    // use something other than default
    evm.env.block.coinbase = *COINBASE;
}

pub fn eth_to_wei(amt: u128) -> rU256 {
    rU256::from(amt).checked_mul(*ONE_ETHER_IN_WEI).unwrap()
}
