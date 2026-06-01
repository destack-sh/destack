use std::sync::Arc;

use destack_artifact::GlobalEnvironment;
use destack_dir as dir;
use destack_source::{ModuleId, ProfileId};
use destack_workspace::ProviderContext;
use indexmap::IndexMap;

use crate::check::{
    CheckDependencyState, CheckEvent, CheckModuleState, CheckTrace, InferenceTable, OperandTable,
    StaticOperand, VariableId,
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

    /// Source operands discovered during checking.
    pub(in crate::check) operands: OperandTable,
    /// Component-wide inference graph.
    pub(in crate::check) inference: InferenceTable,
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
            operands: OperandTable::new(),
            inference: InferenceTable::new(),
            trace: CheckTrace::new(),
        }
    }

    /// Record one check event.
    pub(in crate::check) fn record_trace(&mut self, event: CheckEvent) {
        self.trace.record(event);
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

        // load checked dependency artifacts before walk classifies references
        self.load_module_dependencies()?;

        // walk modules in stable component order
        for module in modules.iter().copied() {
            self.walk_module(module);
        }

        self.propagate_walk_state(modules.as_slice())
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

    /// Return one checked symbol static variable, importing it when missing.
    pub(in crate::check) fn ensure_symbol_static_variable(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> VariableId {
        if self.modules.contains_key(&symbol.module_id) {
            if let Some(operand) = self.operands.symbol_statics.get(&symbol).copied() {
                return match operand {
                    StaticOperand::Variable(variable) => variable,
                    StaticOperand::Term(_) | StaticOperand::Static(_) => {
                        panic!("check symbol {symbol:?} has a static operand, not a solver slot")
                    }
                };
            }

            if let Some(target) = self.import_alias_target(symbol) {
                let variable = self.ensure_symbol_static_variable(module, target);
                let operand = variable.into();

                self.bind_symbol_static_operand(symbol, operand);

                return variable;
            }

            return self.require_local_symbol_static_variable(symbol);
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
