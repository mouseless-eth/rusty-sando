use std::fmt;

use ethers::prelude::{AbiError, ContractError};
use ethers::providers::{Provider, ProviderError, Ws};
use ethers::signers::WalletError;
use ethers::types::H160;
use thiserror::Error;
use tokio::task::JoinError;

use crate::prelude::is_sando_safu::OpCode;
use crate::prelude::DatabaseError;

#[derive(Error, Debug)]
pub enum PairSyncError {
    #[error("Provider error")]
    ProviderError(#[from] ProviderError),
    #[error("Contract error")]
    ContractError(#[from] ContractError<Provider<Ws>>),
    #[error("ABI error")]
    ABIError(#[from] AbiError),
    #[error("Join error")]
    JoinError(#[from] JoinError),
    #[error("Pair for token_a/token_b does not exist in provided dexes")]
    PairDoesNotExistInDexes(H160, H160),
}

#[derive(Error, Debug)]
pub enum SendBundleError {
    #[error("Failed to sign transaction")]
    SigningError(#[from] WalletError),
    #[error("Max fee is less than next base fee")]
    MaxFeeLessThanNextBaseFee(),
    #[error("Negative miner tip")]
    NegativeMinerTip(),
    #[error("Failed to create bundle")]
    FailedToCreateBundle(),
    #[error("Failed to send bundle")]
    FailedToSendBundle(),
    #[error("Revenue does not cover frontrun gas fees")]
    FrontrunGasFeesNotCovered(),
}

#[derive(Debug)]
pub enum SimulationError {
    FrontrunEvmError(revm::primitives::EVMError<DatabaseError>),
    FrontrunHalted(revm::primitives::Halt),
    FrontrunReverted(revm::primitives::Bytes),
    FrontrunNotSafu(Vec<OpCode>),
    BackrunEvmError(revm::primitives::EVMError<DatabaseError>),
    BackrunHalted(revm::primitives::Halt),
    BackrunReverted(revm::primitives::Bytes),
    BackrunNotSafu(Vec<OpCode>),
    FailedToDecodeOutput(AbiError),
    EvmError(revm::primitives::EVMError<DatabaseError>),
    EvmHalted(revm::primitives::Halt),
    EvmReverted(revm::primitives::Bytes),
    AbiError(AbiError),
    ZeroOptimal(),
}

impl fmt::Display for SimulationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SimulationError::FrontrunEvmError(db_err) => {
                write!(f, "Fromrun ran into an EVM error : {:?}", db_err)
            }
            SimulationError::FrontrunHalted(halt_reason) => {
                write!(f, "Frontrun halted due to : {:?}", halt_reason)
            }
            SimulationError::FrontrunReverted(bytes) => {
                write!(f, "Frontrun reverted and returned : {}", hex::encode(bytes))
            }
            SimulationError::FrontrunNotSafu(sus) => {
                write!(f, "Frontrun not safu because of the opcodes : {:?}", sus)
            }
            SimulationError::BackrunEvmError(db_err) => {
                write!(f, "Backrun ran into an EVM error : {:?}", db_err)
            }
            SimulationError::BackrunHalted(halt_reason) => {
                write!(f, "Backrun halted due to : {:?}", halt_reason)
            }
            SimulationError::BackrunReverted(bytes) => {
                write!(f, "Backrun reverted and returned : {}", hex::encode(bytes))
            }
            SimulationError::BackrunNotSafu(sus) => {
                write!(f, "Backrun not safu because of the opcodes : {:?}", sus)
            }
            SimulationError::FailedToDecodeOutput(error_reason) => {
                write!(f, "Failed to decode output : {:?}", error_reason)
            }
            SimulationError::EvmError(db_err) => {
                write!(f, "Ran into an EVM error : {:?}", db_err)
            }
            SimulationError::EvmHalted(halt_reason) => {
                write!(f, "EVM halted due to : {:?}", halt_reason)
            }
            SimulationError::EvmReverted(bytes) => {
                write!(f, "EVM reverted and returned : {}", hex::encode(bytes))
            }
            SimulationError::AbiError(reason) => {
                write!(f, "Failed to decode ABI due to : {:?}", reason)
            }
            SimulationError::ZeroOptimal() => {
                write!(f, "No optimal sandwich found")
            }
        }
    }
}
