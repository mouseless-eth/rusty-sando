use anvil::eth::util::get_precompiles_for;
use anyhow::{anyhow, Result};
use cfmms::pool::Pool;
use cfmms::pool::Pool::{UniswapV2, UniswapV3};
use ethers::abi::{self, Address, ParamType};
use ethers::types::{Bytes, Transaction, U256};
use foundry_evm::executor::inspector::AccessListTracer;
use foundry_evm::executor::{ExecutionResult, Output, TransactTo};
use foundry_evm::{
    executor::fork::SharedBackend,
    revm::{
        db::CacheDB,
        primitives::{Address as rAddress, U256 as rU256},
        EVM,
    },
};

use crate::constants::{GET_RESERVES_SIG, WETH_ADDRESS};
use crate::tx_utils::huff_sando_interface::common::weth_encoder::WethEncoder;
use crate::{managers::block_manager::BlockInfo, simulator::setup_block_state};

/// finds if sandwich is profitable + salmonella free
fn create_recipe(
    meats: Vec<Transaction>,
    optimal_in: U256,
    sandwich_start_balance: U256,
    target_pool: Pool,
    next_block: &BlockInfo,
    shared_backend: SharedBackend,
    searcher: Address,
    sando_address: Address,
) -> Result<()> {
    // setup evm simulation
    let mut fork_db = CacheDB::new(shared_backend);
    let mut evm = EVM::new();
    evm.database(fork_db);
    setup_block_state(&mut evm, &next_block);

    // *´:°•.°+.*•´.*:˚.°*.˚•´.°:°•.°•.*•´.*:˚.°*.˚•´.°:°•.°+.*•´.*:*/
    // *                    FRONTRUN TRANSACTION                    */
    // *.•°:°.´+˚.*°.˚:*.´•*.+°.•°:´*.´•*.•°.•°:°.´:•˚°.*°.˚:*.´+°.•*/
    //
    // encode frontrun_in before passing to sandwich contract
    let frontrun_in = WethEncoder::encode(optimal_in);

    // caluclate frontrun_out using encoded frontrun_in
    let frontrun_out = match target_pool {
        UniswapV2(_) => {
            let target_pool = target_pool.address();
            evm.env.tx.gas_price = next_block.base_fee_per_gas.into();
            evm.env.tx.gas_limit = 700000;
            evm.env.tx.value = rU256::ZERO;
            let amount_out = get_amount_out_evm(frontrun_in, target_pool, &mut evm)?;
            tx_builder::v2::decode_intermediary(amount_out, true, token_out)
        }
        UniswapV3(_) => U256::zero(),
    };

    // create tx.data and tx.value for frontrun_in
    let (frontrun_data, frontrun_value) = match target_pool {
        UniswapV2(_) => sandwich_maker.v2.create_payload_weth_is_input(
            frontrun_in,
            frontrun_out,
            ingredients.intermediary_token,
            ingredients.target_pool,
        ),
        UniswapV3(_) => v3_create_frontrun_payload(
            target_pool.address(),
            frontrun_in.as_u128().into(),
            ingredients.startend_token,
            ingredients.intermediary_token,
        ),
    };

    // setup evm for frontrun transaction
    evm.env.tx.caller = searcher.0.into();
    evm.env.tx.transact_to = TransactTo::Call(sando_address.0.into());
    evm.env.tx.data = frontrun_data.clone().into();
    evm.env.tx.value = frontrun_value.into();
    evm.env.tx.gas_limit = 700000;
    evm.env.tx.gas_price = next_block.base_fee_per_gas.into();
    evm.env.tx.access_list = Vec::default();

    // get access list
    let mut access_list_inspector = AccessListTracer::new(
        Default::default(),
        searcher,
        sando_address,
        get_precompiles_for(evm.env.cfg.spec_id),
    );
    evm.inspect_ref(&mut access_list_inspector)
        .map_err(|e| anyhow!("[EVM ERROR] sando frontrun: {:?}", (e)))?;
    let frontrun_access_list = access_list_inspector.access_list();
    evm.env.tx.access_list = frontrun_access_list;

    // run again but now with access list (so that we get accurate gas used)
    // run with a salmonella inspector to flag `suspicious` opcodes
    let mut salmonella_inspector = SalmonellaInspectoooor::new();
    let frontrun_result = match evm.inspect_commit(&mut salmonella_inspector) {
        Ok(result) => result,
        Err(e) => return Err(anyhow!("[EVM ERROR] sando frontrun: {:?}", e)),
    };
    match frontrun_result {
        ExecutionResult::Success { .. } => { /* continue operation */ }
        ExecutionResult::Revert { output, .. } => {
            return Err(anyhow!("[REVERT] sando frontrun: {:?}", output));
        }
        ExecutionResult::Halt { reason, .. } => {
            return Err(anyhow!("[HALT] sando frontrun: {:?}", reason));
        }
    };
    match salmonella_inspector.is_sando_safu() {
        IsSandoSafu::Safu => { /* continue operation */ }
        IsSandoSafu::NotSafu(not_safu_opcodes) => {
            return Err(anyhow!(
                "[huffsando: FrontrunNotSafu] {:?}",
                not_safu_opcodes
            ))
        }
    }

    let frontrun_gas_used = frontrun_result.gas_used();

    // *´:°•.°+.*•´.*:˚.°*.˚•´.°:°•.°•.*•´.*:˚.°*.˚•´.°:°•.°+.*•´.*:*/
    // *                     MEAT TRANSACTION/s                     */
    // *.•°:°.´+˚.*°.˚:*.´•*.+°.•°:´*.´•*.•°.•°:°.´:•˚°.*°.˚:*.´+°.•*/
    let mut is_meat_good = Vec::new();
    for meat in meats.iter() {
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
        // remove reverted meats because mempool tx/s gas costs are accounted for by fb
        let res = match evm.transact_commit() {
            Ok(result) => result,
            Err(e) => return Err(anyhow!("[huffsando: EVM ERROR] {:?}", e)),
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
    let balance = get_balance_of_evm(token_in, sando_address, next_block, &mut evm)?;
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
    evm.env.tx.transact_to = TransactTo::Call(sando_address.0.into());
    evm.env.tx.data = backrun_data.clone().into();
    evm.env.tx.gas_limit = 700000;
    evm.env.tx.gas_price = next_block.base_fee_per_gas.into();
    evm.env.tx.value = backrun_value.into();

    // create access list
    let mut access_list_inspector = AccessListTracer::new(
        Default::default(),
        searcher,
        sando_address,
        get_precompiles_for(evm.env.cfg.spec_id),
    );
    evm.inspect_ref(&mut access_list_inspector)
        .map_err(|e| anyhow!("[huffsando: EVM ERROR] sando frontrun: {:?}", e))
        .unwrap();
    let backrun_access_list = access_list_inspector.access_list();
    evm.env.tx.access_list = backrun_access_list;

    // run again but now with access list (so that we get accurate gas used)
    // run with a salmonella inspector to flag `suspicious` opcodes
    let mut salmonella_inspector = SalmonellaInspectoooor::new();
    let backrun_result = match evm.inspect_commit(&mut salmonella_inspector) {
        Ok(result) => result,
        Err(e) => return Err(anyhow!("[huffsando: EVM ERROR] sando backrun: {:?}", e)),
    };
    match backrun_result {
        ExecutionResult::Success { .. } => { /* continue */ }
        ExecutionResult::Revert { output, .. } => {
            return Err(anyhow!("[huffsando: REVERT] sando backrun: {:?}", output));
        }
        ExecutionResult::Halt { reason, .. } => {
            return Err(anyhow!("[huffsando: HALT] sando backrun: {:?}", reason))
        }
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
        sando_address,
        next_block,
        &mut evm,
    )?;
    let revenue = post_sandwich_balance
        .checked_sub(sandwich_start_balance)
        .unwrap_or_default();

    // filter only passing meat txs
    let good_meats_only = meats
        .iter()
        .zip(is_meat_good.iter())
        .filter(|&(_, &b)| b)
        .map(|(s, _)| s.to_owned())
        .collect();

    //Ok(OptimalRecipe::new(
    //    frontrun_data.into(),
    //    frontrun_value,
    //    frontrun_gas_used,
    //    convert_access_list(frontrun_access_list),
    //    backrun_data.into(),
    //    backrun_value,
    //    backrun_gas_used,
    //    convert_access_list(backrun_access_list),
    //    good_meats_only,
    //    revenue,
    //    ingredients.target_pool,
    //    ingredients.state_diffs.clone(),
    //))
    Ok(())
}

// Find amount out from an amount in using the k=xy formula
// note: assuming fee is set to 3% for all pools (not case irl)
//
// Arguments:
// * `amount_in`: amount of token in
// * `target_pool`: address of pool
// * `token_in`: address of token in
// * `token_out`: address of token out
// * `evm`: mutable reference to evm used for query
//
// Returns:
// Ok(U256): amount out
// Err(SimulationError): if error during caluclation
pub fn get_amount_out_evm(
    amount_in: U256,
    target_pool: Pool,
    is_frontrun: bool,
    evm: &mut EVM<CacheDB<SharedBackend>>,
) -> Result<U256> {
    // get reserves
    evm.env.tx.transact_to = TransactTo::Call(target_pool.address().0.into());
    evm.env.tx.caller = (*WETH_ADDRESS).0.into(); // spoof weth address for its ether
    evm.env.tx.value = rU256::ZERO;
    evm.env.tx.data = (*GET_RESERVES_SIG).0; // getReserves()
    let result = match evm.transact_ref() {
        Ok(result) => result.result,
        Err(e) => {
            return Err(anyhow!(
                "[huffsando: EVM ERROR, get_amount_out_evm] {:?}",
                e
            ))
        }
    };
    let output: Bytes = match result {
        ExecutionResult::Success { output, .. } => match output {
            Output::Call(o) => o.into(),
            Output::Create(o, _) => o.into(),
        },
        ExecutionResult::Revert { output, .. } => {
            return Err(anyhow!(
                "[huffsando: EVM REVERTED, get_amount_out_evm] {:?}",
                output
            ))
        }
        ExecutionResult::Halt { reason, .. } => {
            return Err(anyhow!(
                "[huffsando: EVM HALT, get_amount_out_evm] {:?}",
                reason
            ))
        }
    };

    let tokens = abi::decode(
        &vec![
            ParamType::Uint(128),
            ParamType::Uint(128),
            ParamType::Uint(32),
        ],
        &output,
    )
    .unwrap();

    let reserves_0 = tokens[0].clone().into_uint().unwrap();
    let reserves_1 = tokens[1].clone().into_uint().unwrap();

    let other_token = [target_pool.token_a, target_pool.token_b]
        .into_iter()
        .find(|&t| t != *WETH_ADDRESS)
        .unwrap();

    let (input_token, output_token) = if is_frontrun {
        // if frontrun we trade WETH -> TOKEN
        (*WETH_ADDRESS, other_token)
    } else {
        // if backrun we trade TOKEN -> WETH
        (other_token, *WETH_ADDRESS)
    };

    let (reserve_in, reserve_out) = match token_in < token_out {
        true => (reserves_0, reserves_1),
        false => (reserves_1, reserves_0),
    };

    let a_in_with_fee: U256 = amount_in * 997;
    let numerator: U256 = a_in_with_fee * reserve_out;
    let denominator: U256 = reserve_in * 1000 + a_in_with_fee;
    let amount_out: U256 = numerator.checked_div(denominator).unwrap_or(U256::zero());

    Ok(amount_out)
}
