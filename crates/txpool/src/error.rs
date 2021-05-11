// Copyright 2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0
use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum Error {
    #[error("Transaction not inserted. Limit tx per account reached.")]
    NotInsertedTxPerAccountFull,
    #[error("Transaction not inserted. account is unknown.")]
    NotInsertedAccountUnknown,
    #[error("Transaction not inserted. Nonce value for that account is already applied")]
    NotInsertedWrongNonce,
    #[error("Transaction not inserted. Account gas is insufficient for this transaction.")]
    NotInsertedBalanceInsufficient,
    #[error("Transaction not inserted. Pool limit reached.")]
    NotInsertedPoolFullIncreaseGas,
    #[error("Transaction not replaced. Increase gas.")]
    NotReplacedIncreaseGas,
    #[error("Transaction already present.")]
    AlreadyPresent,
    #[error("Author unknown.")]
    TxAuthorUnknown,
    #[error("Runtime error.")]
    RuntimeError
}