use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{CheckState, FlowState, StaticCondition};
use crate::CheckError;

use super::{CheckInputState, CheckOutputState, ImportTable};

impl CheckState<'_> {
    /// Return loaded input for one module.
    pub(in crate::check) fn input(&self, module: ModuleId) -> &CheckInputState {
        match self.inputs.get(&module) {
            Some(input) => input,
            None => panic!("check input {module:?} was not loaded"),
        }
    }

    /// Return loaded input for one module mutably.
    pub(in crate::check) fn input_mut(&mut self, module: ModuleId) -> &mut CheckInputState {
        match self.inputs.get_mut(&module) {
            Some(input) => input,
            None => panic!("check input {module:?} was not loaded"),
        }
    }

    /// Return checked output for one module.
    pub(in crate::check) fn output(&self, module: ModuleId) -> &CheckOutputState {
        match self.outputs.get(&module) {
            Some(output) => output,
            None => panic!("check output {module:?} was not loaded"),
        }
    }

    /// Return checked output for one module mutably.
    pub(in crate::check) fn output_mut(&mut self, module: ModuleId) -> &mut CheckOutputState {
        match self.outputs.get_mut(&module) {
            Some(output) => output,
            None => panic!("check output {module:?} was not loaded"),
        }
    }

    /// Return imported dependency ids for one module.
    pub(in crate::check) fn imports(&self, module: ModuleId) -> &ImportTable {
        match self.imports.get(&module) {
            Some(imports) => imports,
            None => panic!("check imports {module:?} were not loaded"),
        }
    }

    /// Return imported dependency ids for one module mutably.
    pub(in crate::check) fn imports_mut(&mut self, module: ModuleId) -> &mut ImportTable {
        match self.imports.get_mut(&module) {
            Some(imports) => imports,
            None => panic!("check imports {module:?} were not loaded"),
        }
    }

    /// Return flow state for one module.
    pub(in crate::check) fn flow(&self, module: ModuleId) -> &FlowState {
        match self.flows.get(&module) {
            Some(flow) => flow,
            None => panic!("check flow {module:?} was not loaded"),
        }
    }

    /// Return flow state for one module mutably.
    pub(in crate::check) fn flow_mut(&mut self, module: ModuleId) -> &mut FlowState {
        match self.flows.get_mut(&module) {
            Some(flow) => flow,
            None => panic!("check flow {module:?} was not loaded"),
        }
    }

    /// Return captures for one module mutably.
    pub(in crate::check) fn captures_mut(&mut self, module: ModuleId) -> &mut Vec<Capture> {
        match self.captures.get_mut(&module) {
            Some(captures) => captures,
            None => panic!("check captures {module:?} were not loaded"),
        }
    }

    /// Return static availability for one module.
    pub(in crate::check) fn availability(
        &self,
        module: ModuleId,
    ) -> &indexmap::IndexMap<dir::GlobalSymbolId, StaticCondition> {
        match self.availability.get(&module) {
            Some(availability) => availability,
            None => panic!("check availability {module:?} was not loaded"),
        }
    }

    /// Return static availability for one module mutably.
    pub(in crate::check) fn availability_mut(
        &mut self,
        module: ModuleId,
    ) -> &mut indexmap::IndexMap<dir::GlobalSymbolId, StaticCondition> {
        match self.availability.get_mut(&module) {
            Some(availability) => availability,
            None => panic!("check availability {module:?} was not loaded"),
        }
    }

    /// Return diagnostics for one module mutably.
    pub(in crate::check) fn diagnostics_mut(&mut self, module: ModuleId) -> &mut Vec<CheckError> {
        match self.diagnostics.get_mut(&module) {
            Some(diagnostics) => diagnostics,
            None => panic!("check diagnostics {module:?} were not loaded"),
        }
    }

    /// Add one checked type.
    pub(in crate::check) fn intern_type(
        &mut self,
        module: ModuleId,
        ty: dir::Type,
        source: dir::LocalNodeIdAny,
    ) -> dir::LocalTypeId {
        for type_id in self.output(module).types.iter_type_ids() {
            if self.output(module).types.get_type(type_id) == &ty {
                return type_id;
            }
        }

        self.output_mut(module).types.insert_type_from_any(ty, source)
    }

    /// Add or reuse one checked static value.
    pub(in crate::check) fn intern_static(
        &mut self,
        module: ModuleId,
        term: dir::StaticTerm,
    ) -> dir::LocalStaticId {
        let table = self.input(module).static_table();
        let output = &mut self.output_mut(module).statics;

        table.intern_static(output, term)
    }

    /// Return one local type from output or input.
    pub(in crate::check) fn local_type(
        &self,
        module: ModuleId,
        type_id: dir::LocalTypeId,
    ) -> dir::Type {
        if let Some(ty) = self.output(module).types.get_type_maybe(type_id) {
            return ty.clone();
        }

        self.input(module).type_table().get_type(type_id).clone()
    }

    /// Return one local type source from output or input.
    pub(in crate::check) fn local_type_source(
        &self,
        module: ModuleId,
        type_id: dir::LocalTypeId,
    ) -> dir::LocalNodeIdAny {
        if self.output(module).types.get_type_maybe(type_id).is_some() {
            return self.output(module).types.get_type_source(type_id);
        }

        self.input(module).type_table().get_type_source(type_id)
    }

    /// Return one local static value from output or input.
    pub(in crate::check) fn local_static(
        &self,
        module: ModuleId,
        static_id: dir::LocalStaticId,
    ) -> dir::StaticTerm {
        if let Some(term) = self.output(module).statics.get_static_maybe(static_id) {
            return term.clone();
        }

        self.input(module).static_table().get_static(static_id).clone()
    }
}

/// Captures discovered for one walked function body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::check) struct Capture {
    /// The function symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// Outer symbols read by this function.
    pub(in crate::check) symbols: Vec<dir::GlobalSymbolId>,
    /// Outer receiver read by this function.
    pub(in crate::check) receiver: Option<ReceiverCapture>,
    /// The explicit capture directive.
    pub(in crate::check) directive: Option<dir::CaptureDirective>,
}

/// Receiver captured by one walked function body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct ReceiverCapture {
    /// The receiver symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// The nominal owner that supplies contextual `this`, when any.
    pub(in crate::check) owner: Option<dir::GlobalSymbolId>,
    /// The receiver type variable.
    pub(in crate::check) ty: super::VariableId,
}
