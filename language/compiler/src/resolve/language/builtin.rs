use std::collections::HashSet;
use std::sync::Arc;

use destack_builtin::{LanguageSymbol, builtin_library};
use destack_core::StringId;
use destack_dir::{
    GlobalSymbolId, LocalSymbolId, StaticKey, SymbolSpace, SymbolSpaceOrder, WellKnownSymbol,
};
use destack_source::ModuleId;
use destack_workspace::{
    ArtifactKey, CanonicalStaticKey, LanguageEnvironment, LibraryEnvironment, ProfileId,
    WellKnownSymbols,
};

use crate::timing::tags;
use crate::{
    ArtifactRequirementCollector, ArtifactRequirementError, Compiler, ResolveError, ResolveResult,
};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Return true when one builtin lib module contributes to the early language surface.
    pub fn is_standard_lib_environment_module(&self, module_id: ModuleId) -> bool {
        let Some(builtins) = self.program.builtins.as_ref() else {
            return false;
        };
        let Some(lib_name) = builtins.library_name_for_module(module_id) else {
            return false;
        };
        let Some(lib) = builtin_library(lib_name) else {
            return false;
        };

        lib.name.starts_with("es")
            || matches!(
                lib.name,
                "js" | "native" | "globals" | "decorators" | "decorators.legacy"
            )
    }

    /// Return one locally exported symbol from a module base DIR.
    fn local_exported_symbol_from_base(
        &self,
        module_id: ModuleId,
        export_name: StringId,
        order: SymbolSpaceOrder,
    ) -> Option<GlobalSymbolId> {
        let dir = self.artifact_dir_base(module_id)?;
        let symbols = &dir.symbols;

        // collect the first exported symbol for each eligible space
        let mut value_symbol = None;
        let mut type_symbol = None;
        let mut type_value_symbol = None;

        for (index, symbol) in symbols.symbols().enumerate() {
            if symbol.export.is_none() {
                continue;
            }

            if symbol.name() != Some(export_name) {
                continue;
            }

            let local_id = LocalSymbolId::new_typed(index as u32, symbol.ty);
            let global_id = local_id.into_global(module_id);

            match symbol.space {
                SymbolSpace::Value if value_symbol.is_none() => {
                    value_symbol = Some(global_id);
                }
                SymbolSpace::Type if type_symbol.is_none() => {
                    type_symbol = Some(global_id);
                }
                SymbolSpace::TypeValue if type_value_symbol.is_none() => {
                    type_value_symbol = Some(global_id);
                }
                _ => {}
            }
        }

        // select the first matching export for the requested order
        for space in order.spaces() {
            match space {
                SymbolSpace::Value => {
                    if let Some(symbol_id) = value_symbol.or(type_value_symbol) {
                        return Some(symbol_id);
                    }
                }
                SymbolSpace::Type => {
                    if let Some(symbol_id) = type_symbol.or(type_value_symbol) {
                        return Some(symbol_id);
                    }
                }
                SymbolSpace::TypeValue => {
                    if let Some(symbol_id) = type_value_symbol.or(type_symbol).or(value_symbol) {
                        return Some(symbol_id);
                    }
                }
                SymbolSpace::Label => {}
            }
        }

        None
    }

    /// Return the language environment for one profile.
    pub fn language_environment(&self, profile: ProfileId) -> Option<Arc<LanguageEnvironment>> {
        self.program.artifacts.language_environment(profile)
    }

    /// Return the library environment for one profile.
    pub fn library_environment(&self, profile: ProfileId) -> Option<Arc<LibraryEnvironment>> {
        self.program.artifacts.library_environment(profile)
    }

    /// Return the selected lib modules for one profile.
    pub fn selected_library_modules(&self, profile: ProfileId) -> Vec<ModuleId> {
        if let Some(environment) = self.library_environment(profile) {
            return environment.supporting_modules();
        }

        self.selected_library_modules_from_input(profile)
            .unwrap_or_default()
    }

    /// Return the ambient lib modules for one profile.
    pub fn ambient_library_modules(&self, profile: ProfileId) -> Vec<ModuleId> {
        if let Some(environment) = self.library_environment(profile) {
            return environment.ambient_modules.clone();
        }

        self.ambient_library_modules_from_input(profile)
            .unwrap_or_default()
    }

    /// Return the modules that support one global environment.
    pub fn library_environment_modules(&self, profile: ProfileId) -> Vec<ModuleId> {
        self.library_environment(profile)
            .map(|environment| environment.supporting_modules())
            .unwrap_or_default()
    }

    /// Return true when one module is ambient for one profile.
    pub fn is_ambient_library_module(&self, profile: ProfileId, module: ModuleId) -> bool {
        self.library_environment(profile)
            .is_some_and(|environment| environment.ambient_modules.contains(&module))
    }

    /// Return true when one module is selected in the library environment for one profile.
    pub fn is_selected_library_module(&self, profile: ProfileId, module: ModuleId) -> bool {
        self.selected_library_modules(profile).contains(&module)
    }

    /// Resolve the language environment for one profile.
    pub fn resolve_language_environment(
        &self,
        profile: ProfileId,
    ) -> ResolveResult<LanguageEnvironment> {
        if self.language_environment(profile).is_some() {
            return Ok(self
                .language_environment(profile)
                .unwrap_or_else(|| unreachable!())
                .as_ref()
                .clone());
        }

        let artifact_key = ArtifactKey::language_environment(profile);
        match self.load_language_environment_image(profile) {
            Ok(Some(environment)) => return Ok(environment),
            Ok(None) => {}
            Err(error) => {
                tracing::warn!(?error, ?artifact_key, "compiler.cache.image.load_failed");
            }
        }

        let Some(builtins) = self.program.builtins.as_ref() else {
            return Ok(LanguageEnvironment::default());
        };
        let _timing = self.timing_scope(tags::RESOLVE_BUILTINS);

        // bind builtin defining modules
        let mut collector = ArtifactRequirementCollector::new();
        for &module_id in builtins.intrinsic_module_by_path.values() {
            if module_id == builtins.prelude_module_id {
                continue;
            }
            if let Err(error) = self.require_dir_base(module_id)
                && let Some(error) = collector.try_collect::<(), _>(Err(error))
            {
                let requirement = error.into_requirement();
                return Err(ResolveError::UnsatisfiedRequirement { requirement });
            }
        }

        if let Some(requirement) = collector.try_into_requirement() {
            return Err(ResolveError::Yield { requirement });
        }

        // collect all language items from the builtin module local export surface
        let mut items = indexmap::IndexMap::new();
        let mut symbols = indexmap::IndexMap::new();
        for item in LanguageSymbol::all() {
            let module_id = builtins.module_for_item(item);
            let name_id = self.program.strings.intern(item.export_name());
            let export_spaces = SymbolSpaceOrder::ValueThenType;
            let Some(symbol_id) =
                self.local_exported_symbol_from_base(module_id, name_id, export_spaces)
            else {
                return Err(ResolveError::MissingLanguageSymbol { item });
            };

            items.insert(item, symbol_id);
            symbols.insert(item.export_name().to_string(), symbol_id);
        }

        Ok(LanguageEnvironment { items, symbols })
    }

    /// Ensure the language environment exists for one profile.
    pub fn require_language_environment(
        &self,
        profile: ProfileId,
    ) -> Result<(), ArtifactRequirementError> {
        self.require_artifact(ArtifactKey::language_environment(profile))
    }

    /// Ensure the library environment exists for one profile.
    pub fn require_library_environment(
        &self,
        profile: ProfileId,
    ) -> Result<(), ArtifactRequirementError> {
        self.require_artifact(ArtifactKey::library_environment(profile))
    }

    /// Get a required language item, returning an error if not found.
    pub fn require_language_symbol(
        &self,
        profile: ProfileId,
        item: LanguageSymbol,
    ) -> ResolveResult<GlobalSymbolId> {
        self.require_language_environment(profile)
            .map_err(ResolveError::from)?;

        self.get_language_symbol(profile, item)
            .ok_or(ResolveError::MissingLanguageSymbol { item })
    }

    /// Get a language item from the cache, returning None if not found.
    pub fn get_language_symbol(
        &self,
        profile: ProfileId,
        item: LanguageSymbol,
    ) -> Option<GlobalSymbolId> {
        self.language_environment(profile)?.item(item)
    }

    /// Get one builtin symbol by export name.
    pub fn get_builtin_symbol(&self, profile: ProfileId, name: StringId) -> Option<GlobalSymbolId> {
        let name = self.program.strings.get(name);
        self.language_environment(profile)?.symbol(name.as_ref())
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
        let name = self.program.strings.get(name);
        self.library_environment(profile_id)?
            .declared_symbol_from(name.as_ref(), order)
    }

    /// Get selected lib symbol sources for a profile, key, and space.
    pub fn get_lib_symbol_sources(
        &self,
        profile_id: ProfileId,
        key: StaticKey,
        space: SymbolSpace,
    ) -> Option<Vec<GlobalSymbolId>> {
        let key = CanonicalStaticKey::from_static_key(key, &self.program.strings);
        self.library_environment(profile_id)?
            .symbol_sources(&key, space)
            .cloned()
    }

    /// Get one selected lib symbol using a space order.
    pub fn get_lib_symbol_from(
        &self,
        profile_id: ProfileId,
        name: StringId,
        order: SymbolSpaceOrder,
    ) -> Option<GlobalSymbolId> {
        let environment = self.library_environment(profile_id)?;
        let name = self.program.strings.get(name);

        // prefer the declared lib surface first
        if let Some(symbol) = environment.declared_symbol_from(name.as_ref(), order) {
            return Some(symbol);
        }

        environment.symbol_from(name.as_ref(), order)
    }

    /// Get selected lib symbol sources for merge (includes type-value sources).
    pub fn get_lib_symbol_sources_for_merge(
        &self,
        profile_id: ProfileId,
        key: StaticKey,
        space: SymbolSpace,
    ) -> Option<Vec<GlobalSymbolId>> {
        let mut sources = Vec::new();
        let mut seen = HashSet::new();
        let mut push_sources = |space| {
            if let Some(group) = self.get_lib_symbol_sources(profile_id, key, space) {
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

    /// Get selected lib symbol sources for a profile, key, and space order.
    pub fn get_lib_symbol_sources_for_space_order(
        &self,
        profile_id: ProfileId,
        key: StaticKey,
        order: SymbolSpaceOrder,
    ) -> Option<Vec<GlobalSymbolId>> {
        let mut sources = Vec::new();
        let mut seen = HashSet::new();
        let mut push_sources = |space| {
            if let Some(group) = self.get_lib_symbol_sources(profile_id, key, space) {
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
        Some(
            self.library_environment(profile_id)?
                .well_known_symbols
                .clone(),
        )
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

        self.library_environment(profile_id)?
            .declared_symbol_from(symbol.export_name(), order)
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
