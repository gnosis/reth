// Copyright 2021 Gnosis Ltd.
// SPDX-License-Identifier: Apache-2.0

use std::error::Error;

/// A trait used to verify the integrity of various types like transaction,
/// block, etc using some kind of identity.
pub trait Verifiable<I> {
    /// Returns true if we can attest that self was originated by identity.
    fn valid(&self, identity: &I) -> Result<bool, Box<dyn Error>>;
}
