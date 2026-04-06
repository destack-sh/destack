use std::collections::HashSet;

use destack_dir::{GlobalSymbolId, StaticKey, SymbolSpace, SymbolTable};
use destack_source::ModuleId;
use destack_workspace::{Module, ProfileId};

use super::super::AnalyzeResult;
use crate::Compiler;
use crate::analyze::common::{AnalyzeIndex, GlobalMergeSourcesKey, ModuleSymbolView};

/// Select merge source categories for global declaration merging.
#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub(crate) enum GlobalMergeCategory {
    /// Merge instance side declarations, like interface and class instance members.
    Instance,
    /// Merge value side declarations, like namespace and class static members.
    Value,
    /// Merge both instance and value declaration spaces.
    Any,
}

impl Compiler {
    /// Normalize one merge source symbol against declared symbol facts.
    fn normalize_declared_merge_source_symbol(
        &self,
        index: &AnalyzeIndex,
        revision: destack_workspace::Revision,
        current_module_id: ModuleId,
        current_symbols: &SymbolTable,
        profile: ProfileId,
        symbol: GlobalSymbolId,
    ) -> AnalyzeResult<GlobalSymbolId> {
        // use local declared facts for the current module
        if symbol.module_id == current_module_id {
            let owner_symbol = current_symbols.get_symbol(symbol.local_id);
            return Ok(GlobalSymbolId::new(
                symbol.module_id,
                symbol.local_id.with_type(owner_symbol.ty),
            ));
        }

        // otherwise read the published declared artifact once
        let owner_dir =
            self.require_indexed_dir_declared(index, revision, symbol.module_id, profile)?;

        let owner_symbol = owner_dir.symbols.get_symbol(symbol.local_id);
        Ok(GlobalSymbolId::new(
            symbol.module_id,
            symbol.local_id.with_type(owner_symbol.ty),
        ))
    }

    /// Normalize one merge candidate group while preserving source order.
    fn collect_normalized_merge_sources(
        &self,
        index: &AnalyzeIndex,
        revision: destack_workspace::Revision,
        current_module_id: ModuleId,
        current_symbols: &SymbolTable,
        profile: ProfileId,
        raw_symbols: impl IntoIterator<Item = GlobalSymbolId>,
    ) -> AnalyzeResult<Vec<GlobalSymbolId>> {
        // normalize and dedupe the source set
        let mut normalized_symbols = Vec::new();
        let mut seen = HashSet::new();
        for symbol in raw_symbols {
            let normalized_symbol = self.normalize_declared_merge_source_symbol(
                index,
                revision,
                current_module_id,
                current_symbols,
                profile,
                symbol,
            )?;
            if seen.insert(normalized_symbol) {
                normalized_symbols.push(normalized_symbol);
            }
        }

        Ok(normalized_symbols)
    }

    /// Collect global and ambient merge sources for a symbol key and category.
    pub(crate) fn collect_global_merge_sources_for_key(
        &self,
        revision: destack_workspace::Revision,
        module: &Module,
        index: &AnalyzeIndex,
        current_symbols: &SymbolTable,
        profile: ProfileId,
        key: StaticKey,
        anchor_space: SymbolSpace,
        category: GlobalMergeCategory,
    ) -> AnalyzeResult<Vec<GlobalSymbolId>> {
        let cache_key = GlobalMergeSourcesKey {
            module: module.id,
            profile,
            key,
            anchor_space,
            category,
        };

        // collect raw merge candidates from global and library facts
        let raw_symbols = if let Some(raw_symbols) = index.global_merge_sources(cache_key) {
            raw_symbols
        } else {
            let mut raw_symbols = Vec::new();
            for space in self.global_merge_spaces_for_category(anchor_space, category) {
                if let Some(group) =
                    self.get_global_symbol_group(revision, module.id, profile, key, *space)
                {
                    raw_symbols.extend(group);
                }
            }

            // include library merge candidates for user modules
            if !self.module_is_ambient_lib(module) {
                for space in self.global_merge_spaces_for_category(anchor_space, category) {
                    if let Some(group) =
                        self.get_library_symbol_sources_for_merge(profile, key, *space)
                    {
                        raw_symbols.extend(group);
                    }
                }
            }

            index.set_global_merge_sources(cache_key, raw_symbols.clone());
            raw_symbols
        };

        self.collect_normalized_merge_sources(
            index,
            revision,
            module.id,
            current_symbols,
            profile,
            raw_symbols,
        )
    }

    /// Select one preferred type-space carrier symbol for a merge key.
    pub(crate) fn select_preferred_type_carrier_symbol(
        &self,
        revision: destack_workspace::Revision,
        module: &Module,
        index: &AnalyzeIndex,
        current_symbols: &SymbolTable,
        profile: ProfileId,
        key: StaticKey,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        // normalize candidates before canonical ordering
        let mut normalized_symbols = self.collect_global_merge_sources_for_key(
            revision,
            module,
            index,
            current_symbols,
            profile,
            key,
            SymbolSpace::Type,
            GlobalMergeCategory::Instance,
        )?;

        // keep value-only symbols out of the canonical type carrier search
        normalized_symbols.retain(|symbol| {
            if symbol.module_id == module.id {
                let owner_symbol = current_symbols.get_symbol(symbol.local_id);
                return owner_symbol.space == SymbolSpace::Type
                    || owner_symbol.space == SymbolSpace::TypeValue;
            }

            if let Some(owner_dir) = index.declared_directory(symbol.module_id, profile) {
                let owner_symbol = owner_dir.symbols.get_symbol(symbol.local_id);
                return owner_symbol.space == SymbolSpace::Type
                    || owner_symbol.space == SymbolSpace::TypeValue;
            }

            true
        });

        // prefer local-module carriers, then canonical global order
        normalized_symbols.sort_unstable_by_key(|symbol| (symbol.module_id != module.id, *symbol));
        Ok(normalized_symbols.into_iter().next())
    }

    /// Remap one symbol from typevalue space to canonical type-space carrier.
    pub(crate) fn remap_typevalue_symbol_to_type_space(
        &self,
        view: ModuleSymbolView<'_>,
        index: &AnalyzeIndex,
        symbol: GlobalSymbolId,
    ) -> AnalyzeResult<GlobalSymbolId> {
        let normalize_local_symbol =
            |candidate| self.normalize_reference_symbol_id(view, candidate);
        let symbol = normalize_local_symbol(symbol);

        // normalize symbol typing first
        let (owner_space, owner_key) = if symbol.module_id == view.module.id {
            let owner_symbol = view.symbols.get_symbol(symbol.local_id);
            (owner_symbol.space, owner_symbol.key)
        } else {
            let owner_dir = self.require_indexed_dir_declared(
                index,
                view.compiler_context.revision(),
                symbol.module_id,
                view.profile,
            )?;
            let owner_symbol = owner_dir.symbols.get_symbol(symbol.local_id);
            (owner_symbol.space, owner_symbol.key)
        };
        if owner_space != SymbolSpace::TypeValue {
            return Ok(symbol);
        }

        let Some(key) = owner_key else {
            return Ok(symbol);
        };
        if let Some(candidate) = self.select_preferred_type_carrier_symbol(
            view.compiler_context.revision(),
            view.module,
            index,
            view.symbols,
            view.profile,
            key,
        )? {
            Ok(normalize_local_symbol(candidate))
        } else {
            Ok(symbol)
        }
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
