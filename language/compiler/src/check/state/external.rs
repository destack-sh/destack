use std::sync::Arc;

use destack_artifact::{DirExpanded, DirParsed, DirResolved};
use destack_dir as dir;
use destack_source::ModuleId;

use indexmap::IndexSet;

use super::CheckState;
use crate::{CompilerError, CompilerResult};

/// Committed tables loaded for one out-of-component external module.
pub(in crate::check) struct CheckExternalModuleState {
    /// The parsed external module, kept for diagnostic source spans.
    pub(in crate::check) parsed: Arc<DirParsed>,
    /// The expanded external module, kept for diagnostic source spans.
    pub(in crate::check) expanded: Arc<DirExpanded>,
    /// The resolved external module carrying import alias targets.
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

impl CheckExternalModuleState {
    /// Return the post-expansion DIR tree view of this module.
    pub(in crate::check) fn view(&self) -> dir::View<'_> {
        dir::View::with_patches(
            &self.parsed.tree,
            std::slice::from_ref(&self.expanded.patch),
        )
    }
}

impl CheckState<'_> {
    /// Return loaded state for one external module.
    pub(in crate::check) fn external_module(&self, module: ModuleId) -> &CheckExternalModuleState {
        self.external_modules
            .get(&module)
            .unwrap_or_else(|| unreachable!("external module {module:?} was not loaded"))
    }

    /// Import and return state for one external module.
    pub(in crate::check) fn import_external_module(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<&CheckExternalModuleState> {
        if !self.external_modules.contains_key(&module) {
            let external = self.import_external_module_state(module)?;

            self.external_modules.insert(module, external);
        }

        Ok(self.external_module(module))
    }

    /// Resolve one external symbol through committed export alias chains.
    pub(in crate::check) fn resolve_external_alias(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<dir::GlobalSymbolId> {
        let mut current = symbol;
        let mut visited = IndexSet::new();

        // hop alias targets until a declaring symbol appears
        loop {
            if self.is_component_module(current.module_id) {
                return Ok(current);
            }
            if !visited.insert(current) {
                return Err(CompilerError::Internal {
                    message: format!("external alias {symbol:?} forwards in a cycle"),
                });
            }
            let external = self.import_external_module(current.module_id)?;
            match external.resolved.imports.symbol_target(current.local_id) {
                Some(dir::ImportTarget::Symbol(target)) => current = target,
                _ => return Ok(current),
            }
        }
    }

    /// Import committed external modules for the whole import closure.
    pub(in crate::check) fn import_component_external_modules(&mut self) -> CompilerResult<()> {
        // load every reachable external module's committed tables
        let external_modules = self.external_components.keys().copied().collect::<Vec<_>>();
        for external_module in external_modules {
            self.import_external_module(external_module)?;
        }

        // record per-module visibility for name and member lookups
        let modules = self.modules.keys().copied().collect::<Vec<_>>();
        for module in modules {
            let visible = self.external_module_ids(module);
            self.module_mut(module).external_modules.extend(visible);
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
        let resolved = self
            .artifacts
            .dir_resolved(module, self.profile)
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
            expanded: Arc::clone(&expanded),
            resolved,
            bindings,
            types,
            statics,
            generics,
            definitions,
        })
    }
}
