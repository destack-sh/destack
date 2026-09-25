use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use super::DeclarationReference;

/// One public package module.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
#[serde(rename_all = "camelCase")]
pub struct ModuleReference {
    /// The resolved module identity used by namespace exports.
    pub module: String,
    /// The public import specifier.
    pub specifier: String,
    /// The package-relative source path.
    pub path: Option<String>,
    /// The module's public exports.
    pub exports: Vec<ExportReference>,
}

/// A module reached through an exported namespace, rather than an import specifier.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
#[serde(rename_all = "camelCase")]
pub struct NamespaceReference {
    /// The resolved module identity used by namespace exports.
    pub module: String,
    /// The package-relative source path.
    pub path: Option<String>,
    /// The namespace's public exports.
    pub exports: Vec<ExportReference>,
}

/// One public exported name.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
#[serde(rename_all = "camelCase")]
pub struct ExportReference {
    /// The exported name.
    pub name: String,
    /// The resolved declarations in overload order.
    pub declarations: Vec<DeclarationReference>,
    /// The resolved namespace module when this is a namespace export.
    pub namespace: Option<String>,
}
