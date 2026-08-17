use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use super::DeclarationReference;

/// One public package module.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
#[serde(rename_all = "camelCase")]
pub struct ModuleReference {
    /// The public import specifier.
    pub specifier: String,
    /// The package-relative source path.
    pub path: Option<String>,
    /// The module's public exports.
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
