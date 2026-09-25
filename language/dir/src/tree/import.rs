use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use crate::{Literal, Name};

/// The kind of one import attribute clause.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum ImportAttributeClauseKind {
    /// The standard `with` attribute clause keyword.
    With,
}

/// One import attribute clause.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct ImportAttributeClause {
    /// The clause introducer.
    pub kind: ImportAttributeClauseKind,
    /// The attribute entries inside the clause body.
    pub attributes: Vec<ImportAttribute>,
}

/// One import attribute entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct ImportAttribute {
    /// The attribute key.
    pub key: Name,
    /// The attribute value.
    pub value: ImportAttributeValue,
}

/// One static import attribute value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum ImportAttributeValue {
    /// A scalar literal value.
    Literal(Literal),
    /// An array value.
    Array(Vec<ImportAttributeValue>),
    /// An object value.
    Object(Vec<ImportAttribute>),
    /// A parser placeholder for invalid syntax.
    Error,
}
