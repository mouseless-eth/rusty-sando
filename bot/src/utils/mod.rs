use std::sync::Arc;

use ethers::{prelude::*, types::transaction::eip2718::TypedTransaction};

pub mod alert;
pub mod constants;
pub mod contracts;
pub mod dotenv;
pub mod encode_packed;
pub mod state_diff;
pub mod testhelper;
pub mod tx_builder;

pub use encode_packed::*;

// ========= GENERAL HELPERS

/// Calculate the next block base fee
// based on math provided here: https://ethereum.stackexchange.com/questions/107173/how-is-the-base-fee-per-gas-computed-for-a-new-block
pub fn calculate_next_block_base_fee(block: Block<TxHash>) -> U256 {
    // Get the block base fee per gas
    let current_base_fee_per_gas = block.base_fee_per_gas.unwrap_or_default();

    // Get the mount of gas used in the block
    let current_gas_used = block.gas_used;

    let current_gas_target = block.gas_limit / 2;

    if current_gas_used == current_gas_target {
        current_base_fee_per_gas
    } else if current_gas_used > current_gas_target {
        let gas_used_delta = current_gas_used - current_gas_target;
        let base_fee_per_gas_delta =
            current_base_fee_per_gas * gas_used_delta / current_gas_target / 8;

        return current_base_fee_per_gas + base_fee_per_gas_delta;
    } else {
        let gas_used_delta = current_gas_target - current_gas_used;
        let base_fee_per_gas_delta =
            current_base_fee_per_gas * gas_used_delta / current_gas_target / 8;

        return current_base_fee_per_gas - base_fee_per_gas_delta;
    }
}

/// Small helper function to convert [U256] into [H256].
pub fn u256_to_h256_be(u: U256) -> H256 {
    let mut h = H256::default();
    u.to_big_endian(h.as_mut());
    h
}

/// Sign eip1559 transactions
pub async fn sign_eip1559(
    tx: Eip1559TransactionRequest,
    signer_wallet: &LocalWallet,
) -> Result<Bytes, WalletError> {
    let tx_typed = TypedTransaction::Eip1559(tx);
    let signed_frontrun_tx_sig = match signer_wallet.sign_transaction(&tx_typed).await {
        Ok(s) => s,
        Err(e) => return Err(e),
    };

    Ok(tx_typed.rlp_signed(&signed_frontrun_tx_sig))
}

/// Create Websocket Client
pub async fn create_websocket_client() -> eyre::Result<Arc<Provider<Ws>>> {
    let client = dotenv::get_ws_provider().await;
    Ok(Arc::new(client))
}

pub async fn get_nonce(
    client: &Arc<Provider<Ws>>,
    address: Address,
) -> Result<U256, ProviderError> {
    client.get_transaction_count(address, None).await
}
