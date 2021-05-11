// Copyright 2020-2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use reth_core::{Address, BlockId, H256, Transaction, U256};
use rlp::Rlp;

/// Trait that allows getting blocks data
#[async_trait]
pub trait WorldState: Send + Sync {
    /// get account info (balance,nonce) from newest world state
    async fn account_info(&self, block_id: BlockId, account: &Address) -> Option<AccountInfo>;
}

#[derive(Copy, Clone)]
pub struct AccountInfo {
    pub balance: U256,
    pub nonce: u64,
}

impl AccountInfo {
    pub fn new(balance: U256, nonce: u64) -> AccountInfo {
        AccountInfo { balance, nonce }
    }
}

// When new block is inserted we or when reorg happens this structure contains everything we need to remove
pub struct BlockUpdate {
    pub old_hash: H256,
    pub new_hash: H256,
    pub reverted_tx: Vec<Vec<u8>>,
    pub reverted_accounts: Vec<(Address,AccountInfo)>,
    pub appliend_accounts: Vec<(Address,AccountInfo)>,
}

#[cfg(any(test, feature = "test_only"))]
pub mod helper {
    use parking_lot::RwLock;
    use reth_core::transaction::transaction::{fake_sign, DUMMY_AUTHOR, DUMMY_AUTHOR1};
    use std::{collections::HashMap, sync::Arc};

    use super::*;

    pub struct WorldStateTest {
        pub accounts_by_block: RwLock<HashMap<BlockId, HashMap<Address, AccountInfo>>>,
    }

    impl WorldStateTest {
        pub fn new_empty() -> Arc<dyn WorldState> {
            Arc::new(WorldStateTest {
                accounts_by_block: RwLock::new(HashMap::new()),
            })
        }

        pub fn new_dummy() -> Arc<dyn WorldState> {
            let wst = Arc::new(WorldStateTest {
                accounts_by_block: RwLock::new(HashMap::new()),
            });
            wst.insert(
                BlockId::Latest,
                DUMMY_AUTHOR.0,
                AccountInfo::new(1_000.into(), 0),
            );
            wst.insert(
                BlockId::Latest,
                DUMMY_AUTHOR1.0,
                AccountInfo::new(1_000.into(), 0),
            );
            wst
        }

        pub fn insert(&self, id: BlockId, account: Address, info: AccountInfo) {
            self.accounts_by_block
                .write()
                .entry(id)
                .or_default()
                .insert(account, info);
        }
    }

    #[async_trait]
    impl WorldState for WorldStateTest {
        async fn account_info(&self, block_id: BlockId, account: &Address) -> Option<AccountInfo> {
            self.accounts_by_block
                .read()
                .get(&block_id)?
                .get(&account)
                .cloned()
        }
    }
}
