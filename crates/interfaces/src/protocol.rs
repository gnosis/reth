// Copyright 2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

pub trait Protocol {

    /// First checks on block received from wire network
    pub fn check_block_solo(&self,block: &Block) -> Result<(),Err>;

    /// add other stuff here 
    pub fn check_block_parent_pre_execution(&self, parent: &Block, block: &Block) ->Result<(),Err>;

}