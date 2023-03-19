use dashmap::DashMap;
use std::sync::Arc;

use crate::prelude::fork_factory::ForkFactory;
use crate::prelude::sandwich_types::RawIngredients;
use crate::prelude::{make_sandwich, Dex, Pool};
use crate::rpc_extensions;
use crate::types::BlockOracle;
use crate::utils;
use crate::utils::tx_builder::SandwichMaker;
use colored::Colorize;
use ethers::prelude::*;
use eyre::Result;
use log;

mod oracles;
use tokio::sync::RwLock;

mod state;
use state::BotState;

mod bundle_sender;
use bundle_sender::*;

pub struct Bot {
    sandwich_state: Arc<BotState>,
    latest_block_oracle: Arc<RwLock<BlockOracle>>,
    client: Arc<Provider<Ws>>,
    all_pools: Arc<DashMap<Address, Pool>>,
    sandwich_maker: Arc<SandwichMaker>,
    bundle_sender: Arc<RwLock<BundleSender>>,
    dexes: Vec<Dex>,
}

impl Bot {
    // Create new bot instance
    //
    // Arguments:
    // * `client`: websocket provider used to make calls
    // * `pool_vec`: vector of pools that the bot will monitor
    //
    // Returns:
    // * Ok(Bot) if successful
    // * Err(eyre::Error) if not successful
    pub async fn new(
        client: Arc<Provider<Ws>>,
        pool_vec: Vec<Pool>,
        dexes: Vec<Dex>,
    ) -> Result<Bot> {
        // create hashmap from our vec of pools (faster access when doing lookups)
        let all_pools: DashMap<Address, Pool> = DashMap::new();
        for pool in pool_vec {
            all_pools.insert(pool.address, pool);
        }

        let all_pools = Arc::new(all_pools);

        let sandwich_inception_block = utils::dotenv::get_sandwich_inception_block();
        let sandwich_state = BotState::new(sandwich_inception_block, &client).await?;
        let sandwich_state = Arc::new(sandwich_state);

        let sandwich_maker = Arc::new(SandwichMaker::new().await);

        let latest_block_oracle = BlockOracle::new(&client).await?;
        let latest_block_oracle = Arc::new(RwLock::new(latest_block_oracle));

        let bundle_sender = Arc::new(RwLock::new(BundleSender::new().await));

        Ok(Bot {
            client,
            all_pools,
            latest_block_oracle,
            sandwich_state,
            sandwich_maker,
            bundle_sender,
            dexes,
        })
    }

    // Run the bot by starting a new mempool stream and filtering txs for opportunities
    //
    // Arguments:
    // * `&mut self`: reference to mutable self
    //
    // Returns:
    // Ok(()) if successful
    // Err(eyre::Error) if encounters error during execution
    pub async fn run(&mut self) -> Result<()> {
        log::info!("Starting bot");

        oracles::start_add_new_pools(&mut self.all_pools, self.dexes.clone());
        oracles::start_block_oracle(&mut self.latest_block_oracle);
        oracles::start_mega_sandwich_oracle(
            self.bundle_sender.clone(),
            self.sandwich_state.clone(),
            self.sandwich_maker.clone(),
        );

        let mut mempool_stream = if let Ok(stream) =
            rpc_extensions::subscribe_pending_txs_with_body(&self.client).await
        {
            stream
        } else {
            panic!("Failed to create mempool stream");
        };

        while let Some(mut victim_tx) = mempool_stream.next().await {
            let client = utils::create_websocket_client().await?;
            let block_oracle = {
                let read_lock = self.latest_block_oracle.read().await;
                (*read_lock).clone()
            };
            let all_pools = &self.all_pools;
            let sandwich_balance = {
                let read_lock = self.sandwich_state.weth_balance.read().await;
                (*read_lock).clone()
            };
            // ignore txs that we can't include in next block
            // enhancement: simulate all txs, store result, and use result when tx can included
            if victim_tx.max_fee_per_gas.unwrap_or(U256::zero()) < block_oracle.next_block.base_fee
            {
                log::info!("{}", format!("{:?} mf<nbf", victim_tx.hash).cyan());
                continue;
            }

            // recover from field from vrs (ECDSA)
            // enhancement: expensive operation, can avoid by modding rpc to share `from` field
            if let Ok(from) = victim_tx.recover_from() {
                victim_tx.from = from;
            } else {
                log::error!(
                    "{}",
                    format!("{:?} ecdsa recovery failed", victim_tx.hash).red()
                );
                continue;
            };

            // get all state diffs that this tx produces
            let state_diffs = if let Some(sd) = utils::state_diff::get_from_txs(
                &self.client,
                &vec![victim_tx.clone()],
                BlockNumber::Number(block_oracle.latest_block.number),
            )
            .await
            {
                sd
            } else {
                log::info!("{:?}", victim_tx.hash);
                continue;
            };

            // if tx has statediff on pool addr then record it in `sandwichable_pools`
            let sandwichable_pools =
                if let Some(sp) = utils::state_diff::extract_pools(&state_diffs, &all_pools) {
                    sp
                } else {
                    log::info!("{:?}", victim_tx.hash);
                    continue;
                };

            let fork_block = Some(BlockId::Number(BlockNumber::Number(
                block_oracle.next_block.number,
            )));

            // create evm simulation handler by setting up `fork_factory`
            let initial_db = utils::state_diff::to_cache_db(&state_diffs, fork_block, &self.client)
                .await
                .unwrap();
            let fork_factory =
                ForkFactory::new_sandbox_factory(client.clone(), initial_db, fork_block);

            // search for opportunities in all pools that the tx touches (concurrently)
            for sandwichable_pool in sandwichable_pools {
                if !sandwichable_pool.is_weth_input {
                    // enhancement: increase opportunities by handling swaps in pools with stables
                    log::info!("{:?} [weth_is_output]", victim_tx.hash);
                    continue;
                } else {
                    log::info!(
                        "{}",
                        format!("{:?} [weth_is_input]", victim_tx.hash).green()
                    );
                }

                // prepare variables for new thread
                let victim_tx = victim_tx.clone();
                let sandwichable_pool = sandwichable_pool.clone();
                let mut fork_factory = fork_factory.clone();
                let block_oracle = block_oracle.clone();
                let sandwich_state = self.sandwich_state.clone();
                let sandwich_maker = self.sandwich_maker.clone();
                let bundle_sender = self.bundle_sender.clone();
                let state_diffs = state_diffs.clone();

                tokio::spawn(async move {
                    // enhancement: increase opportunities by handling swaps in pools with stables
                    let input_token = utils::constants::get_weth_address();
                    let victim_hash = victim_tx.hash;

                    // variables used when searching for opportunity
                    let raw_ingredients = if let Ok(data) = RawIngredients::new(
                        &sandwichable_pool.pool,
                        vec![victim_tx],
                        input_token,
                        state_diffs,
                    )
                    .await
                    {
                        data
                    } else {
                        log::error!("Failed to create raw ingredients for: {:?}", &victim_hash);
                        return;
                    };

                    // find optimal input to sandwich tx
                    let mut optimal_sandwich = match make_sandwich::create_optimal_sandwich(
                        &raw_ingredients,
                        sandwich_balance,
                        &block_oracle.next_block,
                        &mut fork_factory,
                        &sandwich_maker,
                    )
                    .await
                    {
                        Ok(optimal) => optimal,
                        Err(e) => {
                            log::info!(
                                "{}",
                                format!("{:?} sim failed due to {:?}", &victim_hash, e).yellow()
                            );
                            return;
                        }
                    };

                    // check if has dust
                    let other_token = if optimal_sandwich.target_pool.token_0
                        != utils::constants::get_weth_address()
                    {
                        optimal_sandwich.target_pool.token_0
                    } else {
                        optimal_sandwich.target_pool.token_1
                    };

                    if sandwich_state.has_dust(&other_token).await {
                        optimal_sandwich.has_dust = true;
                    }

                    // spawn thread to send tx to builders
                    let optimal_sandwich = optimal_sandwich.clone();
                    let optimal_sandwich_two = optimal_sandwich.clone();
                    let sandwich_maker = sandwich_maker.clone();
                    let sandwich_state = sandwich_state.clone();

                    if optimal_sandwich.revenue > U256::zero() {
                        tokio::spawn(async move {
                            match bundle_sender::send_bundle(
                                &optimal_sandwich,
                                block_oracle.next_block,
                                sandwich_maker,
                                sandwich_state.clone(),
                            )
                            .await
                            {
                                Ok(_) => { /* all reporting already done inside of send_bundle */ }
                                Err(e) => {
                                    log::info!(
                                        "{}",
                                        format!(
                                            "{:?} failed to send bundle, due to {:?}",
                                            optimal_sandwich.print_meats(),
                                            e
                                        )
                                        .bright_magenta()
                                    );
                                }
                            };
                        });
                    }

                    // spawn thread to add tx for mega sandwich calculation
                    let bundle_sender = bundle_sender.clone();
                    tokio::spawn(async move {
                        bundle_sender
                            .write()
                            .await
                            .add_recipe(optimal_sandwich_two)
                            .await;
                    });
                });
            }
        }
        Ok(())
    }
}
