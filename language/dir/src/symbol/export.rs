use destack_core::{StringId, StringPool};
use serde::{Deserialize, Serialize};

use crate::{DependencyItem, DependencyTarget, LocalNodeId, LocalSymbolId, Name, StaticKey};

/// The exported name in one module record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExportKey {
    /// The ECMAScript default export name.
    Default,
    /// A named export key.
    Named(StaticKey),
}

impl ExportKey {
    /// Build one named export key.
    #[inline]
    pub fn named(key: StaticKey) -> Self {
        Self::Named(key)
    }

    /// Build one default export key.
    #[inline]
    pub fn default() -> Self {
        Self::Default
    }

    /// Return the named key when this is a named export.
    #[inline]
    pub fn named_key(self) -> Option<StaticKey> {
        match self {
            Self::Default => None,
            Self::Named(key) => Some(key),
        }
    }

    /// Build one export name from a parsed name.
    pub fn from_name(name: Name, strings: &StringPool) -> Self {
        match name {
            Name::Identifier(name) | Name::String(name) => Self::from_string(name, strings),
            Name::Number(name) => Self::Named(StaticKey::Number(name)),
        }
    }

    /// Build one export name from a string.
    pub fn from_string(name: StringId, strings: &StringPool) -> Self {
        if strings.get(name) == "default" {
            Self::Default
        } else {
            Self::Named(StaticKey::Name(name))
        }
    }
}

/// One local export from a symbol declared in the current module.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LocalExportEntry {
    /// The exported name.
    pub key: ExportKey,
    /// The local symbol exposed by the export.
    pub source: LocalSymbolId,
    /// The export clause item that declared this export.
    pub item: Option<LocalNodeId<DependencyItem>>,
}

/// Which binding a re-export selects from the target module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportSelector {
    /// A named target export.
    Named(StaticKey),
    /// The target module default export.
    Default,
    /// The target module namespace object.
    Namespace,
}

/// One named re-export from another module.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct IndirectExportEntry {
    /// The exported name in the current module.
    pub key: ExportKey,
    /// The dependency item that declared the export.
    pub item: LocalNodeId<DependencyItem>,
    /// The target module selected by the export.
    pub target: DependencyTarget,
    /// The export selected from the target module.
    pub imported: ExportSelector,
}

/// One named export entry.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ExportEntry {
    /// A local export.
    Local(LocalExportEntry),
    /// A re-export from another module.
    Indirect(IndirectExportEntry),
}

impl ExportEntry {
    /// Return the exported key.
    #[inline]
    pub fn key(self) -> ExportKey {
        match self {
            Self::Local(export) => export.key,
            Self::Indirect(export) => export.key,
        }
    }
}

/// One `export * from` edge.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StarExportEntry {
    /// The dependency item that declared the star export.
    pub item: LocalNodeId<DependencyItem>,
    /// The target module selected by the export.
    pub target: DependencyTarget,
}
