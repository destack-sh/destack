use crate::{Expression, LocalNodeId, StringId};

/// A Name is a regular, string, or numeric identifier.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Name {
    /// A regular identifier (regular `x` or `someThing`).
    Identifier(StringId),
    /// A string identifier (like `"Content-Type"`, only in certain contexts).
    String(StringId),
    /// A numeric identifier (like `123` or `2e308` as object key).
    Number(StringId),
}

impl Name {
    /// Get the string identifier.
    #[inline]
    pub fn string(&self) -> StringId {
        match self {
            Name::Identifier(id) => *id,
            Name::String(id) => *id,
            Name::Number(id) => *id,
        }
    }
}

/// A Key is a name or a dynamic key.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Key {
    /// Name (like `x` or `someThing`).
    Name(Name),
    /// Dynamic key (like `["Content-Type"]`).
    Expression(LocalNodeId<Expression>),
    /// Named dynamic key (like `[x: string]: any`).
    NamedExpression {
        name: StringId,
        key: LocalNodeId<Expression>,
    },
}
