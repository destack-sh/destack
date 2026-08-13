use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{StaticKey, StringId};

/// An authored identifier, string, or integer name.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
pub enum Name {
    /// An identifier name.
    Identifier(StringId),
    /// A quoted string name.
    String(StringId),
    /// An integer name.
    Index(usize),
}

impl Name {
    /// Return the string identifier.
    #[inline]
    pub fn string(&self) -> StringId {
        match self {
            Name::Identifier(id) | Name::String(id) => *id,
            Name::Index(index) => panic!("indexed name {index} has no string identifier"),
        }
    }
}

impl From<Name> for StaticKey {
    fn from(name: Name) -> Self {
        match name {
            Name::Identifier(name) | Name::String(name) => Self::Name(name),
            Name::Index(index) => Self::Index(index),
        }
    }
}
