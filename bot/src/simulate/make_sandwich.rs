use ethers::prelude::*;
use revm::primitives::{ExecutionResult, Output, TransactTo, B160 as rAddress, U256 as rU256};

use crate::prelude::access_list::AccessListInspector;
use crate::prelude::fork_db::ForkDB;
use crate::prelude::fork_factory::ForkFactory;
use crate::prelude::is_sando_safu::{IsSandoSafu, SalmonellaInspectoooor};
use crate::prelude::sandwich_types::RawIngredients;
use crate::prelude::{convert_access_list, get_amount_out_evm, get_balance_of_evm, PoolVariant};
use crate::types::sandwich_types::OptimalRecipe;
use crate::types::{BlockInfo, SimulationError};
use crate::utils::dotenv;
use crate::utils::tx_builder::{self, braindance, SandwichMaker};
use async_recursion::async_recursion;

use super::{
    attach_braindance_module, braindance_address, braindance_controller_address,
    braindance_starting_balance, setup_block_state,
};

// Calculate amount in that produces highest revenue and performs honeypot checks
//
// Arguments:
// `&ingredients`: holds onchain information about opportunity
// `sandwich_balance`: balance of sandwich contract
// `&next_block`: holds information about next block
// `&mut fork_factory`: used to create new forked evm instances for simulations
// `sandwich_maker`: handles encoding of transaction for sandwich contract
//
// Returns:
// Ok(OptimalRecipe) if no errors during calculation
// Err(SimulationError) if error during calculation
pub async fn create_optimal_sandwich(
    ingredients: &RawIngredients,
    sandwich_balance: U256,
    next_block: &BlockInfo,
    fork_factory: &mut ForkFactory,
    sandwich_maker: &SandwichMaker,
) -> Result<OptimalRecipe, SimulationError> {
    let optimal = juiced_quadratic_search(
        ingredients,
        U256::zero(),
        sandwich_balance,
        next_block,
        fork_factory,
    )
    .await?;

    #[cfg(test)]
    {
        println!("Optimal amount in: {}", optimal);
    }

    sanity_check(
        sandwich_balance,
        optimal,
        ingredients,
        next_block,
        sandwich_maker,
        fork_factory.new_sandbox_fork(),
    )
}

// Roided implementation of https://research.ijcaonline.org/volume65/number14/pxc3886165.pdf
// splits range in more intervals, search intervals concurrently, compare, repeat till termination
//
// Arguments:
// * `&ingredients`: holds onchain information about opportunity
// * `lower_bound`: lower bound of search interval
// * `upper_bound`: upper bound of search interval, normally equal to sandwich balance
// * `next_block`: holds information about next block
// * `fork_factory`: used to create new forked evm instances for simulations
//
// Returns:
// Ok(U256): optimal amount in, if no errors during calculation
// Err(SimulationError): if error during calculation
#[async_recursion]
async fn juiced_quadratic_search(
    ingredients: &RawIngredients,
    mut lower_bound: U256,
    mut upper_bound: U256,
    next_block: &BlockInfo,
    mut fork_factory: &mut ForkFactory,
) -> Result<U256, SimulationError> {
    //
    //            [EXAMPLE WITH 10 BOUND INTERVALS]
    //
    //     (first)              (mid)               (last)
    //        ▼                   ▼                   ▼
    //        +---+---+---+---+---+---+---+---+---+---+
    //        |   |   |   |   |   |   |   |   |   |   |
    //        +---+---+---+---+---+---+---+---+---+---+
    //        ▲   ▲   ▲   ▲   ▲   ▲   ▲   ▲   ▲   ▲   ▲
    //        0   1   2   3   4   5   6   7   8   9   X
    //
    //  * [0, X] = search range
    //  * Find revenue at each interval
    //  * Find index of interval with highest revenue
    //  * Search again with bounds set to adjacent index of highest
    //

    attach_braindance_module(&mut fork_factory);

    #[cfg(test)]
    {
        // if running test, setup contract sandwich to allow for backtest
        // can also inject new sandwich code for testing
        crate::prelude::inject_sando(&mut fork_factory, upper_bound);
    }

    // setup values for search termination
    let base = U256::from(100000_u64);
    let tolerance = U256::from(1u64);

    // initialize variables for search
    let left_interval_lower = |i: usize, intervals: &Vec<U256>| intervals[i - 1].clone() + 1;
    let right_interval_upper = |i: usize, intervals: &Vec<U256>| intervals[i + 1].clone() - 1;
    let mut highest_sando_input;
    let number_of_intervals = 15;
    let mut counter = 0;

    // continue search until termination condition is met (no point seraching down to closest wei)
    loop {
        counter += 1;

        // split search range into intervals
        let mut intervals = Vec::new();
        for i in 0..=number_of_intervals {
            intervals.push(lower_bound + (((upper_bound - lower_bound) * i) / number_of_intervals));
        }

        // calculate revenue at each interval concurrently
        let mut revenues = Vec::new();
        for bound in &intervals {
            let sim = tokio::task::spawn(evaluate_sandwich_revenue(
                *bound,
                ingredients.clone(),
                next_block.clone(),
                fork_factory.new_sandbox_fork(),
            ));
            revenues.push(sim);
        }

        let revenues = futures::future::join_all(revenues).await;

        let revenues = revenues
            .into_iter()
            .map(|r| r.unwrap().unwrap_or_default())
            .collect::<Vec<_>>();

        // find interval that produces highest revenue
        let (highest_revenue_index, _highest_revenue) = revenues
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.cmp(&b))
            .unwrap();

        highest_sando_input = intervals[highest_revenue_index];

        // enhancement: find better way to increase finding opps incase of all rev=0
        if revenues[highest_revenue_index] == U256::zero() {
            // most likely there is no sandwich possibility
            if counter == 10 {
                return Ok(U256::zero());
            }
            // no revenue found, most likely small optimal so decrease range
            upper_bound = intervals[intervals.len() / 3] - 1;
            continue;
        }

        // if highest revenue is produced at last interval
        if highest_revenue_index == intervals.len() - 1 {
            // hit upper bound (upper bound stays fixed)
            lower_bound = left_interval_lower(highest_revenue_index, &intervals);
            //upper_bound = right_interval_upper(highest_revenue_index, &intervals);
            continue;
        }

        // if highest revenue is produced at first interval (lower bound stays fixed)
        if highest_revenue_index == 0 {
            //lower_bound = left_interval_lower(highest_revenue_index, &intervals);
            upper_bound = right_interval_upper(highest_revenue_index, &intervals);
            continue;
        }

        // set bounds to intervals adjacent to highest revenue index and search again
        lower_bound = left_interval_lower(highest_revenue_index, &intervals);
        upper_bound = right_interval_upper(highest_revenue_index, &intervals);

        let search_range = match upper_bound.checked_sub(lower_bound) {
            Some(range) => range,
            None => break,
        };
        if search_range < ((tolerance * ((upper_bound + lower_bound) / 2)) / base) {
            break;
        }
    }

    // Return the floor (avoid overestimation error which may kill opportunity)
    Ok(highest_sando_input)
}

// Perform simulation using sandwich contract and check for salmonella
//
// Arguments:
// `sandwich_start_balance`: amount of token held by sandwich contract
// `frontrun_in`: amount to use as frontrun
// `ingredients`: holds information about opportunity
// `next_block`: holds information about next block
// `sandwich_maker`: handles encoding of transaction for sandwich contract
// `fork_db`: fork db used for evm simulations
//
// Returns:
// Ok(OptimalRecipe): params to pass to sandwich contract to capture opportunity
// Err(SimulationError): error encountered during simulation
fn sanity_check(
    sandwich_start_balance: U256,
    frontrun_in: U256,
    ingredients: &RawIngredients,
    next_block: &BlockInfo,
    sandwich_maker: &SandwichMaker,
    fork_db: ForkDB,
) -> Result<OptimalRecipe, SimulationError> {
    // setup evm simulation
    let mut evm = revm::EVM::new();
    evm.database(fork_db);
    setup_block_state(&mut evm, &next_block);

    let searcher = dotenv::get_searcher_wallet().address();
    let sandwich_contract = dotenv::get_sandwich_contract_address();
    let pool_variant = ingredients.target_pool.pool_variant;

    // *´:°•.°+.*•´.*:˚.°*.˚•´.°:°•.°•.*•´.*:˚.°*.˚•´.°:°•.°+.*•´.*:*/
    // *                    FRONTRUN TRANSACTION                    */
    // *.•°:°.´+˚.*°.˚:*.´•*.+°.•°:´*.´•*.•°.•°:°.´:•˚°.*°.˚:*.´+°.•*/
    //
    // encode frontrun_in before passing to sandwich contract
    let frontrun_in = match pool_variant {
        PoolVariant::UniswapV2 => tx_builder::v2::encode_weth(frontrun_in),
        PoolVariant::UniswapV3 => tx_builder::v3::encode_weth(frontrun_in),
    };

    // caluclate frontrun_out using encoded frontrun_in
    let frontrun_out = match pool_variant {
        PoolVariant::UniswapV2 => {
            let target_pool = ingredients.target_pool.address;
            let token_in = ingredients.startend_token;
            let token_out = ingredients.intermediary_token;
            evm.env.tx.gas_price = next_block.base_fee.into();
            evm.env.tx.gas_limit = 700000;
            evm.env.tx.value = rU256::ZERO;
            let amount_out =
                get_amount_out_evm(frontrun_in, target_pool, token_in, token_out, &mut evm)?;
            tx_builder::v2::encode_intermediary(amount_out, true, token_out)
        }
        PoolVariant::UniswapV3 => U256::zero(),
    };

    // create tx.data and tx.value for frontrun_in
    let (frontrun_data, frontrun_value) = match pool_variant {
        PoolVariant::UniswapV2 => sandwich_maker.v2.create_payload_weth_is_input(
            frontrun_in,
            frontrun_out,
            ingredients.intermediary_token,
            ingredients.target_pool,
        ),
        PoolVariant::UniswapV3 => sandwich_maker.v3.create_payload_weth_is_input(
            frontrun_in.as_u128().into(),
            ingredients.startend_token,
            ingredients.intermediary_token,
            ingredients.target_pool,
        ),
    };

    // setup evm for frontrun transaction
    evm.env.tx.caller = searcher.0.into();
    evm.env.tx.transact_to = TransactTo::Call(sandwich_contract.0.into());
    evm.env.tx.data = frontrun_data.clone().into();
    evm.env.tx.value = frontrun_value.into();
    evm.env.tx.gas_limit = 700000;
    evm.env.tx.gas_price = next_block.base_fee.into();
    evm.env.tx.access_list = Vec::default();

    // get access list
    let mut access_list_inspector = AccessListInspector::new(searcher, sandwich_contract);
    evm.inspect_ref(&mut access_list_inspector)
        .map_err(|e| SimulationError::FrontrunEvmError(e))
        .unwrap();
    let frontrun_access_list = access_list_inspector.into_access_list();
    evm.env.tx.access_list = frontrun_access_list.clone();

    // run again but now with access list (so that we get accurate gas used)
    // run with a salmonella inspector to flag `suspicious` opcodes
    let mut salmonella_inspector = SalmonellaInspectoooor::new();
    let frontrun_result = match evm.inspect_commit(&mut salmonella_inspector) {
        Ok(result) => result,
        Err(e) => return Err(SimulationError::FrontrunEvmError(e)),
    };
    match frontrun_result {
        ExecutionResult::Success { .. } => { /* continue operation */ }
        ExecutionResult::Revert { output, .. } => {
            return Err(SimulationError::FrontrunReverted(output))
        }
        ExecutionResult::Halt { reason, .. } => {
            return Err(SimulationError::FrontrunHalted(reason))
        }
    };
    match salmonella_inspector.is_sando_safu() {
        IsSandoSafu::Safu => { /* continue operation */ }
        IsSandoSafu::NotSafu(not_safu_opcodes) => {
            return Err(SimulationError::FrontrunNotSafu(not_safu_opcodes))
        }
    }

    let frontrun_gas_used = frontrun_result.gas_used();

    // *´:°•.°+.*•´.*:˚.°*.˚•´.°:°•.°•.*•´.*:˚.°*.˚•´.°:°•.°+.*•´.*:*/
    // *                     MEAT TRANSACTION/s                     */
    // *.•°:°.´+˚.*°.˚:*.´•*.+°.•°:´*.´•*.•°.•°:°.´:•˚°.*°.˚:*.´+°.•*/
    let mut is_meat_good = Vec::new();
    for meat in ingredients.meats.iter() {
        evm.env.tx.caller = rAddress::from_slice(&meat.from.0);
        evm.env.tx.transact_to =
            TransactTo::Call(rAddress::from_slice(&meat.to.unwrap_or_default().0));
        evm.env.tx.data = meat.input.0.clone();
        evm.env.tx.value = meat.value.into();
        evm.env.tx.chain_id = meat.chain_id.map(|id| id.as_u64());
        evm.env.tx.nonce = Some(meat.nonce.as_u64());
        evm.env.tx.gas_limit = meat.gas.as_u64();
        match meat.transaction_type {
            Some(ethers::types::U64([0])) => {
                // legacy tx
                evm.env.tx.gas_price = meat.gas_price.unwrap_or_default().into();
            }
            Some(_) => {
                // type 2 tx
                evm.env.tx.gas_priority_fee = meat.max_priority_fee_per_gas.map(|mpf| mpf.into());
                evm.env.tx.gas_price = meat.max_fee_per_gas.unwrap_or_default().into();
            }
            None => {
                // legacy tx
                evm.env.tx.gas_price = meat.gas_price.unwrap().into();
            }
        }

        // keep track of which meat transactions are successful to filter reverted meats at end
        // filter reverted meats because gas cost of mempool txs are accounted for by fb
        let res = match evm.transact_commit() {
            Ok(result) => result,
            Err(e) => return Err(SimulationError::EvmError(e)),
        };
        match res.is_success() {
            true => is_meat_good.push(true),
            false => is_meat_good.push(false),
        }
    }

    // *´:°•.°+.*•´.*:˚.°*.˚•´.°:°•.°•.*•´.*:˚.°*.˚•´.°:°•.°+.*•´.*:*/
    // *                    BACKRUN TRANSACTION                     */
    // *.•°:°.´+˚.*°.˚:*.´•*.+°.•°:´*.´•*.•°.•°:°.´:•˚°.*°.˚:*.´+°.•*/
    //
    // encode backrun_in before passing to sandwich contract
    let token_in = ingredients.intermediary_token;
    let token_out = ingredients.startend_token;
    let balance = get_balance_of_evm(token_in, sandwich_contract, next_block, &mut evm)?;
    let backrun_in = match pool_variant {
        PoolVariant::UniswapV2 => {
            tx_builder::v2::encode_intermediary_with_dust(balance, false, token_in)
        }
        PoolVariant::UniswapV3 => tx_builder::v3::encode_intermediary_token(balance),
    };

    // caluclate backrun_out using encoded backrun_in
    let backrun_out = match pool_variant {
        PoolVariant::UniswapV2 => {
            let target_pool = ingredients.target_pool.address;
            let out = get_amount_out_evm(backrun_in, target_pool, token_in, token_out, &mut evm)?;
            tx_builder::v2::encode_weth(out)
        }
        PoolVariant::UniswapV3 => U256::zero(),
    };

    // create tx.data and tx.value for backrun_in
    let (backrun_data, backrun_value) = match pool_variant {
        PoolVariant::UniswapV2 => sandwich_maker.v2.create_payload_weth_is_output(
            backrun_in,
            backrun_out,
            ingredients.intermediary_token,
            ingredients.target_pool,
        ),
        PoolVariant::UniswapV3 => (
            sandwich_maker.v3.create_payload_weth_is_output(
                backrun_in.as_u128().into(),
                ingredients.intermediary_token,
                ingredients.startend_token,
                ingredients.target_pool,
            ),
            U256::zero(),
        ),
    };

    // setup evm for backrun transaction
    evm.env.tx.caller = searcher.0.into();
    evm.env.tx.transact_to = TransactTo::Call(sandwich_contract.0.into());
    evm.env.tx.data = backrun_data.clone().into();
    evm.env.tx.gas_limit = 700000;
    evm.env.tx.gas_price = next_block.base_fee.into();
    evm.env.tx.value = backrun_value.into();

    // create access list
    let mut access_list_inspector = AccessListInspector::new(searcher, sandwich_contract);
    evm.inspect_ref(&mut access_list_inspector)
        .map_err(|e| SimulationError::FrontrunEvmError(e))
        .unwrap();
    let backrun_access_list = access_list_inspector.into_access_list();
    evm.env.tx.access_list = backrun_access_list.clone();

    // run again but now with access list (so that we get accurate gas used)
    // run with a salmonella inspector to flag `suspicious` opcodes
    let mut salmonella_inspector = SalmonellaInspectoooor::new();
    let backrun_result = match evm.inspect_commit(&mut salmonella_inspector) {
        Ok(result) => result,
        Err(e) => return Err(SimulationError::BackrunEvmError(e)),
    };
    match backrun_result {
        ExecutionResult::Success { .. } => { /* continue */ }
        ExecutionResult::Revert { output, .. } => {
            return Err(SimulationError::BackrunReverted(output))
        }
        ExecutionResult::Halt { reason, .. } => return Err(SimulationError::BackrunHalted(reason)),
    };
    match salmonella_inspector.is_sando_safu() {
        IsSandoSafu::Safu => { /* continue operation */ }
        IsSandoSafu::NotSafu(not_safu_opcodes) => {
            return Err(SimulationError::BackrunNotSafu(not_safu_opcodes))
        }
    }

    let backrun_gas_used = backrun_result.gas_used();

    // *´:°•.°+.*•´.*:˚.°*.˚•´.°:°•.°•.*•´.*:˚.°*.˚•´.°:°•.°+.*•´.*:*/
    // *                      GENERATE REPORTS                      */
    // *.•°:°.´+˚.*°.˚:*.´•*.+°.•°:´*.´•*.•°.•°:°.´:•˚°.*°.˚:*.´+°.•*/
    //
    // caluclate revenue from balance change
    let post_sandwich_balance = get_balance_of_evm(
        ingredients.startend_token,
        sandwich_contract,
        next_block,
        &mut evm,
    )?;
    let revenue = post_sandwich_balance
        .checked_sub(sandwich_start_balance)
        .unwrap_or_default();

    // filter only passing meat txs
    let good_meats_only = ingredients
        .meats
        .iter()
        .zip(is_meat_good.iter())
        .filter(|&(_, &b)| b)
        .map(|(s, _)| s.to_owned())
        .collect();

    Ok(OptimalRecipe::new(
        frontrun_data.into(),
        frontrun_value,
        frontrun_gas_used,
        convert_access_list(frontrun_access_list),
        backrun_data.into(),
        backrun_value,
        backrun_gas_used,
        convert_access_list(backrun_access_list),
        good_meats_only,
        revenue,
        ingredients.target_pool,
        ingredients.state_diffs.clone(),
    ))
}

/// Sandwich simulation using BrainDance contract (modified router contract)
///
/// Arguments:
/// * `frontrun_in`: amount of to frontrun with
/// * `ingredients`: ingredients of the sandwich
/// * `next_block`: block info of the next block
/// * `fork_db`: database instance used for evm simulations
async fn evaluate_sandwich_revenue(
    frontrun_in: U256,
    ingredients: RawIngredients,
    next_block: BlockInfo,
    fork_db: ForkDB,
) -> Result<U256, SimulationError> {
    let mut evm = revm::EVM::new();
    evm.database(fork_db);
    setup_block_state(&mut evm, &next_block);

    let pool_variant = ingredients.target_pool.pool_variant;

    /*´:°•.°+.*•´.*:˚.°*.˚•´.°:°•.°•.*•´.*:˚.°*.˚•´.°:°•.°+.*•´.*:*/
    /*                    FRONTRUN TRANSACTION                    */
    /*.•°:°.´+˚.*°.˚:*.´•*.+°.•°:´*.´•*.•°.•°:°.´:•˚°.*°.˚:*.´+°.•*/
    let frontrun_data = match pool_variant {
        PoolVariant::UniswapV2 => braindance::build_swap_v2_data(
            frontrun_in,
            ingredients.target_pool.address,
            ingredients.startend_token,
            ingredients.intermediary_token,
        ),
        PoolVariant::UniswapV3 => braindance::build_swap_v3_data(
            frontrun_in.as_u128().into(),
            ingredients.target_pool.address,
            ingredients.startend_token,
            ingredients.intermediary_token,
        ),
    };

    evm.env.tx.caller = braindance_controller_address();
    evm.env.tx.transact_to = TransactTo::Call(braindance_address().0.into());
    evm.env.tx.data = frontrun_data.0;
    evm.env.tx.gas_limit = 700000;
    evm.env.tx.gas_price = next_block.base_fee.into();
    evm.env.tx.value = rU256::ZERO;

    let result = match evm.transact_commit() {
        Ok(result) => result,
        Err(e) => return Err(SimulationError::FrontrunEvmError(e)),
    };
    let output = match result {
        ExecutionResult::Success { output, .. } => match output {
            Output::Call(o) => o,
            Output::Create(o, _) => o,
        },
        ExecutionResult::Revert { output, .. } => {
            return Err(SimulationError::FrontrunReverted(output))
        }
        ExecutionResult::Halt { reason, .. } => {
            return Err(SimulationError::FrontrunHalted(reason))
        }
    };
    let (_frontrun_out, backrun_in) = match pool_variant {
        PoolVariant::UniswapV2 => {
            match tx_builder::braindance::decode_swap_v2_result(output.into()) {
                Ok(output) => output,
                Err(e) => return Err(SimulationError::FailedToDecodeOutput(e)),
            }
        }
        PoolVariant::UniswapV3 => {
            match tx_builder::braindance::decode_swap_v3_result(output.into()) {
                Ok(output) => output,
                Err(e) => return Err(SimulationError::FailedToDecodeOutput(e)),
            }
        }
    };

    /*´:°•.°+.*•´.*:˚.°*.˚•´.°:°•.°•.*•´.*:˚.°*.˚•´.°:°•.°+.*•´.*:*/
    /*                     MEAT TRANSACTION/s                     */
    /*.•°:°.´+˚.*°.˚:*.´•*.+°.•°:´*.´•*.•°.•°:°.´:•˚°.*°.˚:*.´+°.•*/
    for meat in ingredients.meats.iter() {
        evm.env.tx.caller = rAddress::from_slice(&meat.from.0);
        evm.env.tx.transact_to =
            TransactTo::Call(rAddress::from_slice(&meat.to.unwrap_or_default().0));
        evm.env.tx.data = meat.input.0.clone();
        evm.env.tx.value = meat.value.into();
        evm.env.tx.chain_id = meat.chain_id.map(|id| id.as_u64());
        evm.env.tx.nonce = Some(meat.nonce.as_u64());
        evm.env.tx.gas_limit = meat.gas.as_u64();
        match meat.transaction_type {
            Some(ethers::types::U64([0])) => {
                // legacy tx
                evm.env.tx.gas_price = meat.gas_price.unwrap_or_default().into();
            }
            Some(_) => {
                // type 2 tx
                evm.env.tx.gas_priority_fee = meat.max_priority_fee_per_gas.map(|mpf| mpf.into());
                evm.env.tx.gas_price = meat.max_fee_per_gas.unwrap_or_default().into();
            }
            None => {
                // legacy tx
                evm.env.tx.gas_price = meat.gas_price.unwrap_or_default().into();
            }
        }

        let _res = evm.transact_commit();
    }

    /*´:°•.°+.*•´.*:˚.°*.˚•´.°:°•.°•.*•´.*:˚.°*.˚•´.°:°•.°+.*•´.*:*/
    /*                    BACKRUN TRANSACTION                     */
    /*.•°:°.´+˚.*°.˚:*.´•*.+°.•°:´*.´•*.•°.•°:°.´:•˚°.*°.˚:*.´+°.•*/
    let backrun_data = match pool_variant {
        PoolVariant::UniswapV2 => braindance::build_swap_v2_data(
            backrun_in,
            ingredients.target_pool.address,
            ingredients.intermediary_token,
            ingredients.startend_token,
        ),
        PoolVariant::UniswapV3 => braindance::build_swap_v3_data(
            backrun_in.as_u128().into(),
            ingredients.target_pool.address,
            ingredients.intermediary_token,
            ingredients.startend_token,
        ),
    };

    evm.env.tx.caller = braindance_controller_address();
    evm.env.tx.transact_to = TransactTo::Call(braindance_address().0.into());
    evm.env.tx.data = backrun_data.0;
    evm.env.tx.gas_limit = 700000;
    evm.env.tx.gas_price = next_block.base_fee.into();
    evm.env.tx.value = rU256::ZERO;

    let result = match evm.transact_commit() {
        Ok(result) => result,
        Err(e) => return Err(SimulationError::BackrunEvmError(e)),
    };
    let output = match result {
        ExecutionResult::Success { output, .. } => match output {
            Output::Call(o) => o,
            Output::Create(o, _) => o,
        },
        ExecutionResult::Revert { output, .. } => {
            return Err(SimulationError::BackrunReverted(output))
        }
        ExecutionResult::Halt { reason, .. } => return Err(SimulationError::BackrunHalted(reason)),
    };
    let (_backrun_out, post_sandwich_balance) = match pool_variant {
        PoolVariant::UniswapV2 => {
            match tx_builder::braindance::decode_swap_v2_result(output.into()) {
                Ok(output) => output,
                Err(e) => return Err(SimulationError::FailedToDecodeOutput(e)),
            }
        }
        PoolVariant::UniswapV3 => {
            match tx_builder::braindance::decode_swap_v3_result(output.into()) {
                Ok(output) => output,
                Err(e) => return Err(SimulationError::FailedToDecodeOutput(e)),
            }
        }
    };

    let revenue = post_sandwich_balance
        .checked_sub(braindance_starting_balance())
        .unwrap_or_default();

    Ok(revenue)
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use crate::{
        prelude::{fork_factory::ForkFactory, sandwich_types::RawIngredients},
        utils::{self, constants, testhelper, tx_builder::SandwichMaker},
    };
    use dotenv::dotenv;
    use ethers::prelude::*;
    use tokio::{runtime::Runtime, time::Instant};

    async fn create_test(fork_block_num: u64, pool_addr: &str, meats: Vec<&str>, is_v2: bool) {
        dotenv().ok();
        let ws_provider = testhelper::create_ws().await;

        let start = Instant::now();

        let pool = match is_v2 {
            true => {
                testhelper::create_v2_pool(pool_addr.parse::<Address>().unwrap(), &ws_provider)
                    .await
            }
            false => {
                testhelper::create_v3_pool(pool_addr.parse::<Address>().unwrap(), &ws_provider)
                    .await
            }
        };

        let mut victim_txs = vec![];

        for tx_hash in meats {
            let tx_hash = TxHash::from_str(tx_hash).unwrap();
            victim_txs.push(ws_provider.get_transaction(tx_hash).await.unwrap().unwrap());
        }

        let state = utils::state_diff::get_from_txs(
            &ws_provider,
            &victim_txs,
            BlockNumber::Number(U64::from(fork_block_num)),
        )
        .await
        .unwrap();

        let initial_db = utils::state_diff::to_cache_db(
            &state,
            Some(BlockId::Number(BlockNumber::Number(fork_block_num.into()))),
            &ws_provider,
        )
        .await
        .unwrap();
        let mut db = ForkFactory::new_sandbox_factory(
            ws_provider.clone(),
            initial_db,
            Some(fork_block_num.into()),
        );

        let ingredients =
            RawIngredients::new(&pool, victim_txs, constants::get_weth_address(), state)
                .await
                .unwrap();

        match super::create_optimal_sandwich(
            &ingredients,
            ethers::utils::parse_ether("420").unwrap(),
            &testhelper::get_next_block_info(fork_block_num, &ws_provider).await,
            &mut db,
            &SandwichMaker::new().await,
        )
        .await
        {
            Ok(sandwich) => println!("revenue: {:?}", sandwich.revenue),
            Err(_) => println!("not sandwichable"),
        };
        println!("total_duration took: {:?}", start.elapsed());
    }

    #[test]
    fn sandv2_uni() {
        // Can't use [tokio::test] attr with ethersdb for some reason
        // so manually create a runtime
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            create_test(
                16863388,
                "0x55f26293a2cF63589fdb4aE60E286Cfd6b40595C",
                vec!["0x2a91e14d091fd31d281facc995f6c3cc7a282c37df6dca0c022ad5f9bb7d672f"],
                true,
            )
            .await;
        });
    }

    #[test]
    fn sandv3_uniswap_universal_router_one() {
        // Can't use [tokio::test] attr with ethersdb for some reason
        // so manually create a runtime
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            create_test(
                16863224,
                "0x62CBac19051b130746Ec4CF96113aF5618F3A212",
                vec!["0x90dfe56814821e7f76f2e4970a7b35948670a968abffebb7be69fe528283e6d8"],
                false,
            )
            .await;
        });
    }

    #[test]
    fn sandv3_uniswap_universal_router_two() {
        // Can't use [tokio::test] attr with ethersdb for some reason
        // so manually create a runtime
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            create_test(
                16863008,
                "0xa80838D2BB3d6eBaEd1978FA23b38F91775D8378",
                vec!["0xcb0d4dc905ae0662e5f18b4ad0c2af4e700e8b5969d878a2dcfd0d9507435f4d"],
                false,
            )
            .await;
        });
    }

    #[test]
    fn sandv2_kyber_swap() {
        // Can't use [tokio::test] attr with ethersdb for some reason
        // so manually create a runtime
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            create_test(
                16863312,
                "0x08650bb9dc722C9c8C62E79C2BAfA2d3fc5B3293",
                vec!["0x907894174999fdddc8d8f8e90c210cdb894b91c2c0d79ac35603007d3ce54d00"],
                true,
            )
            .await;
        });
    }

    #[test]
    fn sandv2_non_sandwichable() {
        // Can't use [tokio::test] attr with ethersdb for some reason
        // so manually create a runtime
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            create_test(
                16780624,
                "0x657c6a08d49b4f0778f9cce1dc49d196cfce9d08",
                vec!["0x77b0b15a3216885a66b3b800173e0edcae9d8d191f7093b99a46fc9346f67466"],
                true,
            )
            .await;
        });
    }

    #[test]
    fn sandv2_multi_with_three_expect_one_reverts() {
        // Can't use [tokio::test] attr with ethersdb for some reason
        // so manually create a runtime
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            create_test(
                16780624,
                "0x657c6a08d49b4f0778f9cce1dc49d196cfce9d08",
                vec![
                    "0x4791d05bdd6765f036ff4ae44fc27099997417e3bdb053ecb52182bbfc7767c5",
                    "0x923c9ba97fea8d72e60c14d1cc360a8e7d99dd4b31274928d6a79704a8546eda",
                    "0x77b0b15a3216885a66b3b800173e0edcae9d8d191f7093b99a46fc9346f67466",
                ],
                true,
            )
            .await;
        });
    }

    #[test]
    fn sandv2_multi_two() {
        // Can't use [tokio::test] attr with ethersdb for some reason
        // so manually create a runtime
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            create_test(
                16780624,
                "0x657c6a08d49B4F0778f9cce1Dc49d196cFCe9d08",
                vec![
                    "0x4791d05bdd6765f036ff4ae44fc27099997417e3bdb053ecb52182bbfc7767c5",
                    "0x923c9ba97fea8d72e60c14d1cc360a8e7d99dd4b31274928d6a79704a8546eda",
                ],
                true,
            )
            .await;
        });
    }

    // can only do this once encoding type is fixed
    #[test]
    fn sandv2_multi_four() {
        // Can't use [tokio::test] attr with ethersdb for some reason
        // so manually create a runtime
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            create_test(
                16777150,
                "2bf64a137b080c4ec736c4c1140c496e294dd830",
                vec![
                    "0x4aa242241b10015f297757593e7b53b422f16dee4554adfeca1c6e6d6eaf8c6d",
                    "0x92129ff45324595a37ed395d596d7e5c18ecb2a13a234027bdc7e9a04f6e8366",
                    "0x0ca5fa291e0c2be77b03dfcc3f401f878bac1dfdf58a9e9bb0743d84248d4089",
                    "0x7a26713b13d8f773ab6dd4c90a6bc6182c8061958bf410c3f93cf589042625e9",
                ],
                true,
            )
            .await;
        });
    }

    #[test]
    fn sandv2_1inch() {
        // Can't use [tokio::test] attr with ethersdb for some reason
        // so manually create a runtime
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            create_test(
                16863276,
                "0x9556E7c0461bd2C8d89bDD4a6B0a4b855572cA6E",
                vec!["0x0ba8f0c48d36ec6967ee13e785de82edbf5e3217eeee3f9a92e9a57a5239f939"],
                true,
            )
            .await;
        });
    }
}
