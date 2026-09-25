use crate::{Expression, LocalNodeId};
use tspp_core::StringId;

use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;
/// A Name is a regular or string identifier.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
pub enum Name {
    /// A regular identifier (regular `x` or `someThing`).
    Identifier(StringId),
    /// A string identifier (like `["Content-Type"]`, only in certain contexts).
    String(StringId),
}

/// A Key is a name or a dynamic key.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
pub enum Key {
    /// Name (like `x` or `someThing`).
    Name(Name),
    /// Private name (like `#x`).
    Private(StringId),
    /// Dynamic key (like `["Content-Type"]`).
    Expression(LocalNodeId<Expression>),
}
