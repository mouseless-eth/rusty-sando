use crate::{prelude::Pool, utils};
use dashmap::DashMap;
use ethers::prelude::*;
use futures::stream::FuturesUnordered;
use revm::{
    db::{CacheDB, EmptyDB},
    primitives::{AccountInfo, Bytecode},
};
use std::{
    collections::{btree_map::Entry, BTreeMap},
    sync::Arc,
};

/// Holds pools that have the potential to be sandwiched
#[derive(Clone, Copy, Debug)]
pub struct SandwichablePool {
    pub pool: Pool,
    // Is swap direction zero to one?
    pub is_weth_input: bool,
}

impl SandwichablePool {
    pub fn new(pool: Pool, is_weth_input: bool) -> Self {
        Self {
            pool,
            is_weth_input,
        }
    }
}

// Extract state diffs from a given tx
//
// Arguments:
// * `client`: Websocket provider used for making rpc calls
// * `meats`: Vec of transactions to extract state diffs from
// * `block_num`: Block number of the block the txs are in
//
// Returns:
// Some(BTreeMap<Address, AccountDiff>): State diffs for each address)
// None: If encountered error or state diffs are non existant
pub async fn get_from_txs(
    client: &Arc<Provider<Ws>>,
    meats: &Vec<Transaction>,
    block_num: BlockNumber,
) -> Option<BTreeMap<Address, AccountDiff>> {
    // add statediff trace to each transaction
    let req = meats
        .iter()
        .map(|tx| (tx, vec![TraceType::StateDiff]))
        .collect();

    let block_traces = match client.trace_call_many(req, Some(block_num)).await {
        Ok(x) => x,
        Err(_) => {
            // should throw error here but guess None also works :<
            return None;
        }
    };

    let mut merged_state_diffs = BTreeMap::new();

    block_traces
        .into_iter()
        .flat_map(|bt| bt.state_diff.map(|sd| sd.0.into_iter()))
        .flatten()
        .for_each(|(address, account_diff)| {
            match merged_state_diffs.entry(address) {
                Entry::Vacant(entry) => {
                    entry.insert(account_diff);
                }
                Entry::Occupied(_) => {
                    // Do nothing if the key already exists
                    // we only care abt the starting state
                }
            }
        });

    Some(merged_state_diffs)
}

/// Decode statediff to produce Vec of pools interacted with
///
/// Arguments:
/// * `state_diffs`: BTreeMap of Address and AccountDiff
/// * `all_pools`: HashMap of Address and Pool
///
/// Returns:
/// Some(Vec<SandwichablePool>): Vec of pools that have been interacted with
/// None: If state_diffs is empty
pub fn extract_pools(
    state_diffs: &BTreeMap<Address, AccountDiff>,
    all_pools: &DashMap<Address, Pool>,
) -> Option<Vec<SandwichablePool>> {
    // capture all addresses that have a state change and are also a pool
    let touched_pools: Vec<Pool> = state_diffs
        .keys()
        .filter_map(|e| all_pools.get(e).map(|p| (*p.value()).clone()))
        .collect();

    // find direction of swap based on state diff (does weth have state changes?)
    let weth_state_diff = &state_diffs
        .get(&utils::constants::get_weth_address())?
        .storage;

    let mut sandwichable_pools: Vec<SandwichablePool> = vec![];

    // find storage mapping index for each pool
    for pool in touched_pools {
        // find mapping storage location
        let storage_key = TxHash::from(ethers::utils::keccak256(abi::encode(&[
            abi::Token::Address(pool.address),
            abi::Token::Uint(U256::from(3)),
        ])));
        let is_weth_input = match weth_state_diff.get(&storage_key)? {
            Diff::Changed(c) => {
                let from = U256::from(c.from.to_fixed_bytes());
                let to = U256::from(c.to.to_fixed_bytes());
                to > from
            }
            _ => continue,
        };
        sandwichable_pools.push(SandwichablePool::new(pool, is_weth_input));
    }

    Some(sandwichable_pools)
}

// Turn state_diffs into a new cache_db
//
// Arguments:
// * `state`: Statediffs used as values for creation of cache_db
// * `block_num`: Block number to get state from
// * `provider`: Websocket provider used to make rpc calls
//
// Returns:
// Ok(CacheDB<EmptyDB>): cacheDB created from statediffs, if no errors
// Err(ProviderError): If encountered error during rpc calls
pub async fn to_cache_db(
    state: &BTreeMap<Address, AccountDiff>,
    block_num: Option<BlockId>,
    provider: &Arc<Provider<Ws>>,
) -> Result<CacheDB<EmptyDB>, ProviderError> {
    let mut cache_db = CacheDB::new(EmptyDB::default());

    let mut futures = FuturesUnordered::new();

    for (address, acc_diff) in state.iter() {
        let nonce_provider = provider.clone();
        let balance_provider = provider.clone();
        let code_provider = provider.clone();

        let addy = *address;

        let future = async move {
            let nonce = nonce_provider
                .get_transaction_count(addy, block_num)
                .await?;

            let balance = balance_provider.get_balance(addy, block_num).await?;

            let code = code_provider.get_code(addy, block_num).await?;

            Ok::<(AccountDiff, Address, U256, U256, Bytes), ProviderError>((
                acc_diff.clone(),
                *address,
                nonce,
                balance,
                code,
            ))
        };

        futures.push(future);
    }

    while let Some(result) = futures.next().await {
        let (acc_diff, address, nonce, balance, code) = result?;
        let info = AccountInfo::new(balance.into(), nonce.as_u64(), Bytecode::new_raw(code.0));
        cache_db.insert_account_info(address.0.into(), info);

        acc_diff.storage.iter().for_each(|(slot, storage_diff)| {
            let slot_value: U256 = match storage_diff.to_owned() {
                Diff::Changed(v) => v.from.0.into(),
                Diff::Died(v) => v.0.into(),
                _ => {
                    // for cases Born and Same no need to touch
                    return;
                }
            };
            let slot: U256 = slot.0.into();
            cache_db
                .insert_account_storage(address.0.into(), slot.into(), slot_value.into())
                .unwrap();
        });
    }

    Ok(cache_db)
}
