use serde::{Deserialize, Serialize};

use crate::{Expression, LocalNodeId, StringId};

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
