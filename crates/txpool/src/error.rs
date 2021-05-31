// Copyright 2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0
use thiserror::Error;
use rlp::DecoderError;

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

    #[error("Transaction replaced on inclusion")]
    RemovedTxReplaced,
    #[error("Transaction removed on demand")]
    RemovedTxOnDemand,
    #[error("Transaction removed by timeout")]
    RemovedTxTimeout,
    #[error("Transaction removed, account balance cant fund this tx")]
    RemovedTxUnfunded,
    #[error("Transaction removed, max pool limit hit, and this is one of worst tx in pool")]
    RemovedTxLimitHit,

    #[error("On new block. Transaction nonce was obsolete, it is probably included in block.")]
    OnNewBlockNonce,

    #[error("Internal error. Account not found")]
    InternalAccountNotFound,
    #[error("Internal error. Account info is obsolete")]
    InternalAccountObsolete,

    #[error("Rlp decode error")]
    DecoderError(DecoderError),
}
