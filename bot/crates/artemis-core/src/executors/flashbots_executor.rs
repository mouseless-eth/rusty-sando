use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use ethers::{providers::Middleware, signers::Signer};
use ethers_flashbots::{BundleRequest, FlashbotsMiddleware};
use reqwest::Url;
use tracing::{error, info};

use crate::types::Executor;

/// A Flashbots executor that sends transactions to the Flashbots relay.
pub struct FlashbotsExecutor<M, S> {
    /// The Flashbots middleware.
    fb_client: FlashbotsMiddleware<Arc<M>, S>,
}

/// A bundle of transactions to send to the Flashbots relay.
/// Sending vec of bundle request because multiple actions per event not supported
/// See issue: https://github.com/paradigmxyz/artemis/issues/34
pub type FlashbotsBundle = Vec<BundleRequest>;

impl<M: Middleware, S: Signer> FlashbotsExecutor<M, S> {
    pub fn new(client: Arc<M>, relay_signer: S, relay_url: impl Into<Url>) -> Self {
        let fb_client = FlashbotsMiddleware::new(client, relay_url, relay_signer);
        Self { fb_client }
    }
}

#[async_trait]
impl<M, S> Executor<FlashbotsBundle> for FlashbotsExecutor<M, S>
where
    M: Middleware + 'static,
    M::Error: 'static,
    S: Signer + 'static,
{
    /// Send a bundle to transactions to the Flashbots relay.
    async fn execute(&self, action: FlashbotsBundle) -> Result<()> {
        for bundle in action {
            //// Simulate bundle.
            //let simulated_bundle = self.fb_client.simulate_bundle(&bundle).await;

            //match simulated_bundle {
            //    Ok(res) => info!("Simulation Result: {:?}", res),
            //    Err(simulate_error) => error!("Error simulating bundle: {:?}", simulate_error),
            //}

            // Send bundle.
            let pending_bundle = self.fb_client.send_bundle(&bundle).await;

            match pending_bundle {
                Ok(res) => info!("Simulation Result: {:?}", res.await),
                Err(send_error) => error!("Error sending bundle: {:?}", send_error),
            }
        }

        Ok(())
    }
}
