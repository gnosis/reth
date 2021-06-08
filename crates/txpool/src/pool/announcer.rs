// Copyright 2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

use crate::Error;
use async_trait::async_trait;
use parking_lot::RwLock;
use reth_core::Transaction;
use std::sync::Arc;
#[async_trait]
pub trait Announcer: Send + Sync {
    async fn inserted(&self, tx: Arc<Transaction>);

    async fn reinserted(&self, tx: Arc<Transaction>);

    async fn removed(&self, tx: Arc<Transaction>, error: Error);
}

pub struct MultiAnnouncer {
    annons: RwLock<Vec<Arc<dyn Announcer>>>,
}

impl MultiAnnouncer {
    pub fn new() -> Self {
        Self {
            annons: RwLock::new(Vec::new()),
        }
    }

    pub fn add(&self, annon: Arc<dyn Announcer>) {
        self.annons.write().push(annon);
    }
}

#[async_trait]
impl Announcer for MultiAnnouncer {
    async fn inserted(&self, tx: Arc<Transaction>) {
        let annons = self.annons.read().clone();
        for annon in annons.iter() {
            //let annon = **annon;
            let tx = tx.clone();
            &annon.inserted(tx).await;
        }
    }

    async fn reinserted(&self, tx: Arc<Transaction>) {
        let annons = self.annons.read().clone();
        for annon in annons.iter() {
            //let annon = **annon;
            let tx = tx.clone();
            &annon.inserted(tx.clone()).await;
        }
    }

    async fn removed(&self, tx: Arc<Transaction>, error: Error) {
        let annons = self.annons.read().clone();
        for annon in annons.iter() {
            &annon.removed(tx.clone(), error.clone()).await;
        }
    }
}

#[cfg(any(test, feature = "test_only"))]
pub mod test {
    use super::*;
    pub struct AnnouncerTest {}

    impl AnnouncerTest {
        pub fn new() -> Self {
            Self {}
        }
    }

    #[async_trait]
    impl Announcer for AnnouncerTest {
        async fn inserted(&self, _tx: Arc<Transaction>) {}

        async fn removed(&self, _tx: Arc<Transaction>, _error: Error) {}

        async fn reinserted(&self, _tx: Arc<Transaction>) {}
    }
}
