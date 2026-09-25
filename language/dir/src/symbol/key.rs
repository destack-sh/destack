use serde::{Deserialize, Serialize};
use tspp_core::{StringId, StringPool};
use tspp_serde::Reflect;

/// Key for some static identifier or positional index.
#[derive(
    Debug, Clone, Copy, PartialEq, Hash, PartialOrd, Ord, Eq, Serialize, Deserialize, Reflect,
)]
pub enum StaticKey {
    /// Regular name key (like `x` or `"weird identifier"`).
    Name(StringId),
    /// Positional index key (like `0` or `1`).
    Index(usize),
}

impl From<StringId> for StaticKey {
    fn from(name: StringId) -> Self {
        StaticKey::Name(name)
    }
}

impl StaticKey {
    /// Get the name of the symbol key.
    pub fn name(&self) -> Option<StringId> {
        match self {
            StaticKey::Name(name) => Some(*name),
            StaticKey::Index(_) => None,
        }
    }

    /// Return true when this key behaves as a string like key.
    pub fn is_string_like(&self) -> bool {
        matches!(self, StaticKey::Name(_))
    }

    /// Return true when this key behaves as a number like key.
    pub fn is_number_like(&self) -> bool {
        matches!(self, StaticKey::Index(_))
    }

    /// Return whether this exact key stores directly in one primitive representation.
    pub fn widens_to_primitive(&self, primitive: crate::PrimitiveType) -> bool {
        match primitive {
            crate::PrimitiveType::String => self.is_string_like(),
            // index keys are exact values and must fit the integer width
            crate::PrimitiveType::Integer(integer) => match self {
                StaticKey::Index(index) => {
                    i64::try_from(*index).is_ok_and(|value| integer.fits_literal(value))
                }
                _ => false,
            },
            crate::PrimitiveType::Boolean
            | crate::PrimitiveType::Character
            | crate::PrimitiveType::Float(_)
            | crate::PrimitiveType::Bigint => false,
        }
    }

    /// Get the debug string given a mutable string pool.
    pub fn debug_string(&self, strings: &StringPool) -> String {
        match self {
            StaticKey::Name(name) => {
                format!("'{}'", strings.get(*name))
            }
            StaticKey::Index(index) => format!("#{index}"),
        }
    }
}
