use crate::{Expression, NodeId};
use dyst_source::StringId;

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
    /// Dynamic key (like `["Content-Type"]`).
    Expression(NodeId<Expression>),
    /// Named dynamic key (like `[x: string]: any`).
    NamedExpression { name: Name, key: NodeId<Expression> },
}
