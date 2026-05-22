use destack_artifact::{DirCheckedComponentEntry, DirCheckedModule, ToDiagnostic};
use destack_source::{DiagnosticCollection, ModuleId, ProfileId};
use destack_workspace::ProviderContext;
use indexmap::IndexMap;

use crate::{Compiler, CompilerError, CompilerResult};

use super::CheckModuleState;

/// State for checking one resolved component.
pub(in crate::check) struct CheckComponentState<'a> {
    /// The compiler running this check attempt.
    compiler: &'a Compiler,
    /// The provider context that owns artifact reads and diagnostics.
    context: &'a dyn ProviderContext,
    /// The active profile.
    profile: ProfileId,
    /// Loaded modules keyed by module id.
    pub(in crate::check) modules: IndexMap<ModuleId, CheckModuleState>,
}

impl<'a> CheckComponentState<'a> {
    /// Create a component check state.
    pub(in crate::check) fn new(
        compiler: &'a Compiler,
        context: &'a dyn ProviderContext,
        profile: ProfileId,
    ) -> Self {
        Self {
            compiler,
            context,
            profile,
            modules: IndexMap::new(),
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
        let imported = artifacts
            .dir_imported(module, self.profile)
            .map_err(CompilerError::from)?;
        let exported = artifacts
            .dir_exported(module, self.profile)
            .map_err(CompilerError::from)?;
        let resolved = artifacts
            .dir_resolved(module, self.profile)
            .map_err(CompilerError::from)?;
        let expanded = artifacts
            .dir_expanded(module, self.profile)
            .map_err(CompilerError::from)?;

        let check_module = CheckModuleState::new(
            module,
            self.profile,
            parsed,
            bound,
            imported,
            exported,
            resolved,
            expanded,
        );

        // publish loaded module state
        self.modules.insert(module, check_module);

        Ok(())
    }

    /// Walk every loaded module.
    pub(in crate::check) fn walk(&mut self) -> CompilerResult<()> {
        // walk root first, matching source entry order
        for module in self.loaded_modules() {
            let check_module = self.module_mut(module)?;

            check_module.walk().map_err(CompilerError::from)?;
        }

        Ok(())
    }

    /// Validate every loaded module.
    pub(in crate::check) fn validate(&mut self) -> CompilerResult<DiagnosticCollection> {
        let mut collection = DiagnosticCollection::new();

        // validate and finalize diagnostics
        for module in self.loaded_modules() {
            let diagnostics = {
                let check_module = self.module_mut(module)?;
                check_module.validate().map_err(CompilerError::from)?;
                check_module.take_diagnostics()
            };

            for diagnostic in diagnostics {
                collection.insert(diagnostic.to_diagnostic(self.context)?);
            }
        }

        Ok(collection)
    }

    /// Return loaded modules in component order.
    pub(in crate::check) fn loaded_modules(&self) -> Vec<ModuleId> {
        self.modules.keys().copied().collect()
    }

    /// Finish every loaded module into component output modules.
    pub(in crate::check) fn finish(&mut self) -> CompilerResult<Vec<DirCheckedComponentEntry>> {
        let mut modules = Vec::new();

        // finish modules in stable load order
        for module in self.loaded_modules() {
            let checked = self.finish_module(module)?;

            modules.push(DirCheckedComponentEntry { module, checked });
        }

        Ok(modules)
    }

    /// Finish one checked module.
    fn finish_module(&mut self, module: ModuleId) -> CompilerResult<DirCheckedModule> {
        let check_module =
            self.modules
                .swap_remove(&module)
                .ok_or_else(|| CompilerError::Internal {
                    message: format!("checked module {module:?} was not loaded"),
                })?;

        Ok(check_module.finish())
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
