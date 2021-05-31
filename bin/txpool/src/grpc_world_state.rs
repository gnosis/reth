use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use grpc_interfaces::txpool::{
    block_diff::Diff, txpool_control_client::TxpoolControlClient, AccountInfoRequest,
    BlockStreamRequest,
};
use interfaces::{
    txpool::TransactionPool,
    world_state::{AccountInfo, BlockUpdate, WorldState},
};
use reth_core::BlockId;
use tokio::sync::Mutex;
use tonic::{transport::Channel, Request};
use txpool::Pool;

pub struct GrpcWorldState {
    client: Mutex<TxpoolControlClient<Channel>>,
}

impl GrpcWorldState {
    pub async fn new(address: String) -> GrpcWorldState {
        let client = Mutex::new(TxpoolControlClient::connect(address).await.unwrap());
        GrpcWorldState { client }
    }

    pub async fn run(&self, pool: Arc<Pool>) -> Result<()> {
        let mut stream = self
            .client
            .lock()
            .await
            .block_stream(Request::new(BlockStreamRequest {
                start_with: None, // start with latest
            }))
            .await?
            .into_inner();

        while let Some(msg) = stream.message().await? {
            let block_update = match msg.diff {
                Some(Diff::Applied(applied)) => BlockUpdate {
                    new_hash: applied.hash.unwrap().into(),
                    old_hash: applied.parent_hash.unwrap().into(),
                    base_fee: 0.into(),
                    reverted_tx: Vec::new(),
                    changed_accounts: applied
                        .changed_accounts
                        .into_iter()
                        .map(|t| {
                            (
                                t.address.unwrap().into(),
                                AccountInfo::new(t.balance.unwrap().into(), t.nonce),
                            )
                        })
                        .collect(),
                },
                Some(Diff::Reverted(reverted)) => BlockUpdate {
                    new_hash: reverted.new_hash.unwrap().into(),
                    old_hash: reverted.reverted_hash.unwrap().into(),
                    base_fee: 0.into(),
                    reverted_tx: reverted
                        .reverted_transactions
                        .into_iter()
                        .map(|t| t.to_vec())
                        .collect(),
                    changed_accounts: reverted
                        .reverted_accounts
                        .into_iter()
                        .map(|t| {
                            (
                                t.address.unwrap().into(),
                                AccountInfo::new(t.balance.unwrap().into(), t.nonce),
                            )
                        })
                        .collect(),
                },
                None => break, // TODO check what to do.
            };
            pool.block_update(&block_update).await;
        }
        Ok(())
    }
}

#[async_trait]
impl WorldState for GrpcWorldState {
    async fn account_info(
        &self,
        block_id: BlockId,
        account: reth_core::Address,
    ) -> Option<AccountInfo> {
        // grpc supports only block_id by hash
        let id = match block_id {
            BlockId::Latest | BlockId::Number(_) => unimplemented!(),
            BlockId::Hash(hash) => hash,
        };
        let response = self
            .client
            .lock()
            .await
            .account_info(AccountInfoRequest {
                block_hash: Some(id.into()),
                account: Some(account.into()),
            })
            .await
            .ok()?;

        Some(AccountInfo {
            balance: response.get_ref().balance.clone().unwrap().into(),
            nonce: response.get_ref().nonce,
        })
    }
}
