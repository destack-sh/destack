use crate::{Expression, LocalNodeId, StringId};

/// A Key is a name or a dynamic key.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Key {
    /// Name (like `x` or `someThing`).
    Name(StringId),
    /// Dynamic key (like `["Content-Type"]`).
    Expression(LocalNodeId<Expression>),
    /// Named dynamic key (like `[x: string]: any`).
    NamedExpression {
        name: StringId,
        key: LocalNodeId<Expression>,
    },
}
