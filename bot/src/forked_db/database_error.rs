use futures::channel::mpsc::{SendError, TrySendError};
use std::sync::{mpsc::RecvError, Arc};

// Errors that can happen when working with [`revm::Database`]
#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    #[error("Failed to fetch AccountInfo {0:?}")]
    MissingAccount(revm::primitives::B160),
    #[error("Could should already be loaded: {0:?}")]
    MissingCode(revm::primitives::B256),
    #[error(transparent)]
    Recv(#[from] RecvError),
    #[error(transparent)]
    Send(#[from] SendError),
    #[error("{0}")]
    Message(String),
    #[error("Failed to get account for {0:?}: {0:?}")]
    GetAccount(revm::primitives::Address, Arc<eyre::Error>),
    #[error("Failed to get storage for {0:?} at {1:?}: {2:?}")]
    GetStorage(
        revm::primitives::Address,
        revm::primitives::U256,
        Arc<eyre::Error>,
    ),
    #[error("Failed to get block hash for {0}: {1:?}")]
    GetBlockHash(revm::primitives::U256, Arc<eyre::Error>),
}

impl<T> From<TrySendError<T>> for DatabaseError {
    fn from(err: TrySendError<T>) -> Self {
        err.into_send_error().into()
    }
}

impl DatabaseError {
    // Create a new error with a message
    pub fn msg(msg: impl Into<String>) -> Self {
        DatabaseError::Message(msg.into())
    }
}

// Result alias with `DatabaseError` as error
pub type DatabaseResult<T> = Result<T, DatabaseError>;
