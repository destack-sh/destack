use std::sync::Arc;

use destack_artifact::{DirBound, DirExpanded, DirParsed, DirResolved, ProfileKey};
use destack_core::StringPool;
use destack_dir as dir;
use destack_source::ModuleId;
use destack_workspace::Module;
use indexmap::{IndexMap, IndexSet};

use crate::check::{Capture, CheckError, CheckState, Condition, GenericSlotId, VariableId};

/// State owned by one module inside a checked component.
pub(in crate::check) struct CheckModuleState {
    // input state
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

    // working state
    /// Out-of-component modules visible from this module.
    pub(in crate::check) dependencies: IndexSet<ModuleId>,
    /// Imported generic variables keyed by owning source symbol.
    pub(in crate::check) imported_generics: IndexMap<dir::GlobalSymbolId, Vec<VariableId>>,
    /// Imported generic variables keyed by generic slot.
    pub(in crate::check) imported_generic_by_slot: IndexMap<GenericSlotId, VariableId>,
    /// Imported generic variables keyed by parameter symbol.
    pub(in crate::check) imported_generic_by_symbol: IndexMap<dir::GlobalSymbolId, VariableId>,
    /// Captures discovered while walking this module.
    pub(in crate::check) captures: Vec<Capture>,
    /// Static availability of declarations in this module.
    pub(in crate::check) availability: IndexMap<dir::GlobalSymbolId, Condition>,
    /// Diagnostics reported while walking this module.
    pub(in crate::check) diagnostics: Vec<CheckError>,
}

impl CheckModuleState {
    /// Create module state from loaded inputs and empty output tables.
    pub(in crate::check) fn new(
        module_id: ModuleId,
        module: Arc<Module>,
        profile: ProfileKey,
        strings: Arc<StringPool>,
        parsed: Arc<DirParsed>,
        bound: Arc<DirBound>,
        resolved: Arc<DirResolved>,
        expanded: Arc<DirExpanded>,
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
            dependencies: IndexSet::new(),
            imported_generics: IndexMap::new(),
            imported_generic_by_slot: IndexMap::new(),
            imported_generic_by_symbol: IndexMap::new(),
            captures: Vec::new(),
            availability: IndexMap::new(),
            diagnostics: Vec::new(),
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

impl CheckState<'_> {
    /// Return loaded state for one module.
    pub(in crate::check) fn module(&self, module: ModuleId) -> &CheckModuleState {
        match self.modules.get(&module) {
            Some(state) => state,
            None => panic!("check module {module:?} was not loaded"),
        }
    }

    /// Return loaded state for one module mutably.
    pub(in crate::check) fn module_mut(&mut self, module: ModuleId) -> &mut CheckModuleState {
        match self.modules.get_mut(&module) {
            Some(state) => state,
            None => panic!("check module {module:?} was not loaded"),
        }
    }

    /// Return captures for one module mutably.
    pub(in crate::check) fn captures_mut(&mut self, module: ModuleId) -> &mut Vec<Capture> {
        &mut self.module_mut(module).captures
    }

    /// Return static availability for one module mutably.
    pub(in crate::check) fn availability_mut(
        &mut self,
        module: ModuleId,
    ) -> &mut indexmap::IndexMap<dir::GlobalSymbolId, Condition> {
        &mut self.module_mut(module).availability
    }
}
