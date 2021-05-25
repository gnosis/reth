use async_trait::async_trait;
use grpc_interfaces::txpool::{txpool_control_client::TxpoolControlClient, AccountInfoRequest};
use interfaces::world_state::{AccountInfo, WorldState};
use reth_core::BlockId;
use tokio::sync::Mutex;
use tonic::transport::Channel;

pub struct GrpcWorldState {
    client: Mutex<TxpoolControlClient<Channel>>,
}

impl GrpcWorldState {
    pub async fn new(address: String) -> GrpcWorldState {
        let client = Mutex::new(TxpoolControlClient::connect(address).await.unwrap());
        GrpcWorldState { client }
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
