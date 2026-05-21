use destack_artifact::DirChecked;
use destack_dir as dir;
use destack_source::{ModuleId, ProfileId};
use destack_workspace::ProviderContext;
use indexmap::IndexMap;

use crate::{Compiler, CompilerError, CompilerResult};

use super::CheckModuleState;

/// State for one check attempt.
pub(in crate::check) struct CheckState<'a> {
    /// The compiler running this check attempt.
    compiler: &'a Compiler,
    /// The provider context that owns artifact reads and diagnostics.
    context: &'a dyn ProviderContext,
    /// The active profile.
    profile: ProfileId,
    /// Loaded modules keyed by module id.
    pub(in crate::check) modules: IndexMap<ModuleId, CheckModuleState>,
}

impl<'a> CheckState<'a> {
    /// Create a check attempt.
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

    /// Load one root module and its imported module closure.
    pub(in crate::check) fn load(&mut self, root: ModuleId) -> CompilerResult<()> {
        let mut pending = vec![root];

        // load each module once
        while let Some(module) = pending.pop() {
            if self.modules.contains_key(&module) {
                continue;
            }

            let dependencies = self.load_module(module)?;

            // preserve source dependency order
            for dependency in dependencies.into_iter().rev() {
                if dependency != module && !self.modules.contains_key(&dependency) {
                    pending.push(dependency);
                }
            }
        }

        Ok(())
    }

    /// Load one module and return its imported modules.
    fn load_module(&mut self, module: ModuleId) -> CompilerResult<Vec<ModuleId>> {
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
        let dependencies = Self::dependency_modules(&check_module);

        // publish after dependencies are known
        self.modules.insert(module, check_module);

        Ok(dependencies)
    }

    /// Return modules imported by one loaded module.
    fn dependency_modules(check_module: &CheckModuleState) -> Vec<ModuleId> {
        let dependencies = check_module
            .expanded()
            .dependency_table(check_module.imported());
        let mut modules = Vec::new();

        // collect unique module targets
        for edge in dependencies.iter() {
            let Some(target) = edge.target else {
                continue;
            };

            if target != check_module.module() && !modules.contains(&target) {
                modules.push(target);
            }
        }

        modules
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

    /// Solve every loaded module.
    pub(in crate::check) fn solve(&mut self) -> CompilerResult<()> {
        // solve dependencies before their importers
        for module in self.loaded_modules_from_dependencies() {
            self.import_symbol_types(module);

            let check_module = self.module_mut(module)?;

            check_module.solve().map_err(CompilerError::from)?;
        }

        Ok(())
    }

    /// Copy solved imported symbol types into one module.
    fn import_symbol_types(&mut self, module: ModuleId) {
        let sources = self.import_type_sources(module);

        // copy from every already loaded source module
        for source in sources {
            let [Some(target), Some(source)] = self.modules.get_disjoint_mut([&module, &source])
            else {
                continue;
            };

            target.import_symbol_types_from(source);
        }
    }

    /// Return modules that provide imported symbol types.
    fn import_type_sources(&self, module: ModuleId) -> Vec<ModuleId> {
        let Some(check_module) = self.modules.get(&module) else {
            return Vec::new();
        };
        let mut modules = Vec::new();

        // collect unique import target modules
        for target in check_module.resolved().imports.symbol_targets.values() {
            let dir::ImportTarget::Symbol(symbol) = target else {
                continue;
            };
            if symbol.module_id != module && !modules.contains(&symbol.module_id) {
                modules.push(symbol.module_id);
            }
        }

        modules
    }

    /// Validate every loaded module.
    pub(in crate::check) fn validate(&mut self) -> CompilerResult<()> {
        // validate and emit diagnostics per module
        for module in self.loaded_modules() {
            let diagnostics = {
                let check_module = self.module_mut(module)?;

                check_module.validate().map_err(CompilerError::from)?;
                check_module.take_diagnostics()
            };

            for diagnostic in diagnostics {
                self.compiler.emit_diagnostic(self.context, diagnostic)?;
            }
        }

        Ok(())
    }

    /// Return loaded modules in root-first order.
    fn loaded_modules(&self) -> Vec<ModuleId> {
        self.modules.keys().copied().collect()
    }

    /// Return loaded modules in dependency-first order.
    fn loaded_modules_from_dependencies(&self) -> Vec<ModuleId> {
        let mut modules = self.loaded_modules();
        modules.reverse();

        modules
    }

    /// Finish one checked root module.
    pub(in crate::check) fn finish(&mut self, module: ModuleId) -> CompilerResult<DirChecked> {
        let check_module =
            self.modules
                .swap_remove(&module)
                .ok_or_else(|| CompilerError::Internal {
                    message: format!("checked root module {module:?} was not loaded"),
                })?;

        Ok(check_module.finish())
    }

    /// Return one loaded module mutably.
    fn module_mut(&mut self, module: ModuleId) -> CompilerResult<&mut CheckModuleState> {
        self.modules
            .get_mut(&module)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("check module {module:?} was not loaded"),
            })
    }
}
