use std::{str::FromStr, sync::Arc};

use cfmms::pool::{Pool, UniswapV2Pool};
use ethers::{
    prelude::Lazy,
    providers::{Middleware, Provider, Ws},
    types::{Address, Transaction, TxHash, U64},
};
use strategy::{
    bot::SandoBot,
    types::{BlockInfo, RawIngredients, StratConfig},
};

// -- consts --
static WSS_RPC: &str = "ws://localhost:8545";
pub static WETH_ADDRESS: Lazy<Address> = Lazy::new(|| {
    "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
        .parse()
        .unwrap()
});

// -- utils --
async fn setup_bot(provider: Arc<Provider<Ws>>) -> SandoBot<Provider<Ws>> {
    let strat_config = StratConfig {
        sando_address: "0xaAaAaAaaAaAaAaaAaAAAAAAAAaaaAaAaAaaAaaAa"
            .parse()
            .unwrap(),
        sando_inception_block: U64::from(17700000),
        // wallet from privatekey 0x1
        searcher_signer: "0x7E5F4552091A69125d5DfCb7b8C2659029395Bdf"
            .parse()
            .unwrap(),
    };
    SandoBot::new(provider, strat_config)
}

async fn block_num_to_info(block_num: u64, provider: Arc<Provider<Ws>>) -> BlockInfo {
    let block = provider.get_block(block_num).await.unwrap().unwrap();
    block.try_into().unwrap()
}

fn hex_to_address(hex: &str) -> Address {
    hex.parse().unwrap()
}

async fn hex_to_univ2_pool(hex: &str, provider: Arc<Provider<Ws>>) -> Pool {
    let pair_address = hex_to_address(hex);
    let pool = UniswapV2Pool::new_from_address(pair_address, provider)
        .await
        .unwrap();
    Pool::UniswapV2(pool)
}

async fn victim_tx_hash(tx: &str, provider: Arc<Provider<Ws>>) -> Transaction {
    let tx_hash: TxHash = TxHash::from_str(tx).unwrap();
    provider.get_transaction(tx_hash).await.unwrap().unwrap()
}

/// testing against: https://eigenphi.io/mev/ethereum/tx/0x292156c07794bc50952673bf948b90ab71148b81938b6ab4904096adb654d99a
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn can_sandwich_uni_v2() {
    let client = Arc::new(Provider::new(Ws::connect(WSS_RPC).await.unwrap()));

    let bot = setup_bot(client.clone()).await;

    let ingredients = RawIngredients::new(
        vec![
            victim_tx_hash(
                "0xfecf2c78d1418e6905c18a6a6301c9d39b14e5320e345adce52baaecf805580d",
                client.clone(),
            )
            .await,
        ],
        *WETH_ADDRESS,
        hex_to_address("0x3642Cf76c5894B4aB51c1080B2c4F5B9eA734106"),
        hex_to_univ2_pool("0x5d1dd0661E1D22697943C1F50Cc726eA3143329b", client.clone()).await,
    );

    let target_block = block_num_to_info(17754167, client.clone()).await;

    let _ = bot
        .is_sandwichable(ingredients, target_block)
        .await
        .unwrap();
}
