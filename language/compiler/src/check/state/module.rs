use std::sync::Arc;

use destack_artifact::{DirBound, DirExpanded, DirParsed, DirResolved, GlobalEnvironment};
use destack_core::StringPool;
use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexMap;

use crate::CheckError;

use super::{Constraint, FlowState, Variable, VariableId};

/// Check state for one module inside a checked component.
#[derive(Debug)]
pub(in crate::check) struct CheckModuleState {
    /// The requested module.
    pub(in crate::check) module: ModuleId,
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

    /// Checked type segment.
    pub(in crate::check) types: dir::TypeSegment,
    /// Checked static value segment.
    pub(in crate::check) statics: dir::StaticSegment,
    /// Checked resolution segment.
    pub(in crate::check) resolutions: dir::ResolutionSegment,
    /// Checked generic segment.
    pub(in crate::check) generics: dir::GenericSegment,
    /// Checked relation segment.
    pub(in crate::check) relations: dir::RelationSegment,
    /// Checked coercion segment.
    pub(in crate::check) coercions: dir::CoercionSegment,
    /// Checked extension segment.
    pub(in crate::check) extensions: dir::ExtensionSegment,
    /// Checked layout segment.
    pub(in crate::check) layouts: dir::LayoutSegment,
    /// Checked capture segment.
    pub(in crate::check) captures: dir::CaptureSegment,

    /// The visitor options used while walking this module.
    pub(in crate::check) options: dir::NodeVisitorOptions,

    /// Check variables owned by this module.
    pub(in crate::check) variables: Vec<Variable>,
    /// Type variables keyed by node.
    pub(in crate::check) node_type_variables: IndexMap<dir::GlobalNodeIdAny, VariableId>,
    /// Type variables keyed by symbol.
    pub(in crate::check) symbol_type_variables: IndexMap<dir::GlobalSymbolId, VariableId>,
    /// Static variables keyed by node.
    pub(in crate::check) node_static_variables: IndexMap<dir::GlobalNodeIdAny, VariableId>,
    /// Static variables keyed by symbol.
    pub(in crate::check) symbol_static_variables: IndexMap<dir::GlobalSymbolId, VariableId>,
    /// Next generic slot index by owner.
    pub(in crate::check) generic_slot_indexes: IndexMap<dir::GlobalSymbolId, dir::GenericSlotIndex>,
    /// Next induced generic name index by owner.
    pub(in crate::check) induced_generic_indexes: IndexMap<dir::GlobalSymbolId, u32>,
    /// Flow state while walking this module.
    pub(in crate::check) flow: FlowState,

    /// Constraints produced by walking DIR.
    pub(in crate::check) constraints: Vec<Constraint>,
    /// Recoverable diagnostics collected while checking.
    pub(in crate::check) diagnostics: Vec<CheckError>,
}

impl CheckModuleState {
    /// Create check state for one requested module.
    pub(in crate::check) fn new(
        module: ModuleId,
        strings: Arc<StringPool>,
        parsed: Arc<DirParsed>,
        bound: Arc<DirBound>,
        resolved: Arc<DirResolved>,
        expanded: Arc<DirExpanded>,
        environment: Arc<GlobalEnvironment>,
    ) -> Self {
        let types = dir::TypeSegment::from_base(&expanded.types);
        let statics = dir::StaticSegment::from_base(&expanded.statics);
        let resolutions = dir::ResolutionSegment::new(module);
        let generics = dir::GenericSegment::new(module);
        let relations = dir::RelationSegment::new(module);
        let coercions = dir::CoercionSegment::new(module);
        let extensions = dir::ExtensionSegment::new(module);
        let layouts = dir::LayoutSegment::new(module);
        let captures = dir::CaptureSegment::new(module);

        Self {
            module,
            strings,
            parsed,
            bound,
            resolved,
            expanded,
            environment,
            types,
            statics,
            resolutions,
            generics,
            relations,
            coercions,
            extensions,
            layouts,
            captures,
            options: dir::NodeVisitorOptions::default(),
            variables: Vec::new(),
            node_type_variables: IndexMap::new(),
            symbol_type_variables: IndexMap::new(),
            node_static_variables: IndexMap::new(),
            symbol_static_variables: IndexMap::new(),
            generic_slot_indexes: IndexMap::new(),
            induced_generic_indexes: IndexMap::new(),
            flow: FlowState::default(),
            constraints: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    /// Return the cumulative binding table visible to check.
    pub(in crate::check) fn binding_table(&self) -> dir::BindingTable<'static> {
        self.expanded.binding_table(&self.bound)
    }

    /// Return the cumulative type table visible to check inputs.
    pub(in crate::check) fn input_type_table(&self) -> dir::TypeTable<'static> {
        self.expanded.type_table(&self.bound)
    }

    /// Return the cumulative static table visible to check inputs.
    pub(in crate::check) fn input_static_table(&self) -> dir::StaticTable<'static> {
        self.expanded.static_table(&self.bound)
    }

    /// Add one checked type.
    pub(in crate::check) fn intern_type(
        &mut self,
        ty: dir::Type,
        source: dir::LocalNodeIdAny,
    ) -> dir::LocalTypeId {
        for type_id in self.types.iter_type_ids() {
            if self.types.get_type(type_id) == &ty {
                return type_id;
            }
        }

        self.types.insert_type_from_any(ty, source)
    }

    /// Add or reuse one checked static value.
    pub(in crate::check) fn intern_static(&mut self, term: dir::StaticTerm) -> dir::LocalStaticId {
        let table = self.input_static_table();

        table.intern_static(&mut self.statics, term)
    }

    /// Return one visible type by id.
    pub(in crate::check) fn get_type(&self, type_id: dir::LocalTypeId) -> dir::Type {
        if let Some(ty) = self.types.get_type_maybe(type_id) {
            return ty.clone();
        }

        self.input_type_table().get_type(type_id).clone()
    }

    /// Return one visible static value by id.
    pub(in crate::check) fn get_static(&self, static_id: dir::LocalStaticId) -> dir::StaticTerm {
        if let Some(term) = self.statics.get_static_maybe(static_id) {
            return term.clone();
        }

        self.input_static_table().get_static(static_id).clone()
    }
}
