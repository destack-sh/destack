use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

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
    /// Return the string identifier when this name is textual.
    #[inline]
    pub fn as_string_id(&self) -> Option<StringId> {
        match self {
            Name::Identifier(id) | Name::String(id) => Some(*id),
            Name::Index(_) => None,
        }
    }

    /// Return the mutable string identifier when this name is textual.
    #[inline]
    pub fn as_string_id_mut(&mut self) -> Option<&mut StringId> {
        match self {
            Name::Identifier(id) | Name::String(id) => Some(id),
            Name::Index(_) => None,
        }
    }

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
