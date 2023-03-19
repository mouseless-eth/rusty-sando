use ethers::prelude::*;
use std::collections::HashMap;

use crate::prelude::is_sando_safu::OpCode;
use crate::prelude::{sandwich_types::OptimalRecipe, PoolVariant};
use crate::utils;

// Alerts discord channel, via webhook, that a bundle has been sent
pub async fn alert_bundle<'a>(
    bundle_hash: H256,
    target_block: U64,
    is_bundle_included: bool,
    recipe: &OptimalRecipe,
    profit: U256,
) {
    let bundle_hash = format!("{:?}", bundle_hash);
    let meat_hashes = format!("{:?}", recipe.print_meats_new_line());
    let response_status = if is_bundle_included {
        String::from("🟩")
    } else {
        String::from("🟥")
    };

    let webhook = match recipe.target_pool.pool_variant {
        PoolVariant::UniswapV2 => utils::dotenv::get_v2_alert_webhook(),
        PoolVariant::UniswapV3 => utils::dotenv::get_v3_alert_webhook(),
    };

    let msg = format!(
        "
        {}
        bundle hash: {}
        target block: {}
        meats: {}
        ----------
        ----------
        fr gas: {}
        br gas: {}
        ----------
        ----------
        revenue: {}
        profit: {}
        ",
        response_status.repeat(8),
        bundle_hash,
        target_block,
        meat_hashes,
        recipe.frontrun_gas_used,
        recipe.backrun_gas_used,
        recipe.revenue.as_u128(),
        profit
    );

    let max_length = 1900.min(msg.len());
    let message = msg[..max_length].to_string();
    let mut bundle_notif = HashMap::new();
    bundle_notif.insert("content", message.to_string());

    let client = reqwest::Client::new();

    tokio::spawn(async move {
        let res = client.post(webhook).json(&bundle_notif).send().await;
        match res {
            Ok(_) => {}
            Err(err) => {
                log::error!("Could not send alert to discord, err: {}", err);
                log::error!("Message: {}", message);
            }
        }
    })
    .await
    .unwrap();
}

/// Alerts discord channel, via webhook about found a poison token
pub async fn alert_poison(malicious_token: Address, malicious_opcodes: Vec<OpCode>) {
    let msg = format!(
        "
        poison detected:
        token: {:?}
        opcodes: {:?}
        ",
        malicious_token, malicious_opcodes
    );

    let max_length = 1900.min(msg.len());
    let message = msg[..max_length].to_string();
    let mut map = HashMap::new();
    map.insert("content", message.to_string());

    let webhook = utils::dotenv::poison_alert_webhook();
    let client = reqwest::Client::new();

    tokio::spawn(async move {
        let res = client.post(webhook.to_string()).json(&map).send().await;

        match res {
            Ok(_) => {}
            Err(err) => {
                log::error!("Could not send alert to discord, err: {}", err);
                log::error!("Message: {}", message);
            }
        }
    })
    .await
    .unwrap();
}
