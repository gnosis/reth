// Copyright 2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    NotInsertedTxPerAccountFull,
    NotInsertedPoolFullIncreaseGas,
    NotReplacedIncreaseGas,
    AlreadyPresent,
    TxAuthorUnknown,
    TxPeerAccountIsFull,
}
