use std::sync::Arc;

use crate::utils;
use ethers::prelude::*;
use ethers_flashbots::*;
use reqwest::Url;

pub struct BundleRelay {
    pub flashbots_client:
        SignerMiddleware<FlashbotsMiddleware<Arc<Provider<Ws>>, LocalWallet>, LocalWallet>,
    pub relay_name: String,
}

impl BundleRelay {
    pub fn new(
        relay_end_point: Url,
        relay_name: String,
        client: &Arc<Provider<Ws>>,
    ) -> Result<BundleRelay, url::ParseError> {
        // Extract wallets from .env keys
        let bundle_private_key = std::env::var("FLASHBOTS_AUTH_KEY").unwrap();
        let searcher_private_key = std::env::var("PRIVATE_KEY").unwrap();

        let bundle_signer = bundle_private_key.parse::<LocalWallet>().unwrap();
        let searcher_signer = searcher_private_key.parse::<LocalWallet>().unwrap();

        // Setup the Ethereum client with flashbots middleware
        let flashbots_middleware =
            FlashbotsMiddleware::new(client.clone(), relay_end_point, bundle_signer);

        // Local node running mev-geth
        //flashbots_middleware.set_simulation_relay(Url::parse("http://127.0.0.1:8546").unwrap());
        let flashbots_client = SignerMiddleware::new(flashbots_middleware, searcher_signer);

        Ok(BundleRelay {
            flashbots_client,
            relay_name,
        })
    }
}

pub fn construct_bundle(
    signed_txs: Vec<Bytes>,
    target_block: U64, // Current block number
    target_timestamp: u64,
) -> BundleRequest {
    // Create ethers-flashbots bundle request
    let mut bundle_request = BundleRequest::new();

    for tx in signed_txs {
        bundle_request = bundle_request.push_transaction(tx);
    }

    // Set other bundle parameters
    bundle_request = bundle_request
        .set_block(target_block)
        .set_simulation_block(target_block - 1)
        .set_simulation_timestamp(target_timestamp)
        .set_min_timestamp(target_timestamp)
        .set_max_timestamp(target_timestamp);

    bundle_request
}

pub async fn get_all_relay_endpoints() -> Vec<BundleRelay> {
    let client = utils::create_websocket_client().await.unwrap();

    let endpoints = vec![
        ("flashbots", "https://relay.flashbots.net/"),
        ("builder0x69", "http://builder0x69.io/"),
        ("edennetwork", "https://api.edennetwork.io/v1/bundle"),
        ("beaverbuild", "https://rpc.beaverbuild.org/"),
        ("lightspeedbuilder", "https://rpc.lightspeedbuilder.info/"),
        ("eth-builder", "https://eth-builder.com/"),
        ("ultrasound", "https://relay.ultrasound.money/"),
        ("agnostic-relay", "https://agnostic-relay.net/"),
        ("relayoor-wtf", "https://relayooor.wtf/"),
        ("rsync-builder", "https://rsync-builder.xyz/"),
        //"http://relayooor.wtf/",
        //"http://mainnet.aestus.live/",
        //"https://mainnet-relay.securerpc.com",
        //"http://agnostic-relay.net/",
        //"http://relay.ultrasound.money/",
    ];

    let mut relays: Vec<BundleRelay> = vec![];

    for (name, endpoint) in endpoints {
        let relay = BundleRelay::new(Url::parse(endpoint).unwrap(), name.into(), &client).unwrap();
        relays.push(relay);
    }

    relays
}

#[cfg(test)]
mod test {
    use super::*;
    use std::{str::FromStr, sync::Arc, time::Duration};

    use ethers::prelude::rand::thread_rng;
    use ethers_flashbots::FlashbotsMiddleware;
    use reqwest::Url;

    #[tokio::test]
    async fn test_gas_sim() {
        // Connect to the network
        let provider = Provider::<Http>::try_from("https://relay.flashbots.net/").unwrap();

        // This is your searcher identity
        let bundle_signer = LocalWallet::new(&mut thread_rng());
        // This signs transactions
        let wallet = LocalWallet::new(&mut thread_rng());

        // Add signer and Flashbots middleware
        let fb_client = SignerMiddleware::new(
            FlashbotsMiddleware::new(
                provider,
                Url::parse("https://relay.flashbots.net").unwrap(),
                bundle_signer,
            ),
            wallet,
        );

        let ws = Ws::connect("ws://localhost:8545").await.unwrap();
        let ws_provider = Provider::new(ws).interval(Duration::from_millis(100));
        let ws_provider = Arc::new(ws_provider);

        let frontrun_hash =
            TxHash::from_str("0x5c6c3212295421b026c437687ee482ed589ade60567b2dffb29c451d30aa3942")
                .unwrap();
        let frontrun_tx = ws_provider
            .get_transaction(frontrun_hash)
            .await
            .unwrap()
            .unwrap();

        let victim_hash =
            TxHash::from_str("0x8e46e9ec1b826252e85560f20e3bff1ceba9fda1c07766821d2ac32cc01c1c73")
                .unwrap();
        let victim_tx = ws_provider
            .get_transaction(victim_hash)
            .await
            .unwrap()
            .unwrap();

        let backrun_hash =
            TxHash::from_str("0x5df507650549411d6a6741d38b3f0248f17bd4835562e4a5a4d620d2192b2e9f")
                .unwrap();
        let backrun_tx = ws_provider
            .get_transaction(backrun_hash)
            .await
            .unwrap()
            .unwrap();

        let target_block = ws_provider.get_block(16833583).await.unwrap().unwrap();
        let sim_block = ws_provider.get_block(16833582).await.unwrap().unwrap();

        let bundle = super::construct_bundle(
            vec![frontrun_tx.rlp(), victim_tx.rlp(), backrun_tx.rlp()],
            target_block.number.unwrap(),
            sim_block.timestamp.as_u64(),
        );

        let simulation_result = fb_client.inner().simulate_bundle(&bundle).await;

        println!("simulation_result: {:#?}", simulation_result);
    }
}
