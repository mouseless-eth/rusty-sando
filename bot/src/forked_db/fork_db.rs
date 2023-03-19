use std::sync::mpsc::channel as oneshot_channel;

use futures::channel::mpsc::Sender;
use revm::{
    db::{CacheDB, DatabaseRef, EmptyDB},
    primitives::{
        Account, AccountInfo, Address as rAddress, Bytecode as rBytecode, HashMap as rHashMap,
        B160, B256, KECCAK_EMPTY, U256 as rU256,
    },
    Database, DatabaseCommit,
};

use super::{
    database_error::{DatabaseError, DatabaseResult},
    BackendFetchRequest,
};

#[derive(Clone, Debug)]
pub struct ForkDB {
    // used to make calls for missing data
    backend: Sender<BackendFetchRequest>,
    db: CacheDB<EmptyDB>,
}

impl ForkDB {
    pub fn new(backend: Sender<BackendFetchRequest>, db: CacheDB<EmptyDB>) -> Self {
        Self { backend, db }
    }

    fn do_get_basic(&self, address: rAddress) -> DatabaseResult<Option<AccountInfo>> {
        tokio::task::block_in_place(|| {
            let (sender, rx) = oneshot_channel();
            let req = BackendFetchRequest::Basic(address, sender);
            self.backend.clone().try_send(req)?;
            rx.recv()?.map(Some)
        })
    }

    fn do_get_storage(&self, address: rAddress, index: rU256) -> DatabaseResult<rU256> {
        tokio::task::block_in_place(|| {
            let (sender, rx) = oneshot_channel();
            let req = BackendFetchRequest::Storage(address, index, sender);
            self.backend.clone().try_send(req)?;
            rx.recv()?
        })
    }

    fn do_get_block_hash(&self, number: rU256) -> DatabaseResult<B256> {
        tokio::task::block_in_place(|| {
            let (sender, rx) = oneshot_channel();
            let req = BackendFetchRequest::BlockHash(number, sender);
            self.backend.clone().try_send(req)?;
            rx.recv()?
        })
    }
}

impl Database for ForkDB {
    type Error = DatabaseError;

    fn basic(&mut self, address: B160) -> Result<Option<AccountInfo>, Self::Error> {
        // found locally, return it
        match self.db.accounts.get(&address) {
            // basic info is already in db
            Some(account) => Ok(Some(account.info.clone())),
            None => {
                // basic info is not in db, make rpc call to fetch it
                let info = match self.do_get_basic(address) {
                    Ok(i) => i,
                    Err(e) => return Err(e),
                };

                // keep record of fetched acc basic info
                if info.is_some() {
                    self.db.insert_account_info(address, info.clone().unwrap());
                }

                Ok(info)
            }
        }
    }

    fn storage(&mut self, address: B160, index: rU256) -> Result<rU256, Self::Error> {
        // found locally, return it
        if let Some(account) = self.db.accounts.get(&address) {
            if let Some(entry) = account.storage.get(&index) {
                // account storage exists at slot
                return Ok(*entry);
            }
        }

        // get account info
        let acc_info = match self.do_get_basic(address) {
            Ok(a) => a,
            Err(e) => return Err(e),
        };

        if let Some(a) = acc_info {
            self.db.insert_account_info(address, a);
        }

        // make rpc call to fetch storage
        let storage_val = match self.do_get_storage(address, index) {
            Ok(i) => i,
            Err(e) => return Err(e),
        };

        // keep record of fetched storage (can unwrap safely as cacheDB always returns true)
        self.db
            .insert_account_storage(address, index, storage_val)
            .unwrap();

        Ok(storage_val)
    }

    fn block_hash(&mut self, number: rU256) -> Result<B256, Self::Error> {
        match self.db.block_hashes.get(&number) {
            // found locally, return it
            Some(hash) => Ok(*hash),
            None => {
                // rpc call to fetch block hash
                let block_hash = match self.do_get_block_hash(number) {
                    Ok(i) => i,
                    Err(e) => return Err(e),
                };

                // insert fetched block hash into db
                self.db.block_hashes.insert(number, block_hash);

                Ok(block_hash)
            }
        }
    }

    /// Get account code by its hash
    fn code_by_hash(&mut self, code_hash: B256) -> Result<rBytecode, Self::Error> {
        match self.db.code_by_hash(code_hash) {
            Ok(code) => Ok(code),
            Err(_) => {
                // should alr be loaded
                Err(DatabaseError::MissingCode(code_hash))
            }
        }
    }
}

impl DatabaseRef for ForkDB {
    type Error = DatabaseError;

    fn basic(&self, address: B160) -> Result<Option<AccountInfo>, Self::Error> {
        match self.db.accounts.get(&address) {
            Some(account) => Ok(Some(account.info.clone())),
            None => {
                // state doesnt exist so fetch it
                self.do_get_basic(address)
            }
        }
    }

    fn storage(&self, address: B160, index: rU256) -> Result<rU256, Self::Error> {
        match self.db.accounts.get(&address) {
            Some(account) => match account.storage.get(&index) {
                Some(entry) => Ok(*entry),
                None => {
                    // state doesnt exist so fetch it
                    match self.do_get_storage(address, index) {
                        Ok(storage) => Ok(storage),
                        Err(e) => Err(e),
                    }
                }
            },
            None => {
                // state doesnt exist so fetch it
                match self.do_get_storage(address, index) {
                    Ok(storage) => Ok(storage),
                    Err(e) => Err(e),
                }
            }
        }
    }

    fn block_hash(&self, number: rU256) -> Result<B256, Self::Error> {
        if number > rU256::from(u64::MAX) {
            return Ok(KECCAK_EMPTY);
        }
        self.do_get_block_hash(number)
    }

    /// Get account code by its hash
    fn code_by_hash(&self, code_hash: B256) -> Result<revm::primitives::Bytecode, Self::Error> {
        match self.db.code_by_hash(code_hash) {
            Ok(code) => Ok(code),
            Err(_) => {
                // should alr be loaded
                Err(DatabaseError::MissingCode(code_hash))
            }
        }
    }
}

impl DatabaseCommit for ForkDB {
    fn commit(&mut self, changes: rHashMap<B160, Account>) {
        self.db.commit(changes)
    }
}
