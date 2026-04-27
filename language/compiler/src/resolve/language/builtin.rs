use std::sync::Arc;

use destack_artifact::{ArtifactKey, CanonicalStaticKey, LanguageEnvironment, WellKnownSymbols};
use destack_builtin::{LanguageSymbol, builtin_library};
use destack_core::StringId;
use destack_dir::{
    GlobalSymbolId, LocalSymbolId, StaticKey, SymbolSpace, SymbolSpaceOrder, WellKnownSymbol,
};
use destack_source::ModuleId;
use destack_workspace::{BuiltinLibrarySelection, ProfileId, Revision};
use std::collections::HashSet;

use crate::timing::tags;
use crate::{Compiler, RequirementCollector, RequirementError, ResolveError, ResolveResult};

#[allow(clippy::too_many_arguments)]
#[allow(dead_code)]
impl Compiler {
    /// Return true when one builtin library module contributes to the early language surface.
    pub fn is_standard_library_environment_module(&self, module_id: ModuleId) -> bool {
        let builtins = self.repository.builtins.as_ref();
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

    /// Return the selected library modules for one profile.
    pub(crate) fn selected_library_modules(&self, profile: ProfileId) -> Arc<[ModuleId]> {
        self.selected_library_modules_from_input(profile)
            .unwrap_or_else(|_| Arc::<[ModuleId]>::from([]))
    }

    /// Return the builtin library selection derived from current profile input.
    pub(crate) fn builtin_library_selection_from_input(
        &self,
        _revision: destack_workspace::Revision,
        profile_id: ProfileId,
    ) -> ResolveResult<BuiltinLibrarySelection> {
        if !self.options.load_libraries {
            return Ok(BuiltinLibrarySelection::default());
        }

        let builtins = self.repository.builtins.as_ref();

        let profile_key = self.profile(profile_id).key.clone();
        self.builtin_library_selection(builtins, &profile_key)
    }

    /// Return the modules that support one global environment.
    pub(crate) fn library_environment_modules(&self, profile: ProfileId) -> Vec<ModuleId> {
        self.selected_library_modules(profile)
            .iter()
            .copied()
            .collect()
    }

    /// Return true when one module is ambient for one profile.
    pub(crate) fn is_ambient_library_module(&self, profile: ProfileId, module: ModuleId) -> bool {
        self.ambient_library_modules_from_input(profile)
            .map(|modules| modules.contains(&module))
            .unwrap_or(false)
    }

    /// Return true when one module is selected in the library environment for one profile.
    pub(crate) fn is_selected_library_module(&self, profile: ProfileId, module: ModuleId) -> bool {
        self.selected_library_modules(profile).contains(&module)
    }

    /// Resolve the language environment for one profile.
    pub(crate) fn resolve_language_environment(
        &self,
        revision: destack_workspace::Revision,
        profile: ProfileId,
    ) -> ResolveResult<LanguageEnvironment> {
        if let Some(environment) = self.repository.language_environment(revision, profile) {
            return Ok(environment.as_ref().clone());
        }

        let artifact_key = ArtifactKey::language_environment(profile);
        match self.load_language_environment_image(revision, profile) {
            Ok(Some(environment)) => return Ok(environment),
            Ok(None) => {}
            Err(error) => {
                tracing::warn!(?error, ?artifact_key, "compiler.cache.image.load_failed");
            }
        }

        let builtins = self.repository.builtins.as_ref();
        let _timing = self.timing_scope(tags::RESOLVE_BUILTINS);

        // bind builtin defining modules
        let mut collector = RequirementCollector::new();
        let prelude_module_id = builtins.prelude_module_id();

        for module_id in builtins.registered_intrinsic_module_ids() {
            if module_id == prelude_module_id {
                continue;
            }
            if let Err(error) = self.require_dir_base(revision, module_id)
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
            let name_id = self.repository.strings.intern(item.export_name());
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
        revision: Revision,
        profile: ProfileId,
    ) -> Result<(), RequirementError> {
        self.require_artifact(revision, ArtifactKey::language_environment(profile))
    }

    /// Ensure the library environment exists for one profile.
    pub fn require_library_environment(
        &self,
        revision: Revision,
        profile: ProfileId,
    ) -> Result<(), RequirementError> {
        self.require_artifact(revision, ArtifactKey::library_environment(profile))
    }

    /// Ensure the selected library environment exists when the profile uses libraries.
    pub(crate) fn require_selected_library_environment(
        &self,
        revision: Revision,
        profile: ProfileId,
    ) -> Result<(), RequirementError> {
        if self.selected_library_modules(profile).is_empty() {
            return Ok(());
        }

        self.require_library_environment(revision, profile)
    }

    /// Get a language item from the cache, returning None if not found.
    pub(crate) fn get_language_symbol(
        &self,
        profile: ProfileId,
        item: LanguageSymbol,
    ) -> Option<GlobalSymbolId> {
        self.language_environment(profile)?.item(item)
    }

    /// Get one builtin symbol by export name.
    pub(crate) fn get_builtin_symbol(
        &self,
        profile: ProfileId,
        name: StringId,
    ) -> Option<GlobalSymbolId> {
        let name = self.repository.strings.get(name);
        self.language_environment(profile)?.symbol(name.as_ref())
    }

    /// Get a language item from the cache, panicking if not found.
    pub(crate) fn language_symbol(
        &self,
        profile: ProfileId,
        item: LanguageSymbol,
    ) -> GlobalSymbolId {
        self.get_language_symbol(profile, item)
            .unwrap_or_else(|| panic!("language item {item:?} not available"))
    }

    /// Get a cached declared library symbol for a profile and name.
    pub(crate) fn get_declared_library_symbol(
        &self,
        profile_id: ProfileId,
        name: StringId,
    ) -> Option<GlobalSymbolId> {
        self.get_declared_library_symbol_from(profile_id, name, SymbolSpaceOrder::ValueThenType)
    }

    /// Get a cached declared library symbol for a profile, name, and space order.
    pub(crate) fn get_declared_library_symbol_from(
        &self,
        profile_id: ProfileId,
        name: StringId,
        order: SymbolSpaceOrder,
    ) -> Option<GlobalSymbolId> {
        let name = self.repository.strings.get(name);
        let key = CanonicalStaticKey::Name(name.to_string());
        self.library_environment(profile_id)?
            .declared_symbol_from_key(&key, order)
    }

    /// Get one declared library symbol that must use the concrete runtime declaration.
    ///
    /// This only selects one symbol id from the cached library environment.
    /// Callers that need declaration artifacts must require `DirDeclared` separately.
    pub(crate) fn get_declared_concrete_library_symbol_from(
        &self,
        profile_id: ProfileId,
        name: StringId,
        order: SymbolSpaceOrder,
    ) -> Option<GlobalSymbolId> {
        let name = self.repository.strings.get(name);
        let key = CanonicalStaticKey::Name(name.to_string());
        self.library_environment(profile_id)?
            .declared_concrete_symbol_from_key(&key, order)
    }

    /// Get selected library symbol sources for a profile, key, and space.
    pub(crate) fn get_library_symbol_sources(
        &self,
        profile_id: ProfileId,
        key: StaticKey,
        space: SymbolSpace,
    ) -> Option<Vec<GlobalSymbolId>> {
        let key = CanonicalStaticKey::from_static_key(key, &self.repository.strings);
        self.library_environment(profile_id)?
            .symbol_sources(&key, space)
            .map(<[GlobalSymbolId]>::to_vec)
    }

    /// Get one selected library symbol using a space order.
    pub(crate) fn get_library_symbol_from(
        &self,
        profile_id: ProfileId,
        name: StringId,
        order: SymbolSpaceOrder,
    ) -> Option<GlobalSymbolId> {
        let name = self.repository.strings.get(name);
        let key = CanonicalStaticKey::Name(name.to_string());
        let environment = self.library_environment(profile_id)?;

        // prefer the declared library surface first
        if let Some(symbol) = environment.declared_symbol_from_key(&key, order) {
            return Some(symbol);
        }

        environment.symbol_from_key(&key, order)
    }

    /// Get selected library symbol sources for merge (includes type-value sources).
    pub(crate) fn get_library_symbol_sources_for_merge(
        &self,
        profile_id: ProfileId,
        key: StaticKey,
        space: SymbolSpace,
    ) -> Option<Vec<GlobalSymbolId>> {
        let mut sources = Vec::new();
        let mut seen = HashSet::new();
        let mut push_sources = |space| {
            if let Some(group) = self.get_library_symbol_sources(profile_id, key, space) {
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

    /// Get selected library symbol sources for a profile, key, and space order.
    pub(crate) fn get_library_symbol_sources_for_space_order(
        &self,
        profile_id: ProfileId,
        key: StaticKey,
        order: SymbolSpaceOrder,
    ) -> Option<Vec<GlobalSymbolId>> {
        let key = CanonicalStaticKey::from_static_key(key, &self.repository.strings);
        let environment = self.library_environment(profile_id)?;
        let sources = environment.symbol_sources_for_space_order(&key, order);

        (!sources.is_empty()).then_some(sources)
    }

    /// Get a declared library symbol from the cache, panicking if not found.
    pub fn declared_library_symbol(&self, profile_id: ProfileId, name: StringId) -> GlobalSymbolId {
        self.get_declared_library_symbol(profile_id, name)
            .unwrap_or_else(|| {
                let name = self.repository.strings.get(name);
                panic!("declared library symbol '{}' not available", name.as_ref())
            })
    }

    /// Get well-known symbols for a profile.
    pub fn get_well_known_symbols(&self, profile_id: ProfileId) -> Option<WellKnownSymbols> {
        Some(self.library_environment(profile_id)?.well_known_symbols())
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
        let well_known_symbols = self.library_environment(profile_id)?;
        let well_known_symbols = well_known_symbols.well_known_symbols();
        well_known_symbols.get_symbol(symbol)
    }

    /// Get a specific well-known symbol from the type space.
    pub fn get_well_known_type_symbol(
        &self,
        profile_id: ProfileId,
        symbol: WellKnownSymbol,
    ) -> Option<GlobalSymbolId> {
        let well_known_symbols = self.library_environment(profile_id)?;
        let well_known_symbols = well_known_symbols.well_known_symbols();
        well_known_symbols.get_type_symbol(symbol)
    }

    /// Get a specific well-known symbol using a space order preference.
    pub fn get_well_known_symbol_from(
        &self,
        profile_id: ProfileId,
        symbol: WellKnownSymbol,
        order: SymbolSpaceOrder,
    ) -> Option<GlobalSymbolId> {
        let symbol_name = self.repository.strings.intern(symbol.export_name());
        self.get_declared_library_symbol_from(profile_id, symbol_name, order)
    }

    /// Get one well-known symbol using the concrete runtime declaration.
    ///
    /// This only selects one symbol id from the cached library environment.
    /// Callers that need declaration artifacts must require `DirDeclared` separately.
    pub fn get_well_known_concrete_symbol_from(
        &self,
        profile_id: ProfileId,
        symbol: WellKnownSymbol,
        order: SymbolSpaceOrder,
    ) -> Option<GlobalSymbolId> {
        let symbol_name = self.repository.strings.intern(symbol.export_name());
        self.get_declared_concrete_library_symbol_from(profile_id, symbol_name, order)
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
