use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{
    Identifier, IdentifierName, LocalNodeId, ModuleExportName, Node, NodeType, StringLiteral,
};

/// How one declaration is exported.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
pub enum ExportKind {
    /// Named export.
    Named,
    /// Default export.
    Default,
}

/// The name of one import attribute.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
pub enum ImportAttributeName {
    /// An identifier name.
    Identifier(IdentifierName),
    /// A string literal.
    String(StringLiteral),
}

/// One import attribute.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct ImportAttribute {
    /// The attribute name.
    pub name: ImportAttributeName,
    /// The attribute value.
    pub value: StringLiteral,
}

impl Node for ImportAttribute {
    const TYPE: NodeType = NodeType::ImportAttribute;
}

/// One ECMAScript import clause.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum ImportClause {
    /// One default import.
    Default { local: Identifier },
    /// One namespace import.
    Namespace {
        default: Option<Identifier>,
        local: Identifier,
    },
    /// Named imports with an optional default import.
    Named {
        default: Option<Identifier>,
        specifiers: Vec<LocalNodeId<ImportSpecifier>>,
    },
}

/// One named ECMAScript import specifier.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct ImportSpecifier {
    /// The imported name.
    pub imported: ModuleExportName,
    /// The local binding.
    pub local: Identifier,
}

impl Node for ImportSpecifier {
    const TYPE: NodeType = NodeType::ImportSpecifier;
}

/// One local ECMAScript export specifier.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct ExportSpecifier {
    /// The local binding.
    pub local: Identifier,
    /// The exported name.
    pub exported: ModuleExportName,
}

impl Node for ExportSpecifier {
    const TYPE: NodeType = NodeType::ExportSpecifier;
}

/// One ECMAScript re-export specifier.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct ReExportSpecifier {
    /// The imported name.
    pub imported: ModuleExportName,
    /// The exported name.
    pub exported: ModuleExportName,
}

impl Node for ReExportSpecifier {
    const TYPE: NodeType = NodeType::ReExportSpecifier;
}
