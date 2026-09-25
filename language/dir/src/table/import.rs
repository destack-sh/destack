use std::slice;

use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use tspp_core::FxIndexMap as IndexMap;
use tspp_serde::Reflect;
use tspp_source::ModuleId;

use crate::{
    ExportResolution, ExportTarget, GlobalSymbolId, LanguageItem, LocalSymbolId, Reference,
    ReferenceTarget, StaticKey,
};

/// Resolved import targets for one module.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct ImportTable {
    /// The module id of the import table.
    pub module_id: ModuleId,
    /// Imported local symbols keyed to their resolution.
    pub resolution_by_symbol: IndexMap<LocalSymbolId, ImportResolution>,
    /// Global resolutions made visible by the active profile.
    pub global_resolution_by_key: IndexMap<StaticKey, Vec<ExportResolution>>,
    /// Resolved symbols for language items used by this module.
    pub language_symbol_by_item: IndexMap<LanguageItem, GlobalSymbolId>,
    /// The default tree builder pulled in by this module's tree literals.
    pub tree_target: Option<GlobalSymbolId>,
}

impl ImportTable {
    /// Iterate every resolved import target.
    pub fn targets(&self) -> impl Iterator<Item = ReferenceTarget> + '_ {
        self.resolution_by_symbol
            .values()
            .flat_map(ImportResolution::targets)
            .chain(
                self.global_resolution_by_key
                    .values()
                    .flatten()
                    .flat_map(|resolution| resolution.target.iter()),
            )
    }

    /// Create an empty import table.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            resolution_by_symbol: IndexMap::default(),
            global_resolution_by_key: IndexMap::default(),
            language_symbol_by_item: IndexMap::default(),
            tree_target: None,
        }
    }

    /// Insert one local import symbol resolution.
    pub fn insert_symbol(&mut self, symbol: LocalSymbolId, resolution: ImportResolution) {
        self.resolution_by_symbol.insert(symbol, resolution);
    }

    /// Add one imported global resolution.
    pub fn push_global_resolution(&mut self, key: StaticKey, resolution: ExportResolution) {
        let resolutions = self.global_resolution_by_key.entry(key).or_default();
        if !resolutions.contains(&resolution) {
            resolutions.push(resolution);
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
                ImportResolution::Resolved(ExportResolution {
                    target: ExportTarget::Symbols(targets),
                    ..
                }) if targets.len() == 1 => {
                    Some(((*symbol).into_global(self.module_id), targets[0]))
                }
                _ => None,
            })
    }

    /// Return imported global resolutions for one key.
    pub fn global_resolutions(&self, key: StaticKey) -> Option<&[ExportResolution]> {
        self.global_resolution_by_key.get(&key).map(Vec::as_slice)
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
            .map(ReferenceTarget::module);
        let globals = self
            .global_resolution_by_key
            .values()
            .flatten()
            .filter_map(|resolution| resolution.target.module());
        let language = self
            .language_symbol_by_item
            .values()
            .map(|symbol| symbol.module_id);
        let tree = self.tree_target.iter().map(|symbol| symbol.module_id);

        symbols.chain(globals).chain(language).chain(tree)
    }
}

/// Decision of one local import binding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum ImportResolution {
    /// One exact imported declaration and final target.
    Resolved(ExportResolution),
    /// Multiple conflicting imported resolutions.
    Ambiguous(SmallVec<[ExportResolution; 2]>),
    /// No imported target.
    Missing,
}

impl ImportResolution {
    /// Iterate the retained targets.
    pub fn targets(&self) -> impl Iterator<Item = ReferenceTarget> + '_ {
        let resolutions = match self {
            Self::Resolved(resolution) => slice::from_ref(resolution),
            Self::Ambiguous(resolutions) => resolutions.as_slice(),
            Self::Missing => &[],
        };

        resolutions
            .iter()
            .flat_map(|resolution| resolution.target.iter())
    }

    /// Iterate the declarations selected by the imported name.
    pub fn declarations(&self) -> impl Iterator<Item = ReferenceTarget> + '_ {
        let resolutions = match self {
            Self::Resolved(resolution) => slice::from_ref(resolution),
            Self::Ambiguous(resolutions) => resolutions.as_slice(),
            Self::Missing => &[],
        };

        resolutions
            .iter()
            .flat_map(|resolution| resolution.declaration.iter())
    }

    /// Return the final compiler target as a name reference.
    pub fn target_reference(&self) -> Reference {
        match self {
            Self::Resolved(resolution) => Reference::from(&resolution.target),
            Self::Ambiguous(resolutions) => {
                let mut targets = SmallVec::new();

                // retain every conflicting final target once
                for target in resolutions
                    .iter()
                    .flat_map(|resolution| resolution.target.iter())
                {
                    if !targets.contains(&target) {
                        targets.push(target);
                    }
                }

                Reference::Ambiguous(targets)
            }
            Self::Missing => Reference::Missing,
        }
    }

    /// Return the authored declaration as a name reference.
    pub fn declaration_reference(&self) -> Reference {
        match self {
            Self::Resolved(resolution) => Reference::from(&resolution.declaration),
            Self::Ambiguous(resolutions) => {
                let mut declarations = SmallVec::new();

                // retain every conflicting authored declaration once
                for declaration in resolutions
                    .iter()
                    .flat_map(|resolution| resolution.declaration.iter())
                {
                    if !declarations.contains(&declaration) {
                        declarations.push(declaration);
                    }
                }

                Reference::Ambiguous(declarations)
            }
            Self::Missing => Reference::Missing,
        }
    }
}

impl From<ExportResolution> for ImportResolution {
    /// Convert one exact imported target into a resolution.
    fn from(resolution: ExportResolution) -> Self {
        Self::Resolved(resolution)
    }
}
