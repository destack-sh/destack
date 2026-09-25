use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::Reflect;

/// Integer representation used by the binary codec.
#[derive(Debug, Clone, Copy)]
pub(crate) enum IntegerEncoding {
    /// Canonical variable-width integers.
    Compact,
    /// Direct little-endian integers.
    Fixed,
}

/// Type that can use the TS++ binary codec and describe its schema.
pub trait Codec: Serialize + DeserializeOwned + Reflect {}

impl<T> Codec for T where T: Serialize + DeserializeOwned + Reflect {}
