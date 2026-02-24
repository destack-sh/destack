use std::collections::HashSet;

use destack_base::StringId;
use destack_builtin::LanguageSymbol;
use destack_dir::{GlobalSymbolId, StaticKey, SymbolSpace, SymbolSpaceOrder, WellKnownSymbol};
use destack_workspace::{ProfileId, WellKnownSymbols};

use crate::timing::tags;
use crate::{Compiler, ResolveError, ResolveResult};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Resolve all builtin modules and required language items.
    pub fn resolve_builtins(&self, profile: ProfileId) -> ResolveResult<()> {
        let Some(builtins) = self.program.builtins.as_ref() else {
            return Ok(());
        };
        let _timing = self.timing_scope(tags::RESOLVE_BUILTINS);

        // resolve prelude/core modules
        self.require_resolve_module(builtins.prelude_module_id, profile)?;
        for &module_id in builtins.core_module_by_path.values() {
            self.require_resolve_module(module_id, profile)?;
        }

        // resolve all language items
        for item in LanguageSymbol::all() {
            self.require_language_symbol(profile, item)?;
        }

        Ok(())
    }

    /// Get a required language item, returning an error if not found.
    pub fn require_language_symbol(
        &self,
        profile: ProfileId,
        item: LanguageSymbol,
    ) -> ResolveResult<GlobalSymbolId> {
        // check if builtins are available
        let Some(builtins) = self.program.builtins.as_ref() else {
            return Err(ResolveError::MissingLanguageSymbol { item });
        };

        // check cache
        if let Some(cached) = builtins.items.get(&(profile, item)) {
            return Ok(*cached);
        }

        // resolve the module for this language item
        let module_id = builtins.module_for_item(item);
        self.require_resolve_module(module_id, profile)?;

        // resolve the symbol in the module
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let dir = module.dir(profile);

        // find the symbol in the module's export table
        let name_id = self.program.strings.intern(item.export_name());
        let key = StaticKey::Name(name_id);
        let exports = dir.exported_symbols.read();
        let tree = dir.tree.read();
        let export_spaces = SymbolSpaceOrder::ValueThenType;
        let Some(symbol_id) =
            self.resolve_exported_symbol(&module, profile, &exports, &tree, export_spaces, key)
        else {
            return Err(ResolveError::MissingLanguageSymbol { item });
        };

        // result
        builtins.items.insert((profile, item), symbol_id);
        Ok(symbol_id)
    }

    /// Get a language item from the cache, returning None if not found.
    pub fn get_language_symbol(
        &self,
        profile: ProfileId,
        item: LanguageSymbol,
    ) -> Option<GlobalSymbolId> {
        let builtins = self.program.builtins.as_ref()?;
        builtins.items.get(&(profile, item)).map(|r| *r)
    }

    /// Get a language item from the cache, panicking if not found.
    pub fn language_symbol(&self, profile: ProfileId, item: LanguageSymbol) -> GlobalSymbolId {
        self.get_language_symbol(profile, item)
            .unwrap_or_else(|| panic!("language item {item:?} not available"))
    }

    /// Get a cached declared lib symbol for a profile and name.
    pub fn get_declared_lib_symbol(
        &self,
        profile_id: ProfileId,
        name: StringId,
    ) -> Option<GlobalSymbolId> {
        self.get_declared_lib_symbol_from(profile_id, name, SymbolSpaceOrder::ValueThenType)
    }

    /// Get a cached declared lib symbol for a profile, name, and space order.
    pub fn get_declared_lib_symbol_from(
        &self,
        profile_id: ProfileId,
        name: StringId,
        order: SymbolSpaceOrder,
    ) -> Option<GlobalSymbolId> {
        let builtins = self.program.builtins.as_ref()?;
        let profile = self.program.profile(profile_id);
        builtins.get_declared_lib_symbol_from(&profile.key, name, order)
    }

    /// Get ambient lib symbol sources for a profile, key, and space.
    pub fn get_ambient_lib_symbol_sources(
        &self,
        profile_id: ProfileId,
        key: StaticKey,
        space: SymbolSpace,
    ) -> Option<Vec<GlobalSymbolId>> {
        let builtins = self.program.builtins.as_ref()?;
        let profile = self.program.profile(profile_id);
        builtins.get_ambient_lib_symbol_sources(&profile.key, key, space)
    }

    /// Get ambient lib symbol sources for merge (includes type-value sources).
    pub fn get_ambient_lib_symbol_sources_for_merge(
        &self,
        profile_id: ProfileId,
        key: StaticKey,
        space: SymbolSpace,
    ) -> Option<Vec<GlobalSymbolId>> {
        let mut sources = Vec::new();
        let mut seen = HashSet::new();
        let mut push_sources = |space| {
            if let Some(group) = self.get_ambient_lib_symbol_sources(profile_id, key, space) {
                for symbol in group {
                    if seen.insert(symbol) {
                        sources.push(symbol);
                    }
                }
            }
        };

        match space {
            SymbolSpace::Type => {
                push_sources(SymbolSpace::Type);
                push_sources(SymbolSpace::TypeValue);
            }
            SymbolSpace::Value => {
                push_sources(SymbolSpace::Value);
                push_sources(SymbolSpace::TypeValue);
            }
            SymbolSpace::TypeValue => {
                push_sources(SymbolSpace::Type);
                push_sources(SymbolSpace::Value);
                push_sources(SymbolSpace::TypeValue);
            }
            SymbolSpace::Label => {}
        }

        if sources.is_empty() {
            None
        } else {
            Some(sources)
        }
    }

    /// Get ambient lib symbol sources for a profile, key, and space order.
    pub fn get_ambient_lib_symbol_sources_for_space_order(
        &self,
        profile_id: ProfileId,
        key: StaticKey,
        order: SymbolSpaceOrder,
    ) -> Option<Vec<GlobalSymbolId>> {
        let mut sources = Vec::new();
        let mut seen = HashSet::new();
        let mut push_sources = |space| {
            if let Some(group) = self.get_ambient_lib_symbol_sources(profile_id, key, space) {
                for symbol in group {
                    if seen.insert(symbol) {
                        sources.push(symbol);
                    }
                }
            }
        };

        for space in order.spaces() {
            push_sources(*space);
            if matches!(space, SymbolSpace::Type | SymbolSpace::Value) {
                push_sources(SymbolSpace::TypeValue);
            }
        }

        if sources.is_empty() {
            None
        } else {
            Some(sources)
        }
    }

    /// Get a declared lib symbol from the cache, panicking if not found.
    pub fn declared_lib_symbol(&self, profile_id: ProfileId, name: StringId) -> GlobalSymbolId {
        self.get_declared_lib_symbol(profile_id, name)
            .unwrap_or_else(|| {
                let name = self.program.strings.get(name);
                panic!("declared lib symbol '{}' not available", name.as_ref())
            })
    }

    /// Get well-known symbols for a profile.
    pub fn get_well_known_symbols(&self, profile_id: ProfileId) -> Option<WellKnownSymbols> {
        let builtins = self.program.builtins.as_ref()?;
        let profile = self.program.profile(profile_id);
        builtins.well_known_symbols(&profile.key)
    }

    /// Get well-known symbols for a profile, panicking if not found.
    pub fn well_known_symbols(&self, profile_id: ProfileId) -> WellKnownSymbols {
        self.get_well_known_symbols(profile_id).unwrap_or_else(|| {
            panic!("well-known symbols not available for profile {profile_id:?}")
        })
    }

    /// Get a specific well-known symbol for a profile (value-space preferred).
    pub fn get_well_known_symbol(
        &self,
        profile_id: ProfileId,
        symbol: WellKnownSymbol,
    ) -> Option<GlobalSymbolId> {
        let well_known_symbols = self.get_well_known_symbols(profile_id)?;
        well_known_symbols.get_symbol(symbol)
    }

    /// Get a specific well-known symbol from the type space.
    pub fn get_well_known_type_symbol(
        &self,
        profile_id: ProfileId,
        symbol: WellKnownSymbol,
    ) -> Option<GlobalSymbolId> {
        let well_known_symbols = self.get_well_known_symbols(profile_id)?;
        well_known_symbols.get_type_symbol(symbol)
    }

    /// Get a specific well-known symbol using a space order preference.
    pub fn get_well_known_symbol_from(
        &self,
        profile_id: ProfileId,
        symbol: WellKnownSymbol,
        order: SymbolSpaceOrder,
    ) -> Option<GlobalSymbolId> {
        if let Some(well_known_symbols) = self.get_well_known_symbols(profile_id)
            && let Some(symbol_id) = well_known_symbols
                .get_group(symbol)
                .and_then(|group| group.symbol_for_space_order(order))
        {
            return Some(symbol_id);
        }

        let name = self.program.strings.intern(symbol.export_name());
        self.get_declared_lib_symbol_from(profile_id, name, order)
    }

    /// Get a specific well-known symbol for a profile, panicking if not found.
    pub fn well_known_symbol(
        &self,
        profile_id: ProfileId,
        symbol: WellKnownSymbol,
    ) -> GlobalSymbolId {
        self.get_well_known_symbol(profile_id, symbol)
            .unwrap_or_else(|| {
                panic!("well-known symbol {symbol:?} not available for profile {profile_id:?}")
            })
    }

    /// Check if a symbol matches a well-known symbol (checking both type and value spaces).
    pub fn is_well_known_symbol(
        &self,
        profile_id: ProfileId,
        symbol: GlobalSymbolId,
        well_known: WellKnownSymbol,
    ) -> bool {
        self.get_well_known_symbol(profile_id, well_known)
            .is_some_and(|s| s == symbol)
            || self
                .get_well_known_type_symbol(profile_id, well_known)
                .is_some_and(|s| s == symbol)
    }
}
