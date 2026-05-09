use std::collections::{HashMap, VecDeque};

use crate::common::mir::terminator_arguments_for_successor;
use crate::declare_mir_pass;
use destack_mir as mir;
use mir::{
    Instruction, Place, PlaceOrigin, ReferenceKind, Terminator, Type, Value, ValueReference,
};

use crate::verify::{VerifyError, VerifyState, VerifyWarning};

declare_mir_pass! {
    /// Verify MIR memory rules.
    #[pass(id = "memory-check")]
    pub MemoryCheck,
    "Verify memory rules"
}

/// Lifetime of one MIR value or place inside a function body.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
enum StorageLifetime {
    /// The value is not an escaping reference.
    #[default]
    None,
    /// Storage belongs to this function activation.
    Frame,
    /// Storage is rooted outside this function.
    External(mir::Lifetime),
    /// Storage is heap-backed and can escape the frame.
    Heap,
}

impl StorageLifetime {
    /// Return true when this lifetime can escape the current frame.
    fn can_escape_frame(&self) -> bool {
        !matches!(self, Self::Frame)
    }

    /// Return true when this lifetime satisfies one declared return lifetime.
    fn is_covered_by(&self, required: &mir::Lifetime) -> bool {
        match self {
            Self::None => true,
            Self::Heap => true,
            Self::External(lifetime) => lifetime.origins.iter().all(|origin| match origin {
                mir::LifetimeOrigin::Static => required.includes_static(),
                mir::LifetimeOrigin::Parameter(index) => required.includes_parameter(*index),
            }),
            Self::Frame => false,
        }
    }

    /// Merge two lifetimes at a control-flow join.
    fn merge(&self, other: &Self) -> Self {
        match (self, other) {
            (Self::Frame, _) | (_, Self::Frame) => Self::Frame,
            (Self::Heap, _) | (_, Self::Heap) => Self::Heap,
            (Self::None, lifetime) | (lifetime, Self::None) => lifetime.clone(),
            (Self::External(left), Self::External(right)) => {
                let origins = left.origins.iter().chain(&right.origins).copied();

                Self::External(mir::Lifetime::new(origins))
            }
        }
    }

    /// Return a compact label for diagnostics.
    fn label(&self) -> String {
        match self {
            Self::None => "none".to_string(),
            Self::Frame => "frame".to_string(),
            Self::External(lifetime) if lifetime.is_static() => "static".to_string(),
            Self::External(lifetime) => {
                let parameters: Vec<_> = lifetime.parameter_indices().collect();
                if parameters.len() == 1 {
                    format!("parameter {}", parameters[0])
                } else {
                    format!("parameters {parameters:?}")
                }
            }
            Self::Heap => "heap".to_string(),
        }
    }
}

/// Ownership state for one move-only value.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ValueState {
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

/// Change that can invalidate active loans.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlaceChange {
    /// The whole value is moved.
    Move,
    /// The whole value is dropped.
    Drop,
    /// A local binding is assigned.
    AssignLocal,
}

/// Memory facts tracked while checking one function.
#[derive(Debug, Clone, PartialEq, Eq)]
struct MemoryState {
    /// Move state for move-only values.
    values: HashMap<Value, ValueState>,
    /// Storage lifetime for reference-like values.
    lifetimes: HashMap<Value, StorageLifetime>,
    /// Active loans by reference value.
    loans: HashMap<Value, Loan>,
}

impl MemoryState {
    /// Create empty memory state.
    fn new() -> Self {
        Self {
            values: HashMap::new(),
            lifetimes: HashMap::new(),
            loans: HashMap::new(),
        }
    }

    /// Merge another state into this state at a control-flow join.
    fn merge(&mut self, other: &Self) -> bool {
        let mut changed = false;

        for (&value, state) in &other.values {
            changed |= self.merge_value(value, state);
        }

        for (&value, lifetime) in &other.lifetimes {
            changed |= self.merge_lifetime(value, lifetime);
        }

        for (&value, loan) in &other.loans {
            changed |= self.loans.insert(value, loan.clone()).as_ref() != Some(loan);
        }

        changed
    }

    /// Merge one move state.
    fn merge_value(&mut self, value: Value, state: &ValueState) -> bool {
        let next = match self.values.get(&value) {
            None => state.clone(),
            Some(current) => current.merge(state),
        };

        self.values.insert(value, next.clone()).as_ref() != Some(&next)
    }

    /// Merge one lifetime.
    fn merge_lifetime(&mut self, value: Value, lifetime: &StorageLifetime) -> bool {
        let next = self
            .lifetimes
            .get(&value)
            .map(|current| current.merge(lifetime))
            .unwrap_or_else(|| lifetime.clone());

        self.lifetimes.insert(value, next.clone()).as_ref() != Some(&next)
    }

    /// Copy state from one value to another.
    fn bind_value(&mut self, source: Value, destination: Value) {
        if let Some(state) = self.values.get(&source).cloned() {
            self.values.insert(destination, state);
        }

        if let Some(lifetime) = self.lifetimes.get(&source).cloned() {
            self.lifetimes.insert(destination, lifetime);
        }

        if let Some(mut loan) = self.loans.get(&source).cloned() {
            loan.reference = destination;
            self.loans.insert(destination, loan);
        }
    }
}

impl ValueState {
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

/// Checker for one function.
struct FunctionMemory<'a, 'b> {
    /// The function being verified.
    function: &'a mir::Function,
    /// The MIR tree.
    tree: &'a mir::Tree,
    /// Verification context.
    context: &'a mut VerifyState<'b>,
    /// SSA value liveness.
    liveness: mir::FunctionLiveness,
    /// Whether this pass should emit diagnostics.
    emit_diagnostics: bool,
    /// Current memory facts.
    state: MemoryState,
}

impl<'a, 'b> FunctionMemory<'a, 'b> {
    /// Create a function memory checker.
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
            emit_diagnostics: true,
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

            state.bind_value(source, destination);
        }

        state
            .loans
            .retain(|_, loan| self.liveness.is_value_live_in(successor, loan.reference));

        state
    }

    /// Transfer one block from entry to exit state.
    fn transfer_block(
        &mut self,
        block_id: mir::LocalNodeId<mir::Block>,
        state: MemoryState,
        emit_diagnostics: bool,
    ) -> MemoryState {
        self.state = state;
        self.emit_diagnostics = emit_diagnostics;

        let block = self.tree.get(block_id);
        for &instruction_id in &block.instructions {
            let instruction = self.tree.get(instruction_id);
            self.check_instruction(instruction_id, instruction);
            self.expire_instruction_loans(block_id, instruction_id);
        }

        self.check_terminator(block_id, self.tree.get(block.terminator));

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
                state.values.insert(value, ValueState::Available);
            }

            let lifetime = self.parameter_lifetime(index as u32, parameter.ty);
            if lifetime != StorageLifetime::None {
                state.lifetimes.insert(value, lifetime);
            }
        }

        state
    }

    /// Return parameter storage lifetime.
    fn parameter_lifetime(&self, index: u32, ty: mir::TypeReference) -> StorageLifetime {
        let mir::TypeReference::Type(ty) = ty else {
            return StorageLifetime::None;
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
            } => {
                if lifetime.is_empty() {
                    StorageLifetime::External(mir::Lifetime::parameter(index))
                } else {
                    StorageLifetime::External(lifetime.clone())
                }
            }
            Type::Reference {
                kind: ReferenceKind::Managed | ReferenceKind::Owned,
                ..
            }
            | Type::Slice {
                kind: ReferenceKind::Managed | ReferenceKind::Owned,
                ..
            }
            | Type::TensorView {
                kind: ReferenceKind::Managed | ReferenceKind::Owned,
                ..
            } => StorageLifetime::Heap,
            _ => StorageLifetime::None,
        }
    }

    /// Verify one instruction and update memory facts.
    fn check_instruction(
        &mut self,
        instruction_id: mir::LocalNodeId<Instruction>,
        instruction: &Instruction,
    ) {
        for value in instruction.uses() {
            self.check_use(value, instruction_id.into_any());
        }
        if let Some(arguments) = instruction.argument_slice() {
            for &value in self.tree.get_arguments(arguments) {
                self.check_use(value, instruction_id.into_any());
            }
        }

        match instruction {
            Instruction::LocalSet { local, value } => {
                self.check_place_change(
                    &Place::local(*local),
                    PlaceChange::AssignLocal,
                    instruction_id.into_any(),
                );
                self.move_value(*value, instruction_id.into_any());
            }
            Instruction::Store { pointer, value } => {
                self.check_store(*pointer, *value, instruction_id.into_any());
                self.move_value(*value, instruction_id.into_any());
            }
            Instruction::FieldAddr {
                destination,
                aggregate,
                ..
            } => self.create_loan(*destination, *aggregate, instruction_id.into_any()),
            Instruction::ElementAddr {
                destination, array, ..
            } => self.create_loan(*destination, *array, instruction_id.into_any()),
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
                self.propagate_lifetime(*aggregate, *destination);
                self.move_value(*aggregate, instruction_id.into_any());
                self.move_value(*value, instruction_id.into_any());
            }
            Instruction::ElementSet {
                destination,
                array,
                value,
                ..
            } => {
                self.check_aggregate_set(*array, *value, instruction_id.into_any());
                self.propagate_lifetime(*array, *destination);
                self.move_value(*array, instruction_id.into_any());
                self.move_value(*value, instruction_id.into_any());
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
            } => self.propagate_lifetime(*pointer, *destination),
            Instruction::StackAlloc { destination, .. } => {
                self.define_lifetime(
                    *destination,
                    StorageLifetime::Frame,
                    instruction_id.into_any(),
                );
            }
            Instruction::New { destination, .. }
            | Instruction::NewSlice { destination, .. }
            | Instruction::RawAlloc { destination, .. } => {
                self.define_lifetime(
                    *destination,
                    StorageLifetime::Heap,
                    instruction_id.into_any(),
                );
            }
            Instruction::GlobalAddr { destination, .. } => {
                self.define_lifetime(
                    *destination,
                    StorageLifetime::External(mir::Lifetime::static_storage()),
                    instruction_id.into_any(),
                );
            }
            Instruction::RawFree { pointer } | Instruction::Drop { value: pointer } => {
                self.check_value_change(*pointer, PlaceChange::Drop, instruction_id.into_any());
                self.move_value(*pointer, instruction_id.into_any());
            }
            Instruction::Call { call, .. }
            | Instruction::CallVirtual { call, .. }
            | Instruction::CallInterface { call, .. }
            | Instruction::CallIndirect { call, .. } => {
                for &value in self.tree.get_arguments(call.arguments) {
                    self.move_value(value.into(), instruction_id.into_any());
                }
                self.define_destination(instruction);
            }
            Instruction::Intrinsic { arguments, .. } => {
                for &value in self.tree.get_arguments(*arguments) {
                    self.move_value(value.into(), instruction_id.into_any());
                }
                self.define_destination(instruction);
            }
            _ => self.define_destination(instruction),
        }
    }

    /// Verify one terminator.
    fn check_terminator(
        &mut self,
        block_id: mir::LocalNodeId<mir::Block>,
        terminator: &Terminator,
    ) {
        for value in terminator.uses() {
            self.check_use(value, block_id.into_any());
        }

        match terminator {
            Terminator::Return { value: Some(value) } => {
                self.check_return(*value, block_id.into_any());
                self.move_value(*value, block_id.into_any());
            }
            Terminator::Throw { value }
            | Terminator::Yield { value, .. }
            | Terminator::TailCallIndirect { callee: value, .. }
            | Terminator::Trap {
                payload: Some(value),
                ..
            } => self.move_value(*value, block_id.into_any()),
            Terminator::TailCall { call, .. }
            | Terminator::TailCallVirtual { call, .. }
            | Terminator::TailCallInterface { call, .. }
            | Terminator::Invoke { call, .. }
            | Terminator::InvokeIndirect { call, .. }
            | Terminator::InvokeVirtual { call, .. }
            | Terminator::InvokeInterface { call, .. } => {
                for &value in &call.arguments {
                    self.move_value(value, block_id.into_any());
                }
            }
            _ => {}
        }
    }

    /// Define destination state when the instruction produces a move-only value.
    fn define_destination(&mut self, instruction: &Instruction) {
        let Some(destination) = instruction.destination().and_then(ValueReference::value) else {
            return;
        };

        if self.is_move_only(destination.into()) {
            self.state.values.insert(destination, ValueState::Available);
        }
    }

    /// Check one value use.
    fn check_use(&mut self, value: ValueReference, anchor: mir::LocalNodeIdAny) {
        let Some(value) = value.value() else {
            return;
        };

        match self.state.values.get(&value) {
            Some(ValueState::Moved { at }) => {
                let moved_at = self.context.anchor(self.tree, *at);
                self.emit_error(
                    VerifyError::UseAfterMove {
                        anchor: self.context.anchor(self.tree, anchor),
                        moved_at: moved_at.clone(),
                    }
                    .label(moved_at, "value moved here"),
                );
            }
            Some(ValueState::MaybeMoved { at }) => {
                let moved_at = self.context.anchor(self.tree, *at);
                self.emit_error(
                    VerifyError::MaybeUseAfterMove {
                        anchor: self.context.anchor(self.tree, anchor),
                        moved_at: moved_at.clone(),
                    }
                    .label(moved_at, "value moved on this path"),
                );
            }
            Some(ValueState::Available) | None => {}
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

        self.check_value_change(value.into(), PlaceChange::Move, anchor);
        self.state
            .values
            .insert(value, ValueState::Moved { at: anchor });
    }

    /// Create a loan projected from another value.
    fn create_loan(
        &mut self,
        reference: ValueReference,
        source: ValueReference,
        anchor: mir::LocalNodeIdAny,
    ) {
        let place = self.place_for_value(source);
        self.create_loan_from_place(reference, place, anchor);

        if let (Some(source), Some(reference)) = (source.value(), reference.value()) {
            self.propagate_lifetime(source.into(), reference.into());
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

        let access = self
            .reference_access(reference)
            .unwrap_or(mir::Access::Mutable);
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

        let lifetime = self.lifetime_for_place(&place);
        self.state
            .lifetimes
            .entry(reference_value)
            .or_insert(lifetime);
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

    /// Check a pointer store for access and lifetime escape.
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

        let value_lifetime = self.lifetime_for_value(value);
        let target_lifetime = self.lifetime_for_place(&pointer_place);
        if value_lifetime == StorageLifetime::Frame && target_lifetime.can_escape_frame() {
            self.emit_error(VerifyError::FrameReferenceEscapes {
                anchor: self.context.anchor(self.tree, anchor),
            });
        }
    }

    /// Check a structural aggregate update for lifetime escape.
    fn check_aggregate_set(
        &mut self,
        aggregate: ValueReference,
        value: ValueReference,
        anchor: mir::LocalNodeIdAny,
    ) {
        let aggregate_lifetime = self.lifetime_for_value(aggregate);
        let value_lifetime = self.lifetime_for_value(value);
        if value_lifetime == StorageLifetime::Frame && aggregate_lifetime.can_escape_frame() {
            self.emit_error(VerifyError::FrameReferenceEscapes {
                anchor: self.context.anchor(self.tree, anchor),
            });
        }
    }

    /// Check one returned value.
    fn check_return(&mut self, value: ValueReference, anchor: mir::LocalNodeIdAny) {
        let lifetime = self.lifetime_for_value(value);
        if lifetime == StorageLifetime::None {
            if !self.function.return_lifetime.is_empty() {
                self.emit_warning(VerifyWarning::ReturnLifetimeIgnored {
                    anchor: self.context.anchor(self.tree, anchor),
                });
            }
            return;
        }

        if lifetime == StorageLifetime::Frame {
            self.emit_error(VerifyError::FrameReferenceEscapes {
                anchor: self.context.anchor(self.tree, anchor),
            });
            return;
        }

        if !lifetime.is_covered_by(&self.function.return_lifetime) {
            self.emit_error(VerifyError::ReturnLifetimeMismatch {
                anchor: self.context.anchor(self.tree, anchor),
                origin: lifetime.label(),
            });
        }
    }

    /// Check whether changing a value invalidates active loans.
    fn check_value_change(
        &mut self,
        value: ValueReference,
        change: PlaceChange,
        anchor: mir::LocalNodeIdAny,
    ) {
        let place = self.place_for_value(value);
        self.check_place_change(&place, change, anchor);
    }

    /// Check whether changing a place invalidates active loans.
    fn check_place_change(
        &mut self,
        place: &Place,
        change: PlaceChange,
        anchor: mir::LocalNodeIdAny,
    ) {
        let loans: Vec<_> = self.state.loans.values().cloned().collect();
        for loan in loans {
            if !loan.place.may_overlap(place) {
                continue;
            }

            let borrowed_at = self.context.anchor(self.tree, loan.created_at);
            let error = match change {
                PlaceChange::Move => VerifyError::MoveOfBorrowedValue {
                    anchor: self.context.anchor(self.tree, anchor),
                    borrowed_at: borrowed_at.clone(),
                },
                PlaceChange::Drop => VerifyError::DropWhileBorrowed {
                    anchor: self.context.anchor(self.tree, anchor),
                    borrowed_at: borrowed_at.clone(),
                },
                PlaceChange::AssignLocal => VerifyError::LocalSetWhileBorrowed {
                    anchor: self.context.anchor(self.tree, anchor),
                    borrowed_at: borrowed_at.clone(),
                },
            };
            self.emit_error(error.label(borrowed_at, "loan is here"));
        }
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

    /// Return whether a value is move-only.
    fn is_move_only(&self, value: ValueReference) -> bool {
        let Some(ty) = value.value().and_then(|value| self.type_for_value(value)) else {
            return false;
        };

        self.tree.get(ty).copy().is_no()
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

    /// Propagate known storage lifetime from one value to another.
    fn propagate_lifetime(&mut self, source: ValueReference, destination: ValueReference) {
        let Some(destination) = destination.value() else {
            return;
        };
        let lifetime = self.lifetime_for_value(source);
        if lifetime != StorageLifetime::None {
            self.state.lifetimes.insert(destination, lifetime);
        }
    }

    /// Define the storage lifetime for one destination value.
    fn define_lifetime(
        &mut self,
        destination: ValueReference,
        lifetime: StorageLifetime,
        anchor: mir::LocalNodeIdAny,
    ) {
        let Some(destination) = destination.value() else {
            self.emit_error(VerifyError::Internal {
                anchor: self.context.anchor(self.tree, anchor),
                message: "expected concrete value destination".to_string(),
            });
            return;
        };

        self.state.lifetimes.insert(destination, lifetime);
    }

    /// Return the lifetime for one value.
    fn lifetime_for_value(&self, value: ValueReference) -> StorageLifetime {
        let Some(value) = value.value() else {
            return StorageLifetime::None;
        };

        self.state
            .lifetimes
            .get(&value)
            .cloned()
            .unwrap_or_else(|| self.type_lifetime(value))
    }

    /// Return the lifetime implied by one value type.
    fn type_lifetime(&self, value: Value) -> StorageLifetime {
        let Some(ty) = self.type_for_value(value) else {
            return StorageLifetime::None;
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
            } if !lifetime.is_empty() => StorageLifetime::External(lifetime.clone()),
            Type::Reference {
                kind: ReferenceKind::Managed | ReferenceKind::Owned,
                ..
            }
            | Type::Slice {
                kind: ReferenceKind::Managed | ReferenceKind::Owned,
                ..
            }
            | Type::TensorView {
                kind: ReferenceKind::Managed | ReferenceKind::Owned,
                ..
            } => StorageLifetime::Heap,
            _ => StorageLifetime::None,
        }
    }

    /// Return the lifetime for one place.
    fn lifetime_for_place(&self, place: &Place) -> StorageLifetime {
        match place.origin {
            PlaceOrigin::Local(_) => StorageLifetime::Frame,
            PlaceOrigin::Global(_) => StorageLifetime::External(mir::Lifetime::static_storage()),
            PlaceOrigin::Value(value) => self.lifetime_for_value(value),
        }
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
    fn emit_error(&mut self, error: impl Into<destack_artifact::DiagnosticBuilder<VerifyError>>) {
        if self.emit_diagnostics {
            self.context.emit_error(error);
        }
    }

    /// Emit one warning when diagnostics are enabled.
    fn emit_warning(
        &mut self,
        warning: impl Into<destack_artifact::DiagnosticBuilder<VerifyWarning>>,
    ) {
        if self.emit_diagnostics {
            self.context.emit_warning(warning);
        }
    }
}

impl MemoryCheck {
    /// Verify memory rules for one MIR tree.
    pub fn run(&self, tree: &mut mir::Tree, context: &mut VerifyState<'_>) {
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

            FunctionMemory::new(function, tree, context).check();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verify::tests::VerifyProgram;

    #[test]
    fn test_reject_use_after_move() {
        let mut program = VerifyProgram::new(
            r#"
function test(v0: ref<int32, owned>): void {
b0(v0: ref<int32, owned>):
    drop v0
    drop v0
    return
}"#,
        );

        let (errors, _) = program.run_memory();

        assert!(
            errors
                .iter()
                .any(|error| matches!(error, VerifyError::UseAfterMove { .. }))
        );
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

        let (errors, _) = program.run_memory();

        assert!(
            errors
                .iter()
                .any(|error| matches!(error, VerifyError::ConflictingLoan { .. }))
        );
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

        let (errors, _) = program.run_memory();

        assert!(errors.is_empty());
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

        let (errors, _) = program.run_memory();

        assert!(errors.is_empty());
    }

    #[test]
    fn test_reject_move_while_borrowed() {
        let mut program = VerifyProgram::new(
            r#"
type Box {
    int32;
}
function consume(v0: ref<Box, owned>): void {
b0(v0: ref<Box, owned>):
    return
}
function test(v0: ref<Box, owned>): void {
b0(v0: ref<Box, owned>):
    v1: ref<int32, borrowed> = field.address v0, 0
    call consume(v0): (ref<Box, owned>) -> void
    v2: int32 = load v1
    return
}"#,
        );

        let (errors, _) = program.run_memory();

        assert!(
            errors
                .iter()
                .any(|error| matches!(error, VerifyError::MoveOfBorrowedValue { .. }))
        );
    }

    #[test]
    fn test_reject_drop_while_borrowed() {
        let mut program = VerifyProgram::new(
            r#"
type Box {
    int32;
}
function test(v0: ref<Box, owned>): void {
b0(v0: ref<Box, owned>):
    v1: ref<int32, borrowed> = field.address v0, 0
    drop v0
    v2: int32 = load v1
    return
}"#,
        );

        let (errors, _) = program.run_memory();

        assert!(
            errors
                .iter()
                .any(|error| matches!(error, VerifyError::DropWhileBorrowed { .. }))
        );
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

        let (errors, _) = program.run_memory();

        assert!(
            errors
                .iter()
                .any(|error| matches!(error, VerifyError::LocalSetWhileBorrowed { .. }))
        );
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

        let (errors, _) = program.run_memory();

        assert!(
            errors
                .iter()
                .any(|error| matches!(error, VerifyError::ReadonlyWrite { .. }))
        );
    }

    #[test]
    fn test_reject_use_after_call_move() {
        let mut program = VerifyProgram::new(
            r#"
function consume(v0: ref<int32, owned>): void {
b0(v0: ref<int32, owned>):
    return
}
function test(v0: ref<int32, owned>): void {
b0(v0: ref<int32, owned>):
    call consume(v0): (ref<int32, owned>) -> void
    drop v0
    return
}"#,
        );

        let (errors, _) = program.run_memory();

        assert!(
            errors
                .iter()
                .any(|error| matches!(error, VerifyError::UseAfterMove { .. }))
        );
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

        let (errors, _) = program.run_memory();

        assert!(
            errors
                .iter()
                .any(|error| matches!(error, VerifyError::FrameReferenceEscapes { .. }))
        );
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

        let (errors, _) = program.run_memory();

        assert!(errors.is_empty());
    }

    #[test]
    fn test_reject_maybe_moved_after_join() {
        let mut program = VerifyProgram::new(
            r#"
function test(v0: ref<int32, owned>, v1: boolean): void {
b0(v0: ref<int32, owned>, v1: boolean):
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

        let (errors, _) = program.run_memory();

        assert!(
            errors
                .iter()
                .any(|error| matches!(error, VerifyError::MaybeUseAfterMove { .. }))
        );
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

        let (errors, _) = program.run_memory();

        assert!(errors.is_empty());
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

        let (errors, _) = program.run_memory();

        assert!(
            errors
                .iter()
                .any(|error| matches!(error, VerifyError::ReturnLifetimeMismatch { .. }))
        );
    }
}
