use std::sync::Arc;

use anyhow::Result;
use artemis_core::{
    collectors::{block_collector::BlockCollector, mempool_collector::MempoolCollector},
    engine::Engine,
    executors::flashbots_executor::FlashbotsExecutor,
    types::{CollectorMap, ExecutorMap},
};
use ethers::providers::{Provider, Ws};
use log::info;
use reqwest::Url;
use rusty_sando::{
    config::Config,
    initialization::{print_banner, setup_logger},
};
use strategy::{
    bot::SandoBot,
    types::{Action, Event, StratConfig},
};

#[tokio::main]
async fn main() -> Result<()> {
    // Setup
    setup_logger()?;
    print_banner();
    let config = Config::read_from_dotenv().await?;

    // Setup ethers provider
    let ws = Ws::connect(config.wss_rpc).await?;
    let provider = Arc::new(Provider::new(ws));

    // Setup signers
    let flashbots_signer = config.bundle_signer;
    let searcher_signer = config.searcher_signer;

    // Create engine
    let mut engine: Engine<Event, Action> = Engine::default();

    // Setup block collector
    let block_collector = Box::new(BlockCollector::new(provider.clone()));
    let block_collector = CollectorMap::new(block_collector, Event::NewBlock);
    engine.add_collector(Box::new(block_collector));

    // Setup mempool collector
    let mempool_collector = Box::new(MempoolCollector::new(provider.clone()));
    let mempool_collector = CollectorMap::new(mempool_collector, Event::NewTransaction);
    engine.add_collector(Box::new(mempool_collector));

    // Setup strategy
    let configs = StratConfig {
        sando_address: config.sando_address,
        sando_inception_block: config.sando_inception_block,
        searcher_signer,
    };
    let strategy = SandoBot::new(provider.clone(), configs);
    engine.add_strategy(Box::new(strategy));

    // Setup flashbots executor
    let executor = Box::new(FlashbotsExecutor::new(
        provider.clone(),
        flashbots_signer,
        Url::parse("https://relay.flashbots.net")?,
    ));
    let executor = ExecutorMap::new(executor, |action| match action {
        Action::SubmitToFlashbots(bundle) => Some(bundle),
    });
    engine.add_executor(Box::new(executor));

    // Start engine
    if let Ok(mut set) = engine.run().await {
        while let Some(res) = set.join_next().await {
            info!("res: {:?}", res)
        }
    }

    Ok(())
}
