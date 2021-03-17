// Copyright 2020 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

use super::blockchain::Blockchain;
use crate::common_types::{BlockBody, BlockHeader, BlockId, BlockNumber, GetBlockHeaders};
use primitive_types::H256;
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

impl Blockchain for HeadersInMemory {
    fn block_header(&self, number: BlockNumber) -> Option<BlockHeader> {
        clone_option(self.headers.get(&number))
    }

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

    fn block_body(&self, hash: &H256) -> Option<BlockBody> {
        None
    }

    fn block_receipt(&self) {
        unimplemented!()
    }

    fn best_block_header(&self) -> Option<&BlockNumber> {
        self.headers.keys().max()
    }

    fn import_block_header(&mut self, header: &BlockHeader) {
        self.headers.insert(header.number, header.clone());
    }

    fn import_block_body(&mut self, body: &BlockBody) {
        info!("Received block body, ignoring.");
    }

    fn import_old_block(&self) {
        unimplemented!()
    }
}
