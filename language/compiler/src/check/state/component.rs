use std::sync::Arc;

use destack_artifact::GlobalEnvironment;
use destack_dir as dir;
use destack_repository::{ArtifactReader, ProviderContext};
use destack_source::{ModuleId, ProfileId};
use indexmap::IndexMap;

use crate::check::{
    CheckComponentArtifact, CheckExternalModuleState, CheckModuleState, DefinitionTable,
    InferenceTable, InputTable,
};
use crate::{Compiler, CompilerError, CompilerResult};

/// State for checking one resolved component.
pub(in crate::check) struct CheckState<'a> {
    /// The compiler running this check attempt.
    pub(in crate::check) compiler: &'a Compiler,
    /// The provider context that owns artifact reads and diagnostics.
    pub(in crate::check) context: &'a dyn ProviderContext,
    /// The provider-scoped artifact reader.
    pub(in crate::check) artifacts: &'a ArtifactReader<'a>,
    /// The active profile.
    pub(in crate::check) profile: ProfileId,

    /// The active global environment.
    pub(in crate::check) environment: Arc<GlobalEnvironment>,
    /// Loaded component modules keyed by module id.
    pub(in crate::check) modules: IndexMap<ModuleId, CheckModuleState>,
    /// Loaded out-of-component modules keyed by module id.
    pub(in crate::check) external_modules: IndexMap<ModuleId, CheckExternalModuleState>,
    /// Checked component artifact containing each external module.
    pub(in crate::check) external_components: IndexMap<ModuleId, CheckComponentArtifact>,

    /// Input identity to check operand index.
    pub(in crate::check) inputs: InputTable,
    /// Component-wide inference graph.
    pub(in crate::check) inference: InferenceTable,
    /// Checked declarations built from walked operands.
    pub(in crate::check) definitions: DefinitionTable,

    /// Whether check events should print as they are recorded in debug builds.
    pub(in crate::check) emit_events: bool,
}

impl<'a> CheckState<'a> {
    /// Create a component check state.
    pub(in crate::check) fn new(
        compiler: &'a Compiler,
        context: &'a dyn ProviderContext,
        artifacts: &'a ArtifactReader<'a>,
        profile: ProfileId,
        environment: Arc<GlobalEnvironment>,
        external_components: IndexMap<ModuleId, CheckComponentArtifact>,
        emit_events: bool,
    ) -> Self {
        Self {
            compiler,
            context,
            artifacts,
            profile,
            environment,
            modules: IndexMap::new(),
            external_modules: IndexMap::new(),
            external_components,
            inputs: InputTable::new(),
            inference: InferenceTable::new(),
            definitions: DefinitionTable::new(),
            emit_events,
        }
    }

    /// Load all modules in one check component.
    pub(in crate::check) fn load(&mut self, modules: &[ModuleId]) -> CompilerResult<()> {
        // load modules in stable component order
        for module in modules {
            self.load_module(*module)?;
        }

        Ok(())
    }

    /// Walk every loaded module.
    pub(in crate::check) fn walk(&mut self) -> CompilerResult<()> {
        let modules = self.modules.keys().copied().collect::<Vec<_>>();

        // import external checked artifacts
        self.import_component_external_modules()?;
        self.import_external_symbol_type_operands()?;
        self.import_external_definitions()?;

        // declare component headers before any body can read them
        for module in modules.iter().copied() {
            self.declare_module_headers(module)?;
        }

        // walk modules in stable component order
        for module in modules.iter().copied() {
            self.walk_module(module)?;
        }

        self.finish_walk_generics(modules.as_slice())
    }

    /// Load one module into component state.
    fn load_module(&mut self, module_id: ModuleId) -> CompilerResult<()> {
        if self.is_component_module(module_id) {
            return Ok(());
        }

        let profile = self
            .compiler
            .profile(self.context.revision(), self.profile)?
            .key;
        let module = self.compiler.module(self.context.revision(), module_id)?;
        let parsed = self
            .artifacts
            .dir_parsed(module_id)
            .map_err(CompilerError::from)?;
        let bound = self
            .artifacts
            .dir_bound(module_id, self.profile)
            .map_err(CompilerError::from)?;
        let resolved = self
            .artifacts
            .dir_resolved(module_id, self.profile)
            .map_err(CompilerError::from)?;
        let expanded = self
            .artifacts
            .dir_expanded(module_id, self.profile)
            .map_err(CompilerError::from)?;
        let strings = Arc::clone(self.compiler.repository.string_pool());

        let module = CheckModuleState::new(
            module,
            profile,
            strings,
            parsed,
            bound,
            resolved,
            Arc::clone(&expanded),
        );

        self.modules.insert(module_id, module);

        Ok(())
    }

    /// Return one language symbol resolved for one module.
    pub(in crate::check) fn language_symbol(&self, item: dir::LanguageItem) -> dir::GlobalSymbolId {
        let symbol = self.environment.language.symbol(item).unwrap_or_else(|| {
            unreachable!("language item {item} is missing from the global environment")
        });

        symbol
    }

    /// Return whether one symbol is the resolved language item for one module.
    pub(in crate::check) fn is_language_symbol(
        &self,
        symbol: dir::GlobalSymbolId,
        item: dir::LanguageItem,
    ) -> bool {
        symbol == self.language_symbol(item)
    }
}
