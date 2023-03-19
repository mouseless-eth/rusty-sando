use std::{sync::Arc, time::Duration};

use ethers::{
    prelude::*,
    utils::{Anvil, AnvilInstance},
};

use crate::{
    prelude::{Erc20, Pool, PoolVariant, UniswapV3Pool},
    types::BlockInfo,
};

use super::{constants::get_weth_address, dotenv::get_sandwich_contract_address};

pub async fn get_next_block_info(prev_block_number: u64, client: &Arc<Provider<Ws>>) -> BlockInfo {
    let prev_block = client.get_block(prev_block_number).await.unwrap().unwrap();
    BlockInfo::find_next_block_info(prev_block)
}

// need to return anvil instance to keep it alive (so that we can make calls)
pub async fn create_fork_ws(fork_block_num: u64) -> (Arc<Provider<Ws>>, AnvilInstance) {
    let port_num: u16 = rand::Rng::gen_range(&mut rand::thread_rng(), 3000..4000);

    let _anvil = Anvil::new()
        .fork(format!(
            "https://eth-mainnet.g.alchemy.com/v2/nijvYQzHc4Fej8kvRvdesJXT5CqZEXSo@{}",
            fork_block_num
        ))
        .port(port_num)
        .spawn();
    let ws_fork = Ws::connect(format!("ws://127.0.0.1:{}", port_num))
        .await
        .unwrap();
    let ws_provider_fork = Provider::new(ws_fork).interval(Duration::from_millis(100));
    (Arc::new(ws_provider_fork), _anvil)
}

pub async fn create_ws() -> Arc<Provider<Ws>> {
    let ws = Ws::connect("ws://localhost:8545").await.unwrap();
    let ws_provider = Provider::new(ws).interval(Duration::from_millis(100));
    Arc::new(ws_provider)
}

pub async fn create_v2_pool(pool_address: Address, client: &Arc<Provider<Ws>>) -> Pool {
    let pool_contract = super::contracts::get_pair_v2_contract(&pool_address, client);
    let token_0 = pool_contract.token_0().call().await.unwrap();
    let token_1 = pool_contract.token_1().call().await.unwrap();

    Pool::new(
        pool_address,
        token_0,
        token_1,
        U256::from(3000),
        PoolVariant::UniswapV2,
    )
}

pub async fn create_v3_pool(pool_address: Address, client: &Arc<Provider<Ws>>) -> Pool {
    let pool = UniswapV3Pool::new(pool_address, client.into());

    let token_0 = pool.token_0().call().await.unwrap();
    let token_1 = pool.token_1().call().await.unwrap();
    let fee = pool.fee().call().await.unwrap();

    Pool::new(
        pool_address,
        token_0,
        token_1,
        U256::from(fee),
        PoolVariant::UniswapV3,
    )
}

/// Override an address's weth balance
pub fn mutate_weth_balance(
    state: &mut call_raw::spoof::State,
    address_to_mutate: Address,
    mutate_amount: U256,
) {
    let key = super::u256_to_h256_be(
        U256::from_str_radix(
            "ffa74f65cef4257058238f071cd4f631a58040052dd622d00aa7a451153252f4",
            16,
        )
        .unwrap(),
    );
    // Spoofing WETH balance
    // cast index address [address] 3 : WETH (give our sandwich contract 100weth)
    let val = super::u256_to_h256_be(U256::from("100000000000000000000"));
    // Give our acc a fuckton of WETH
    state
        .account(super::constants::get_weth_address())
        .store(key, val);
    state.account(address_to_mutate).balance(mutate_amount);
}

pub async fn get_weth_balance_at_block(block: u64) -> U256 {
    let (fork, _instance) = create_fork_ws(block).await;
    let weth_contract = Erc20::new(get_weth_address(), fork);
    let owner = get_sandwich_contract_address();
    weth_contract.balance_of(owner).call().await.unwrap()
}

#[macro_export]
macro_rules! time_function {
    ($x:expr) => {{
        let start = std::time::Instant::now();
        let result = $x;
        let elapsed = start.elapsed();
        println!("Elapsed time: {:?}", elapsed);
        result
    }};
}
