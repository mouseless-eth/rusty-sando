use anyhow::{anyhow, Result};
use colored::Colorize;
use ethers::{
    providers::Middleware,
    signers::{LocalWallet, Signer},
    types::{Address, BlockNumber, Filter, U256, U64},
};
use log::info;
use std::sync::Arc;

use crate::{
    abi::Erc20,
    constants::{ERC20_TRANSFER_EVENT_SIG, WETH_ADDRESS},
    startup_info_log,
};

pub struct SandoStateManager {
    sando_contract: Address,
    sando_inception_block: U64,
    searcher_signer: LocalWallet,
    weth_inventory: U256,
    token_dust: Vec<Address>,
}

impl SandoStateManager {
    pub fn new(
        sando_contract: Address,
        searcher_signer: LocalWallet,
        sando_inception_block: U64,
    ) -> Self {
        Self {
            sando_contract,
            sando_inception_block,
            searcher_signer,
            weth_inventory: Default::default(),
            token_dust: Default::default(),
        }
    }

    pub async fn setup<M: Middleware + 'static>(&mut self, provider: Arc<M>) -> Result<()> {
        // find weth inventory
        let weth = Erc20::new(*WETH_ADDRESS, provider.clone());
        let weth_balance = weth.balance_of(self.sando_contract).call().await?;
        startup_info_log!("weth inventory   : {}", weth_balance);
        self.weth_inventory = weth_balance;

        // find weth dust
        let step = 10000;

        let latest_block = provider
            .get_block(BlockNumber::Latest)
            .await
            .map_err(|_| anyhow!("Failed to get latest block"))?
            .ok_or(anyhow!("Failed to get latest block"))?
            .number
            .ok_or(anyhow!("Field block number does not exist on latest block"))?
            .as_u64();

        let mut token_dust = vec![];

        let start_block = self.sando_inception_block.as_u64();

        // for each block within the range, get all transfer events asynchronously
        for from_block in (start_block..=latest_block).step_by(step) {
            let to_block = from_block + step as u64;

            // check for all incoming and outgoing txs within step range
            let transfer_logs = provider
                .get_logs(
                    &Filter::new()
                        .topic0(*ERC20_TRANSFER_EVENT_SIG)
                        .topic1(self.sando_contract)
                        .from_block(BlockNumber::Number(U64([from_block])))
                        .to_block(BlockNumber::Number(U64([to_block]))),
                )
                .await?;

            for log in transfer_logs {
                token_dust.push(log.address);
            }
        }

        startup_info_log!("token dust found : {}", token_dust.len());
        self.token_dust = token_dust;

        Ok(())
    }

    pub fn get_sando_address(&self) -> Address {
        self.sando_contract
    }

    pub fn get_searcher_address(&self) -> Address {
        self.searcher_signer.address()
    }

    pub fn get_searcher_signer(&self) -> &LocalWallet {
        &self.searcher_signer
    }

    pub fn get_weth_inventory(&self) -> U256 {
        self.weth_inventory
    }
}
