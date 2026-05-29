use std::collections::HashMap;
use std::sync::Arc;

use destack_artifact::GlobalEnvironment;
use destack_dir as dir;
use destack_source::{ModuleId, ProfileId};
use destack_workspace::ProviderContext;
use indexmap::IndexMap;

use crate::check::{
    CheckDependencyState, CheckModuleState, ExportLookupKey, ExportLookupState, FlowState,
    Obligation, SolutionTable, TermTable, VariableId, VariableTable,
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
    /// Transient flow states for modules currently being walked.
    pub(in crate::check) flow: IndexMap<ModuleId, FlowState>,
    /// Component-wide variable graph.
    pub(in crate::check) variables: VariableTable,
    /// Component-wide post-solve obligations.
    pub(in crate::check) obligations: Vec<Obligation>,
    /// Component-wide term table.
    pub(in crate::check) terms: TermTable,
    /// Component-wide solver solutions.
    pub(in crate::check) solutions: SolutionTable,
    /// Export lookups already computed during this check component.
    pub(in crate::check) exports: HashMap<ExportLookupKey, ExportLookupState>,
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
            flow: IndexMap::new(),
            variables: VariableTable::new(),
            obligations: Vec::new(),
            terms: TermTable::new(),
            solutions: SolutionTable::new(),
            exports: HashMap::new(),
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

        // import checked dependency values before walk classifies references
        self.import_module_dependencies()?;

        // walk modules in stable component order
        for module in modules {
            self.walk_module(module);
        }

        Ok(())
    }

    /// Prepare fixed generic and variable state before solve.
    pub(in crate::check) fn prepare(&mut self) -> CompilerResult<()> {
        let modules = self.modules.keys().copied().collect::<Vec<_>>();

        // induce concrete owner generics from transparent type leaves
        self.prepare_generics()?;

        // define declaration references after all induced slots are known
        for module in modules {
            self.add_declaration_type_definitions(module);
        }

        Ok(())
    }

    /// Load one module.
    fn load_module(&mut self, module_id: ModuleId) -> CompilerResult<()> {
        if self.modules.contains_key(&module_id) {
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
            module_id,
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

        if self.modules.contains_key(&symbol.module_id) {
            return symbol;
        }
        if self.module(module).dependencies.contains(&symbol.module_id) {
            return symbol;
        }

        panic!("language item {item} module was not loaded for module {module:?}")
    }

    /// Return the type variable for one symbol visible from a component module.
    pub(in crate::check) fn symbol_type_variable(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> VariableId {
        if self.modules.contains_key(&symbol.module_id) {
            return self.intern_local_symbol_type_variable(symbol.module_id, symbol);
        }

        self.import_symbol_type_variable(module, symbol)
    }

    /// Return the static variable for one symbol visible from a component module.
    pub(in crate::check) fn symbol_static_variable(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> VariableId {
        if self.modules.contains_key(&symbol.module_id) {
            return self.intern_symbol_static_variable(symbol.module_id, symbol);
        }

        self.import_symbol_static_variable(module, symbol)
    }

    /// Return the static generic variable for one symbol visible from a component module.
    pub(in crate::check) fn generic_static_variable(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<VariableId>> {
        if !self.modules.contains_key(&symbol.module_id)
            && !self.module(module).dependencies.contains(&symbol.module_id)
        {
            let item = match self.environment.language.item(symbol) {
                Some(item) => format!(" language_item={item}"),
                None => String::new(),
            };

            return Err(CompilerError::Internal {
                message: format!("symbol {symbol:?}{item} was not loaded for module {module:?}"),
            });
        }

        Ok(self.generic_static_variable_for_symbol(module, symbol))
    }
}
