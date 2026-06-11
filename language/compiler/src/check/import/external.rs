use destack_source::ModuleId;
use indexmap::IndexSet;

use crate::check::{CheckExternalModuleState, CheckState};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Import and return state for one external module.
    fn import_external_module(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<&CheckExternalModuleState> {
        if !self.external_modules.contains_key(&module) {
            let external = self.import_external_module_state(module)?;

            self.external_modules.insert(module, external);
        }

        Ok(self.external_module(module))
    }

    /// Import committed external modules named by component imports.
    pub(in crate::check) fn import_component_external_modules(&mut self) -> CompilerResult<()> {
        let modules = self.modules.keys().copied().collect::<Vec<_>>();

        // import external modules in stable component order
        for module in modules {
            let external_modules = self.external_module_ids(module);
            for external_module in external_modules {
                self.import_external_module(external_module)?;
                self.module_mut(module)
                    .external_modules
                    .insert(external_module);
            }
        }

        Ok(())
    }

    /// Return external modules that can be named from one component module.
    fn external_module_ids(&self, module: ModuleId) -> IndexSet<ModuleId> {
        let mut external_modules = IndexSet::new();
        let imports = &self.module(module).resolved.imports;

        // collect resolved import modules outside the component
        for external_module in imports.modules() {
            if !self.is_component_module(external_module) {
                external_modules.insert(external_module);
            }
        }

        external_modules
    }

    /// Import external module state from committed artifacts.
    fn import_external_module_state(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<CheckExternalModuleState> {
        let component = self
            .external_components
            .get(&module)
            .copied()
            .ok_or_else(|| CompilerError::Internal {
                message: format!("check external module {module:?} has no component artifact"),
            })?;
        let parsed = self
            .artifacts
            .dir_parsed(module)
            .map_err(CompilerError::from)?;
        let bound = self
            .artifacts
            .dir_bound(module, self.profile)
            .map_err(CompilerError::from)?;
        let expanded = self
            .artifacts
            .dir_expanded(module, self.profile)
            .map_err(CompilerError::from)?;
        let checked_component = self
            .artifacts
            .dir_checked_component(component.entry, component.component, self.profile)
            .map_err(CompilerError::from)?;
        let checked = &checked_component
            .module(module)
            .ok_or_else(|| CompilerError::Internal {
                message: format!(
                    "checked component {} does not contain external module {module:?}",
                    component.component
                ),
            })?
            .checked;
        let bindings = expanded.binding_table(bound.as_ref());
        let types = checked.type_table(bound.as_ref(), expanded.as_ref());
        let statics = checked.static_table(bound.as_ref(), expanded.as_ref());
        let generics = checked.generic_table();
        let definitions = checked.definition_table();

        Ok(CheckExternalModuleState {
            parsed,
            expanded,
            bindings,
            types,
            statics,
            generics,
            definitions,
        })
    }
}
