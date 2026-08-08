use std::sync::Arc;

use destack_artifact::{ArtifactProjectionKey, DirBound, DirDeclared, DirExpanded, DirResolved};
use destack_core::FxIndexSet;
use destack_dir as dir;
use destack_source::ModuleId;

use super::CheckState;
use crate::{CompilerError, CompilerResult};

/// Committed tables loaded for one out-of-component external module.
pub(in crate::check) struct CheckExternalModuleState {
    /// The resolved external module holding the import alias targets.
    pub(in crate::check) resolved: Arc<DirResolved>,
    /// The committed binding table.
    pub(in crate::check) bindings: dir::BindingTable<'static>,
    /// The committed type table.
    pub(in crate::check) types: dir::TypeTable<'static>,
    /// The committed static table.
    pub(in crate::check) statics: dir::StaticTable<'static>,
    /// The committed generic table.
    pub(in crate::check) generics: dir::GenericTable<'static>,
    /// The committed definition table.
    pub(in crate::check) definitions: dir::DefinitionTable<'static>,
}

impl CheckState<'_> {
    /// Return loaded state for one external module.
    pub(in crate::check) fn external_module(&self, module: ModuleId) -> &CheckExternalModuleState {
        self.external_modules
            .get(&module)
            .unwrap_or_else(|| unreachable!("external module {module:?} was not loaded"))
    }

    /// Import and return state for one external module while checking.
    pub(in crate::check) fn import_external_module(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<Option<&CheckExternalModuleState>> {
        // declared tables of other modules are checking inputs only
        if !self.is_checking {
            return Ok(None);
        }

        if !self.external_modules.contains_key(&module) {
            let external = self.import_external_module_state(module)?;

            self.generics
                .index_template_symbols(module, external.generics.iter_templates());
            self.external_modules.insert(module, external);
        }

        Ok(Some(self.external_module(module)))
    }

    /// Import directly imported external modules and record their visibility.
    pub(in crate::check) fn import_external_modules(&mut self) -> CompilerResult<()> {
        let modules = vec![self.module_id];
        for module in modules {
            // record visibility and load the foreign modules
            let visible = self.external_module_ids(module);
            for external in &visible {
                self.import_external_module(*external)?;
            }
            self.module_mut(module).external_modules.extend(visible);
        }

        Ok(())
    }

    /// Return external modules that can be named from one component module.
    fn external_module_ids(&self, module: ModuleId) -> FxIndexSet<ModuleId> {
        let mut external_modules = FxIndexSet::default();
        let imports = &self.module(module).resolved.imports;

        // collect resolved target modules outside the component
        for external_module in imports.target_modules() {
            if !self.is_own_module(external_module) {
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
        let bound = self
            .artifacts
            .read_content::<DirBound>((module, self.profile))
            .map_err(CompilerError::from)?;
        let expanded = self
            .artifacts
            .read_content::<DirExpanded>((module, self.profile))
            .map_err(CompilerError::from)?;
        let resolved = self
            .artifacts
            .read_content::<DirResolved>((module, self.profile))
            .map_err(CompilerError::from)?;

        // read the module's sealed declared module
        let declared = self
            .artifacts
            .read_projection::<DirDeclared>((module, self.profile), ArtifactProjectionKey::Declared)
            .map_err(CompilerError::from)?;

        Ok(CheckExternalModuleState {
            bindings: declared.binding_table(bound.as_ref(), expanded.as_ref()),
            types: declared.type_table(bound.as_ref(), expanded.as_ref()),
            statics: declared.static_table(bound.as_ref(), expanded.as_ref()),
            generics: declared.generic_table(),
            definitions: declared.definition_table(),
            resolved,
        })
    }
}
