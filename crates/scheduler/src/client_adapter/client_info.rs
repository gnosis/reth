use crate::common_types::*;
use ethereum_forkid::{ForkHash, ForkId};
use primitive_types::{H256, U256};
use std::str::FromStr;

pub struct ClientStatus {
    pub total_difficulty: U256,
    pub highest_block: (BlockNumber, H256),
    pub genesis_block_hash: H256,
    pub network_id: u64,
    pub fork: ForkId,
}

//2020-12-30 01:38:20 UTC protocol:64,network:1,diff:19749264164891984230159,
//best_block:0x59abe49245dc9acf9b27d8c80ef0602b84900a35f5305b42b0170bb998fbb1c8,
//genesis:0xd4e56740f876aef8c010b86a40d5f56745a118d0906a34e69aec8c0db1cb8fa3
//2020-12-30 01:38:20 UTC fork chain:ForkId { hash: ForkHash(3760843153), next: 0 }

/*
Task pushed: InsertPeer(HandshakeInfo { peer_id: 22, eth_protocol_version: 64,
    genesis_hash: 0x29a742ba74d89fc24d2e48aeb1030fcb7276f4b2421488c05f2cc0f39aa1a273,
    network_id: 3125659152, latest_hash: 0x6b4c4662949520de51576fdc210de2afc2195eac6123765f20daee5f4a3188d5,
    total_difficulty: Some(22354868936099755350), fork_id: Some(ForkId { hash: ForkHash(606858910), next: 0 }),
     snapshot: None })*/

pub trait Client: Send + Sync {
    //pub fn get
    fn status(&self) -> ClientStatus {
        //TODO dummy values
        ClientStatus {
            total_difficulty: U256::from_str("321371050299138").unwrap(),
            highest_block: (
                9792,
                H256::from_str("e5e55fc298c68782ecb71b95f6202362be01b9c7706d9732e2083a82939bb849")
                    .unwrap(),
            ),
            genesis_block_hash: H256::from_str(
                "d4e56740f876aef8c010b86a40d5f56745a118d0906a34e69aec8c0db1cb8fa3",
            )
            .unwrap(),
            network_id: 1,
            fork: ForkId {
                hash: ForkHash(4234472452),
                next: 1150000,
            },
        }
    }
}

pub struct SnapshotManifestStatus {
    pub block_number: BlockNumber,
    pub hash: H256,
}

impl SnapshotManifestStatus {
    pub fn not_exist(&self) -> bool {
        self.block_number == 0
    }
}

pub trait Snapshot: Send + Sync {
    fn manifest_status(&self) -> SnapshotManifestStatus {
        SnapshotManifestStatus {
            block_number: 0,
            hash: H256::zero(),
        }
    }
}
