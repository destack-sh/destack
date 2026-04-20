use destack_core::StringId;
use serde::{Deserialize, Serialize};

use crate::{Expression, LocalNodeId, Name, ScalarLiteral};

/// The source of an import declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImportSource {
    /// Standard import statement.
    ImportStatement,
    /// TypeScript triple-slash `reference path` directive.
    ReferencePathDirective,
    /// TypeScript triple-slash `reference types` directive.
    ReferenceTypesDirective,
    /// TypeScript triple-slash `reference lib` directive.
    ReferenceLibDirective,
    /// TypeScript triple-slash `reference no-default-lib` directive.
    ReferenceNoDefaultLibDirective,
    /// Legacy import-equals expression used by older lowerings.
    ImportEquals,
    /// Dynamic import call (`import("mod")`).
    ImportCall,
}

/// The target of an import declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ImportTarget {
    /// Static import target string (like `"foo"`).
    String(StringId),
    /// Dynamic import target expression (like `join(base, name)`).
    Expression { target: LocalNodeId<Expression> },
}

/// The kind of one import attribute clause.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImportAttributeClauseKind {
    /// The standard `with` attribute clause keyword.
    With,
}

/// One import attribute clause.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImportAttributeClause {
    /// The clause introducer.
    pub kind: ImportAttributeClauseKind,
    /// The attribute entries inside the clause body.
    pub attributes: Vec<ImportAttribute>,
}

/// One import attribute entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImportAttribute {
    /// The attribute key.
    pub key: Name,
    /// The attribute value.
    pub value: ImportAttributeValue,
}

/// One static import attribute value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
