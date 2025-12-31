use crate::{Expression, LocalNodeId, StaticKey, StringId};

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

impl DynamicKey {
    /// Convert a dynamic key into a static key when possible.
    pub fn as_static_key(&self) -> Option<StaticKey> {
        match self {
            DynamicKey::Name(name) => Some(StaticKey::Name(*name)),
            DynamicKey::Number(name) => Some(StaticKey::Number(*name)),
            DynamicKey::Expression(_) | DynamicKey::NamedExpression { .. } => None,
        }
    }
}
