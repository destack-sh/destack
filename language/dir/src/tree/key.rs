use crate::{Expression, NodeId, StringId};

/// A Key is a name or a dynamic key.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Key {
    /// Name (like `x` or `someThing`).
    Name(StringId),
    /// Dynamic key (like `["Content-Type"]`).
    Expression(NodeId<Expression>),
    /// Named dynamic key (like `[x: string]: any`).
    NamedExpression {
        name: StringId,
        key: NodeId<Expression>,
    },
}
