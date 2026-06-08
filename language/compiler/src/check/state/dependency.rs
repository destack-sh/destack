use std::sync::Arc;

use destack_artifact::{DirExpanded, DirParsed};
use destack_dir as dir;
use destack_source::ModuleId;

use super::CheckState;

/// Committed tables loaded for one out-of-component dependency module.
pub(in crate::check) struct CheckDependencyState {
    /// The parsed dependency module.
    pub(in crate::check) parsed: Arc<DirParsed>,
    /// The expanded dependency module.
    pub(in crate::check) expanded: Arc<DirExpanded>,
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

impl CheckDependencyState {
    /// Return the post-expansion DIR tree view visible to check.
    pub(in crate::check) fn view(&self) -> dir::View<'_> {
        dir::View::with_patches(
            &self.parsed.tree,
            std::slice::from_ref(&self.expanded.patch),
        )
    }
}

impl CheckState<'_> {
    /// Return loaded state for one dependency module.
    pub(in crate::check) fn dependency(&self, module: ModuleId) -> &CheckDependencyState {
        self.dependencies
            .get(&module)
            .unwrap_or_else(|| unreachable!("dependency {module:?} was not loaded"))
    }
}
