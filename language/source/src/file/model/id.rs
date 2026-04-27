use std::fmt;

use serde::de::{Error, Visitor};
use serde::{Deserializer, Serializer};

/// Serialize one stable `u128` id.
pub(super) fn serialize_u128<S>(value: u128, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if serializer.is_human_readable() {
        serializer.serialize_str(&value.to_string())
    } else {
        serializer.serialize_u128(value)
    }
}

/// Deserialize one stable `u128` id.
pub(super) fn deserialize_u128<'de, D>(deserializer: D) -> Result<u128, D::Error>
where
    D: Deserializer<'de>,
{
    if deserializer.is_human_readable() {
        deserializer.deserialize_str(U128Visitor)
    } else {
        deserializer.deserialize_u128(U128Visitor)
    }
}

struct U128Visitor;

impl Visitor<'_> for U128Visitor {
    type Value = u128;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a decimal u128 string")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        value.parse::<u128>().map_err(E::custom)
    }

    fn visit_u128<E>(self, value: u128) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(value)
    }
}
