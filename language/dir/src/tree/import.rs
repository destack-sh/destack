use destack_core::StringId;
use destack_source::AdaptImage;
use serde::{Deserialize, Serialize};

use crate::{Expression, LocalNodeId, Name, ScalarLiteral};

/// The target of an import declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, AdaptImage)]
pub enum ImportTarget {
    /// Static import target string (like `"foo"`).
    String(StringId),
    /// Dynamic import target expression (like `join(base, name)`).
    Expression { target: LocalNodeId<Expression> },
}

/// The kind of one import attribute clause.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, AdaptImage)]
pub enum ImportAttributeClauseKind {
    /// The standard `with` attribute clause keyword.
    With,
    /// The legacy `assert` attribute clause keyword.
    Assert,
}

/// One import attribute clause.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, AdaptImage)]
pub struct ImportAttributeClause {
    /// The clause introducer.
    pub kind: ImportAttributeClauseKind,
    /// The attribute entries inside the clause body.
    pub attributes: Vec<ImportAttribute>,
}

/// One import attribute entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, AdaptImage)]
pub struct ImportAttribute {
    /// The attribute key.
    pub key: Name,
    /// The attribute value.
    pub value: ImportAttributeValue,
}

/// One static import attribute value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, AdaptImage)]
pub enum ImportAttributeValue {
    /// A scalar literal value.
    ScalarLiteral(ScalarLiteral),
    /// An array value.
    Array(Vec<ImportAttributeValue>),
    /// An object value.
    Object(Vec<ImportAttribute>),
    /// A parser placeholder for invalid syntax.
    Error,
}
