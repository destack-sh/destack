use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{Expression, LocalNodeId, StaticKey, StringId, View};

/// A name is a regular, string, or numeric identifier.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
pub enum Name {
    /// A regular identifier.
    Identifier(StringId),
    /// A string identifier.
    String(StringId),
    /// A positional index.
    Index(usize),
}

impl Name {
    /// Get the string identifier.
    #[inline]
    pub fn string(&self) -> StringId {
        match self {
            Name::Identifier(id) | Name::String(id) => *id,
            Name::Index(index) => panic!("indexed name {index} has no string identifier"),
        }
    }

    /// Return this name as a static lookup key.
    #[inline]
    pub fn static_key(self) -> StaticKey {
        match self {
            Self::Identifier(name) | Self::String(name) => StaticKey::Name(name),
            Self::Index(index) => StaticKey::Index(index),
        }
    }
}

/// A key in value or type property position.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
pub enum Key {
    /// A named key.
    Name(Name),
    /// A dynamic value-space key.
    Expression(LocalNodeId<Expression>),
}

impl Key {
    /// Return this direct key as a static lookup key when possible.
    #[inline]
    pub fn direct_static_key(self) -> Option<StaticKey> {
        match self {
            Self::Name(name) => Some(name.static_key()),
            Self::Expression(_) => None,
        }
    }

    /// Return this key as a static lookup key when locally obvious.
    pub fn static_key(self, tree: &View<'_>) -> Option<StaticKey> {
        match self {
            Self::Name(name) => Some(name.static_key()),
            Self::Expression(expression) => tree.get(expression).static_key(),
        }
    }
}
