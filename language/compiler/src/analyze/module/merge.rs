use std::collections::HashSet;

use destack_dir::{GlobalSymbolId, StaticKey, SymbolSpace};
use destack_workspace::{Module, ProfileId};

use super::DirReadBoundary;
use crate::analyze::common::ModuleSymbolView;
use crate::{BuildRequirementError, Compiler};

/// Select merge source categories for global declaration merging.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum GlobalMergeCategory {
    /// Merge instance side declarations, like interface and class instance members.
    Instance,
    /// Merge value side declarations, like namespace and class static members.
    Value,
    /// Merge both instance and value declaration spaces.
    Any,
}

impl Compiler {
    /// Normalize a merge source symbol to its declared symbol type.
    fn normalize_merge_source_symbol_type(
        &self,
        profile: ProfileId,
        symbol: GlobalSymbolId,
    ) -> GlobalSymbolId {
        // read the owner symbol metadata
        let owner_module = self.program.modules.get(symbol.module_id);
        let owner_module = owner_module.read();
        let owner_symbols = owner_module.dir(profile).symbols.read();
        let owner_symbol = owner_symbols.get_symbol(symbol.local_id);

        // normalize to the owner-declared symbol type
        GlobalSymbolId::new(symbol.module_id, symbol.local_id.with_type(owner_symbol.ty))
    }

    /// Collect global and ambient merge sources for a symbol key and category.
    pub(crate) fn collect_global_merge_sources_for_key(
        &self,
        module: &Module,
        profile: ProfileId,
        key: StaticKey,
        anchor_space: SymbolSpace,
        category: GlobalMergeCategory,
    ) -> Vec<GlobalSymbolId> {
        // collect global symbols from compatible spaces
        let mut symbols = Vec::new();
        for space in self.global_merge_spaces_for_category(anchor_space, category) {
            if let Some(group) = self.get_global_symbol_group(module.id, profile, key, *space) {
                symbols.extend(group);
            }
        }

        // collect ambient symbols from compatible spaces for user modules
        if !self.module_is_ambient_lib(module) {
            for space in self.global_merge_spaces_for_category(anchor_space, category) {
                if let Some(group) = self.get_lib_symbol_sources_for_merge(profile, key, *space) {
                    symbols.extend(group);
                }
            }
        }

        // normalize symbol typing before deduplication
        let symbols = symbols
            .into_iter()
            .map(|symbol| self.normalize_merge_source_symbol_type(profile, symbol))
            .collect::<Vec<_>>();

        // keep stable order while deduplicating
        let mut seen = HashSet::new();
        let mut deduped = Vec::with_capacity(symbols.len());
        for symbol in symbols {
            if seen.insert(symbol) {
                deduped.push(symbol);
            }
        }

        deduped
    }

    /// Select one canonical type-space symbol for a merge key.
    pub(crate) fn select_canonical_type_symbol(
        &self,
        module: &Module,
        profile: ProfileId,
        key: StaticKey,
    ) -> Option<GlobalSymbolId> {
        // collect type-space candidates from global and ambient sources
        let mut candidates = Vec::new();
        if let Some(group) =
            self.get_global_symbol_group(module.id, profile, key, SymbolSpace::Type)
        {
            candidates.extend(group);
        }
        if let Some(group) = self.get_lib_symbol_sources_for_merge(profile, key, SymbolSpace::Type)
        {
            candidates.extend(group);
        }

        // normalize and dedupe candidates first
        let mut normalized = candidates
            .into_iter()
            .map(|symbol| self.normalize_merge_source_symbol_type(profile, symbol))
            .collect::<Vec<_>>();
        normalized.sort_unstable();
        normalized.dedup();

        // prefer local-module carriers, then canonical global order
        normalized.sort_unstable_by_key(|symbol| (symbol.module_id != module.id, *symbol));
        normalized.into_iter().next()
    }

    /// Remap one symbol from typevalue space to canonical type-space carrier.
    pub(crate) fn remap_typevalue_symbol_to_type_space(
        &self,
        module: &Module,
        profile: ProfileId,
        symbol: GlobalSymbolId,
    ) -> Result<GlobalSymbolId, BuildRequirementError> {
        let module_symbols = module.dir(profile).symbols.read();
        let view = ModuleSymbolView::new(module, profile, &module_symbols);

        // normalize symbol typing first
        let symbol = self.normalize_reference_symbol_id(view, symbol);

        self.with_module_symbols_at_boundary(
            module,
            profile,
            symbol.module_id,
            DirReadBoundary::Declared,
            |_, owner_symbols| {
                let owner_symbol = owner_symbols.get_symbol(symbol.local_id);
                if owner_symbol.space != SymbolSpace::TypeValue {
                    return symbol;
                }

                let Some(key) = owner_symbol.key else {
                    return symbol;
                };
                if let Some(candidate) = self.select_canonical_type_symbol(module, profile, key) {
                    self.normalize_reference_symbol_id(view, candidate)
                } else {
                    symbol
                }
            },
        )
    }

    /// Return compatible symbol spaces for one merge source category.
    pub(crate) fn global_merge_spaces_for_category(
        &self,
        anchor_space: SymbolSpace,
        category: GlobalMergeCategory,
    ) -> &'static [SymbolSpace] {
        const INSTANCE_TYPE_SPACES: [SymbolSpace; 1] = [SymbolSpace::Type];
        const INSTANCE_TYPE_VALUE_SPACES: [SymbolSpace; 2] =
            [SymbolSpace::Type, SymbolSpace::TypeValue];
        const VALUE_TYPE_VALUE_SPACES: [SymbolSpace; 2] =
            [SymbolSpace::TypeValue, SymbolSpace::Value];
        const VALUE_ONLY_SPACES: [SymbolSpace; 1] = [SymbolSpace::Value];
        const ANY_TYPE_VALUE_SPACES: [SymbolSpace; 3] = [
            SymbolSpace::Type,
            SymbolSpace::Value,
            SymbolSpace::TypeValue,
        ];
        const EMPTY_SPACES: [SymbolSpace; 0] = [];

        match (anchor_space, category) {
            (SymbolSpace::Label, _) => &EMPTY_SPACES,
            (SymbolSpace::Type, GlobalMergeCategory::Instance) => &INSTANCE_TYPE_SPACES,
            (SymbolSpace::TypeValue, GlobalMergeCategory::Instance) => &INSTANCE_TYPE_VALUE_SPACES,
            (SymbolSpace::Value, GlobalMergeCategory::Instance) => &EMPTY_SPACES,
            (SymbolSpace::TypeValue, GlobalMergeCategory::Value) => &VALUE_TYPE_VALUE_SPACES,
            (SymbolSpace::Type, GlobalMergeCategory::Value) => &VALUE_ONLY_SPACES,
            (SymbolSpace::Value, GlobalMergeCategory::Value) => &VALUE_ONLY_SPACES,
            (_, GlobalMergeCategory::Any) => &ANY_TYPE_VALUE_SPACES,
        }
    }
}
