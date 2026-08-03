use std::slice;

use destack_core::FxIndexMap as IndexMap;
use destack_serde::Reflect;
use destack_source::ModuleId;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::{ExportTarget, GlobalSymbolId, LanguageItem, LocalSymbolId, Reference, StaticKey};

/// Resolved import targets for one module.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct ImportTable {
    /// The module id of the import table.
    pub module_id: ModuleId,
    /// Imported local symbols keyed to their resolution.
    pub resolution_by_symbol: IndexMap<LocalSymbolId, ImportResolution>,
    /// Global targets made visible by the active profile.
    pub global_target_by_key: IndexMap<StaticKey, Vec<ImportTarget>>,
    /// Resolved symbols for language items used by this module.
    pub language_symbol_by_item: IndexMap<LanguageItem, GlobalSymbolId>,
    /// The default tree builder pulled in by this module's tree literals.
    pub tree_target: Option<GlobalSymbolId>,
}

impl ImportTable {
    /// Iterate every resolved import target.
    pub fn targets(&self) -> impl Iterator<Item = ImportTarget> + '_ {
        self.resolution_by_symbol
            .values()
            .flat_map(ImportResolution::targets)
            .chain(self.global_target_by_key.values().flatten().copied())
    }

    /// Create an empty import table.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            resolution_by_symbol: IndexMap::default(),
            global_target_by_key: IndexMap::default(),
            language_symbol_by_item: IndexMap::default(),
            tree_target: None,
        }
    }

    /// Insert one local import symbol resolution.
    pub fn insert_symbol(&mut self, symbol: LocalSymbolId, resolution: ImportResolution) {
        self.resolution_by_symbol.insert(symbol, resolution);
    }

    /// Add one imported global target.
    pub fn push_global_target(&mut self, key: StaticKey, target: ImportTarget) {
        let targets = self.global_target_by_key.entry(key).or_default();
        if !targets.contains(&target) {
            targets.push(target);
        }
    }

    /// Insert one imported language item symbol.
    pub fn insert_language_symbol(&mut self, item: LanguageItem, symbol: GlobalSymbolId) {
        self.language_symbol_by_item.insert(item, symbol);
    }

    /// Return one local import symbol resolution.
    pub fn symbol_resolution(&self, symbol: LocalSymbolId) -> Option<&ImportResolution> {
        self.resolution_by_symbol.get(&symbol)
    }

    /// Return imported local symbols that resolve to concrete exported symbols.
    pub fn symbol_targets(&self) -> impl Iterator<Item = (GlobalSymbolId, GlobalSymbolId)> + '_ {
        self.resolution_by_symbol
            .iter()
            .filter_map(|(symbol, resolution)| match resolution {
                ImportResolution::Resolved(ImportTarget::Symbol(target)) => {
                    Some(((*symbol).into_global(self.module_id), *target))
                }
                ImportResolution::Resolved(ImportTarget::Namespace(_))
                | ImportResolution::Ambiguous(_)
                | ImportResolution::Missing => None,
            })
    }

    /// Return imported global targets for one key.
    pub fn global_targets(&self, key: StaticKey) -> Option<&[ImportTarget]> {
        self.global_target_by_key.get(&key).map(Vec::as_slice)
    }

    /// Return one imported language item symbol.
    pub fn language_symbol(&self, item: LanguageItem) -> Option<GlobalSymbolId> {
        self.language_symbol_by_item.get(&item).copied()
    }

    /// Return imported language item symbols.
    pub fn language_symbols(&self) -> impl Iterator<Item = GlobalSymbolId> + '_ {
        self.language_symbol_by_item.values().copied()
    }

    /// Return modules that own resolved import targets.
    pub fn target_modules(&self) -> impl Iterator<Item = ModuleId> + '_ {
        let symbols = self
            .resolution_by_symbol
            .values()
            .flat_map(ImportResolution::targets)
            .map(ImportTarget::module);
        let globals = self
            .global_target_by_key
            .values()
            .flatten()
            .map(|target| target.module());
        let language = self
            .language_symbol_by_item
            .values()
            .map(|symbol| symbol.module_id);
        let tree = self.tree_target.iter().map(|symbol| symbol.module_id);

        symbols.chain(globals).chain(language).chain(tree)
    }
}

/// Resolution of one local import binding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum ImportResolution {
    /// One exact imported target.
    Resolved(ImportTarget),
    /// Multiple conflicting imported targets.
    Ambiguous(SmallVec<[ImportTarget; 2]>),
    /// No imported target.
    Missing,
}

impl ImportResolution {
    /// Iterate the retained targets.
    pub fn targets(&self) -> impl Iterator<Item = ImportTarget> + '_ {
        let targets = match self {
            Self::Resolved(target) => slice::from_ref(target),
            Self::Ambiguous(targets) => targets.as_slice(),
            Self::Missing => &[],
        };

        targets.iter().copied()
    }
}

impl From<ImportTarget> for ImportResolution {
    /// Convert one exact imported target into a resolution.
    fn from(target: ImportTarget) -> Self {
        Self::Resolved(target)
    }
}

impl From<&ImportResolution> for Reference {
    /// Convert one import binding resolution into a source reference.
    fn from(resolution: &ImportResolution) -> Self {
        match resolution {
            ImportResolution::Resolved(ImportTarget::Symbol(symbol)) => {
                Self::from_symbols([*symbol])
            }
            ImportResolution::Resolved(ImportTarget::Namespace(module)) => Self::Namespace(*module),
            ImportResolution::Ambiguous(targets) => Self::Ambiguous(targets.clone()),
            ImportResolution::Missing => Self::Missing,
        }
    }
}

/// Target selected by one import binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum ImportTarget {
    /// A symbol exported by a target module.
    Symbol(GlobalSymbolId),
    /// A namespace object for a target module.
    Namespace(ModuleId),
}

impl ImportTarget {
    /// Return the target module.
    pub fn module(self) -> ModuleId {
        match self {
            Self::Symbol(symbol) => symbol.module_id,
            Self::Namespace(module) => module,
        }
    }
}

impl From<ExportTarget> for ImportTarget {
    /// Convert one exported target into an imported target.
    fn from(target: ExportTarget) -> Self {
        match target {
            ExportTarget::Symbol(symbol) => Self::Symbol(symbol),
            ExportTarget::Namespace(module) => Self::Namespace(module),
        }
    }
}

impl From<ImportTarget> for ExportTarget {
    /// Convert one imported target into its exported target form.
    fn from(target: ImportTarget) -> Self {
        match target {
            ImportTarget::Symbol(symbol) => Self::Symbol(symbol),
            ImportTarget::Namespace(module) => Self::Namespace(module),
        }
    }
}
