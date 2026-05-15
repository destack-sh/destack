use serde::{Deserialize, Serialize};

use crate::{Expression, LocalNodeId, StaticKey, StringId, Tree};

/// A name is a regular, string, or numeric identifier.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Name {
    /// A regular identifier.
    Identifier(StringId),
    /// A string identifier.
    String(StringId),
    /// A numeric identifier.
    Number(StringId),
}

impl Name {
    /// Get the string identifier.
    #[inline]
    pub fn string(&self) -> StringId {
        match self {
            Name::Identifier(id) | Name::String(id) | Name::Number(id) => *id,
        }
    }

    /// Return this name as a static lookup key.
    #[inline]
    pub fn static_key(self) -> StaticKey {
        match self {
            Self::Identifier(name) | Self::String(name) => StaticKey::Name(name),
            Self::Number(name) => StaticKey::Number(name),
        }
    }
}

/// A key in value or type property position.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Key {
    /// A named key.
    Name(Name),
    /// A private key.
    Private(StringId),
    /// A dynamic value-space key.
    Expression(LocalNodeId<Expression>),
}

impl Key {
    /// Return this direct key as a static lookup key when possible.
    #[inline]
    pub fn direct_static_key(self) -> Option<StaticKey> {
        match self {
            Self::Name(name) => Some(name.static_key()),
            Self::Private(_) => None,
            Self::Expression(_) => None,
        }
    }

    /// Return this key as a static lookup key when locally obvious.
    pub fn static_key(self, tree: &Tree) -> Option<StaticKey> {
        match self {
            Self::Name(name) => Some(name.static_key()),
            Self::Private(_) => None,
            Self::Expression(expression) => tree.get(expression).static_key(),
        }
    }

    /// Return the private name when this is a private member key.
    #[inline]
    pub fn private_name(self) -> Option<StringId> {
        match self {
            Self::Private(name) => Some(name),
            Self::Name(_) | Self::Expression(_) => None,
        }
    }
}
