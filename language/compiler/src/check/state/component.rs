use std::sync::Arc;

use destack_artifact::GlobalEnvironment;
use destack_dir as dir;
use destack_source::{ModuleId, ProfileId};
use destack_workspace::ProviderContext;
use indexmap::IndexMap;

use crate::check::{
    CheckDependencyState, CheckModuleState, CheckTrace, ExtensionTable, InferenceTable, InputTable,
    NominalTable,
};
use crate::{Compiler, CompilerError, CompilerResult};

/// State for checking one resolved component.
pub(in crate::check) struct CheckState<'a> {
    /// The compiler running this check attempt.
    pub(in crate::check) compiler: &'a Compiler,
    /// The provider context that owns artifact reads and diagnostics.
    pub(in crate::check) context: &'a dyn ProviderContext,
    /// The active profile.
    pub(in crate::check) profile: ProfileId,

    /// The active global environment.
    pub(in crate::check) environment: Arc<GlobalEnvironment>,
    /// Loaded component modules keyed by module id.
    pub(in crate::check) modules: IndexMap<ModuleId, CheckModuleState>,
    /// Loaded out-of-component dependencies keyed by module id.
    pub(in crate::check) dependencies: IndexMap<ModuleId, CheckDependencyState>,

    /// Input identity to check operand index.
    pub(in crate::check) inputs: InputTable,
    /// Component-wide inference graph.
    pub(in crate::check) inference: InferenceTable,
    /// Checked nominal declarations built from walked operands.
    pub(in crate::check) nominals: NominalTable,
    /// Checked extension declarations built from walked operands.
    pub(in crate::check) extensions: ExtensionTable,
    /// Trace events emitted during checking.
    pub(in crate::check) trace: CheckTrace,
}

impl<'a> CheckState<'a> {
    /// Create a component check state.
    pub(in crate::check) fn new(
        compiler: &'a Compiler,
        context: &'a dyn ProviderContext,
        profile: ProfileId,
        environment: Arc<GlobalEnvironment>,
    ) -> Self {
        Self {
            compiler,
            context,
            profile,
            environment,
            modules: IndexMap::new(),
            dependencies: IndexMap::new(),
            inputs: InputTable::new(),
            inference: InferenceTable::new(),
            nominals: NominalTable::new(),
            extensions: ExtensionTable::new(),
            trace: CheckTrace::new(),
        }
    }

    /// Read all modules in one check component.
    pub(in crate::check) fn read_component_modules(
        &mut self,
        modules: &[ModuleId],
    ) -> CompilerResult<()> {
        // read modules in stable component order
        for module in modules {
            self.read_component_module(*module)?;
        }

        Ok(())
    }

    /// Walk every loaded module.
    pub(in crate::check) fn walk(&mut self) -> CompilerResult<()> {
        let modules = self.modules.keys().copied().collect::<Vec<_>>();

        // import checked dependency artifacts before walk classifies references
        self.import_component_dependencies()?;

        // walk modules in stable component order
        for module in modules.iter().copied() {
            self.walk_module(module);
        }

        self.propagate_walk_state(modules.as_slice())
    }

    /// Read one module into component state.
    fn read_component_module(&mut self, module_id: ModuleId) -> CompilerResult<()> {
        if self.is_component_module(module_id) {
            return Ok(());
        }

        let artifacts = self.compiler.artifact_reader(self.context);
        let profile = self
            .compiler
            .profile(self.context.revision(), self.profile)?
            .key;
        let module = self.compiler.module(self.context.revision(), module_id)?;
        let parsed = artifacts
            .dir_parsed(module_id)
            .map_err(CompilerError::from)?;
        let bound = artifacts
            .dir_bound(module_id, self.profile)
            .map_err(CompilerError::from)?;
        let resolved = artifacts
            .dir_resolved(module_id, self.profile)
            .map_err(CompilerError::from)?;
        let expanded = artifacts
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
    pub(in crate::check) fn language_symbol(
        &self,
        module: ModuleId,
        item: dir::LanguageItem,
    ) -> dir::GlobalSymbolId {
        let symbol = self
            .module(module)
            .resolved
            .imports
            .language_symbol(item)
            .unwrap_or_else(|| {
                panic!("language item {item} was not resolved for module {module:?}")
            });

        symbol
    }
}
