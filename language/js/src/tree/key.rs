use crate::{Expression, LocalNodeId, TypeExpression};
use destack_core::StringId;

/// A Name is a regular or string identifier.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Name {
    /// A regular identifier (regular `x` or `someThing`).
    Identifier(StringId),
    /// A string identifier (like `["Content-Type"]`, only in certain contexts).
    String(StringId),
}

/// A Key is a name or a dynamic key.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Key {
    /// Name (like `x` or `someThing`).
    Name(Name),
    /// Private name (like `#x`).
    Private(StringId),
    /// Dynamic key (like `["Content-Type"]`).
    Expression(LocalNodeId<Expression>),
    /// Named dynamic key (like `[x: string]: any`).
    NamedExpression {
        name: Name,
        key: LocalNodeId<TypeExpression>,
    },
}
