use ethers::prelude::*;
use std::str::FromStr;

// Construct the searcher wallet
pub fn get_searcher_wallet() -> LocalWallet {
    let searcher_private_key = std::env::var("SEARCHER_PRIVATE_KEY")
        .expect("Required environment variable \"SEARCHER_PRIVATE_KEY\" not set");
    searcher_private_key
        .parse::<LocalWallet>()
        .expect("Failed to parse private key")
}

// Get block number that sandwich contract was deployed in
pub fn get_sandwich_inception_block() -> U64 {
    let inception_block = std::env::var("SANDWICH_INCEPTION_BLOCK")
        .expect("Required environment variable \"SANDWICH_INCEPTION_BLOCK\" not set");

    let inception_block = inception_block
        .parse::<u64>()
        .expect("Failed to parse \"SANDWICH_INCEPTION_BLOCK\" into u64");

    U64::from(inception_block)
}

/// Construct the bundle signer
/// This is your flashbots searcher identity
pub fn get_bundle_signer() -> LocalWallet {
    let private_key = std::env::var("FLASHBOTS_AUTH_KEY")
        .expect("Required environment variable \"FLASHBOTS_AUTH_KEY\" not set");
    private_key
        .parse::<LocalWallet>()
        .expect("Failed to parse flashbots signer")
}

/// Returns the configured Sandwich Contract Address
pub fn get_sandwich_contract_address() -> Address {
    let addr = std::env::var("SANDWICH_CONTRACT")
        .expect("Required environment variable \"SANDWICH_CONTRACT\" not set");
    Address::from_str(&addr).expect("Failed to parse \"SANDWICH_CONTRACT\"")
}

/// Read environment variables
pub fn read_env_vars() -> Vec<(String, String)> {
    let mut env_vars = Vec::new();
    let keys = vec![
        "RPC_URL_WSS",
        "SEARCHER_PRIVATE_KEY",
        "FLASHBOTS_AUTH_KEY",
        "SANDWICH_CONTRACT",
        "V2_ALERT_DISCORD_WEBHOOK",
        "V3_ALERT_DISCORD_WEBHOOK",
        "POISON_ALERT_DISCORD_WEBHOOK",
        "SANDWICH_INCEPTION_BLOCK",
    ];
    for key in keys {
        let value = dotenv::var(key).expect(&format!(
            "Required environment variable \"{}\" not set",
            key
        ));
        env_vars.push((key.to_string(), value));
    }
    env_vars
}

/// Return a new ws provider
pub async fn get_ws_provider() -> Provider<Ws> {
    let url =
        dotenv::var("RPC_URL_WSS").expect("Required environment variable \"RPC_URL_WSS\" not set");
    Provider::<Ws>::connect(&url)
        .await
        .expect("RPC Connection Error")
}

/// Return a webhook for v2 discord alert channel
pub fn get_v2_alert_webhook() -> String {
    dotenv::var("V2_ALERT_DISCORD_WEBHOOK")
        .expect("Required environment variable \"V2_ALERT_DISCORD_WEBHOOK\" not set")
}

/// Return a webhook for v3 discord alert channel
pub fn get_v3_alert_webhook() -> String {
    dotenv::var("V3_ALERT_DISCORD_WEBHOOK")
        .expect("Required environment variable \"V3_ALERT_DISCORD_WEBHOOK\" not set")
}

/// Return a webhook for poison discord alert channel
pub fn poison_alert_webhook() -> String {
    dotenv::var("POISON_ALERT_DISCORD_WEBHOOK")
        .expect("Required environment variable \"POISON_ALERT_DISCORD_WEBHOOK\" not set")
}
/// Return a interval block for update new pools info
pub fn get_interval_block_new_pool() -> u64 {
    dotenv::var("INTERVAL_BLOCK_NEW_POOL")
        .expect("Required environment variable \"INTERVAL_BLOCK_NEW_POOL\" not set")
        .parse()
        .expect("INTERVAL_BLOCK_NEW_POOL is not a valid u64")
}