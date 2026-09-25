use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use tspp_core::{StringId, StringPool};
use tspp_serde::Reflect;
use tspp_source::ModuleId;

use crate::{
    DependencyItem, GlobalSymbolId, LocalNodeId, LocalSymbolId, Name, ReferenceTarget, StaticKey,
};

/// One resolved target exposed through an exported name.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect)]
pub enum ExportTarget {
    /// One declaration overload group.
    Symbols(SmallVec<[GlobalSymbolId; 2]>),
    /// One module namespace object.
    Namespace(ModuleId),
}

impl ExportTarget {
    /// Create one symbol group.
    pub fn symbol(symbol: GlobalSymbolId) -> Self {
        Self::Symbols(SmallVec::from_slice(&[symbol]))
    }

    /// Create one non-empty symbol group.
    pub fn symbols(symbols: impl IntoIterator<Item = GlobalSymbolId>) -> Option<Self> {
        let mut target = SmallVec::new();

        // retain every exact declaration once in source order
        for symbol in symbols {
            if !target.contains(&symbol) {
                target.push(symbol);
            }
        }

        (!target.is_empty()).then_some(Self::Symbols(target))
    }

    /// Iterate the scalar reference targets.
    pub fn iter(&self) -> impl Iterator<Item = ReferenceTarget> + '_ {
        let symbols = match self {
            Self::Symbols(symbols) => Some(symbols.as_slice()),
            Self::Namespace(_) => None,
        };
        let namespace = match self {
            Self::Symbols(_) => None,
            Self::Namespace(module) => Some(*module),
        };

        symbols
            .into_iter()
            .flatten()
            .copied()
            .map(ReferenceTarget::Symbol)
            .chain(namespace.map(ReferenceTarget::Namespace))
    }

    /// Return the symbol declarations in this target.
    pub fn symbol_ids(&self) -> Option<&[GlobalSymbolId]> {
        match self {
            Self::Symbols(symbols) => Some(symbols),
            Self::Namespace(_) => None,
        }
    }

    /// Return the only symbol declaration in this target.
    pub fn single_symbol(&self) -> Option<GlobalSymbolId> {
        let symbols = self.symbol_ids()?;
        let [symbol] = symbols else {
            return None;
        };

        Some(*symbol)
    }

    /// Return the module namespace in this target.
    pub fn namespace(&self) -> Option<ModuleId> {
        match self {
            Self::Symbols(_) => None,
            Self::Namespace(module) => Some(*module),
        }
    }

    /// Return the target module when every declaration belongs to one module.
    pub fn module(&self) -> Option<ModuleId> {
        match self {
            Self::Symbols(symbols) => {
                let module = symbols.first()?.module_id;
                symbols
                    .iter()
                    .all(|symbol| symbol.module_id == module)
                    .then_some(module)
            }
            Self::Namespace(module) => Some(*module),
        }
    }
}

/// One resolved export declaration and its final compiler target.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect)]
pub struct ExportResolution {
    /// The declaration selected by the exported name.
    pub declaration: ExportTarget,
    /// The final exported target.
    pub target: ExportTarget,
}

impl ExportResolution {
    /// Create a direct export resolution.
    pub fn direct(target: ExportTarget) -> Self {
        Self {
            declaration: target.clone(),
            target,
        }
    }

    /// Replace the selected declaration while retaining the final target.
    pub fn with_declaration(self, declaration: ExportTarget) -> Self {
        Self {
            declaration,
            target: self.target,
        }
    }
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
    /// Return whether consumers require the exporter's inference.
    pub fn requires_inference(self) -> bool {
        matches!(self, Self::Inferred)
    }
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

/// How one exported name is bound.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum ExportBinding {
    /// Declarations from the exporting module.
    Local {
        /// The local declaration overload group.
        symbols: SmallVec<[LocalSymbolId; 2]>,
    },
    /// A local import exported by this module.
    Import {
        /// The local import symbol.
        local: LocalSymbolId,
        /// The imported module.
        module: Option<ModuleId>,
        /// The selected exported name.
        selector: ExportSelector,
    },
    /// A declaration re-exported directly from another module.
    ReExport {
        /// The imported module.
        module: Option<ModuleId>,
        /// The selected exported name.
        selector: ExportSelector,
    },
}

impl ExportBinding {
    /// Return the imported module selected by this binding.
    pub fn target_module(&self) -> Option<ModuleId> {
        match self {
            Self::Local { .. } => None,
            Self::Import { module, .. } | Self::ReExport { module, .. } => *module,
        }
    }
}

/// One named export entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct NamedExport {
    /// The exported name.
    pub key: ExportKey,
    /// The dependency item that declared this export.
    pub item: Option<LocalNodeId<DependencyItem>>,
    /// The local declaration selected by the exported name.
    pub declaration: Option<LocalSymbolId>,
    /// How the exported name is bound.
    pub binding: ExportBinding,
}

impl NamedExport {
    /// Return the exported key.
    #[inline]
    pub fn key(&self) -> ExportKey {
        self.key
    }

    /// Return the export clause item that declared this export.
    #[inline]
    pub fn item(&self) -> Option<LocalNodeId<DependencyItem>> {
        self.item
    }

    /// Return the target module selected by an indirect export.
    #[inline]
    pub fn target_module(&self) -> Option<ModuleId> {
        self.binding.target_module()
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
