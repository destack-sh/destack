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
    reference_origins: HashMap<Value, Value>,
    /// For each reference value, where it was created.
    reference_locations: HashMap<Value, mir::LocalNodeId<Instruction>>,
}

impl BorrowMap {
    /// Create an empty borrow map.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the borrow state for a value.
    pub fn get(&self, value: Value) -> BorrowState {
        self.states
            .get(&value)
            .cloned()
            .unwrap_or(BorrowState::NotBorrowed)
    }

    /// Check if a value is possibly borrowed.
    pub fn is_possibly_borrowed(&self, value: Value) -> bool {
        self.get(value).is_possibly_borrowed()
    }

    /// Get the origin of a reference value.
    pub fn reference_origin(&self, reference: Value) -> Option<Value> {
        self.reference_origins.get(&reference).copied()
    }

    /// Record a new borrow.
    pub fn add_borrow(
        &mut self,
        reference: Value,
        origin: Value,
        at: mir::LocalNodeId<Instruction>,
    ) {
        self.states.insert(origin, BorrowState::Borrowed { at });
        self.reference_origins.insert(reference, origin);
        self.reference_locations.insert(reference, at);
    }

    /// Transfer borrow relationship from one reference to another.
    ///
    /// Used when a reference is passed through a block parameter, the block
    /// parameter inherits the borrow relationship of the argument.
    pub fn transfer_borrow(&mut self, from_ref: Value, to_ref: Value) {
        let origin = match self.reference_origins.get(&from_ref) {
            Some(&o) => o,
            None => return,
        };
        let at = match self.reference_locations.get(&from_ref) {
            Some(&a) => a,
            None => return,
        };

        self.reference_origins.insert(to_ref, origin);
        self.reference_locations.insert(to_ref, at);
    }

    /// Remove a borrow when its reference dies.
    pub fn expire_borrow(&mut self, reference: Value) {
        if let Some(origin) = self.reference_origins.remove(&reference) {
            // only remove borrow state if no other references borrow from this origin
            let has_other_borrows = self.reference_origins.values().any(|&orig| orig == origin);
            if !has_other_borrows {
                self.states.remove(&origin);
            }
        }
        self.reference_locations.remove(&reference);
    }

    /// Get all references that are currently active.
    pub fn active_references(&self) -> impl Iterator<Item = Value> + '_ {
        self.reference_origins.keys().copied()
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
            .filter_map(|(&reference, &origin)| {
                self.reference_locations
                    .get(&reference)
                    .map(|&loc| (reference, origin, loc))
            })
    }
}

impl Lattice for BorrowMap {
    fn meet(&self, other: &Self) -> Self {
        let mut result_states = self.states.clone();

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

        // merge reference tracking
        let mut result_origins = self.reference_origins.clone();
        for (&reference, &origin) in &other.reference_origins {
            result_origins.entry(reference).or_insert(origin);
        }

        let mut result_locations = self.reference_locations.clone();
        for (&reference, &loc) in &other.reference_locations {
            result_locations.entry(reference).or_insert(loc);
        }

        BorrowMap {
            states: result_states,
            reference_origins: result_origins,
            reference_locations: result_locations,
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

                // transfer borrow relationships from predecessor jump args to block params
                for &pred_id in cfg.predecessors(block_id) {
                    let pred_block = tree.get(pred_id);
                    let arguments =
                        terminator_arguments_for_successor(&pred_block.terminator, block_id);
                    for (param, arg) in block.parameters.iter().zip(arguments) {
                        state.transfer_borrow(*arg, param.value);
                    }
                }

                // expire borrows whose references are not live-in to this block
                let dead_refs: Vec<Value> = state
                    .active_references()
                    .filter(|&reference| !liveness.is_live_in(block_id, reference))
                    .collect();
                for reference in dead_refs {
                    state.expire_borrow(reference);
                }

                // process each instruction
                for &inst_id in &block.instructions {
                    let inst = tree.get(inst_id);
                    apply_instruction_effects(&mut state, inst_id, inst);
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

        Self {
            block_entry: result.block_entry,
            block_exit: result.block_exit,
        }
    }
}

/// Apply the effects of an instruction on borrow state.
fn apply_instruction_effects(
    state: &mut BorrowMap,
    inst_id: mir::LocalNodeId<Instruction>,
    inst: &Instruction,
) {
    match inst {
        // field.addr creates a borrow of the aggregate
        Instruction::FieldAddr {
            destination,
            aggregate,
            ..
        } => {
            state.add_borrow(*destination, *aggregate, inst_id);
        }

        // element.addr creates a borrow of the array
        Instruction::ElementAddr {
            destination, array, ..
        } => {
            state.add_borrow(*destination, *array, inst_id);
        }

        _ => {}
    }
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
