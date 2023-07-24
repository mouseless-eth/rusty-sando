use anyhow::{anyhow, Result};
use ethers::{
    signers::{LocalWallet, Signer},
    types::{
        transaction::{
            eip2718::TypedTransaction,
            eip2930::{AccessList, AccessListItem},
        },
        BigEndianHash, Bytes, Eip1559TransactionRequest, H256,
    },
};
use foundry_evm::{
    executor::{rU256, B160},
    utils::{b160_to_h160, h160_to_b160, ru256_to_u256, u256_to_ru256},
};

/// Sign eip1559 transactions
pub async fn sign_eip1559(
    tx: Eip1559TransactionRequest,
    signer_wallet: &LocalWallet,
) -> Result<Bytes> {
    let tx_typed = TypedTransaction::Eip1559(tx);
    let signed_frontrun_tx_sig = signer_wallet
        .sign_transaction(&tx_typed)
        .await
        .map_err(|e| anyhow!("Failed to sign eip1559 request: {:?}", e))?;

    Ok(tx_typed.rlp_signed(&signed_frontrun_tx_sig))
}

/// convert revm access list to ethers access list
pub fn access_list_to_ethers(access_list: Vec<(B160, Vec<rU256>)>) -> AccessList {
    AccessList::from(
        access_list
            .into_iter()
            .map(|(address, slots)| AccessListItem {
                address: b160_to_h160(address),
                storage_keys: slots
                    .into_iter()
                    .map(|y| H256::from_uint(&ru256_to_u256(y)))
                    .collect(),
            })
            .collect::<Vec<AccessListItem>>(),
    )
}

/// convert ethers access list to revm access list
pub fn access_list_to_revm(access_list: AccessList) -> Vec<(B160, Vec<rU256>)> {
    access_list
        .0
        .into_iter()
        .map(|x| {
            (
                h160_to_b160(x.address),
                x.storage_keys
                    .into_iter()
                    .map(|y| u256_to_ru256(y.0.into()))
                    .collect(),
            )
        })
        .collect()
}

//
// -- Logging Macros --
//
#[macro_export]
macro_rules! log_info_cyan {
    ($($arg:tt)*) => {
        info!("{}", format_args!($($arg)*).to_string().cyan());
    };
}

#[macro_export]
macro_rules! log_not_sandwichable {
    ($($arg:tt)*) => {
        info!("{}", format_args!($($arg)*).to_string().yellow())
    };
}

#[macro_export]
macro_rules! log_opportunity {
    ($meats:expr, $optimal_input:expr, $revenue:expr) => {{
        info!("\n{}", "[OPPORTUNITY DETECTED]".green().on_black().bold());
        info!(
            "{}",
            format!("meats: {}", $meats.to_string().green().on_black()).bold()
        );
        info!(
            "{}",
            format!(
                "optimal_input: {} wETH",
                $optimal_input.to_string().green().on_black()
            )
            .bold()
        );
        info!(
            "{}",
            format!(
                "revenue      : {} wETH",
                $revenue.to_string().green().on_black()
            )
            .bold()
        );
    }};
}

#[macro_export]
macro_rules! startup_info_log {
    ($($arg:tt)*) => {
        info!("{}", format_args!($($arg)*).to_string().on_black().yellow().bold());
    };
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        error!("{}", format_args!($($arg)*).to_string().red());
    };
}

#[macro_export]
macro_rules! log_new_block_info {
    ($new_block:expr) => {
        log::info!(
            "{}",
            format!(
                "\nFound New Block\nLatest Block: (number:{:?}, timestamp:{:?}, basefee:{:?})",
                $new_block.number, $new_block.timestamp, $new_block.base_fee_per_gas,
            )
            .bright_purple()
            .on_black()
        );
    };
}
