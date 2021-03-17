// Copyright 2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0
use super::data::DataTrait;

pub struct AccessListData {}

impl DataTrait for AccessListData {
    fn encode(
        &self,
        chain_id: Option<super::ChainId>,
        signature: Option<&super::Signature>,
    ) -> Vec<u8> {
        todo!()
    }

    fn decode(rlp: &[u8]) -> Result<crate::Transaction, rlp::DecoderError> {
        todo!()
    }
}
