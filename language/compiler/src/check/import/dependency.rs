use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexSet;

use crate::check::{CheckDependencyState, CheckState};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Import and return state for one dependency module.
    pub(in crate::check) fn import_dependency(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<&CheckDependencyState> {
        if !self.dependencies.contains_key(&module) {
            let dependency = self.import_dependency_state(module)?;

            self.dependencies.insert(module, dependency);
        }

        Ok(self.dependency(module))
    }

    /// Import committed dependency modules visible to component modules.
    pub(in crate::check) fn import_component_dependencies(&mut self) -> CompilerResult<()> {
        let modules = self.modules.keys().copied().collect::<Vec<_>>();

        // import dependencies in stable component order
        for module in modules {
            let dependencies = self.dependency_modules(module);
            for dependency in dependencies {
                self.import_dependency(dependency)?;
                self.module_mut(module).dependencies.insert(dependency);
            }
        }

        Ok(())
    }

    /// Return dependency modules that can be named from one component module.
    fn dependency_modules(&self, module: ModuleId) -> IndexSet<ModuleId> {
        let mut dependencies = IndexSet::new();
        let imports = &self.module(module).resolved.imports;

        // collect direct dependency modules
        for dependency in &imports.dependencies {
            if !self.is_component_module(*dependency) {
                dependencies.insert(*dependency);
            }
        }

        // collect resolved explicit import targets
        for (_, target) in imports.symbol_targets() {
            let dependency = target.module_id;
            if !self.is_component_module(dependency) {
                dependencies.insert(dependency);
            }
        }

        // collect resolved namespace import targets
        for target in imports.target_by_symbol.values() {
            let dir::ImportTarget::Namespace(dependency) = target else {
                continue;
            };

            if !self.is_component_module(*dependency) {
                dependencies.insert(*dependency);
            }
        }

        // collect resolved profile global targets
        for target in imports
            .global_target_by_key
            .values()
            .flat_map(|targets| targets.iter().copied())
        {
            let dependency = match target {
                dir::ImportTarget::Symbol(symbol) => symbol.module_id,
                dir::ImportTarget::Namespace(module) => module,
            };

            if !self.is_component_module(dependency) {
                dependencies.insert(dependency);
            }
        }

        // collect syntax-required language items
        for target in imports.language_symbols() {
            let dependency = target.module_id;
            if !self.is_component_module(dependency) {
                dependencies.insert(dependency);
            }
        }

        dependencies
    }

    /// Import dependency state from committed artifacts.
    fn import_dependency_state(&self, module: ModuleId) -> CompilerResult<CheckDependencyState> {
        let artifacts = self.compiler.artifact_reader(self.context);
        let parsed = artifacts.dir_parsed(module).map_err(CompilerError::from)?;
        let bound = artifacts
            .dir_bound(module, self.profile)
            .map_err(CompilerError::from)?;
        let expanded = artifacts
            .dir_expanded(module, self.profile)
            .map_err(CompilerError::from)?;
        let checked = artifacts
            .dir_checked(module, self.profile)
            .map_err(CompilerError::from)?;
        let bindings = expanded.binding_table(bound.as_ref());
        let types = checked.type_table(bound.as_ref(), expanded.as_ref());
        let statics = checked.static_table(bound.as_ref(), expanded.as_ref());
        let generics = checked.generic_table();
        let nominals = checked.nominal_table();
        let extensions = checked.extension_table();

        Ok(CheckDependencyState {
            parsed,
            expanded,
            bindings,
            types,
            statics,
            generics,
            nominals,
            extensions,
        })
    }
}
