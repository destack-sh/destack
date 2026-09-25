use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use super::{ModuleReference, NamespaceReference};

/// The current package reference schema version.
pub const PACKAGE_REFERENCE_SCHEMA_VERSION: u32 = 8;

/// Checked public documentation for one package.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
#[serde(rename_all = "camelCase")]
pub struct PackageReference {
    /// The artifact schema version.
    pub schema_version: u32,
    /// The generating toolchain version.
    pub toolchain_version: String,
    /// The documented package identity.
    pub package: PackageIdentity,
    /// The package's public modules.
    pub modules: Vec<ModuleReference>,
    /// Modules reachable through public namespace exports.
    pub namespaces: Vec<NamespaceReference>,
}

/// Public package identity and authored catalog metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
#[serde(rename_all = "camelCase")]
pub struct PackageIdentity {
    /// The package name.
    pub name: String,
    /// The package version when declared.
    pub version: Option<String>,
    /// The package description when declared.
    pub description: Option<String>,
    /// The package license when declared.
    pub license: Option<String>,
}
