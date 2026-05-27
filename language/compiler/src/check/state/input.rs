use std::sync::Arc;

use destack_artifact::{
    DirBound, DirExpanded, DirParsed, DirResolved, GlobalEnvironment, ProfileKey,
};
use destack_core::StringPool;
use destack_dir as dir;
use destack_source::ModuleId;
use destack_workspace::Module;

/// Check inputs for one module.
#[derive(Debug)]
pub(in crate::check) struct CheckInputState {
    /// The requested module id.
    pub(in crate::check) module_id: ModuleId,
    /// The requested source module.
    pub(in crate::check) module: Arc<Module>,
    /// The active semantic profile.
    pub(in crate::check) profile: ProfileKey,
    /// The shared string pool.
    pub(in crate::check) strings: Arc<StringPool>,
    /// The parsed DIR input.
    pub(in crate::check) parsed: Arc<DirParsed>,
    /// The bound DIR input.
    pub(in crate::check) bound: Arc<DirBound>,
    /// The resolved DIR input.
    pub(in crate::check) resolved: Arc<DirResolved>,
    /// The expanded DIR input.
    pub(in crate::check) expanded: Arc<DirExpanded>,
    /// The active global environment.
    pub(in crate::check) environment: Arc<GlobalEnvironment>,
}

impl CheckInputState {
    /// Create check inputs for one module.
    pub(in crate::check) fn new(
        module_id: ModuleId,
        module: Arc<Module>,
        profile: ProfileKey,
        strings: Arc<StringPool>,
        parsed: Arc<DirParsed>,
        bound: Arc<DirBound>,
        resolved: Arc<DirResolved>,
        expanded: Arc<DirExpanded>,
        environment: Arc<GlobalEnvironment>,
    ) -> Self {
        Self {
            module_id,
            module,
            profile,
            strings,
            parsed,
            bound,
            resolved,
            expanded,
            environment,
        }
    }

    /// Return the post-expansion DIR tree view visible to check.
    pub(in crate::check) fn view(&self) -> dir::View<'_> {
        dir::View::with_patches(
            &self.parsed.tree,
            std::slice::from_ref(&self.expanded.patch),
        )
    }

    /// Return the cumulative binding table visible to check.
    pub(in crate::check) fn binding_table(&self) -> dir::BindingTable<'static> {
        self.expanded.binding_table(&self.bound)
    }

    /// Return the cumulative type table visible to check inputs.
    pub(in crate::check) fn type_table(&self) -> dir::TypeTable<'static> {
        self.expanded.type_table(&self.bound)
    }

    /// Return the cumulative static table visible to check inputs.
    pub(in crate::check) fn static_table(&self) -> dir::StaticTable<'static> {
        self.expanded.static_table(&self.bound)
    }
}
