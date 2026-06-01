use std::sync::Arc;

use destack_artifact::{DirExpanded, DirParsed};
use destack_dir as dir;
use destack_source::ModuleId;

use crate::{CompilerError, CompilerResult};

use super::CheckState;

/// Checked tables loaded for one out-of-component dependency module.
pub(in crate::check) struct CheckDependencyState {
    /// The parsed dependency module.
    pub(in crate::check) parsed: Arc<DirParsed>,
    /// The expanded dependency module.
    pub(in crate::check) expanded: Arc<DirExpanded>,
    /// The checked binding table.
    pub(in crate::check) bindings: dir::BindingTable<'static>,
    /// The checked type table.
    pub(in crate::check) types: dir::TypeTable<'static>,
    /// The checked static table.
    pub(in crate::check) statics: dir::StaticTable<'static>,
    /// The checked generic table.
    pub(in crate::check) generics: dir::GenericTable<'static>,
    /// The checked extension table.
    pub(in crate::check) extensions: dir::ExtensionTable<'static>,
}

impl CheckDependencyState {
    /// Return the post-expansion DIR tree view visible to check.
    pub(in crate::check) fn view(&self) -> dir::View<'_> {
        dir::View::with_patches(
            &self.parsed.tree,
            std::slice::from_ref(&self.expanded.patch),
        )
    }

    /// Return the committed slot for one source parameter type.
    pub(in crate::check) fn generic_slot(
        &self,
        parameter: dir::GenericParameterRef,
    ) -> Option<dir::GenericSlot> {
        self.generics
            .iter_slots()
            .find_map(|(_, slot)| {
                let template = self.generics.get_template(slot.template());
                (template.owner == parameter.owner
                    && slot.key() == parameter.key
                    && slot.index() == parameter.index)
                    .then_some(slot)
            })
            .copied()
    }
}

impl CheckState<'_> {
    /// Load and return state for one dependency module.
    pub(in crate::check) fn load_dependency(
        &mut self,
        module: ModuleId,
    ) -> CompilerResult<&CheckDependencyState> {
        if !self.dependencies.contains_key(&module) {
            let dependency = self.load_dependency_state(module)?;

            self.dependencies.insert(module, dependency);
        }

        Ok(self.dependency(module))
    }

    /// Return loaded state for one dependency module.
    pub(in crate::check) fn dependency(&self, module: ModuleId) -> &CheckDependencyState {
        self.dependencies
            .get(&module)
            .unwrap_or_else(|| panic!("check dependency {module:?} was not loaded"))
    }

    /// Load dependency state from checked artifacts.
    fn load_dependency_state(&self, module: ModuleId) -> CompilerResult<CheckDependencyState> {
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
        let extensions = checked.extension_table();

        Ok(CheckDependencyState {
            parsed,
            expanded,
            bindings,
            types,
            statics,
            generics,
            extensions,
        })
    }
}
