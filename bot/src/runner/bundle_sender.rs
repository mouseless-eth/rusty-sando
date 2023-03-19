use colored::Colorize;
use ethers::prelude::{rand::Rng, *};
use hashbrown::HashMap;
use std::{collections::BTreeMap, sync::Arc};
use tokio::sync::RwLock;

use crate::{
    prelude::{
        fork_factory::ForkFactory,
        make_sandwich,
        sandwich_types::{OptimalRecipe, RawIngredients},
        BlockInfo, Pool, PoolVariant, SendBundleError,
    },
    relay, utils,
    utils::tx_builder::SandwichMaker,
};

use super::state::BotState;

pub struct BundleSender {
    pub pending_sandwiches: HashMap<Pool, Arc<RwLock<Vec<OptimalRecipe>>>>,
}

impl BundleSender {
    // Create a new `TxSender` instance
    //
    // Arguments: no arguments
    //
    // Returns: a new `TxSender` instance
    pub async fn new() -> Self {
        Self {
            pending_sandwiches: HashMap::new(),
        }
    }

    // Add a recipe to the pending transaction list for a given pool
    //
    // Arguments:
    // * `&mut self`: a mutable reference to the `TxSender` instance
    // * `recipe`: an `OptimalRecipe` instance representing the recipe to add
    //
    // Returns: This function returns nothing
    pub async fn add_recipe(&mut self, recipe: OptimalRecipe) {
        let target_pool = recipe.target_pool;
        if let Some(sandwich_vec) = self.pending_sandwiches.get(&target_pool) {
            sandwich_vec.write().await.push(recipe);
        } else {
            let new_vector = Arc::new(RwLock::new(vec![recipe]));
            self.pending_sandwiches.insert(target_pool, new_vector);
        }
    }

    // enchanement: proof of concept atm, a lot of room for improvement
    // bump victim transactions if detect tx with same sender + nonce
    // don't clear at end and just remove recipes based on mined txs in latest block (collect backlog)
    pub async fn make_mega_sandwich(
        &mut self,
        next_block: BlockInfo,
        sandwich_state: Arc<BotState>,
        sandwich_maker: Arc<SandwichMaker>,
    ) {
        for (target_pool, recipes) in self.pending_sandwiches.iter() {
            let mut recipes = {
                let read_lock = recipes.read().await;
                (*read_lock).clone()
            };

            // we alr sent this tx
            if recipes.len() <= 1 {
                continue;
            }

            let sandwich_state = sandwich_state.clone();
            let next_block = next_block.clone();
            let sandwich_maker = sandwich_maker.clone();
            let target_pool = target_pool.clone();
            // could cleanup this code because a lot of copy + pasting from runner/mod.rs
            tokio::spawn(async move {
                // sort recipes by revenue
                recipes.sort_by(|a, b| a.revenue.cmp(&b.revenue));

                let has_dust = recipes[0].has_dust;
                let combined_state_diffs = {
                    let mut combined: BTreeMap<H160, AccountDiff> = BTreeMap::new();

                    for recipe in recipes.iter() {
                        for (key, value) in recipe.state_diffs.clone() {
                            combined.insert(key, value);
                        }
                    }

                    combined
                };

                let client = utils::create_websocket_client().await.unwrap();
                let fork_block = Some(BlockId::Number(BlockNumber::Number(next_block.number)));

                // create evm simulation handler by setting up `fork_factory`
                let initial_db =
                    utils::state_diff::to_cache_db(&combined_state_diffs, fork_block, &client)
                        .await
                        .unwrap();
                let mut fork_factory =
                    ForkFactory::new_sandbox_factory(client.clone(), initial_db, fork_block);

                // create raw ingredients
                let meats = recipes
                    .iter()
                    .flat_map(|recipe| recipe.meats.clone())
                    .collect();

                let raw_ingredients = match RawIngredients::new(
                    &target_pool,
                    meats,
                    utils::constants::get_weth_address(),
                    combined_state_diffs,
                )
                .await
                {
                    Ok(recipe) => recipe,
                    Err(_) => return,
                };

                let weth_balance = {
                    let read_lock = sandwich_state.weth_balance.read().await;
                    (*read_lock).clone()
                };

                //// find optimal input to for multi sandwich
                let mut optimal_sandwich = match make_sandwich::create_optimal_sandwich(
                    &raw_ingredients,
                    weth_balance,
                    &next_block,
                    &mut fork_factory,
                    &sandwich_maker,
                )
                .await
                {
                    Ok(optimal) => optimal,
                    Err(_) => {
                        return;
                    }
                };

                optimal_sandwich.set_has_dust(has_dust);

                // if revenue is non zero send multi-meat sandwich to relays
                if optimal_sandwich.revenue != U256::zero() {
                    match send_bundle(
                        &optimal_sandwich,
                        next_block,
                        sandwich_maker,
                        sandwich_state,
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
                }
            });
        }
    }
}

// Construct and send bundle based on recipe
//
// Arguments:
// * `&recipe`: information on how to construct sandwich bundle
// * `target_block`: holds basefee and timestamp of target block
// * `sandwich_maker`: holds signer, bot address for constructing frontslice and backslice
//
// Returns:
// Ok(()): return nothing if sent succesful
// Err(SendBundleError): return error if send bundle fails
pub async fn send_bundle(
    recipe: &OptimalRecipe,
    target_block: BlockInfo,
    sandwich_maker: Arc<SandwichMaker>,
    sandwich_state: Arc<BotState>,
) -> Result<(), SendBundleError> {
    let nonce = {
        let read_lock = sandwich_maker.nonce.read().await;
        (*read_lock).clone()
    };

    let front_slice_request = Eip1559TransactionRequest {
        to: Some(NameOrAddress::Address(sandwich_maker.sandwich_address)),
        from: Some(sandwich_maker.searcher_wallet.address()),
        data: Some(recipe.frontrun_data.clone()),
        chain_id: Some(U64::from(1)),
        max_priority_fee_per_gas: Some(U256::from(0)),
        max_fee_per_gas: Some(target_block.base_fee),
        gas: Some((U256::from(recipe.frontrun_gas_used) * 10) / 7), // gasused = 70% gaslimit
        nonce: Some(nonce),
        value: Some(recipe.frontrun_value),
        access_list: recipe.frontrun_access_list.clone(),
    };

    let raw_signed_frontrun_tx =
        utils::sign_eip1559(front_slice_request, &sandwich_maker.searcher_wallet).await?;

    let raw_signed_meat_txs: Vec<Bytes> = recipe.meats.iter().map(|meat| meat.rlp()).collect();

    let max_fee = calculate_bribe_for_max_fee(&recipe, &target_block)?;

    let back_slice_request = Eip1559TransactionRequest {
        to: Some(NameOrAddress::Address(sandwich_maker.sandwich_address)),
        from: Some(sandwich_maker.searcher_wallet.address()),
        data: Some(recipe.backrun_data.clone()),
        chain_id: Some(U64::from(1)),
        max_priority_fee_per_gas: Some(max_fee),
        max_fee_per_gas: Some(max_fee),
        gas: Some((U256::from(recipe.backrun_gas_used) * 10) / 7), // gasused = 70% gaslimit
        nonce: Some(nonce + 1),
        value: Some(recipe.backrun_value),
        access_list: recipe.backrun_access_list.clone(),
    };

    let raw_signed_backrun_tx =
        utils::sign_eip1559(back_slice_request, &sandwich_maker.searcher_wallet).await?;

    let bundle = relay::construct_bundle(
        {
            let mut bundled_transactions: Vec<Bytes> = vec![raw_signed_frontrun_tx];
            for meat in raw_signed_meat_txs {
                bundled_transactions.push(meat.clone());
            }
            bundled_transactions.push(raw_signed_backrun_tx);
            bundled_transactions
        },
        target_block.number,
        target_block.timestamp.as_u64(),
    );

    let profit = recipe
        .revenue
        .checked_sub(
            (U256::from(recipe.frontrun_gas_used) * target_block.base_fee)
                + (U256::from(recipe.backrun_gas_used) * max_fee),
        )
        .unwrap_or_default();

    // send bundle to all relay endpoints (concurrently)
    for relay in relay::get_all_relay_endpoints().await {
        let sandwich_state = sandwich_state.clone();
        let sandwich_maker = sandwich_maker.clone();
        let bundle = bundle.clone();
        let recipe = recipe.clone();

        tokio::spawn(async move {
            let pending_bundle = match relay.flashbots_client.inner().send_bundle(&bundle).await {
                Ok(pb) => pb,
                Err(_) => {
                    //log::error!("Failed to send bundle: {:?}", e);
                    return;
                }
            };

            log::info!(
                "{:?} {}",
                recipe.print_meats(),
                format!("Bundle sent to {}", relay.relay_name)
                    .bold()
                    .white()
                    .on_black()
            );

            let bundle_hash = pending_bundle.bundle_hash;

            let is_bundle_included = match pending_bundle.await {
                Ok(_) => true,
                Err(ethers_flashbots::PendingBundleError::BundleNotIncluded) => false,
                Err(e) => {
                    log::error!(
                        "{:?} Bundle rejected due to error : {:?}",
                        recipe.print_meats(),
                        e
                    );
                    false
                }
            };

            // only do this operation once (could do this in a cleaner way :<)
            if relay.relay_name == "flashbots" {
                utils::alert::alert_bundle(
                    bundle_hash,
                    target_block.number,
                    is_bundle_included,
                    &recipe,
                    profit,
                )
                .await;

                match (is_bundle_included, recipe.has_dust) {
                    (true, false) => {
                        let other_token =
                            if recipe.target_pool.token_0 != utils::constants::get_weth_address() {
                                recipe.target_pool.token_0
                            } else {
                                recipe.target_pool.token_1
                            };

                        sandwich_state.update_weth_balance(profit).await;
                        sandwich_state.add_dust(other_token).await;
                        let mut nonce = sandwich_maker.nonce.write().await;
                        *nonce += U256::from(2);
                        log::info!("Adding new dust: {:?}", other_token);
                    }
                    (true, _) => {
                        // update weth balance
                        sandwich_state.update_weth_balance(profit).await;
                        let mut nonce = sandwich_maker.nonce.write().await;
                        *nonce += U256::from(2);
                        log::info!("Updating weth balance");
                    }
                    (false, _) => { /* bundle not included, do nothing */ }
                }
            }
        });
    }

    Ok(())
}

// calculates the optimal bribe for a given opportunity
//
// Arguments
// * `recipe`: information on sandwich bundle
// * `target_block`: information on target_block
//
// Returns:
// Ok(U256) -> The maximum fee for opportunity if calculated succesfully
// Err(SendBundleError) -> Error in bribe amount calculation
fn calculate_bribe_for_max_fee(
    recipe: &OptimalRecipe,
    target_block: &BlockInfo,
) -> Result<U256, SendBundleError> {
    // frontrun txfee is fixed, exclude it from bribe calculations
    let revenue_minus_frontrun_tx_fee = match recipe
        .revenue
        .checked_sub(U256::from(recipe.frontrun_gas_used) * target_block.base_fee)
    {
        Some(revenue) => revenue,
        None => return Err(SendBundleError::FrontrunGasFeesNotCovered()),
    };

    // overpay to get dust onto sandwich contractIf
    // more info: https://twitter.com/libevm/status/1474870661373779969
    let bribe_amount = if !recipe.has_dust {
        revenue_minus_frontrun_tx_fee + ethers::utils::parse_ether("0.00015").unwrap()
    } else {
        let mut rng = rand::thread_rng();

        // enchanement: make bribe adaptive based on competitors
        match recipe.target_pool.pool_variant {
            PoolVariant::UniswapV2 => {
                (revenue_minus_frontrun_tx_fee * (990000000 + rng.gen_range(0..10000000)))
                    / 1000000000
            }
            PoolVariant::UniswapV3 => {
                (revenue_minus_frontrun_tx_fee * (970000000 + rng.gen_range(0..30000000)))
                    / 1000000000
            }
        }
    };

    // calculating bribe amount
    let max_fee = bribe_amount / recipe.backrun_gas_used;

    if max_fee < target_block.base_fee {
        return Err(SendBundleError::MaxFeeLessThanNextBaseFee());
    }

    let effective_miner_tip = max_fee.checked_sub(target_block.base_fee);

    if effective_miner_tip.is_none() {
        return Err(SendBundleError::NegativeMinerTip());
    }

    Ok(max_fee)
}
