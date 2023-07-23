use std::{str::FromStr, sync::Arc};

use cfmms::pool::{Pool, UniswapV2Pool, UniswapV3Pool};
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

fn setup_logger() {
    fern::Dispatch::new()
        .level(log::LevelFilter::Error)
        .level_for("strategy", log::LevelFilter::Info)
        .chain(std::io::stdout())
        .apply()
        .unwrap();
}

async fn setup_bot(provider: Arc<Provider<Ws>>) -> SandoBot<Provider<Ws>> {
    setup_logger();

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

async fn hex_to_univ3_pool(hex: &str, provider: Arc<Provider<Ws>>) -> Pool {
    let pair_address = hex_to_address(hex);
    let pool = UniswapV3Pool::new_from_address(pair_address, provider)
        .await
        .unwrap();
    Pool::UniswapV3(pool)
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

/// testing against: https://eigenphi.io/mev/ethereum/tx/0x056ede919e31be59b7e1e8676b3be1272ce2bbd3d18f42317a26a3d1f2951fc8
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn can_sandwich_sushi_swap() {
    let client = Arc::new(Provider::new(Ws::connect(WSS_RPC).await.unwrap()));

    let bot = setup_bot(client.clone()).await;

    let ingredients = RawIngredients::new(
        vec![
            victim_tx_hash(
                "0xb344fdc6a3b7c65c5dd971cb113567e2ee6d0636f261c3b8d624627b90694cdb",
                client.clone(),
            )
            .await,
        ],
        *WETH_ADDRESS,
        hex_to_address("0x3b484b82567a09e2588A13D54D032153f0c0aEe0"),
        hex_to_univ2_pool("0xB84C45174Bfc6b8F3EaeCBae11deE63114f5c1b2", client.clone()).await,
    );

    let target_block = block_num_to_info(16873148, client.clone()).await;

    let _ = bot
        .is_sandwichable(ingredients, target_block)
        .await
        .unwrap();
}

/// testing against: https://eigenphi.io/mev/ethereum/tx/0x64158690880d053adc2c42fbadd1838bc6d726cb81982443be00f83b51d8c25d
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn can_sandwich_uni_v3() {
    let client = Arc::new(Provider::new(Ws::connect(WSS_RPC).await.unwrap()));

    let bot = setup_bot(client.clone()).await;

    let ingredients = RawIngredients::new(
        vec![
            victim_tx_hash(
                "0x90dfe56814821e7f76f2e4970a7b35948670a968abffebb7be69fe528283e6d8",
                client.clone(),
            )
            .await,
        ],
        *WETH_ADDRESS,
        hex_to_address("0x24C19F7101c1731b85F1127EaA0407732E36EcDD"),
        hex_to_univ3_pool("0x62CBac19051b130746Ec4CF96113aF5618F3A212", client.clone()).await,
    );

    let target_block = block_num_to_info(16863225, client.clone()).await;

    let _ = bot
        .is_sandwichable(ingredients, target_block)
        .await
        .unwrap();
}
