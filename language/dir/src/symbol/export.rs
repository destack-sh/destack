use destack_core::{StringId, StringPool};
use destack_serde::Reflect;
use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use crate::{DependencyItem, GlobalSymbolId, LocalNodeId, LocalSymbolId, Name, StaticKey};

/// One exact target exposed through a module export.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
pub enum ExportTarget {
    /// One declaration symbol.
    Symbol(GlobalSymbolId),
    /// One module namespace object.
    Namespace(ModuleId),
}

/// The exported name in one module record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
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

    /// Build the default export key.
    #[inline]
    pub fn default_key() -> Self {
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
            Name::Index(index) => Self::Named(StaticKey::Index(index)),
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

/// How one exported symbol's type reaches its consumers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum ExportForm {
    /// The declaration writes its complete type.
    Declared,
    /// A literal binding carries its written value.
    Literal,
    /// The type exists only after inference.
    Inferred,
}

impl ExportForm {
    /// Return whether consumers couple to the exporter's inference.
    pub fn couples(self) -> bool {
        matches!(self, Self::Inferred)
    }
}

/// One local export from a symbol declared in the current module.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
pub struct LocalExport {
    /// The exported name.
    pub key: ExportKey,
    /// The local symbol exposed by the export.
    pub source: LocalSymbolId,
    /// The declared form of the exported symbol's type.
    pub form: ExportForm,
    /// The export clause item that declared this export.
    pub item: Option<LocalNodeId<DependencyItem>>,
}

/// Which binding a re-export selects from the target module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum ExportSelector {
    /// A named target export.
    Named(StaticKey),
    /// The target module default export.
    Default,
    /// The target module namespace object.
    Namespace,
}

impl ExportSelector {
    /// Return the export key selected from the target module.
    #[inline]
    pub fn selected_export_key(self) -> Option<ExportKey> {
        match self {
            Self::Named(key) => Some(ExportKey::Named(key)),
            Self::Default => Some(ExportKey::Default),
            Self::Namespace => None,
        }
    }
}

/// One named re-export from another module.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
pub struct IndirectExport {
    /// The exported name in the current module.
    pub key: ExportKey,
    /// The dependency item that declared the export.
    pub item: LocalNodeId<DependencyItem>,
    /// The target module selected by the export.
    pub target: Option<ModuleId>,
    /// The export selected from the target module.
    pub imported: ExportSelector,
}

/// One named export entry.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
pub enum NamedExport {
    /// A local export.
    Local(LocalExport),
    /// A re-export from another module.
    Indirect(IndirectExport),
}

impl NamedExport {
    /// Return the exported key.
    #[inline]
    pub fn key(self) -> ExportKey {
        match self {
            Self::Local(export) => export.key,
            Self::Indirect(export) => export.key,
        }
    }

    /// Return the export clause item that declared this export.
    #[inline]
    pub fn item(self) -> Option<LocalNodeId<DependencyItem>> {
        match self {
            Self::Local(export) => export.item,
            Self::Indirect(export) => Some(export.item),
        }
    }
}

/// One `export * from` edge.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
pub struct StarExport {
    /// The dependency item that declared the star export.
    pub item: LocalNodeId<DependencyItem>,
    /// The target module selected by the export.
    pub target: Option<ModuleId>,
}
