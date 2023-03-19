use std::str::FromStr;

use colored::Colorize;
use dotenv::dotenv;
use ethers::prelude::*;
use eyre::Result;

use fern::colors::{Color, ColoredLevelConfig};

use rusty_sandwich::{
    prelude::{sync_dex, Dex, PoolVariant},
    runner::Bot,
    utils::{self, dotenv::read_env_vars},
};

#[tokio::main]
async fn main() -> Result<()> {
    log::info!("Starting Bot Initialization");
    dotenv().ok();

    // setup logger configs
    let mut colors = ColoredLevelConfig::new();
    colors.trace = Color::Cyan;
    colors.debug = Color::Magenta;
    colors.info = Color::Green;
    colors.warn = Color::Red;
    colors.error = Color::BrightRed;

    // setup logging both to stdout and file
    fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "{}[{}] {}",
                chrono::Local::now().format("[%H:%M:%S]"),
                colors.color(record.level()),
                message
            ))
        })
        .chain(std::io::stdout())
        .chain(fern::log_file("output.log")?)
        // hide all logs for everything other than bot
        .level(log::LevelFilter::Error)
        .level_for("rusty_sandwich", log::LevelFilter::Info)
        .apply()?;

    read_env_vars();

    log::info!(
        "{}",
        format!("{}", utils::constants::get_banner().green().bold())
    );

    // Create the websocket client
    let client = utils::create_websocket_client().await.unwrap();

    ///////////////////////////////////////
    //  Setup all dexes and their pools  //
    ///////////////////////////////////////
    let mut dexes = vec![];

    // Add UniswapV2 pairs
    dexes.push(Dex::new(
        H160::from_str("0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f").unwrap(),
        PoolVariant::UniswapV2,
        10000835,
    ));

    //// Add Sushiswap pairs
    dexes.push(Dex::new(
        H160::from_str("0xC0AEe478e3658e2610c5F7A4A2E1777cE9e4f2Ac").unwrap(),
        PoolVariant::UniswapV2,
        10794229,
    ));

    //// Add CryptoCom-Swap pairs
    dexes.push(Dex::new(
        H160::from_str("0x9DEB29c9a4c7A88a3C0257393b7f3335338D9A9D").unwrap(),
        PoolVariant::UniswapV2,
        10828414,
    ));

    //// Add Convergence-Swap pairs
    dexes.push(Dex::new(
        H160::from_str("0x4eef5746ED22A2fD368629C1852365bf5dcb79f1").unwrap(),
        PoolVariant::UniswapV2,
        12385067,
    ));

    //// Add Pancake-Swap pairs
    dexes.push(Dex::new(
        H160::from_str("0x1097053Fd2ea711dad45caCcc45EfF7548fCB362").unwrap(),
        PoolVariant::UniswapV2,
        15614590,
    ));

    //// Add Shiba-Swap pairs, home of shitcoins
    dexes.push(Dex::new(
        H160::from_str("0x115934131916C8b277DD010Ee02de363c09d037c").unwrap(),
        PoolVariant::UniswapV2,
        12771526,
    ));

    //// Add Saitaswap pools
    dexes.push(Dex::new(
        H160::from_str("0x35113a300ca0D7621374890ABFEAC30E88f214b1").unwrap(),
        PoolVariant::UniswapV2,
        15210780,
    ));

    //// Add UniswapV3 pools
    dexes.push(Dex::new(
        H160::from_str("0x1F98431c8aD98523631AE4a59f267346ea31F984").unwrap(),
        PoolVariant::UniswapV3,
        12369621,
    ));

    let current_block = client.get_block_number().await.unwrap();
    let all_pools = sync_dex(dexes.clone(), &client, current_block, None)
        .await
        .unwrap();

    log::info!("all_pools_len: {}", all_pools.len());

    // Execution loop (reconnect bot if it dies)
    loop {
        let client = utils::create_websocket_client().await.unwrap();
        let mut bot = Bot::new(client, all_pools.clone(), dexes.clone())
            .await
            .unwrap();

        bot.run().await.unwrap();
        log::error!("Websocket disconnected");
    }
}

#[cfg(test)]
mod test {
    use ethers::providers::Middleware;
    use futures::StreamExt;
    use rusty_sandwich::utils::testhelper;

    #[tokio::test]
    async fn sub_blocks() {
        let client = testhelper::create_ws().await;
        // let client = Provider::<Ws>::connect("ws://localhost:8545").await.unwrap();

        let mut stream = client.subscribe_blocks().await.unwrap();
        let mut prev = 0;
        while let Some(block) = stream.next().await {
            println!("{:#?}", block.timestamp.as_u32() - prev);
            prev = block.timestamp.as_u32();
        }
    }
}
