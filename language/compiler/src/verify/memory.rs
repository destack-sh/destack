use std::collections::{HashMap, VecDeque};

use crate::common::mir::terminator_arguments_for_successor;
use crate::declare_mir_pass;
use destack_artifact::DiagnosticBuilder;
use destack_mir as mir;
use mir::{
    Instruction, Place, PlaceOrigin, ReferenceKind, Terminator, Type, Value, ValueReference,
};

use crate::verify::value::{instruction_consumes, instruction_uses, terminator_consumes};
use crate::verify::{VerifyError, VerifyState};

declare_mir_pass! {
    /// Verify MIR memory rules.
    #[pass(id = "memory-check")]
    pub(crate) MemoryCheck,
    "Verify memory rules"
}

/// Lifetime origin a MIR value or place is known to depend on.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
enum BorrowOrigin {
    /// The value carries no borrowed access.
    #[default]
    None,
    /// The value depends on this function activation.
    Local,
    /// The value depends on a declared lifetime.
    Lifetime(mir::Lifetime),
}

impl BorrowOrigin {
    /// Return true when this origin is valid beyond the current activation.
    fn is_escaping(&self) -> bool {
        !matches!(self, Self::Local)
    }

    /// Return true when this origin satisfies one required lifetime.
    fn is_covered_by(&self, required: &mir::Lifetime) -> bool {
        match self {
            Self::None => true,
            Self::Lifetime(lifetime) => lifetime.origins.iter().all(|origin| match origin {
                mir::LifetimeOrigin::Static => required.includes_static(),
                mir::LifetimeOrigin::Parameter(index) => required.includes_parameter(*index),
            }),
            Self::Local => false,
        }
    }

    /// Merge two origins at a control-flow join.
    fn merge(&self, other: &Self) -> Self {
        match (self, other) {
            (Self::Local, _) | (_, Self::Local) => Self::Local,
            (Self::None, lifetime) | (lifetime, Self::None) => lifetime.clone(),
            (Self::Lifetime(left), Self::Lifetime(right)) => {
                let origins = left.origins.iter().chain(&right.origins).copied();

                Self::Lifetime(mir::Lifetime::new(origins))
            }
        }
    }

    /// Return a compact label for diagnostics.
    fn label(&self) -> String {
        match self {
            Self::None => "none".to_string(),
            Self::Local => "local".to_string(),
            Self::Lifetime(lifetime) if lifetime.is_static() => "static".to_string(),
            Self::Lifetime(lifetime) => {
                let parameters: Vec<_> = lifetime.parameter_indices().collect();
                if parameters.len() == 1 {
                    format!("parameter {}", parameters[0])
                } else {
                    format!("parameters {parameters:?}")
                }
            }
        }
    }
}

/// Move state for one move-only value.
#[derive(Debug, Clone, PartialEq, Eq)]
enum MoveState {
    /// The value can be used.
    Available,
    /// The value was moved at this instruction.
    Moved { at: mir::LocalNodeIdAny },
    /// The value was moved on at least one predecessor path.
    MaybeMoved { at: mir::LocalNodeIdAny },
}

/// One active loan.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Loan {
    /// The reference value created by the borrow.
    reference: Value,
    /// The place protected by this loan.
    place: Place,
    /// Access granted by the loan.
    access: mir::Access,
    /// Source node for diagnostics.
    created_at: mir::LocalNodeIdAny,
}

/// Memory state at one program point.
#[derive(Debug, Clone, PartialEq, Eq)]
struct MemoryState {
    /// Move state for move-only values.
    moves: HashMap<Value, MoveState>,
    /// Lifetime origins for reference-like values.
    origins: HashMap<Value, BorrowOrigin>,
    /// Active loans by reference value.
    loans: HashMap<Value, Loan>,
}

impl MemoryState {
    /// Create empty memory state.
    fn new() -> Self {
        Self {
            moves: HashMap::new(),
            origins: HashMap::new(),
            loans: HashMap::new(),
        }
    }

    /// Merge another state into this state at a control-flow join.
    fn merge(&mut self, other: &Self) -> bool {
        let mut changed = false;

        for (&value, state) in &other.moves {
            changed |= self.merge_moves(value, state);
        }

        for (&value, source) in &other.origins {
            changed |= self.merge_origin(value, source);
        }

        for (&value, loan) in &other.loans {
            changed |= self.loans.insert(value, loan.clone()).as_ref() != Some(loan);
        }

        changed
    }

    /// Merge one move state.
    fn merge_moves(&mut self, value: Value, state: &MoveState) -> bool {
        let next = match self.moves.get(&value) {
            None => state.clone(),
            Some(current) => current.merge(state),
        };

        self.moves.insert(value, next.clone()).as_ref() != Some(&next)
    }

    /// Merge one lifetime origin.
    fn merge_origin(&mut self, value: Value, source: &BorrowOrigin) -> bool {
        let next = self
            .origins
            .get(&value)
            .map(|current| current.merge(source))
            .unwrap_or_else(|| source.clone());

        self.origins.insert(value, next.clone()).as_ref() != Some(&next)
    }

    /// Bind one successor parameter to one predecessor argument.
    fn bind(&mut self, source: Value, destination: Value) {
        if let Some(state) = self.moves.get(&source).cloned() {
            self.moves.insert(destination, state);
        }

        if let Some(origin) = self.origins.get(&source).cloned() {
            self.origins.insert(destination, origin);
        }

        if let Some(mut loan) = self.loans.get(&source).cloned() {
            loan.reference = destination;
            self.loans.insert(destination, loan);
        }
    }
}

impl MoveState {
    /// Merge two move states at a control-flow join.
    fn merge(&self, other: &Self) -> Self {
        match (self, other) {
            (Self::Available, Self::Available) => Self::Available,
            (Self::Moved { at }, Self::Moved { .. }) => Self::Moved { at: *at },
            (Self::MaybeMoved { at }, _) | (_, Self::MaybeMoved { at }) => {
                Self::MaybeMoved { at: *at }
            }
            (Self::Moved { at }, Self::Available) | (Self::Available, Self::Moved { at }) => {
                Self::MaybeMoved { at: *at }
            }
        }
    }
}

/// Memory checker for one function.
struct FunctionCheck<'a, 'b> {
    /// The function being verified.
    function: &'a mir::Function,
    /// The MIR tree.
    tree: &'a mir::Tree,
    /// Verification context.
    context: &'a mut VerifyState<'b>,
    /// SSA value liveness.
    liveness: mir::FunctionLiveness,
    /// Whether this pass should emit diagnostics.
    is_diagnostics_enabled: bool,
    /// Current memory state.
    state: MemoryState,
}

impl<'a, 'b> FunctionCheck<'a, 'b> {
    /// Create a function checker.
    fn new(
        function: &'a mir::Function,
        tree: &'a mir::Tree,
        context: &'a mut VerifyState<'b>,
    ) -> Self {
        let liveness = mir::FunctionLiveness::build(function, tree);

        Self {
            function,
            tree,
            context,
            liveness,
            is_diagnostics_enabled: true,
            state: MemoryState::new(),
        }
    }

    /// Verify one function.
    fn check(mut self) {
        let entries = self.compute_block_entries();

        for &block_id in &self.function.blocks {
            let Some(entry) = entries.get(&block_id).cloned() else {
                continue;
            };

            self.transfer_block(block_id, entry, true);
        }
    }

    /// Compute fixed-point entry state for each reachable block.
    fn compute_block_entries(&mut self) -> HashMap<mir::LocalNodeId<mir::Block>, MemoryState> {
        let Some(entry) = self.function.entry else {
            return HashMap::new();
        };

        let graph = mir::ControlFlowGraph::build(self.function, self.tree);
        let mut entries = HashMap::new();
        let mut exits = HashMap::new();
        let mut worklist = VecDeque::new();

        entries.insert(entry, self.initial_state());
        worklist.push_back(entry);

        while let Some(block_id) = worklist.pop_front() {
            let entry_state = self.entry_state(block_id, &graph, &entries, &exits);
            let old_entry = entries.insert(block_id, entry_state.clone());
            let exit_state = self.transfer_block(block_id, entry_state, false);
            let old_exit = exits.insert(block_id, exit_state);

            if old_entry.as_ref() == entries.get(&block_id)
                && old_exit.as_ref() == exits.get(&block_id)
            {
                continue;
            }

            let block = self.tree.get(block_id);
            let terminator = self.tree.get(block.terminator);
            for successor in terminator.successors() {
                let Some(successor) = successor.block() else {
                    continue;
                };
                if !worklist.contains(&successor) {
                    worklist.push_back(successor);
                }
            }
        }

        entries
    }

    /// Return the entry state for one block from predecessor exits.
    fn entry_state(
        &self,
        block_id: mir::LocalNodeId<mir::Block>,
        graph: &mir::ControlFlowGraph,
        entries: &HashMap<mir::LocalNodeId<mir::Block>, MemoryState>,
        exits: &HashMap<mir::LocalNodeId<mir::Block>, MemoryState>,
    ) -> MemoryState {
        if Some(block_id) == self.function.entry {
            return entries
                .get(&block_id)
                .cloned()
                .unwrap_or_else(|| self.initial_state());
        }

        let mut merged = MemoryState::new();
        for &predecessor in graph.predecessors(block_id) {
            let Some(exit) = exits.get(&predecessor) else {
                continue;
            };

            let edge = self.bind_edge_arguments(predecessor, block_id, exit.clone());
            merged.merge(&edge);
        }

        merged
    }

    /// Bind predecessor arguments to successor block parameters.
    fn bind_edge_arguments(
        &self,
        predecessor: mir::LocalNodeId<mir::Block>,
        successor: mir::LocalNodeId<mir::Block>,
        mut state: MemoryState,
    ) -> MemoryState {
        let predecessor = self.tree.get(predecessor);
        let terminator = self.tree.get(predecessor.terminator);
        let arguments = terminator_arguments_for_successor(terminator, successor);
        let successor_block = self.tree.get(successor);

        for (parameter, argument) in successor_block.parameters.iter().zip(arguments) {
            let (Some(destination), Some(source)) = (parameter.value.value(), argument.value())
            else {
                continue;
            };

            state.bind(source, destination);
        }

        state
            .loans
            .retain(|_, loan| self.is_value_live_in_successor(successor, loan.reference));

        state
    }

    /// Transfer one block from entry to exit state.
    fn transfer_block(
        &mut self,
        block_id: mir::LocalNodeId<mir::Block>,
        state: MemoryState,
        is_diagnostics_enabled: bool,
    ) -> MemoryState {
        self.state = state;
        self.is_diagnostics_enabled = is_diagnostics_enabled;

        let block = self.tree.get(block_id);
        for (index, &instruction_id) in block.instructions.iter().enumerate() {
            let instruction = self.tree.get(instruction_id);
            self.check_instruction(block_id, index, instruction_id, instruction);
            self.expire_instruction_loans(block_id, instruction_id);
        }

        self.check_terminator(block.terminator, self.tree.get(block.terminator));

        self.state.clone()
    }

    /// Return initial memory state for function entry.
    fn initial_state(&self) -> MemoryState {
        let mut state = MemoryState::new();

        for (index, parameter) in self.function.parameters.iter().enumerate() {
            let Some(value) = parameter.value.value() else {
                continue;
            };

            if self.is_move_only(value.into()) {
                state.moves.insert(value, MoveState::Available);
            }

            let source = self.parameter_origin(index as u32, parameter.ty);
            if source != BorrowOrigin::None {
                state.origins.insert(value, source);
            }
        }

        state
    }

    /// Return the lifetime origin implied by one parameter type.
    fn parameter_origin(&self, index: u32, ty: mir::TypeReference) -> BorrowOrigin {
        let mir::TypeReference::Type(ty) = ty else {
            return BorrowOrigin::None;
        };

        match self.tree.get(ty) {
            Type::Reference {
                kind: ReferenceKind::Managed,
                ..
            }
            | Type::Slice {
                kind: ReferenceKind::Managed,
                ..
            }
            | Type::TensorView {
                kind: ReferenceKind::Managed,
                ..
            } => BorrowOrigin::Lifetime(mir::Lifetime::parameter(index)),
            Type::Reference {
                kind: ReferenceKind::Borrowed,
                lifetime,
                ..
            }
            | Type::Slice {
                kind: ReferenceKind::Borrowed,
                lifetime,
                ..
            }
            | Type::TensorView {
                kind: ReferenceKind::Borrowed,
                lifetime,
                ..
            } => {
                if lifetime.is_empty() {
                    BorrowOrigin::Lifetime(mir::Lifetime::parameter(index))
                } else {
                    BorrowOrigin::Lifetime(lifetime.clone())
                }
            }
            _ => BorrowOrigin::None,
        }
    }

    /// Verify one instruction and update memory state.
    fn check_instruction(
        &mut self,
        block_id: mir::LocalNodeId<mir::Block>,
        index: usize,
        instruction_id: mir::LocalNodeId<Instruction>,
        instruction: &Instruction,
    ) {
        for value in instruction_uses(instruction, self.tree) {
            self.check_use(value, instruction_id.into_any());
        }

        match instruction {
            Instruction::LocalSet { local, .. } => {
                self.check_place_change(&Place::local(*local), instruction_id.into_any());
            }
            Instruction::Store { pointer, value } => {
                self.check_store(*pointer, *value, instruction_id.into_any());
            }
            Instruction::FieldAddr {
                destination,
                aggregate,
                ..
            } => self.create_projected_loan(*destination, *aggregate, instruction_id.into_any()),
            Instruction::ElementAddr {
                destination, array, ..
            } => {
                self.create_projected_loan(*destination, *array, instruction_id.into_any());
            }
            Instruction::LocalAddr {
                destination, local, ..
            } => {
                let place = Place::local(*local);
                self.create_loan_from_place(*destination, place, instruction_id.into_any());
            }
            Instruction::FieldSet {
                destination,
                aggregate,
                value,
                ..
            } => {
                self.check_aggregate_set(*aggregate, *value, instruction_id.into_any());
                self.propagate_origin(*aggregate, *destination);
            }
            Instruction::ElementSet {
                destination,
                array,
                value,
                ..
            } => {
                self.check_aggregate_set(*array, *value, instruction_id.into_any());
                self.propagate_origin(*array, *destination);
            }
            Instruction::Load {
                destination,
                pointer,
                ..
            }
            | Instruction::Cast {
                destination,
                argument: pointer,
                ..
            }
            | Instruction::Pin {
                destination,
                value: pointer,
                ..
            } => self.propagate_origin(*pointer, *destination),
            Instruction::StackAlloc { destination, .. } => {
                self.define_origin(*destination, BorrowOrigin::Local);
            }
            Instruction::GlobalAddr { destination, .. } => {
                self.define_origin(
                    *destination,
                    BorrowOrigin::Lifetime(mir::Lifetime::static_storage()),
                );
            }
            Instruction::Drop { value: pointer } => {
                self.check_value_change(*pointer, instruction_id.into_any());
            }
            _ => {}
        }

        for value in instruction_consumes(instruction, self.tree) {
            if matches!(instruction, Instruction::Drop { .. }) {
                self.mark_moved(value, instruction_id.into_any());
            } else {
                self.move_value(value, instruction_id.into_any());
            }
        }

        if instruction
            .call_behavior()
            .is_some_and(|behavior| behavior.suspend.may_suspend())
        {
            self.check_instruction_suspend(block_id, index, instruction_id.into_any());
        }

        self.define_destination(instruction);
    }

    /// Verify one terminator.
    fn check_terminator(
        &mut self,
        terminator_id: mir::LocalNodeId<Terminator>,
        terminator: &Terminator,
    ) {
        for value in terminator.uses() {
            self.check_use(value, terminator_id.into_any());
        }

        match terminator {
            Terminator::Return { value: Some(value) } => {
                self.check_return(*value, terminator_id.into_any());
            }
            Terminator::Yield { .. } => {
                self.check_terminator_suspend(terminator_id.into_any());
            }
            _ => {}
        }

        for value in terminator_consumes(terminator) {
            self.move_value(value, terminator_id.into_any());
        }
    }

    /// Define destination state when the instruction produces a move-only value.
    fn define_destination(&mut self, instruction: &Instruction) {
        let Some(destination) = instruction.destination().and_then(ValueReference::value) else {
            return;
        };

        if self.is_move_only(destination.into()) {
            self.state.moves.insert(destination, MoveState::Available);
        }
    }

    /// Check one value use.
    fn check_use(&mut self, value: ValueReference, anchor: mir::LocalNodeIdAny) {
        let Some(value) = value.value() else {
            return;
        };

        match self.state.moves.get(&value) {
            Some(MoveState::Moved { at }) => {
                let moved_at = self.context.anchor(self.tree, *at);
                self.emit_error(
                    VerifyError::UseAfterMove {
                        anchor: self.context.anchor(self.tree, anchor),
                        moved_at: moved_at.clone(),
                    }
                    .label(moved_at, "value moved here"),
                );
            }
            Some(MoveState::MaybeMoved { at }) => {
                let moved_at = self.context.anchor(self.tree, *at);
                self.emit_error(
                    VerifyError::MaybeUseAfterMove {
                        anchor: self.context.anchor(self.tree, anchor),
                        moved_at: moved_at.clone(),
                    }
                    .label(moved_at, "value moved on this path"),
                );
            }
            Some(MoveState::Available) | None => {}
        }
    }

    /// Move one value when it is move-only.
    fn move_value(&mut self, value: ValueReference, anchor: mir::LocalNodeIdAny) {
        let Some(value) = value.value() else {
            return;
        };

        if !self.is_move_only(value.into()) {
            return;
        }

        self.check_value_change(value.into(), anchor);
        self.mark_moved(value.into(), anchor);
    }

    /// Mark one move-only value as moved.
    fn mark_moved(&mut self, value: ValueReference, anchor: mir::LocalNodeIdAny) {
        let Some(value) = value.value() else {
            return;
        };
        if !self.is_move_only(value.into()) {
            return;
        }

        self.state
            .moves
            .insert(value, MoveState::Moved { at: anchor });
    }

    /// Create a loan whose projected place is already recorded on the reference.
    fn create_projected_loan(
        &mut self,
        reference: ValueReference,
        source: ValueReference,
        anchor: mir::LocalNodeIdAny,
    ) {
        let place = self.place_for_value(reference);
        self.create_loan_from_place(reference, place, anchor);

        if let (Some(source), Some(reference)) = (source.value(), reference.value()) {
            self.propagate_origin(source.into(), reference.into());
        }
    }

    /// Create one active loan.
    fn create_loan_from_place(
        &mut self,
        reference: ValueReference,
        place: Place,
        anchor: mir::LocalNodeIdAny,
    ) {
        let Some(reference_value) = reference.value() else {
            return;
        };
        if !self.is_borrowed_reference(reference) {
            return;
        }

        let access = self.reference_access(reference).unwrap_or_default();
        let loans: Vec<_> = self.state.loans.values().cloned().collect();
        for loan in loans {
            if !loan.place.may_overlap(&place) {
                continue;
            }
            if !loan.access.is_exclusive() && !access.is_exclusive() {
                continue;
            }

            let existing_loan = self.context.anchor(self.tree, loan.created_at);
            self.emit_error(
                VerifyError::ConflictingLoan {
                    anchor: self.context.anchor(self.tree, anchor),
                    existing_loan: existing_loan.clone(),
                }
                .label(existing_loan, "active loan is here"),
            );
        }

        let source = self.origin_for_place(&place);
        self.state.origins.entry(reference_value).or_insert(source);
        self.state.loans.insert(
            reference_value,
            Loan {
                reference: reference_value,
                place,
                access,
                created_at: anchor,
            },
        );
    }

    /// Check a pointer store for access and borrow escape.
    fn check_store(
        &mut self,
        pointer: ValueReference,
        value: ValueReference,
        anchor: mir::LocalNodeIdAny,
    ) {
        if self
            .reference_access(pointer)
            .is_some_and(|access| !access.can_write())
        {
            self.emit_error(VerifyError::ReadonlyWrite {
                anchor: self.context.anchor(self.tree, anchor),
            });
        }

        let pointer_place = self.place_for_value(pointer);

        let value_source = self.origin_for_value(value);
        let target_source = self.origin_for_place(&pointer_place);
        if value_source == BorrowOrigin::Local && target_source.is_escaping() {
            self.emit_error(VerifyError::BorrowOutlivesOrigin {
                anchor: self.context.anchor(self.tree, anchor),
                origin: value_source.label(),
            });
        }
    }

    /// Check a structural aggregate update for borrow escape.
    fn check_aggregate_set(
        &mut self,
        aggregate: ValueReference,
        value: ValueReference,
        anchor: mir::LocalNodeIdAny,
    ) {
        let aggregate_source = self.origin_for_value(aggregate);
        let value_source = self.origin_for_value(value);
        if value_source == BorrowOrigin::Local && aggregate_source.is_escaping() {
            self.emit_error(VerifyError::BorrowOutlivesOrigin {
                anchor: self.context.anchor(self.tree, anchor),
                origin: value_source.label(),
            });
        }
    }

    /// Check one returned value.
    fn check_return(&mut self, value: ValueReference, anchor: mir::LocalNodeIdAny) {
        let source = self.origin_for_value(value);
        if source == BorrowOrigin::None {
            return;
        }

        if source == BorrowOrigin::Local {
            self.emit_error(VerifyError::BorrowOutlivesOrigin {
                anchor: self.context.anchor(self.tree, anchor),
                origin: source.label(),
            });
            return;
        }

        let Some(required) = self.explicit_return_lifetime() else {
            return;
        };

        if !source.is_covered_by(required) {
            self.emit_error(VerifyError::BorrowOutlivesOrigin {
                anchor: self.context.anchor(self.tree, anchor),
                origin: source.label(),
            });
        }
    }

    /// Check whether changing a value invalidates active loans.
    fn check_value_change(&mut self, value: ValueReference, anchor: mir::LocalNodeIdAny) {
        let place = self.place_for_value(value);
        self.check_place_change(&place, anchor);
    }

    /// Check whether changing a place invalidates active loans.
    fn check_place_change(&mut self, place: &Place, anchor: mir::LocalNodeIdAny) {
        let loans: Vec<_> = self.state.loans.values().cloned().collect();
        for loan in loans {
            if !loan.place.may_overlap(place) {
                continue;
            }

            let borrowed_at = self.context.anchor(self.tree, loan.created_at);
            let error = VerifyError::ChangeOfBorrowedPlace {
                anchor: self.context.anchor(self.tree, anchor),
                borrowed_at: borrowed_at.clone(),
            };
            self.emit_error(error.label(borrowed_at, "loan is here"));
        }
    }

    /// Check whether borrowed access crosses one suspending instruction.
    fn check_instruction_suspend(
        &mut self,
        block_id: mir::LocalNodeId<mir::Block>,
        index: usize,
        anchor: mir::LocalNodeIdAny,
    ) {
        let loans: Vec<_> = self.state.loans.values().cloned().collect();
        for loan in loans {
            if !self.liveness.is_value_live_after_instruction(
                block_id,
                index,
                loan.reference,
                self.tree,
            ) {
                continue;
            }

            self.emit_borrow_across_suspend(anchor, loan.created_at);
        }
    }

    /// Check whether borrowed access crosses one suspending terminator.
    fn check_terminator_suspend(&mut self, anchor: mir::LocalNodeIdAny) {
        let loans: Vec<_> = self.state.loans.values().cloned().collect();
        for loan in loans {
            self.emit_borrow_across_suspend(anchor, loan.created_at);
        }
    }

    /// Emit one borrow-across-suspend diagnostic.
    fn emit_borrow_across_suspend(
        &mut self,
        anchor: mir::LocalNodeIdAny,
        borrowed_at: mir::LocalNodeIdAny,
    ) {
        let borrowed_at = self.context.anchor(self.tree, borrowed_at);
        self.emit_error(
            VerifyError::BorrowAcrossSuspend {
                anchor: self.context.anchor(self.tree, anchor),
                borrowed_at: borrowed_at.clone(),
            }
            .label(borrowed_at, "borrow is created here"),
        );
    }

    /// Expire loans whose reference is no longer live after this instruction.
    fn expire_instruction_loans(
        &mut self,
        block_id: mir::LocalNodeId<mir::Block>,
        instruction_id: mir::LocalNodeId<Instruction>,
    ) {
        let block = self.tree.get(block_id);
        let Some(index) = block
            .instructions
            .iter()
            .position(|id| *id == instruction_id)
        else {
            return;
        };
        self.state.loans.retain(|_, loan| {
            self.liveness.is_value_live_after_instruction(
                block_id,
                index,
                loan.reference,
                self.tree,
            )
        });
    }

    /// Return whether a value remains live after entering one successor.
    fn is_value_live_in_successor(
        &self,
        block_id: mir::LocalNodeId<mir::Block>,
        value: Value,
    ) -> bool {
        if self.liveness.is_value_live_in(block_id, value) {
            return true;
        }

        let block = self.tree.get(block_id);
        let is_parameter = block
            .parameters
            .iter()
            .any(|parameter| parameter.value.value() == Some(value));
        if !is_parameter {
            return false;
        }

        let is_used_by_instruction = block.instructions.iter().any(|instruction_id| {
            instruction_uses(self.tree.get(*instruction_id), self.tree)
                .iter()
                .any(|used| used.value() == Some(value))
        });

        let is_used_by_terminator = self
            .tree
            .get(block.terminator)
            .uses()
            .iter()
            .any(|used| used.value() == Some(value));

        is_used_by_instruction || is_used_by_terminator
    }

    /// Return whether a value is move-only.
    fn is_move_only(&self, value: ValueReference) -> bool {
        let Some(ty) = value.value().and_then(|value| self.type_for_value(value)) else {
            return false;
        };

        self.tree.get(ty).copy().is_no()
    }

    /// Return true when one value is a borrowed reference-like value.
    fn is_borrowed_reference(&self, value: ValueReference) -> bool {
        let Some(value) = value.value() else {
            return false;
        };
        let Some(ty) = self.type_for_value(value) else {
            return false;
        };

        self.tree.get(ty).is_borrowed_reference()
    }

    /// Return access for one reference-like value.
    fn reference_access(&self, value: ValueReference) -> Option<mir::Access> {
        let value = value.value()?;
        let ty = self.type_for_value(value)?;
        match self.tree.get(ty) {
            Type::Reference { access, .. }
            | Type::Slice { access, .. }
            | Type::TensorView { access, .. } => Some(*access),
            _ => None,
        }
    }

    /// Propagate the known lifetime origin from one value to another.
    fn propagate_origin(&mut self, source: ValueReference, destination: ValueReference) {
        let Some(destination) = destination.value() else {
            return;
        };
        let source = self.origin_for_value(source);
        if source != BorrowOrigin::None {
            self.state.origins.insert(destination, source);
        }
    }

    /// Define the lifetime origin for one destination value.
    fn define_origin(&mut self, destination: ValueReference, source: BorrowOrigin) {
        let Some(destination) = destination.value() else {
            return;
        };

        self.state.origins.insert(destination, source);
    }

    /// Return the lifetime origin for one value.
    fn origin_for_value(&self, value: ValueReference) -> BorrowOrigin {
        let Some(value) = value.value() else {
            return BorrowOrigin::None;
        };

        self.state
            .origins
            .get(&value)
            .cloned()
            .unwrap_or_else(|| self.type_origin(value))
    }

    /// Return the lifetime origin implied by one value type.
    fn type_origin(&self, value: Value) -> BorrowOrigin {
        let Some(ty) = self.type_for_value(value) else {
            return BorrowOrigin::None;
        };

        match self.tree.get(ty) {
            Type::Reference {
                kind: ReferenceKind::Borrowed,
                lifetime,
                ..
            }
            | Type::Slice {
                kind: ReferenceKind::Borrowed,
                lifetime,
                ..
            }
            | Type::TensorView {
                kind: ReferenceKind::Borrowed,
                lifetime,
                ..
            } if !lifetime.is_empty() => BorrowOrigin::Lifetime(lifetime.clone()),
            _ => self
                .tree
                .type_reference_lifetime(ty.into())
                .map(BorrowOrigin::Lifetime)
                .unwrap_or_default(),
        }
    }

    /// Return the lifetime origin for one place.
    fn origin_for_place(&self, place: &Place) -> BorrowOrigin {
        match place.origin {
            PlaceOrigin::Local(_) => BorrowOrigin::Local,
            PlaceOrigin::Global(_) => BorrowOrigin::Lifetime(mir::Lifetime::static_storage()),
            PlaceOrigin::Value(value) => self.storage_origin_for_value(value),
        }
    }

    /// Return the lifetime origin implied by a value used as storage.
    fn storage_origin_for_value(&self, value: ValueReference) -> BorrowOrigin {
        let Some(value) = value.value() else {
            return BorrowOrigin::None;
        };

        if let Some(source) = self.state.origins.get(&value) {
            return source.clone();
        }

        let Some(ty) = self.type_for_value(value) else {
            return BorrowOrigin::None;
        };

        match self.tree.get(ty) {
            Type::Reference {
                kind: ReferenceKind::Managed,
                ..
            }
            | Type::Slice {
                kind: ReferenceKind::Managed,
                ..
            }
            | Type::TensorView {
                kind: ReferenceKind::Managed,
                ..
            } => BorrowOrigin::Local,
            Type::Reference {
                kind: ReferenceKind::Unique,
                ..
            }
            | Type::Slice {
                kind: ReferenceKind::Unique,
                ..
            }
            | Type::TensorView {
                kind: ReferenceKind::Unique,
                ..
            } => BorrowOrigin::Local,
            Type::Reference {
                kind: ReferenceKind::Borrowed,
                lifetime,
                ..
            }
            | Type::Slice {
                kind: ReferenceKind::Borrowed,
                lifetime,
                ..
            }
            | Type::TensorView {
                kind: ReferenceKind::Borrowed,
                lifetime,
                ..
            } if !lifetime.is_empty() => BorrowOrigin::Lifetime(lifetime.clone()),
            _ => BorrowOrigin::None,
        }
    }

    /// Return the explicitly declared return lifetime.
    fn explicit_return_lifetime(&self) -> Option<&mir::Lifetime> {
        if self
            .tree
            .type_reference_lifetime(self.function.return_type)
            .is_none()
        {
            return None;
        }

        Some(&self.function.return_lifetime)
    }

    /// Return the best known place for one value.
    fn place_for_value(&self, value: ValueReference) -> Place {
        let Some(value) = value.value() else {
            return Place::value(value);
        };

        self.function
            .value_place(value)
            .cloned()
            .unwrap_or_else(|| Place::value(value.into()))
    }

    /// Return the known type for one value.
    fn type_for_value(&self, value: Value) -> Option<mir::LocalNodeId<Type>> {
        if let Some(ty) = self.function.value_type(value) {
            return Some(ty);
        }

        self.function
            .parameters
            .iter()
            .find(|parameter| parameter.value.value() == Some(value))
            .and_then(|parameter| parameter.ty.ty())
    }

    /// Emit one error when diagnostics are enabled.
    fn emit_error(&mut self, error: impl Into<DiagnosticBuilder<VerifyError>>) {
        if self.is_diagnostics_enabled {
            self.context.emit_error(error);
        }
    }
}

impl MemoryCheck {
    /// Verify memory rules for one MIR tree.
    pub(crate) fn run(&self, tree: &mut mir::Tree, context: &mut VerifyState<'_>) {
        let functions: Vec<_> = tree
            .iter_nodes::<mir::Function>()
            .map(|(id, _)| id)
            .collect();

        for function_id in functions {
            tree.infer_and_set_function_return_lifetime(function_id);
            tree.rebuild_function_places(function_id);

            let function = tree.get(function_id);
            if function.entry.is_none() {
                continue;
            }

            FunctionCheck::new(function, tree, context).check();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verify::tests::VerifyProgram;

    /// Assert exactly one error of the expected shape.
    fn assert_one_error(errors: &[VerifyError], expected: fn(&VerifyError) -> bool) {
        assert_eq!(errors.len(), 1, "{errors:#?}");
        assert!(expected(&errors[0]), "{errors:#?}");
    }

    /// Assert no verify errors.
    fn assert_no_errors(errors: &[VerifyError]) {
        assert!(errors.is_empty(), "{errors:#?}");
    }

    #[test]
    fn test_reject_use_after_move() {
        let mut program = VerifyProgram::new(
            r#"
function test(v0: ref<int32, unique>): void {
b0(v0: ref<int32, unique>):
    drop v0
    drop v0
    return
}"#,
        );

        let errors = program.run_memory();

        assert_one_error(&errors, |error| {
            matches!(error, VerifyError::UseAfterMove { .. })
        });
    }

    #[test]
    fn test_reject_exclusive_overlap() {
        let mut program = VerifyProgram::new(
            r#"
type Box {
    int32;
}
function test(v0: ref<Box, borrowed>): void {
b0(v0: ref<Box, borrowed>):
    v1: ref<int32, borrowed, exclusive> = field.address v0, 0
    v2: ref<int32, borrowed, exclusive> = field.address v0, 0
    v3: int32 = load v1
    v4: int32 = load v2
    return
}"#,
        );

        let errors = program.run_memory();

        assert_one_error(&errors, |error| {
            matches!(error, VerifyError::ConflictingLoan { .. })
        });
    }

    #[test]
    fn test_allow_aliasable_mutable_overlap() {
        let mut program = VerifyProgram::new(
            r#"
type Box {
    int32;
}
function test(v0: ref<Box, borrowed>): void {
b0(v0: ref<Box, borrowed>):
    v1: ref<int32, borrowed> = field.address v0, 0
    v2: ref<int32, borrowed> = field.address v0, 0
    v3: int32 = load v1
    v4: int32 = load v2
    return
}"#,
        );

        let errors = program.run_memory();

        assert_no_errors(&errors);
    }

    #[test]
    fn test_allow_exclusive_disjoint_fields() {
        let mut program = VerifyProgram::new(
            r#"
type Pair {
    int32;
    int32;
}
function test(v0: ref<Pair, borrowed>): void {
b0(v0: ref<Pair, borrowed>):
    v1: ref<int32, borrowed, exclusive> = field.address v0, 0
    v2: ref<int32, borrowed, exclusive> = field.address v0, 1
    v3: int32 = load v1
    v4: int32 = load v2
    return
}"#,
        );

        let errors = program.run_memory();

        assert_no_errors(&errors);
    }

    #[test]
    fn test_reject_exclusive_after_readonly_overlap() {
        let mut program = VerifyProgram::new(
            r#"
type Box {
    int32;
}
function test(v0: ref<Box, borrowed>): void {
b0(v0: ref<Box, borrowed>):
    v1: ref<int32, borrowed, readonly> = field.address v0, 0
    v2: ref<int32, borrowed, exclusive> = field.address v0, 0
    v3: int32 = load v1
    v4: int32 = load v2
    return
}"#,
        );

        let errors = program.run_memory();

        assert_one_error(&errors, |error| {
            matches!(error, VerifyError::ConflictingLoan { .. })
        });
    }

    #[test]
    fn test_allow_exclusive_after_last_borrow_use() {
        let mut program = VerifyProgram::new(
            r#"
type Box {
    int32;
}
function test(v0: ref<Box, borrowed>): void {
b0(v0: ref<Box, borrowed>):
    v1: ref<int32, borrowed, readonly> = field.address v0, 0
    v2: int32 = load v1
    v3: ref<int32, borrowed, exclusive> = field.address v0, 0
    v4: int32 = load v3
    return
}"#,
        );

        let errors = program.run_memory();

        assert_no_errors(&errors);
    }

    #[test]
    fn test_allow_store_through_live_borrow() {
        let mut program = VerifyProgram::new(
            r#"
function test(v0: ref<int32, borrowed>, v1: int32): void {
b0(v0: ref<int32, borrowed>, v1: int32):
    store v0, v1
    v2: int32 = load v0
    return
}"#,
        );

        let errors = program.run_memory();

        assert_no_errors(&errors);
    }

    #[test]
    fn test_reject_move_while_borrowed() {
        let mut program = VerifyProgram::new(
            r#"
type Box {
    int32;
}
function consume(v0: ref<Box, unique>): void {
b0(v0: ref<Box, unique>):
    return
}
function test(v0: ref<Box, unique>): void {
b0(v0: ref<Box, unique>):
    v1: ref<int32, borrowed> = field.address v0, 0
    call consume(v0): (ref<Box, unique>) -> void
    v2: int32 = load v1
    return
}"#,
        );

        let errors = program.run_memory();

        assert_one_error(&errors, |error| {
            matches!(error, VerifyError::ChangeOfBorrowedPlace { .. })
        });
    }

    #[test]
    fn test_reject_drop_while_borrowed() {
        let mut program = VerifyProgram::new(
            r#"
type Box {
    int32;
}
function test(v0: ref<Box, unique>): void {
b0(v0: ref<Box, unique>):
    v1: ref<int32, borrowed> = field.address v0, 0
    drop v0
    v2: int32 = load v1
    return
}"#,
        );

        let errors = program.run_memory();

        assert_one_error(&errors, |error| {
            matches!(error, VerifyError::ChangeOfBorrowedPlace { .. })
        });
    }

    #[test]
    fn test_reject_drop_while_borrowed_through_block_parameter() {
        let mut program = VerifyProgram::new(
            r#"
type Box {
    int32;
}
function test(v0: ref<Box, unique>, v1: boolean): void {
b0(v0: ref<Box, unique>, v1: boolean):
    v2: ref<int32, borrowed> = field.address v0, 0
    branch v1, b1(v2), b2
b1(v3: ref<int32, borrowed>):
    drop v0
    v4: int32 = load v3
    return
b2:
    return
}"#,
        );

        let errors = program.run_memory();

        assert_one_error(&errors, |error| {
            matches!(error, VerifyError::ChangeOfBorrowedPlace { .. })
        });
    }

    #[test]
    fn test_reject_local_set_while_borrowed() {
        let mut program = VerifyProgram::new(
            r#"
function test(v0: int32, v1: int32): void {
    local local0: int32, owned
b0(v0: int32, v1: int32):
    local.set local0, v0
    v2: ref<int32, borrowed, space(frame)> = local.address local0
    local.set local0, v1
    v3: int32 = load v2
    return
}"#,
        );

        let errors = program.run_memory();

        assert_one_error(&errors, |error| {
            matches!(error, VerifyError::ChangeOfBorrowedPlace { .. })
        });
    }

    #[test]
    fn test_reject_store_through_readonly_borrow() {
        let mut program = VerifyProgram::new(
            r#"
function test(v0: ref<int32, borrowed, readonly>, v1: int32): void {
b0(v0: ref<int32, borrowed, readonly>, v1: int32):
    store v0, v1
    return
}"#,
        );

        let errors = program.run_memory();

        assert_one_error(&errors, |error| {
            matches!(error, VerifyError::ReadonlyWrite { .. })
        });
    }

    #[test]
    fn test_reject_use_after_call_move() {
        let mut program = VerifyProgram::new(
            r#"
function consume(v0: ref<int32, unique>): void {
b0(v0: ref<int32, unique>):
    return
}
function test(v0: ref<int32, unique>): void {
b0(v0: ref<int32, unique>):
    call consume(v0): (ref<int32, unique>) -> void
    drop v0
    return
}"#,
        );

        let errors = program.run_memory();

        assert_one_error(&errors, |error| {
            matches!(error, VerifyError::UseAfterMove { .. })
        });
    }

    #[test]
    fn test_reject_use_after_struct_move() {
        let mut program = VerifyProgram::new(
            r#"
type Box {
    ref<int32, unique>;
}
function test(v0: ref<int32, unique>): void {
b0(v0: ref<int32, unique>):
    v1: Box = struct Box (v0)
    drop v0
    return
}"#,
        );

        let errors = program.run_memory();

        assert_one_error(&errors, |error| {
            matches!(error, VerifyError::UseAfterMove { .. })
        });
    }

    #[test]
    fn test_ignore_raw_free_for_moves() {
        let mut program = VerifyProgram::new(
            r#"
function test(v0: ref<int32, raw>): void {
b0(v0: ref<int32, raw>):
    raw.free v0
    raw.free v0
    return
}"#,
        );

        let errors = program.run_memory();

        assert_no_errors(&errors);
    }

    #[test]
    fn test_reject_frame_return() {
        let mut program = VerifyProgram::new(
            r#"
type Box {
    int32;
}
function test(): ref<int32, borrowed> {
b0:
    v0: ref<Box, raw, space(stack)> = stack.alloc Box
    v1: ref<int32, borrowed> = field.address v0, 0
    return v1
}"#,
        );

        let errors = program.run_memory();

        assert_one_error(&errors, |error| {
            matches!(error, VerifyError::BorrowOutlivesOrigin { .. })
        });
    }

    #[test]
    fn test_allow_parameter_return_when_declared() {
        let mut program = VerifyProgram::new(
            r#"
function test(v0: ref<int32, borrowed>): ref<int32, borrowed, lifetime(0)> {
b0(v0: ref<int32, borrowed>):
    return v0
}"#,
        );

        let errors = program.run_memory();

        assert_no_errors(&errors);
    }

    #[test]
    fn test_infer_parameter_return_lifetime() {
        let mut program = VerifyProgram::new(
            r#"
function test(v0: ref<int32, borrowed>): ref<int32, borrowed> {
b0(v0: ref<int32, borrowed>):
    return v0
}"#,
        );

        let errors = program.run_memory();

        assert_no_errors(&errors);
    }

    #[test]
    fn test_infer_managed_return_lifetime() {
        let mut program = VerifyProgram::new(
            r#"
type User {
    int32;
}
function test(v0: ref<User, managed>): ref<int32, borrowed, readonly> {
b0(v0: ref<User, managed>):
    v1: ref<int32, borrowed, readonly> = field.address v0, 0
    return v1
}"#,
        );

        let errors = program.run_memory();

        assert_no_errors(&errors);
    }

    #[test]
    fn test_reject_unique_borrow_return() {
        let mut program = VerifyProgram::new(
            r#"
type User {
    int32;
}
function test(v0: ref<User, unique>): ref<int32, borrowed> {
b0(v0: ref<User, unique>):
    v1: ref<int32, borrowed> = field.address v0, 0
    return v1
}"#,
        );

        let errors = program.run_memory();

        assert_one_error(&errors, |error| {
            matches!(error, VerifyError::BorrowOutlivesOrigin { .. })
        });
    }

    #[test]
    fn test_reject_managed_return_as_static() {
        let mut program = VerifyProgram::new(
            r#"
type User {
    int32;
}
function test(v0: ref<User, managed>): ref<int32, borrowed, readonly, lifetime(static)> {
b0(v0: ref<User, managed>):
    v1: ref<int32, borrowed, readonly> = field.address v0, 0
    return v1
}"#,
        );

        let errors = program.run_memory();

        assert_one_error(&errors, |error| {
            matches!(error, VerifyError::BorrowOutlivesOrigin { .. })
        });
    }

    #[test]
    fn test_reject_borrow_across_yield() {
        let mut program = VerifyProgram::new(
            r#"
function test(v0: int32): int32 {
    local local0: int32, owned
entry0(v0: int32):
    local.set local0, v0
    v1: ref<int32, borrowed, space(frame)> = local.address local0
    yield v0, block1(v1)
block1(v2: int32, v3: ref<int32, borrowed, space(frame)>):
    v4: int32 = load v3
    return v4
}"#,
        );

        let errors = program.run_memory();

        assert_one_error(&errors, |error| {
            matches!(error, VerifyError::BorrowAcrossSuspend { .. })
        });
    }

    #[test]
    fn test_allow_borrow_before_yield() {
        let mut program = VerifyProgram::new(
            r#"
function test(v0: int32): int32 {
    local local0: int32, owned
entry0(v0: int32):
    local.set local0, v0
    v1: ref<int32, borrowed, space(frame)> = local.address local0
    v2: int32 = load v1
    yield v2, block1(v2)
block1(v3: int32, v4: int32):
    return v4
}"#,
        );

        let errors = program.run_memory();

        assert_no_errors(&errors);
    }

    #[test]
    fn test_allow_static_borrow_return() {
        let mut program = VerifyProgram::new(
            r#"
global value: int32, readonly = 1int32
function test(): ref<int32, borrowed, lifetime(static)> {
b0:
    v0: ref<int32, raw, readonly> = global.address value
    v1: ref<int32, borrowed, readonly, lifetime(static)> = cast.bit v0 -> ref<int32, borrowed, readonly, lifetime(static)>
    return v1
}"#,
        );

        let errors = program.run_memory();

        assert_no_errors(&errors);
    }

    #[test]
    fn test_reject_wrong_parameter_lifetime_return() {
        let mut program = VerifyProgram::new(
            r#"
function test(v0: ref<int32, borrowed>, v1: ref<int32, borrowed>): ref<int32, borrowed, lifetime(0)> {
b0(v0: ref<int32, borrowed>, v1: ref<int32, borrowed>):
    return v1
}"#,
        );

        let errors = program.run_memory();

        assert_one_error(&errors, |error| {
            matches!(error, VerifyError::BorrowOutlivesOrigin { .. })
        });
    }

    #[test]
    fn test_reject_maybe_moved_after_join() {
        let mut program = VerifyProgram::new(
            r#"
function test(v0: ref<int32, unique>, v1: boolean): void {
b0(v0: ref<int32, unique>, v1: boolean):
    branch v1, b1, b2
b1:
    drop v0
    jump b3
b2:
    jump b3
b3:
    drop v0
    return
}"#,
        );

        let errors = program.run_memory();

        assert_one_error(&errors, |error| {
            matches!(error, VerifyError::MaybeUseAfterMove { .. })
        });
    }

    #[test]
    fn test_carry_lifetime_through_block_parameter() {
        let mut program = VerifyProgram::new(
            r#"
function test(v0: ref<int32, borrowed>, v1: boolean): ref<int32, borrowed, lifetime(0)> {
b0(v0: ref<int32, borrowed>, v1: boolean):
    branch v1, b1(v0), b2(v0)
b1(v2: ref<int32, borrowed>):
    return v2
b2(v3: ref<int32, borrowed>):
    return v3
}"#,
        );

        let errors = program.run_memory();

        assert_no_errors(&errors);
    }

    #[test]
    fn test_merge_lifetimes_at_join() {
        let mut program = VerifyProgram::new(
            r#"
function test(v0: ref<int32, borrowed>, v1: ref<int32, borrowed>, v2: boolean): ref<int32, borrowed, lifetime(0)> {
b0(v0: ref<int32, borrowed>, v1: ref<int32, borrowed>, v2: boolean):
    branch v2, b1, b2
b1:
    jump b3(v0)
b2:
    jump b3(v1)
b3(v3: ref<int32, borrowed>):
    return v3
}"#,
        );

        let errors = program.run_memory();

        assert_one_error(&errors, |error| {
            matches!(error, VerifyError::BorrowOutlivesOrigin { .. })
        });
    }
}
