use crate::{Expression, LocalNodeId, StringId};

/// A dynamic key is a name or a dynamic key.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DynamicKey {
    /// Name (like `x` or `someThing`).
    Name(StringId),
    /// Numeric name (like `123` or `2e308` as object key).
    Number(StringId),
    /// Dynamic key (like `["Content-Type"]`).
    Expression(LocalNodeId<Expression>),
    /// Named dynamic key (like `[x: string]: any`).
    NamedExpression {
        name: StringId,
        key: LocalNodeId<Expression>,
    },
}
