// Copyright 2020-2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

use crate::common_types::GetBlockHeaders;
use core::{BlockBody, BlockHeader, BlockId, BlockNumber, BlockReceipt, WireBlock, H256};
use interfaces::{
    blockchain::BlockchainReadOnly,
    importer::{Importer, ImporterInfo, ImporterStatus},
};
use std::collections::HashMap;

pub struct HeadersInMemory {
    headers: HashMap<BlockNumber, BlockHeader>,
}

impl HeadersInMemory {
    pub fn new() -> Self {
        HeadersInMemory {
            headers: HashMap::new(),
        }
    }
}

fn clone_option(from_header: Option<&BlockHeader>) -> Option<BlockHeader> {
    if let Some(header) = from_header {
        Some(header.clone())
    } else {
        None
    }
}

impl BlockchainReadOnly for HeadersInMemory {
    fn header(&self, number: BlockNumber) -> Option<BlockHeader> {
        clone_option(self.headers.get(&number))
    }

    fn header_list(&self, request: Vec<BlockId>) -> Vec<BlockHeader> {
        vec![] // TODO
    }

    fn header_request(
        &self,
        block_id: BlockId,
        max_header: u64,
        skip: u64,
        reverse: bool,
    ) -> Vec<BlockHeader> {
        vec![] // TODO
    }

    fn body(&self, hash: &H256) -> Option<BlockBody> {
        None
    }

    fn receipt(&self) -> Option<BlockReceipt> {
        None // TODO
    }

    fn best_header(&self) -> Option<BlockNumber> {
        self.headers.keys().max().cloned()
    }

    fn tx(&self) {
        unimplemented!()
    }
}

impl Importer for HeadersInMemory {
    fn import_block(&mut self, block: &WireBlock) {
        self.headers
            .insert(block.header.number, block.header.clone());
    }

    fn import_ancient_block(&self) {
        unimplemented!()
    }

    fn verificator_info(&self) -> &ImporterInfo {
        unimplemented!()
    }

    //fn status(&self) -> ImporterStatus {
    //    unimplemented!()
    //}
}

/*impl HeadersInMemory {
    fn block_headers(&self, request: GetBlockHeaders) -> Vec<BlockHeader> {
        let mut headers = vec![];
        let mut block_number = match request.block_id {
            BlockId::Hash(hash) => {
                return headers;
            } // TODO
            BlockId::Number(number) => number,
        };
        while let Some(header) = self.block_header(block_number) {
            headers.push(header);
            if headers.len() as u64 >= request.max_headers {
                break;
            }
            if request.reverse {
                block_number -= request.skip + 1
            } else {
                block_number += request.skip + 1
            }
        }
        headers
    }
}
*/
