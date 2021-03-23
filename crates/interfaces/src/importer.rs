use core::{BlockNumber, H256, U256};
use std::str::FromStr;

use ethereum_forkid::{ForkHash, ForkId};

pub struct ImporterStatus {
    pub total_difficulty: U256,
    pub highest_block: (BlockNumber, H256),
    pub genesis_block_hash: H256,
    pub network_id: u64,
    pub fork: ForkId,
}

// TODO big TODO cleanup this after a proper trait is made. Leave this nasty hardcoded data for now.
pub trait Importer: Send + Sync {
    //pub fn get
    fn status(&self) -> ImporterStatus {
        //TODO dummy values
        ImporterStatus {
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
