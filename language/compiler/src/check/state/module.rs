use std::sync::Arc;

use destack_artifact::{DirBound, DirExpanded, DirParsed, DirResolved, GlobalEnvironment};
use destack_core::StringPool;
use destack_dir as dir;
use destack_source::ModuleId;

use super::{CheckDecisionState, CheckInputState, CheckOutputState, CheckWorkState};

/// Check state for one module inside a checked component.
#[derive(Debug)]
pub(in crate::check) struct CheckModuleState {
    /// Module inputs read by check.
    pub(in crate::check) input: CheckInputState,
    /// Module work graph built and solved by check.
    pub(in crate::check) work: CheckWorkState,
    /// Solver decisions recorded for commit.
    pub(in crate::check) decisions: CheckDecisionState,
    /// Checked DIR outputs written by commit.
    pub(in crate::check) output: CheckOutputState,
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
        let input = CheckInputState::new(
            module,
            strings,
            parsed,
            bound,
            resolved,
            Arc::clone(&expanded),
            environment,
        );
        let work = CheckWorkState::new();
        let decisions = CheckDecisionState::new();
        let output = CheckOutputState::new(module, &expanded);

        Self {
            input,
            work,
            decisions,
            output,
        }
    }

    /// Return this module id.
    pub(in crate::check) fn module(&self) -> ModuleId {
        self.input.module
    }

    /// Return the cumulative binding table visible to check.
    pub(in crate::check) fn binding_table(&self) -> dir::BindingTable<'static> {
        self.input.binding_table()
    }

    /// Return the cumulative type table visible to check inputs.
    pub(in crate::check) fn input_type_table(&self) -> dir::TypeTable<'static> {
        self.input.type_table()
    }

    /// Return the cumulative static table visible to check inputs.
    pub(in crate::check) fn input_static_table(&self) -> dir::StaticTable<'static> {
        self.input.static_table()
    }

    /// Add one checked type.
    pub(in crate::check) fn intern_type(
        &mut self,
        ty: dir::Type,
        source: dir::LocalNodeIdAny,
    ) -> dir::LocalTypeId {
        for type_id in self.output.types.iter_type_ids() {
            if self.output.types.get_type(type_id) == &ty {
                return type_id;
            }
        }

        self.output.types.insert_type_from_any(ty, source)
    }

    /// Add or reuse one checked static value.
    pub(in crate::check) fn intern_static(&mut self, term: dir::StaticTerm) -> dir::LocalStaticId {
        let table = self.input_static_table();

        table.intern_static(&mut self.output.statics, term)
    }

    /// Return one visible type by id.
    pub(in crate::check) fn get_type(&self, type_id: dir::LocalTypeId) -> dir::Type {
        if let Some(ty) = self.output.types.get_type_maybe(type_id) {
            return ty.clone();
        }

        self.input_type_table().get_type(type_id).clone()
    }

    /// Return one visible static value by id.
    pub(in crate::check) fn get_static(&self, static_id: dir::LocalStaticId) -> dir::StaticTerm {
        if let Some(term) = self.output.statics.get_static_maybe(static_id) {
            return term.clone();
        }

        self.input_static_table().get_static(static_id).clone()
    }
}
