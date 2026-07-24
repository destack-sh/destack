use destack_serde::Reflect;
use destack_source::ModuleId;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{ExportTarget, GlobalSymbolId, LanguageItem, LocalSymbolId, StaticKey};

/// Resolved import targets for one module.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct ImportTable {
    /// The module id of the import table.
    pub module_id: ModuleId,
    /// Imported local symbols keyed to their resolved target.
    pub target_by_symbol: IndexMap<LocalSymbolId, ImportTarget>,
    /// Global targets made visible by the active profile.
    pub global_target_by_key: IndexMap<StaticKey, Vec<ImportTarget>>,
    /// Resolved symbols for language items used by this module.
    pub language_symbol_by_item: IndexMap<LanguageItem, GlobalSymbolId>,
}

impl ImportTable {
    /// Iterate every resolved import target.
    pub fn targets(&self) -> impl Iterator<Item = ImportTarget> + '_ {
        self.target_by_symbol
            .values()
            .copied()
            .chain(self.global_target_by_key.values().flatten().copied())
    }

    /// Create an empty import table.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            target_by_symbol: IndexMap::new(),
            global_target_by_key: IndexMap::new(),
            language_symbol_by_item: IndexMap::new(),
        }
    }

    /// Insert one resolved local import symbol target.
    pub fn insert_symbol(&mut self, symbol: LocalSymbolId, target: ImportTarget) {
        self.target_by_symbol.insert(symbol, target);
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

    /// Return one resolved local import symbol target.
    pub fn symbol_target(&self, symbol: LocalSymbolId) -> Option<ImportTarget> {
        self.target_by_symbol.get(&symbol).copied()
    }

    /// Return imported local symbols that resolve to concrete exported symbols.
    pub fn symbol_targets(&self) -> impl Iterator<Item = (GlobalSymbolId, GlobalSymbolId)> + '_ {
        self.target_by_symbol
            .iter()
            .filter_map(|(symbol, target)| match target {
                ImportTarget::Symbol(target) => {
                    Some(((*symbol).into_global(self.module_id), *target))
                }
                ImportTarget::Namespace(_) => None,
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
        let symbols = self.target_by_symbol.values().map(|target| target.module());
        let globals = self
            .global_target_by_key
            .values()
            .flatten()
            .map(|target| target.module());
        let language = self
            .language_symbol_by_item
            .values()
            .map(|symbol| symbol.module_id);

        symbols.chain(globals).chain(language)
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
