use dotenv::dotenv;
use reqwest::Url;
use std::{env, str::FromStr};

use anyhow::{anyhow, Result};
use ethers::{
    signers::LocalWallet,
    types::{Address, U64},
};

pub struct Config {
    pub searcher_signer: LocalWallet,
    pub sando_inception_block: U64,
    pub sando_address: Address,
    pub bundle_signer: LocalWallet,
    pub wss_rpc: Url,
    pub discord_webhook: String,
}

impl Config {
    pub async fn read_from_dotenv() -> Result<Self> {
        dotenv().ok();

        let get_env = |var| {
            env::var(var).map_err(|_| anyhow!("Required environment variable \"{}\" not set", var))
        };

        let searcher_signer = get_env("SEARCHER_PRIVATE_KEY")?
            .parse::<LocalWallet>()
            .map_err(|_| anyhow!("Failed to parse \"SEARCHER_PRIVATE_KEY\""))?;

        let sando_inception_block = get_env("SANDWICH_INCEPTION_BLOCK")?
            .parse::<u64>()
            .map(U64::from)
            .map_err(|_| anyhow!("Failed to parse \"SANDWICH_INCEPTION_BLOCK\" into u64"))?;

        let sando_address = Address::from_str(&get_env("SANDWICH_CONTRACT")?)
            .map_err(|_| anyhow!("Failed to parse \"SANDWICH_CONTRACT\""))?;

        let bundle_signer = get_env("FLASHBOTS_AUTH_KEY")?
            .parse::<LocalWallet>()
            .map_err(|_| anyhow!("Failed to parse \"FLASHBOTS_AUTH_KEY\""))?;

        let wss_rpc = get_env("WSS_RPC")?
            .parse()
            .map_err(|_| anyhow!("Failed to parse \"WSS_RPC\""))?;

        let discord_webhook = get_env("DISCORD_WEBHOOK")?;

        Ok(Self {
            searcher_signer,
            sando_inception_block,
            sando_address,
            bundle_signer,
            wss_rpc,
            discord_webhook,
        })
    }
}
