//! Borrow analysis using forward dataflow.
//!
//! Tracks active borrows across control flow to detect borrow violations
//! that span multiple basic blocks.

use std::collections::HashMap;

use destack_mir as mir;
use mir::{Instruction, Value};

use super::{ControlFlowGraph, Lattice, LivenessAnalysis, forward_dataflow};
use crate::optimize::common::terminator_arguments_for_successor;
use crate::optimize::{Analysis, AnalysisId, FunctionAnalyses, FunctionAnalysis};

/// Location where a borrow was created.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BorrowLocation {
    /// The instruction that created the borrow.
    pub instruction: mir::LocalNodeId<Instruction>,
    /// The reference value created by the borrow.
    pub reference: Value,
    /// The value being borrowed from.
    pub origin: Value,
    /// Whether this is a mutable borrow.
    pub is_mutable: bool,
}

/// Borrow state for a single value (whether it's currently borrowed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BorrowState {
    /// Not borrowed on any path reaching this point.
    NotBorrowed,
    /// Borrowed on some paths but not others.
    MaybeBorrowed {
        /// First location where a borrow was created.
        first_at: mir::LocalNodeId<Instruction>,
    },
    /// Borrowed on all paths reaching this point.
    Borrowed {
        /// Location where the borrow was created.
        at: mir::LocalNodeId<Instruction>,
    },
}

impl BorrowState {
    /// Check if this state represents a possibly active borrow.
    pub fn is_possibly_borrowed(&self) -> bool {
        !matches!(self, BorrowState::NotBorrowed)
    }

    /// Check if this is a "maybe borrowed" state.
    pub fn is_maybe_borrowed(&self) -> bool {
        matches!(self, BorrowState::MaybeBorrowed { .. })
    }

    /// Get the borrow location if borrowed.
    pub fn borrow_location(&self) -> Option<mir::LocalNodeId<Instruction>> {
        match self {
            BorrowState::NotBorrowed => None,
            BorrowState::MaybeBorrowed { first_at } => Some(*first_at),
            BorrowState::Borrowed { at } => Some(*at),
        }
    }

    /// Compute the meet of two borrow states.
    fn meet(self, other: Self) -> Self {
        match (self, other) {
            (BorrowState::NotBorrowed, BorrowState::NotBorrowed) => BorrowState::NotBorrowed,
            (BorrowState::Borrowed { at }, BorrowState::Borrowed { .. }) => {
                BorrowState::Borrowed { at }
            }
            // different states = maybe borrowed
            (BorrowState::NotBorrowed, BorrowState::Borrowed { at })
            | (BorrowState::Borrowed { at }, BorrowState::NotBorrowed) => {
                BorrowState::MaybeBorrowed { first_at: at }
            }
            (BorrowState::MaybeBorrowed { first_at }, _)
            | (_, BorrowState::MaybeBorrowed { first_at }) => {
                BorrowState::MaybeBorrowed { first_at }
            }
        }
    }
}

/// Active borrow information for all values at a program point.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BorrowMap {
    /// For each value, its borrow state.
    states: HashMap<Value, BorrowState>,
    /// For each reference value, what it borrows from.
    reference_origins: HashMap<Value, Vec<Value>>,
    /// For each reference value, where it was created.
    reference_locations: HashMap<Value, mir::LocalNodeId<Instruction>>,
    /// For each local slot, its borrow state.
    local_states: HashMap<mir::LocalNodeId<mir::Local>, BorrowState>,
    /// For each reference value, which local slot it borrows from.
    local_reference_origins: HashMap<Value, Vec<mir::LocalNodeId<mir::Local>>>,
    /// For each local reference value, where it was created.
    local_reference_locations: HashMap<Value, mir::LocalNodeId<Instruction>>,
}

impl BorrowMap {
    /// Create an empty borrow map.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the borrow state for a value.
    pub fn get(&self, value: impl Into<mir::ValueReference>) -> BorrowState {
        let Some(value) = value.into().value() else {
            return BorrowState::NotBorrowed;
        };

        self.states
            .get(&value)
            .cloned()
            .unwrap_or(BorrowState::NotBorrowed)
    }

    /// Check if a value is possibly borrowed.
    pub fn is_possibly_borrowed(&self, value: impl Into<mir::ValueReference>) -> bool {
        self.get(value).is_possibly_borrowed()
    }

    /// Get the origin of a reference value when there is exactly one.
    pub fn reference_origin(&self, reference: impl Into<mir::ValueReference>) -> Option<Value> {
        let reference = reference.into().value()?;

        self.reference_origins
            .get(&reference)
            .and_then(|origins| origins.first().copied())
    }

    /// Get all possible origins for a reference value.
    pub fn reference_origins(&self, reference: impl Into<mir::ValueReference>) -> Option<&[Value]> {
        let reference = reference.into().value()?;

        self.reference_origins
            .get(&reference)
            .map(|origins| origins.as_slice())
    }

    /// Get the local origin for a reference value.
    pub fn local_reference_origin(
        &self,
        reference: impl Into<mir::ValueReference>,
    ) -> Option<mir::LocalNodeId<mir::Local>> {
        let reference = reference.into().value()?;

        self.local_reference_origins
            .get(&reference)
            .and_then(|locals| locals.first().copied())
    }

    /// Get all possible local origins for a reference value.
    pub fn local_reference_origins(
        &self,
        reference: impl Into<mir::ValueReference>,
    ) -> Option<&[mir::LocalNodeId<mir::Local>]> {
        let reference = reference.into().value()?;

        self.local_reference_origins
            .get(&reference)
            .map(|locals| locals.as_slice())
    }

    /// Record a new borrow.
    pub fn add_borrow(
        &mut self,
        reference: impl Into<mir::ValueReference>,
        origin: impl Into<mir::ValueReference>,
        at: mir::LocalNodeId<Instruction>,
    ) {
        let (Some(reference), Some(origin)) = (reference.into().value(), origin.into().value())
        else {
            return;
        };

        self.states.insert(origin, BorrowState::Borrowed { at });
        self.reference_origins.insert(reference, vec![origin]);
        self.reference_locations.insert(reference, at);
    }

    /// Record a new local borrow.
    pub fn add_local_borrow(
        &mut self,
        reference: impl Into<mir::ValueReference>,
        local: impl Into<mir::LocalReference>,
        at: mir::LocalNodeId<Instruction>,
    ) {
        let (Some(reference), Some(local)) = (reference.into().value(), local.into().local())
        else {
            return;
        };

        self.local_states
            .insert(local, BorrowState::Borrowed { at });
        self.local_reference_origins.insert(reference, vec![local]);
        self.local_reference_locations.insert(reference, at);
    }

    /// Transfer borrow relationship from one reference to another.
    ///
    /// Used when a reference is passed through a block parameter, the block
    /// parameter inherits the borrow relationship of the argument.
    pub fn transfer_borrow(
        &mut self,
        from_ref: impl Into<mir::ValueReference>,
        to_ref: impl Into<mir::ValueReference>,
    ) {
        let (Some(from_ref), Some(to_ref)) = (from_ref.into().value(), to_ref.into().value())
        else {
            return;
        };

        if let Some(origins) = self.reference_origins.get(&from_ref)
            && let Some(&at) = self.reference_locations.get(&from_ref)
        {
            self.reference_origins.insert(to_ref, origins.clone());
            self.reference_locations.insert(to_ref, at);
        }

        if let Some(locals) = self.local_reference_origins.get(&from_ref)
            && let Some(&at) = self.local_reference_locations.get(&from_ref)
        {
            self.local_reference_origins.insert(to_ref, locals.clone());
            self.local_reference_locations.insert(to_ref, at);
        }
    }

    /// Merge borrow origins from multiple references into a single reference.
    pub fn merge_borrows(
        &mut self,
        destination: impl Into<mir::ValueReference>,
        sources: &[mir::ValueReference],
        at: mir::LocalNodeId<Instruction>,
    ) {
        let Some(destination) = destination.into().value() else {
            return;
        };

        let mut origins = Vec::new();
        let mut locals = Vec::new();

        for &source in sources {
            let Some(source) = source.value() else {
                continue;
            };

            if let Some(source_origins) = self.reference_origins.get(&source) {
                for &origin in source_origins {
                    if !origins.contains(&origin) {
                        origins.push(origin);
                    }
                }
            }

            if let Some(source_locals) = self.local_reference_origins.get(&source) {
                for &local in source_locals {
                    if !locals.contains(&local) {
                        locals.push(local);
                    }
                }
            }
        }

        if !origins.is_empty() {
            for &origin in &origins {
                self.states.insert(origin, BorrowState::Borrowed { at });
            }
            self.reference_origins.insert(destination, origins);
            self.reference_locations.insert(destination, at);
        }

        if !locals.is_empty() {
            for &local in &locals {
                self.local_states
                    .insert(local, BorrowState::Borrowed { at });
            }
            self.local_reference_origins.insert(destination, locals);
            self.local_reference_locations.insert(destination, at);
        }
    }

    /// Merge borrow relationships from multiple block arguments into a parameter.
    pub fn merge_param_borrows(
        &mut self,
        param: impl Into<mir::ValueReference>,
        sources: &[mir::ValueReference],
    ) {
        let mut merged_sources: Vec<mir::ValueReference> = Vec::new();
        let mut borrow_location = None;

        for &source in sources {
            let Some(source) = source.value() else {
                continue;
            };

            let has_borrow = self.reference_origins.contains_key(&source)
                || self.local_reference_origins.contains_key(&source);
            if !has_borrow {
                continue;
            }

            let source_reference = source.into();
            if !merged_sources.contains(&source_reference) {
                merged_sources.push(source_reference);
            }

            if borrow_location.is_none() {
                borrow_location = self
                    .reference_locations
                    .get(&source)
                    .copied()
                    .or_else(|| self.local_reference_locations.get(&source).copied());
            }
        }

        if let Some(at) = borrow_location {
            self.merge_borrows(param, &merged_sources, at);
        }
    }

    /// Remove a borrow when its reference dies.
    pub fn expire_borrow(&mut self, reference: impl Into<mir::ValueReference>) {
        let Some(reference) = reference.into().value() else {
            return;
        };

        if let Some(origins) = self.reference_origins.remove(&reference) {
            // only remove borrow state if no other references borrow from this origin
            for origin in origins {
                let has_other_borrows = self
                    .reference_origins
                    .values()
                    .any(|origins| origins.contains(&origin));
                if !has_other_borrows {
                    self.states.remove(&origin);
                }
            }
        }
        self.reference_locations.remove(&reference);

        if let Some(locals) = self.local_reference_origins.remove(&reference) {
            for local in locals {
                let has_other_borrows = self
                    .local_reference_origins
                    .values()
                    .any(|locals| locals.contains(&local));
                if !has_other_borrows {
                    self.local_states.remove(&local);
                }
            }
        }
        self.local_reference_locations.remove(&reference);
    }

    /// Get all references that are currently active.
    pub fn active_references(&self) -> impl Iterator<Item = Value> + '_ {
        self.reference_origins
            .keys()
            .copied()
            .chain(self.local_reference_origins.keys().copied())
    }

    /// Get all values that are currently borrowed.
    pub fn borrowed_values(&self) -> impl Iterator<Item = (Value, &BorrowState)> {
        self.states.iter().map(|(&v, s)| (v, s))
    }

    /// Get all active borrows as (reference, origin, location) tuples.
    pub fn active_borrows(
        &self,
    ) -> impl Iterator<Item = (Value, Value, mir::LocalNodeId<Instruction>)> + '_ {
        self.reference_origins
            .iter()
            .flat_map(|(&reference, origins)| {
                let location = self.reference_locations.get(&reference).copied();
                origins
                    .iter()
                    .filter_map(move |&origin| location.map(|loc| (reference, origin, loc)))
            })
    }

    /// Get all active local borrows as (reference, local, location) tuples.
    pub fn active_local_borrows(
        &self,
    ) -> impl Iterator<
        Item = (
            Value,
            mir::LocalNodeId<mir::Local>,
            mir::LocalNodeId<Instruction>,
        ),
    > + '_ {
        self.local_reference_origins
            .iter()
            .flat_map(|(&reference, locals)| {
                let location = self.local_reference_locations.get(&reference).copied();
                locals
                    .iter()
                    .filter_map(move |&local| location.map(|loc| (reference, local, loc)))
            })
    }
}

impl Lattice for BorrowMap {
    fn meet(&self, other: &Self) -> Self {
        let mut result_states = self.states.clone();
        let mut result_local_states = self.local_states.clone();

        // merge borrow states
        for (&value, state_b) in &other.states {
            result_states
                .entry(value)
                .and_modify(|state_a| {
                    *state_a = state_a.clone().meet(state_b.clone());
                })
                .or_insert_with(|| {
                    // value borrowed in other but not self = MaybeBorrowed
                    BorrowState::MaybeBorrowed {
                        first_at: state_b.borrow_location().unwrap(),
                    }
                });
        }

        for (&local, state_b) in &other.local_states {
            result_local_states
                .entry(local)
                .and_modify(|state_a| {
                    *state_a = state_a.clone().meet(state_b.clone());
                })
                .or_insert_with(|| BorrowState::MaybeBorrowed {
                    first_at: state_b.borrow_location().unwrap(),
                });
        }

        // merge reference tracking
        let mut result_origins = self.reference_origins.clone();
        for (&reference, other_origins) in &other.reference_origins {
            let entry = result_origins.entry(reference).or_default();
            for &origin in other_origins {
                if !entry.contains(&origin) {
                    entry.push(origin);
                }
            }
        }

        let mut result_locations = self.reference_locations.clone();
        for (&reference, &loc) in &other.reference_locations {
            result_locations.entry(reference).or_insert(loc);
        }

        let mut result_local_origins = self.local_reference_origins.clone();
        for (&reference, other_locals) in &other.local_reference_origins {
            let entry = result_local_origins.entry(reference).or_default();
            for &local in other_locals {
                if !entry.contains(&local) {
                    entry.push(local);
                }
            }
        }

        let mut result_local_locations = self.local_reference_locations.clone();
        for (&reference, &loc) in &other.local_reference_locations {
            result_local_locations.entry(reference).or_insert(loc);
        }

        BorrowMap {
            states: result_states,
            reference_origins: result_origins,
            reference_locations: result_locations,
            local_states: result_local_states,
            local_reference_origins: result_local_origins,
            local_reference_locations: result_local_locations,
        }
    }
}

/// Borrow analysis computes active borrows at each program point.
///
/// Uses forward dataflow to track which values are borrowed and by which
/// references. At control flow join points, borrow states are merged using
/// a lattice meet operation.
#[derive(Debug)]
pub struct BorrowAnalysis {
    /// Borrow state at entry to each block.
    block_entry: HashMap<mir::LocalNodeId<mir::Block>, BorrowMap>,
    /// Borrow state at exit of each block.
    block_exit: HashMap<mir::LocalNodeId<mir::Block>, BorrowMap>,
}

impl BorrowAnalysis {
    /// Get borrow state at entry to a block.
    pub fn state_at_entry(&self, block: mir::LocalNodeId<mir::Block>) -> Option<&BorrowMap> {
        self.block_entry.get(&block)
    }

    /// Get borrow state at exit of a block.
    pub fn state_at_exit(&self, block: mir::LocalNodeId<mir::Block>) -> Option<&BorrowMap> {
        self.block_exit.get(&block)
    }

    /// Build the analysis.
    fn build(
        function: &mir::Function,
        tree: &mir::NodeTree,
        cfg: &ControlFlowGraph,
        liveness: &LivenessAnalysis,
    ) -> Self {
        if function.entry.is_none() {
            return Self {
                block_entry: HashMap::new(),
                block_exit: HashMap::new(),
            };
        }

        // run forward dataflow
        let result = forward_dataflow(
            function,
            tree,
            cfg,
            BorrowMap::new(),
            |block_id, mut state, tree| {
                let block = tree.get(block_id);

                // transfer borrow relationships from predecessor args to block params
                let mut param_sources = vec![Vec::new(); block.parameters.len()];
                for &pred_id in cfg.predecessors(block_id) {
                    let pred_block = tree.get(pred_id);
                    let pred_terminator = tree.get(pred_block.terminator);
                    let arguments = terminator_arguments_for_successor(pred_terminator, block_id);
                    for (index, &arg) in arguments.iter().enumerate() {
                        if let Some(slot) = param_sources.get_mut(index) {
                            slot.push(arg);
                        }
                    }
                }
                for (param, sources) in block.parameters.iter().zip(param_sources) {
                    state.merge_param_borrows(param.value, &sources);
                }

                // expire borrows whose references are not live-in to this block
                let is_live_in = |value| {
                    liveness.is_live_in(block_id, value)
                        || block
                            .parameters
                            .iter()
                            .filter_map(|param| param.value.value())
                            .any(|param| param == value)
                };
                let dead_refs: Vec<Value> = state
                    .active_references()
                    .filter(|&reference| !is_live_in(reference))
                    .collect();
                for reference in dead_refs {
                    state.expire_borrow(reference);
                }

                // process each instruction
                for &inst_id in &block.instructions {
                    let inst = tree.get(inst_id);
                    apply_instruction_effects(&mut state, inst_id, inst, tree);
                }

                // expire borrows whose references are not live-out
                let dead_refs: Vec<Value> = state
                    .active_references()
                    .filter(|&reference| !liveness.is_live_out(block_id, reference))
                    .collect();
                for reference in dead_refs {
                    state.expire_borrow(reference);
                }

                state
            },
        );

        let mut entry_with_transfers = HashMap::new();
        for &block_id in &function.blocks {
            let Some(entry_state) = result.block_entry.get(&block_id) else {
                continue;
            };

            let block = tree.get(block_id);
            let mut state = entry_state.clone();

            // transfer borrow relationships from predecessor args to block params
            let mut param_sources = vec![Vec::new(); block.parameters.len()];
            for &pred_id in cfg.predecessors(block_id) {
                let pred_block = tree.get(pred_id);
                let pred_terminator = tree.get(pred_block.terminator);
                let arguments = terminator_arguments_for_successor(pred_terminator, block_id);
                for (index, &arg) in arguments.iter().enumerate() {
                    if let Some(slot) = param_sources.get_mut(index) {
                        slot.push(arg);
                    }
                }
            }
            for (param, sources) in block.parameters.iter().zip(param_sources) {
                state.merge_param_borrows(param.value, &sources);
            }

            // expire borrows whose references are not live-in to this block
            let is_live_in = |value| {
                liveness.is_live_in(block_id, value)
                    || block
                        .parameters
                        .iter()
                        .filter_map(|param| param.value.value())
                        .any(|param| param == value)
            };
            let dead_refs: Vec<Value> = state
                .active_references()
                .filter(|&reference| !is_live_in(reference))
                .collect();
            for reference in dead_refs {
                state.expire_borrow(reference);
            }

            entry_with_transfers.insert(block_id, state);
        }

        Self {
            block_entry: entry_with_transfers,
            block_exit: result.block_exit,
        }
    }
}

/// Apply the effects of an instruction on borrow state.
fn apply_instruction_effects(
    state: &mut BorrowMap,
    inst_id: mir::LocalNodeId<Instruction>,
    inst: &Instruction,
    tree: &mir::NodeTree,
) {
    match inst {
        // address projections create a borrow of the container
        Instruction::ElementAddr {
            destination,
            array: origin,
            result_type,
            ..
        }
        | Instruction::FieldAddr {
            destination,
            aggregate: origin,
            result_type,
            ..
        } => {
            let Some(result_type) = result_type.ty() else {
                return;
            };
            if !reference_is_borrowed(result_type, tree) {
                return;
            }

            state.add_borrow(*destination, *origin, inst_id);
        }

        // local.address creates a borrow of the local slot
        Instruction::LocalAddr {
            destination,
            local,
            result_type,
        } => {
            let Some(result_type) = result_type.ty() else {
                return;
            };
            if !reference_is_borrowed(result_type, tree) {
                return;
            }

            state.add_local_borrow(*destination, *local, inst_id);
        }

        // cast propagates borrow from the source reference
        Instruction::Cast {
            destination,
            argument,
            ..
        } => {
            state.transfer_borrow(*argument, *destination);
        }

        // select between references merges borrow origins
        Instruction::Select {
            destination,
            then_value,
            else_value,
            ..
        } => {
            state.merge_borrows(*destination, &[*then_value, *else_value], inst_id);
        }

        _ => {}
    }
}

/// Return true when a reference type is borrowed.
fn reference_is_borrowed(ty_id: mir::LocalNodeId<mir::Type>, tree: &mir::NodeTree) -> bool {
    let ty = tree.get(ty_id);
    ty.is_borrowed_reference()
}

impl Analysis for BorrowAnalysis {
    const ID: AnalysisId = AnalysisId("borrow");
    const DEPENDENCIES: &'static [AnalysisId] = &[LivenessAnalysis::ID];
}

impl FunctionAnalysis for BorrowAnalysis {
    fn compute(
        function: &mir::Function,
        tree: &mir::NodeTree,
        analyses: &FunctionAnalyses<'_>,
    ) -> Self {
        let cfg = analyses.get::<ControlFlowGraph>();
        let liveness = analyses.get::<LivenessAnalysis>();
        Self::build(function, tree, &cfg, &liveness)
    }
}

#[cfg(test)]
mod tests {
    use super::reference_is_borrowed;
    use destack_mir as mir;

    /// Borrow analysis recognizes borrowed tensor views.
    #[test]
    fn test_reference_is_borrowed_tensor_view() {
        let mut tree = mir::NodeTree::new();
        let element = tree.insert_type(mir::Type::Float { width: 32 });
        let tensor_ref = tree.insert_type(mir::Type::TensorView {
            kind: mir::ReferenceKind::Borrowed,
            address_space: mir::AddressSpace::Stack,
            mutability: mir::Mutability::Mutable,
            element: element.into(),
            shape: vec![mir::TensorDimension::Static(4)],
            layout: mir::TensorLayout::RowMajor,
            is_nullable: false,
        });

        assert!(reference_is_borrowed(tensor_ref, &tree));
    }
}
