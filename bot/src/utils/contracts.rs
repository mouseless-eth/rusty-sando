use std::sync::Arc;

use ethers::{prelude::*, providers};

use crate::prelude::{Erc20, UniswapV2Pair};

/// Create erc20 contract that we can interact with
pub fn get_erc20_contract(
    erc20_address: &Address,
    client: &Arc<Provider<Ws>>,
) -> Erc20<providers::Provider<Ws>> {
    Erc20::new(*erc20_address, client.clone())
}

/// Create v2 pair contract that we can interact with
pub fn get_pair_v2_contract(
    pair_address: &Address,
    client: &Arc<Provider<Ws>>,
) -> UniswapV2Pair<providers::Provider<Ws>> {
    UniswapV2Pair::new(*pair_address, client.clone())
}
