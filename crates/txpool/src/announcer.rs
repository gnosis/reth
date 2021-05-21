// Copyright 2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0


use crate::Error;
use async_trait::async_trait;
use reth_core::Transaction;
use std::sync::Arc;

#[async_trait]
pub trait Announcer: Send + Sync {
    async fn inserted(&self, tx: Arc<Transaction>);

    async fn removed(&self, tx: Arc<Transaction>, error: Error);
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
    }
}
