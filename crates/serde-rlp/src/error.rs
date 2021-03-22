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
    /// RLP encoding does not support signed integers
    RlpSignedIntegersNotSupported,
    /// RLP encoding does not support floating point numbers
    RlpFloatingPorintNotSupported,
    /// An attempt to deserialize RLP into &str. deserialize into String instead
    RlpIntoBorrowedStringDeserializationNotSupported,
    /// Serde has a deserialize_any method that lets the format hint to the
    /// object which route to take in deserializing.
    RlpAnyNotSupported,
    /// Error caused by the underlying IO. Most likey by std::io::reader el at.
    IOError(String),
    /// Custom rlp decoding error.
    Custom(String),
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
        ErrorKind::Custom(msg.to_string()).into()
    }
}

impl serde::de::Error for ErrorKind {
    fn custom<T: std::fmt::Display>(msg: T) -> Self
    where
        T: Display,
    {
        ErrorKind::Custom(msg.to_string()).into()
    }
}
