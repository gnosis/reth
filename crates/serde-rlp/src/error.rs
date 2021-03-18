// Copyright 2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

use std::fmt::Display;

/// Errors that could occur during de/serialization
pub type Error = Box<ErrorKind>;

/// Result of de/serialization operations
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ErrorKind {
    /// Data has additional bytes at the end of the valid RLP fragment.
    RlpIsTooBig,
    /// Data has too few bytes for valid RLP.
    RlpIsTooShort,
    /// Declared length is invalid and results in overflow
    RlpInvalidLength,
    /// Custom rlp decoding error.
    Custom(&'static str),
}

impl Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self, f)
    }
}

impl std::error::Error for ErrorKind {}
impl serde::ser::Error for ErrorKind {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        todo!()
    }
}

impl serde::de::Error for ErrorKind {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        todo!()
    }
}
