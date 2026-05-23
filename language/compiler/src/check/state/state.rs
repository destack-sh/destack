use std::sync::Arc;

use destack_artifact::GlobalEnvironment;
use destack_dir as dir;
use destack_source::{ModuleId, ProfileId};
use destack_workspace::ProviderContext;
use indexmap::IndexMap;
use indexmap::map::Entry;

use crate::{Compiler, CompilerError, CompilerResult};

use super::{CheckModuleState, StaticTerm, TypeTerm, VariableId};

/// State for checking one resolved component.
pub(in crate::check) struct CheckComponentState<'a> {
    /// The compiler running this check attempt.
    pub(in crate::check) compiler: &'a Compiler,
    /// The provider context that owns artifact reads and diagnostics.
    pub(in crate::check) context: &'a dyn ProviderContext,
    /// The active profile.
    pub(in crate::check) profile: ProfileId,
    /// The component modules in stable order.
    pub(in crate::check) component_modules: Vec<ModuleId>,

    /// The active global environment.
    pub(in crate::check) environment: Arc<GlobalEnvironment>,
    /// Loaded modules keyed by module id.
    pub(in crate::check) modules: IndexMap<ModuleId, CheckModuleState>,
    /// Call-local generic variables keyed by call, symbol, and slot index.
    pub(in crate::check) call_generic_variables: IndexMap<
        (
            dir::GlobalNodeIdAny,
            Option<dir::GlobalSymbolId>,
            dir::GenericSlotIndex,
        ),
        VariableId,
    >,
}

impl<'a> CheckComponentState<'a> {
    /// Create a component check state.
    pub(in crate::check) fn new(
        compiler: &'a Compiler,
        context: &'a dyn ProviderContext,
        profile: ProfileId,
        component_modules: Vec<ModuleId>,
        environment: Arc<GlobalEnvironment>,
    ) -> Self {
        Self {
            compiler,
            context,
            profile,
            component_modules,
            environment,
            modules: IndexMap::new(),
            call_generic_variables: IndexMap::new(),
        }
    }

    /// Load all modules in one check component.
    pub(in crate::check) fn load(&mut self) -> CompilerResult<()> {
        let modules = self.component_modules.clone();

        // load modules in stable component order
        for module in modules {
            self.load_module(module)?;
        }

        Ok(())
    }

    /// Load one module.
    fn load_module(&mut self, module: ModuleId) -> CompilerResult<()> {
        if self.modules.contains_key(&module) {
            return Ok(());
        }

        let artifacts = self.compiler.artifact_reader(self.context);
        let parsed = artifacts.dir_parsed(module).map_err(CompilerError::from)?;
        let bound = artifacts
            .dir_bound(module, self.profile)
            .map_err(CompilerError::from)?;
        let resolved = artifacts
            .dir_resolved(module, self.profile)
            .map_err(CompilerError::from)?;
        let expanded = artifacts
            .dir_expanded(module, self.profile)
            .map_err(CompilerError::from)?;
        let strings = Arc::clone(self.compiler.repository.string_pool());

        let check_module = CheckModuleState::new(
            module,
            strings,
            parsed,
            bound,
            resolved,
            expanded,
            Arc::clone(&self.environment),
        );

        // publish loaded module state
        self.modules.insert(module, check_module);

        Ok(())
    }

    /// Alias imported symbols to their resolved targets.
    fn alias_imported_symbols(&mut self) -> CompilerResult<()> {
        let modules = self.component_modules.clone();
        let mut dependency_tables = IndexMap::new();

        // alias each imported symbol after every component module is walked
        for module in modules {
            let imports = self
                .module(module)?
                .resolved
                .imports
                .symbol_targets()
                .collect::<Vec<_>>();

            for (symbol, target) in imports {
                // component internal symbol
                if self.modules.contains_key(&target.module_id) {
                    self.alias_component_imported_symbol(symbol, target)?;
                }
                // import external symbol values locally
                else {
                    let tables = match dependency_tables.entry(target.module_id) {
                        Entry::Occupied(entry) => entry.into_mut(),
                        Entry::Vacant(entry) => {
                            let tables = self.load_dependency_tables(target.module_id)?;

                            entry.insert(tables)
                        }
                    };
                    let (types, statics) = tables;

                    self.import_dependency_symbol(symbol, target, types, statics)?;
                }
            }
        }

        Ok(())
    }

    /// Alias one imported symbol to another module in the same component.
    fn alias_component_imported_symbol(
        &mut self,
        symbol: dir::GlobalSymbolId,
        target: dir::GlobalSymbolId,
    ) -> CompilerResult<()> {
        let type_variable = self
            .module_mut(symbol.module_id)?
            .symbol_type_variable(symbol);
        let target_type_variable = self
            .module_mut(target.module_id)?
            .symbol_type_variable(target);
        let has_static_alias = self
            .module(symbol.module_id)?
            .symbol_static_variables
            .contains_key(&symbol);
        let module = self.module_mut(symbol.module_id)?;

        module.define_type_term(type_variable, TypeTerm::Variable(target_type_variable));
        if has_static_alias {
            let static_variable = self
                .module_mut(symbol.module_id)?
                .symbol_static_variable(symbol);
            let target_static_variable = self
                .module_mut(target.module_id)?
                .symbol_static_variable(target);
            let module = self.module_mut(symbol.module_id)?;

            module.define_static_term(
                static_variable,
                StaticTerm::Variable(target_static_variable),
            );
        }

        Ok(())
    }

    /// Load checked tables for one dependency module.
    fn load_dependency_tables(
        &self,
        module: ModuleId,
    ) -> CompilerResult<(dir::TypeTable<'static>, dir::StaticTable<'static>)> {
        let artifacts = self.compiler.artifact_reader(self.context);
        let bound = artifacts
            .dir_bound(module, self.profile)
            .map_err(CompilerError::from)?;
        let expanded = artifacts
            .dir_expanded(module, self.profile)
            .map_err(CompilerError::from)?;
        let checked = artifacts
            .dir_checked(module, self.profile)
            .map_err(CompilerError::from)?;
        let types = checked.type_table(bound.as_ref(), expanded.as_ref());
        let statics = checked.static_table(bound.as_ref(), expanded.as_ref());

        Ok((types, statics))
    }

    /// Import one checked symbol from a dependency module.
    fn import_dependency_symbol(
        &mut self,
        symbol: dir::GlobalSymbolId,
        target: dir::GlobalSymbolId,
        types: &dir::TypeTable<'static>,
        statics: &dir::StaticTable<'static>,
    ) -> CompilerResult<()> {
        let imported = self
            .module_mut(symbol.module_id)?
            .import_symbol(symbol, target, types, statics);
        if !imported {
            return Err(CompilerError::Internal {
                message: format!("checked dependency symbol {target:?} has no checked value"),
            });
        }

        Ok(())
    }

    /// Walk every loaded module.
    pub(in crate::check) fn walk(&mut self) -> CompilerResult<()> {
        let modules = self.component_modules.clone();

        // walk modules in stable component order
        for module in modules {
            let check_module = self.module_mut(module)?;

            check_module.walk().map_err(CompilerError::from)?;
        }

        // alias imported symbols after walk has created the demanded variables
        self.alias_imported_symbols()?;

        Ok(())
    }

    /// Return one loaded module.
    pub(in crate::check) fn module(&self, module: ModuleId) -> CompilerResult<&CheckModuleState> {
        self.modules
            .get(&module)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("check module {module:?} was not loaded"),
            })
    }

    /// Return one loaded module mutably.
    pub(in crate::check) fn module_mut(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<&mut CheckModuleState> {
        self.modules
            .get_mut(&module)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("check module {module:?} was not loaded"),
            })
    }
}
