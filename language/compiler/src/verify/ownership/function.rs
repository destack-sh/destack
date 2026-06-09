use std::collections::{HashMap, VecDeque};

use crate::common::mir::terminator_arguments_for_successor;
use crate::verify::value::{instruction_consumes, instruction_uses, terminator_consumes};
use crate::verify::{BorrowObligationRecord, VerifyError, VerifyState};
use destack_artifact::DiagnosticBuilder;
use destack_mir as mir;

use super::alias::PlaceAlias;
use super::borrow::{BorrowSource, BorrowSources, BorrowSuspension};
use super::flow::FlowState;
use super::loan::Loan;
use super::r#move::{MoveState, MoveUse};

/// Borrow contract inferred for one function.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct FunctionBorrowContract {
    /// Borrow obligations required by this function body.
    pub(super) borrow_obligations: Vec<mir::BorrowObligation>,
    /// Actual lifetime slots returned by this function body.
    pub(super) return_lifetime: mir::Lifetime,
}

/// Ownership verifier for one function.
pub(super) struct FunctionVerifyState<'a, 'b> {
    /// The function being verified.
    function: &'a mir::Function,
    /// The MIR tree.
    tree: &'a mir::Tree,
    /// Verification context.
    context: &'a mut VerifyState<'b>,
    /// Solved return lifetimes for known functions.
    function_lifetimes: &'a HashMap<mir::LocalNodeId<mir::Function>, mir::Lifetime>,
    /// Borrow obligations required by this function body.
    borrow_obligations: Vec<mir::BorrowObligation>,
    /// Borrow sources returned by this function body.
    return_sources: BorrowSources,
    /// SSA value liveness.
    liveness: mir::FunctionLiveness,
    /// Place alias relation.
    aliases: PlaceAlias,
    /// Whether this pass should emit diagnostics.
    is_diagnostics_enabled: bool,
    /// Whether diagnostic replay is enabled for this run.
    should_emit_diagnostics: bool,
    /// Current flow state.
    flow: FlowState,
}

impl<'a, 'b> FunctionVerifyState<'a, 'b> {
    /// Create a function verifier.
    pub(super) fn new(
        function: &'a mir::Function,
        tree: &'a mir::Tree,
        context: &'a mut VerifyState<'b>,
        function_lifetimes: &'a HashMap<mir::LocalNodeId<mir::Function>, mir::Lifetime>,
    ) -> Self {
        Self {
            function,
            tree,
            context,
            function_lifetimes,
            borrow_obligations: Vec::new(),
            return_sources: BorrowSources::none(),
            liveness: mir::FunctionLiveness::build(function, tree),
            aliases: PlaceAlias::new(function, tree),
            is_diagnostics_enabled: true,
            should_emit_diagnostics: true,
            flow: FlowState::new(),
        }
    }

    /// Create a function verifier for contract solving.
    pub(super) fn for_contract(
        function: &'a mir::Function,
        tree: &'a mir::Tree,
        context: &'a mut VerifyState<'b>,
        function_lifetimes: &'a HashMap<mir::LocalNodeId<mir::Function>, mir::Lifetime>,
    ) -> Self {
        let mut state = Self::new(function, tree, context, function_lifetimes);
        state.should_emit_diagnostics = false;

        state
    }

    /// Verify one function.
    pub(super) fn check(mut self) -> FunctionBorrowContract {
        let entries = self.solve_entries();

        // replay blocks with fixed entry states
        self.replay_blocks(entries);

        FunctionBorrowContract {
            borrow_obligations: self.borrow_obligations,
            return_lifetime: self.return_sources.lifetime(),
        }
    }

    /// Replay reachable blocks with diagnostics enabled.
    fn replay_blocks(&mut self, entries: HashMap<mir::LocalNodeId<mir::Block>, FlowState>) {
        for &block_id in &self.function.blocks {
            let Some(entry) = entries.get(&block_id).cloned() else {
                continue;
            };

            self.transfer_block(block_id, entry, true);
        }
    }

    /// Solve fixed-point entry flow for every reachable block.
    fn solve_entries(&mut self) -> HashMap<mir::LocalNodeId<mir::Block>, FlowState> {
        let Some(entry) = self.function.entry else {
            return HashMap::new();
        };

        let graph = mir::ControlFlowGraph::build(self.function, self.tree);
        let mut entries = HashMap::new();
        let mut exits = HashMap::new();
        let mut worklist = VecDeque::new();

        entries.insert(entry, self.initial_state());
        worklist.push_back(entry);

        // propagate move, loan, and root state to a fixed point
        while let Some(block_id) = worklist.pop_front() {
            let entry_state = self.solve_entry(block_id, &graph, &entries, &exits);
            let old_entry = entries.insert(block_id, entry_state.clone());
            let exit_state = self.transfer_block(block_id, entry_state, false);
            let old_exit = exits.insert(block_id, exit_state);

            // skip successors when the block state is stable
            if old_entry.as_ref() == entries.get(&block_id)
                && old_exit.as_ref() == exits.get(&block_id)
            {
                continue;
            }

            // revisit successors after changed exits
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

    /// Return entry flow for one block.
    fn solve_entry(
        &self,
        block_id: mir::LocalNodeId<mir::Block>,
        graph: &mir::ControlFlowGraph,
        entries: &HashMap<mir::LocalNodeId<mir::Block>, FlowState>,
        exits: &HashMap<mir::LocalNodeId<mir::Block>, FlowState>,
    ) -> FlowState {
        if Some(block_id) == self.function.entry {
            return entries
                .get(&block_id)
                .cloned()
                .expect("entry block must have initial ownership state");
        }

        // merge only reached predecessor exits
        let predecessors = graph
            .predecessors(block_id)
            .iter()
            .filter_map(|predecessor| {
                exits
                    .get(predecessor)
                    .cloned()
                    .map(|flow| (*predecessor, flow))
            })
            .map(|(predecessor, flow)| self.bind_edge_arguments(predecessor, block_id, flow))
            .collect::<Vec<_>>();

        FlowState::merge_predecessors(&predecessors)
    }

    /// Bind predecessor arguments to successor block parameters.
    fn bind_edge_arguments(
        &self,
        predecessor: mir::LocalNodeId<mir::Block>,
        successor: mir::LocalNodeId<mir::Block>,
        mut flow: FlowState,
    ) -> FlowState {
        let predecessor_id = predecessor;
        let predecessor = self.tree.get(predecessor_id);
        let terminator = self.tree.get(predecessor.terminator);
        let arguments = terminator_arguments_for_successor(terminator, successor);
        let successor_block = self.tree.get(successor);

        // bind edge arguments to successor block parameters
        for (parameter, argument) in successor_block
            .parameters
            .iter()
            .zip(arguments.iter().copied())
        {
            let (Some(parameter), Some(argument)) = (parameter.value.value(), argument.value())
            else {
                continue;
            };

            flow.bind(argument, parameter);
        }

        // bind the implicit call result when the continuation receives one
        if successor_block.parameters.len() == arguments.len() + 1
            && let Some(sources) =
                self.call_terminator_result_sources(predecessor_id, terminator, &flow)
        {
            let parameter = successor_block
                .parameters
                .last()
                .and_then(|parameter| parameter.value.value());
            if let Some(parameter) = parameter {
                flow.borrows.insert(parameter, sources);
            }
        }

        // keep loan references live in the successor
        flow.loans
            .retain_live(|reference| self.is_value_live_in_successor(successor, reference));

        flow
    }

    /// Transfer one block from entry to exit state.
    fn transfer_block(
        &mut self,
        block_id: mir::LocalNodeId<mir::Block>,
        flow: FlowState,
        is_diagnostics_enabled: bool,
    ) -> FlowState {
        self.flow = flow;
        self.is_diagnostics_enabled = self.should_emit_diagnostics && is_diagnostics_enabled;

        let block = self.tree.get(block_id);

        // apply instruction reads, moves, borrows, and definitions
        for (index, &instruction_id) in block.instructions.iter().enumerate() {
            let instruction = self.tree.get(instruction_id);
            self.transfer_instruction(block_id, index, instruction_id, instruction);
            self.expire_instruction_loans(block_id, index);
        }

        // apply terminator returns, branches, and consumes
        self.transfer_terminator(block_id, block.terminator, self.tree.get(block.terminator));

        self.flow.clone()
    }

    /// Return initial flow state for the function entry.
    fn initial_state(&self) -> FlowState {
        let mut flow = FlowState::new();

        // seed parameter borrow sources from declared types
        for (index, parameter) in self.function.parameters.iter().enumerate() {
            let Some(value) = parameter.value.value() else {
                continue;
            };

            let sources = self.parameter_sources(index as u32, &parameter.ty);
            flow.borrows.insert(value, sources);
        }

        flow
    }

    /// Return the borrow sources implied by one parameter type.
    fn parameter_sources(&self, index: u32, ty_ref: &mir::TypeReference) -> BorrowSources {
        let Some(ty) = ty_ref.ty() else {
            return BorrowSources::none();
        };
        let ty = self.tree.get(ty);

        match ty.reference_kind() {
            // track managed parameters as managed sources
            Some(mir::ReferenceKind::Managed) => self.managed_source_for_type(ty, Some(index)),
            // prefer explicit borrowed lifetimes over parameter defaults
            Some(mir::ReferenceKind::Borrowed) => {
                let Some(lifetime) = ty.reference_lifetime() else {
                    return BorrowSources::none();
                };

                self.sources_from_lifetime_or_parameter(lifetime, index)
            }
            _ => {
                // derive aggregate sources from contained borrowed references
                if let Some(lifetime) = self.tree.type_reference_lifetime(ty_ref) {
                    return self.sources_from_lifetime(&lifetime);
                }
                if self.tree.type_reference_contains_borrowed_refs(ty_ref) {
                    return BorrowSources::one(BorrowSource::Slot(index));
                }

                BorrowSources::none()
            }
        }
    }

    /// Transfer one instruction into the current flow state.
    fn transfer_instruction(
        &mut self,
        _block_id: mir::LocalNodeId<mir::Block>,
        _index: usize,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
        instruction: &mir::Instruction,
    ) {
        let anchor = instruction_id.into_any();

        // check uses against initialized places
        self.check_instruction_uses(instruction, anchor);

        // update loans and borrow sources from instruction forms
        match instruction {
            mir::Instruction::LocalSet { local, .. } => {
                let place = mir::Place::local(*local);
                if self.check_place_change(&place, anchor) {
                    self.flow.moves.assign_place(&place);
                }
            }
            mir::Instruction::Store { pointer, value } => {
                self.check_store(*pointer, *value, anchor);
            }
            mir::Instruction::FieldAddr {
                destination,
                aggregate,
                ..
            } => {
                self.create_projection_loan(*destination, *aggregate, anchor);
            }
            mir::Instruction::ElementAddr {
                destination, array, ..
            } => {
                self.create_projection_loan(*destination, *array, anchor);
            }
            mir::Instruction::Slice {
                destination,
                source,
                ..
            } => {
                self.create_projection_loan(*destination, *source, anchor);
            }
            mir::Instruction::FieldGet {
                destination,
                aggregate,
                index,
                ..
            } => {
                self.move_projection_value(
                    *destination,
                    *aggregate,
                    mir::Projection::Field { index: *index },
                    anchor,
                );
                self.propagate_sources(*aggregate, *destination);
            }
            mir::Instruction::ElementGet {
                destination,
                array,
                index,
                ..
            } => {
                self.move_projection_value(
                    *destination,
                    *array,
                    mir::Projection::fixed_element(*index),
                    anchor,
                );
                self.propagate_sources(*array, *destination);
            }
            mir::Instruction::LocalAddr {
                destination, local, ..
            } => {
                self.create_loan_from_place(*destination, mir::Place::local(*local), anchor, None);
            }
            mir::Instruction::FieldSet {
                destination,
                aggregate,
                value,
                ..
            } => {
                self.check_aggregate_set(*aggregate, *value, anchor);
                self.propagate_sources(*aggregate, *destination);
            }
            mir::Instruction::ElementSet {
                destination,
                array,
                value,
                ..
            } => {
                self.check_aggregate_set(*array, *value, anchor);
                self.propagate_sources(*array, *destination);
            }
            mir::Instruction::Load {
                destination,
                pointer,
                ..
            }
            | mir::Instruction::Cast {
                destination,
                argument: pointer,
                ..
            }
            | mir::Instruction::Pin {
                destination,
                value: pointer,
                ..
            } => {
                self.propagate_sources(*pointer, *destination);
            }
            mir::Instruction::FrameAllocZeroed { destination, .. }
            | mir::Instruction::FrameAllocUninit { destination, .. }
            | mir::Instruction::NewZeroed { destination, .. }
            | mir::Instruction::NewUninit { destination, .. }
            | mir::Instruction::NewSliceZeroed { destination, .. }
            | mir::Instruction::NewSliceUninit { destination, .. } => {
                let sources = self.sources_for_destination_storage(*destination);
                self.define_sources(*destination, sources);
            }
            mir::Instruction::NewComplete {
                destination, value, ..
            } => {
                self.propagate_sources(*value, *destination);
            }
            mir::Instruction::GlobalAddr { destination, .. } => {
                self.define_sources(*destination, BorrowSources::one(BorrowSource::Static));
            }
            mir::Instruction::Call { function, call, .. } => {
                let arguments = self.tree.get_arguments(call.arguments).to_vec();
                self.check_call_obligations(
                    function.function(),
                    &call.signature,
                    arguments.clone(),
                    anchor,
                );
                self.define_call_result_sources(
                    instruction.destination(),
                    function.function(),
                    &call.signature,
                    &arguments,
                );
            }
            mir::Instruction::CallVirtual { receiver, call, .. } => {
                let arguments = self.call_arguments_with_receiver(*receiver, call.arguments);
                let function = self.resolved_instruction_target(instruction_id, instruction);
                self.check_call_obligations(function, &call.signature, arguments.clone(), anchor);
                self.define_call_result_sources(
                    instruction.destination(),
                    function,
                    &call.signature,
                    &arguments,
                );
            }
            mir::Instruction::CallDynamic { receiver, call, .. } => {
                let arguments = self.call_arguments_with_receiver(*receiver, call.arguments);
                self.check_call_obligations(None, &call.signature, arguments.clone(), anchor);
                self.define_call_result_sources(
                    instruction.destination(),
                    None,
                    &call.signature,
                    &arguments,
                );
            }
            mir::Instruction::CallIndirect { call, .. } => {
                let arguments = self.tree.get_arguments(call.arguments).to_vec();
                self.check_call_obligations(None, &call.signature, arguments.clone(), anchor);
                self.define_call_result_sources(
                    instruction.destination(),
                    None,
                    &call.signature,
                    &arguments,
                );
            }
            _ => {}
        }

        // move values consumed by the instruction
        for value in instruction_consumes(instruction, self.tree) {
            self.move_value(value, anchor);
        }

        // define destinations after instruction effects
        self.define_destination(instruction);
    }

    /// Transfer one terminator into the current flow state.
    fn transfer_terminator(
        &mut self,
        block_id: mir::LocalNodeId<mir::Block>,
        terminator_id: mir::LocalNodeId<mir::Terminator>,
        terminator: &mir::Terminator,
    ) {
        let anchor = terminator_id.into_any();

        // check terminator uses against initialized values
        for value in terminator.uses() {
            self.check_value_use(value, anchor);
        }

        // check returned borrows against the function return lifetime
        if let mir::Terminator::Return { value: Some(value) } = terminator {
            self.check_return(*value, anchor);
        }

        // check callee borrow obligations at the call site
        self.check_terminator_call_obligations(block_id, terminator, anchor);

        // check tail-call result borrows against the function return lifetime
        self.check_tail_call_return(block_id, terminator, anchor);

        // reject exclusive loans across reentrant suspension
        if matches!(terminator, mir::Terminator::Yield { .. }) {
            self.check_suspension(terminator, anchor);
        }

        // move terminator arguments consumed by the current function
        for value in terminator_consumes(terminator) {
            self.move_value(value, anchor);
        }
    }

    /// Define the instruction destination as initialized.
    fn define_destination(&mut self, instruction: &mir::Instruction) {
        let Some(destination) = instruction
            .destination()
            .and_then(mir::ValueReference::value)
        else {
            return;
        };

        let place = self.place_for_value(destination.into());
        self.flow.moves.assign_place(&place);
    }

    /// Check initialized places used by one instruction.
    fn check_instruction_uses(
        &mut self,
        instruction: &mir::Instruction,
        anchor: mir::LocalNodeIdAny,
    ) {
        match instruction {
            mir::Instruction::FieldGet {
                aggregate, index, ..
            }
            | mir::Instruction::FieldAddr {
                aggregate, index, ..
            } => {
                let projection = mir::Projection::Field { index: *index };
                self.check_projection_use(*aggregate, projection, anchor);
            }
            mir::Instruction::ElementGet { array, index, .. } => {
                let projection = mir::Projection::fixed_element(*index);
                self.check_projection_use(*array, projection, anchor);
            }
            mir::Instruction::ElementAddr { array, index, .. } => {
                let projection = mir::Projection::dynamic_element(*index);
                self.check_projection_use(*array, projection, anchor);
                self.check_value_use(*index, anchor);
            }
            mir::Instruction::Slice {
                source,
                start,
                length,
                ..
            } => {
                let projection = mir::Projection::Slice {
                    start: *start,
                    length: *length,
                };
                self.check_projection_use(*source, projection, anchor);
                self.check_value_use(*start, anchor);
                self.check_value_use(*length, anchor);
            }
            _ => {
                for value in instruction_uses(instruction, self.tree) {
                    self.check_value_use(value, anchor);
                }
            }
        }
    }

    /// Check one projected place use.
    fn check_projection_use(
        &mut self,
        base: mir::ValueReference,
        projection: mir::Projection,
        anchor: mir::LocalNodeIdAny,
    ) {
        let place = self.projected_place(base, projection);

        self.check_place_use(&place, anchor);
    }

    /// Check one place use.
    fn check_place_use(&mut self, place: &mir::Place, anchor: mir::LocalNodeIdAny) {
        let Some(moved) = self.flow.moves.check_use(place, |left, right| {
            self.aliases.moved_may_alias(left, right)
        }) else {
            return;
        };

        self.emit_move_error(moved, anchor);
    }

    /// Check one value use.
    fn check_value_use(&mut self, value: mir::ValueReference, anchor: mir::LocalNodeIdAny) {
        let place = self.place_for_value(value);
        let Some(moved) = self.flow.moves.check_use(&place, |left, right| {
            self.aliases.moved_may_alias(left, right)
        }) else {
            return;
        };

        self.emit_move_error(moved, anchor);
    }

    /// Emit a move-use diagnostic.
    fn emit_move_error(&mut self, moved: MoveUse, anchor: mir::LocalNodeIdAny) {
        let moved_at = self.context.anchor(self.tree, moved.at);
        match moved.state {
            // report places moved by every predecessor
            MoveState::Moved => {
                self.emit_error(
                    VerifyError::UseAfterMove {
                        anchor: self.context.anchor(self.tree, anchor),
                        moved_at: moved_at.clone(),
                    }
                    .label(moved_at, "value moved here"),
                );
            }
            // report places moved by at least one predecessor
            MoveState::MaybeMoved => {
                self.emit_error(
                    VerifyError::MaybeUseAfterMove {
                        anchor: self.context.anchor(self.tree, anchor),
                        moved_at: moved_at.clone(),
                    }
                    .label(moved_at, "value moved on this path"),
                );
            }
        }
    }

    /// Move one value when it is move-only.
    fn move_value(&mut self, value: mir::ValueReference, anchor: mir::LocalNodeIdAny) {
        if !self.is_move_only(value) {
            return;
        }

        if !self.check_value_change(value, anchor) {
            return;
        }

        let place = self.place_for_value(value);
        self.flow.moves.move_place(place, anchor);
    }

    /// Move one projected place when the produced value is move-only.
    fn move_projection_value(
        &mut self,
        value: mir::ValueReference,
        base: mir::ValueReference,
        projection: mir::Projection,
        anchor: mir::LocalNodeIdAny,
    ) {
        if !self.is_move_only(value) {
            return;
        }

        // reject partial moves through custom drop glue
        if !self.is_union_value(base) && self.has_custom_drop(base) {
            self.emit_error(VerifyError::PartialMoveOfCustomDrop {
                anchor: self.context.anchor(self.tree, anchor),
            });
            return;
        }

        // mark the projected place as moved
        let place = self.place_moved_by_projection(base, projection);
        if self.check_place_change(&place, anchor) {
            self.flow.moves.move_place(place, anchor);
        }
    }

    /// Create a loan whose projected place is recorded on the reference.
    fn create_projection_loan(
        &mut self,
        reference: mir::ValueReference,
        root: mir::ValueReference,
        anchor: mir::LocalNodeIdAny,
    ) {
        if root
            .value()
            .is_some_and(|value| self.flow.is_invalid_borrow(value))
        {
            return;
        }

        let place = self.place_for_value(reference);
        if self.create_loan_from_place(reference, place, anchor, root.value()) {
            self.propagate_sources(root, reference);
        }
    }

    /// Create one active loan.
    fn create_loan_from_place(
        &mut self,
        reference: mir::ValueReference,
        place: mir::Place,
        anchor: mir::LocalNodeIdAny,
        parent: Option<mir::Value>,
    ) -> bool {
        let Some(reference) = reference.value() else {
            return false;
        };
        if !self.is_borrowed_reference(reference.into()) {
            return false;
        }
        if self.flow.is_invalid_borrow(reference) {
            return false;
        }

        let access = self
            .reference_access(reference.into())
            .expect("borrowed reference must have access");
        let sources = self.sources_for_place(&place);
        if !sources.allows_borrow(access) {
            self.emit_error(VerifyError::ExclusiveBorrowFromSharedManaged {
                anchor: self.context.anchor(self.tree, anchor),
            });

            self.flow.invalidate_borrow(reference);
            return false;
        }

        let loan = Loan {
            place: place.clone(),
            access,
            reference,
            created_at: anchor,
        };

        if let Some(conflict) = self
            .flow
            .loans
            .conflict(&loan, parent, |left, right| {
                self.aliases.may_alias(left, right)
            })
            .cloned()
        {
            let active_borrow = self.context.anchor(self.tree, conflict.created_at);
            self.emit_error(
                VerifyError::BorrowConflict {
                    anchor: self.context.anchor(self.tree, anchor),
                    active_borrow: active_borrow.clone(),
                }
                .label(active_borrow, "borrow starts here"),
            );

            self.flow.invalidate_borrow(reference);
            return false;
        }

        self.flow.loans.insert(loan);
        self.flow.borrows.insert(reference, sources);

        true
    }

    /// Check a pointer store for access and borrow escape.
    fn check_store(
        &mut self,
        pointer: mir::ValueReference,
        value: mir::ValueReference,
        anchor: mir::LocalNodeIdAny,
    ) {
        // reject readonly reference writes
        if self
            .reference_access(pointer)
            .is_some_and(|access| !access.can_write())
        {
            self.emit_error(VerifyError::WriteThroughReadonlyReference {
                anchor: self.context.anchor(self.tree, anchor),
            });
        }

        // reject writes blocked by exclusive loans
        let pointer_place = self.place_for_value(pointer);
        if let Some(loan) = self
            .flow
            .loans
            .blocking_write(&pointer_place, pointer.value(), |left, right| {
                self.aliases.may_alias(left, right)
            })
            .cloned()
        {
            let borrowed_at = self.context.anchor(self.tree, loan.created_at);
            self.emit_error(
                VerifyError::InvalidationOfBorrowedPlace {
                    anchor: self.context.anchor(self.tree, anchor),
                    borrowed_at: borrowed_at.clone(),
                }
                .label(borrowed_at, "borrow starts here"),
            );
        }

        // reject local borrows stored into escaping locations
        let value_sources = self.sources_for_value(value);
        let target_sources = self.sources_for_place(&pointer_place);
        if value_sources.has_function_local_source() && target_sources.is_escaping() {
            self.emit_error(VerifyError::BorrowOutlivesOrigin {
                anchor: self.context.anchor(self.tree, anchor),
            });
        }
    }

    /// Check a structural aggregate update for borrow escape.
    fn check_aggregate_set(
        &mut self,
        aggregate: mir::ValueReference,
        value: mir::ValueReference,
        anchor: mir::LocalNodeIdAny,
    ) {
        let aggregate_sources = self.sources_for_value(aggregate);
        let value_sources = self.sources_for_value(value);

        // reject local borrows embedded into escaping aggregates
        if value_sources.has_function_local_source() && aggregate_sources.is_escaping() {
            self.emit_error(VerifyError::BorrowOutlivesOrigin {
                anchor: self.context.anchor(self.tree, anchor),
            });
        }
    }

    /// Check one returned value.
    fn check_return(&mut self, value: mir::ValueReference, anchor: mir::LocalNodeIdAny) {
        let sources = self.sources_for_value(value);

        self.check_return_sources(sources, anchor);
    }

    /// Check returned borrow sources against the function return lifetime.
    fn check_return_sources(&mut self, sources: BorrowSources, anchor: mir::LocalNodeIdAny) {
        if sources.is_empty() {
            return;
        }

        // reject frame-local borrows escaping the function
        if sources.has_function_local_source() {
            self.emit_error(VerifyError::BorrowOutlivesOrigin {
                anchor: self.context.anchor(self.tree, anchor),
            });
            return;
        }

        // remember actual escaped sources for callers
        self.return_sources = self.return_sources.merge(&sources);

        // apply explicit return lifetimes to nonlocal sources
        let Some(required) = self.explicit_return_lifetime() else {
            return;
        };

        if !sources.is_covered_by(&required) {
            self.emit_error(VerifyError::BorrowOutlivesOrigin {
                anchor: self.context.anchor(self.tree, anchor),
            });
        }
    }

    /// Check tail-call result borrows against the function return lifetime.
    fn check_tail_call_return(
        &mut self,
        block_id: mir::LocalNodeId<mir::Block>,
        terminator: &mir::Terminator,
        anchor: mir::LocalNodeIdAny,
    ) {
        let sources = match terminator {
            mir::Terminator::TailCall { function, call } => self.sources_for_call_result(
                function.function(),
                &call.signature,
                &call.arguments,
                &self.flow,
            ),
            mir::Terminator::TailCallVirtual { receiver, call, .. } => {
                let arguments = self.call_arguments_with_receiver_vec(*receiver, &call.arguments);
                let function = self.resolved_terminator_target(block_id, terminator);
                self.sources_for_call_result(function, &call.signature, &arguments, &self.flow)
            }
            mir::Terminator::TailCallDynamic { receiver, call, .. } => {
                let arguments = self.call_arguments_with_receiver_vec(*receiver, &call.arguments);
                self.sources_for_call_result(None, &call.signature, &arguments, &self.flow)
            }
            mir::Terminator::TailCallIndirect { call, .. } => {
                self.sources_for_call_result(None, &call.signature, &call.arguments, &self.flow)
            }
            _ => return,
        };

        self.check_return_sources(sources, anchor);
    }

    /// Check live borrows at one suspension point.
    fn check_suspension(&mut self, terminator: &mir::Terminator, anchor: mir::LocalNodeIdAny) {
        // check borrowed references that remain visible after suspension
        for value in self.borrowed_references_across_suspension(terminator) {
            let sources = self.sources_for_value(value.into());
            match sources.suspension() {
                BorrowSuspension::Stable => {}
                BorrowSuspension::Requires(lifetimes) => {
                    self.require_suspension_sources(lifetimes, anchor);
                }
                BorrowSuspension::Rejected => {
                    let borrowed_at = self
                        .context
                        .anchor(self.tree, self.anchor_for_borrowed_reference(value, anchor));
                    self.emit_error(
                        VerifyError::ManagedBorrowAcrossSuspension {
                            anchor: self.context.anchor(self.tree, anchor),
                            borrowed_at: borrowed_at.clone(),
                        }
                        .label(borrowed_at, "managed borrow is live here"),
                    );
                }
            }
        }
    }

    /// Require suspension-stable sources for borrowed lifetimes.
    fn require_suspension_sources(
        &mut self,
        lifetimes: Vec<mir::Lifetime>,
        anchor: mir::LocalNodeIdAny,
    ) {
        let diagnostic_anchor = self
            .is_diagnostics_enabled
            .then(|| self.context.anchor(self.tree, anchor));

        // record one proof obligation for each required lifetime
        for lifetime in lifetimes {
            let obligation = mir::BorrowObligation::SuspensionStable { lifetime };
            if !self.borrow_obligations.contains(&obligation) {
                self.borrow_obligations.push(obligation.clone());
            }

            // attach diagnostics to the replay pass only
            if let Some(anchor) = &diagnostic_anchor {
                self.context.require_borrow(BorrowObligationRecord {
                    obligation,
                    anchor: anchor.clone(),
                });
            }
        }
    }

    /// Check call-site borrow obligations for one terminator.
    fn check_terminator_call_obligations(
        &mut self,
        block_id: mir::LocalNodeId<mir::Block>,
        terminator: &mir::Terminator,
        anchor: mir::LocalNodeIdAny,
    ) {
        match terminator {
            mir::Terminator::Call { function, call, .. }
            | mir::Terminator::TailCall { function, call } => {
                self.check_call_obligations(
                    function.function(),
                    &call.signature,
                    call.arguments.clone(),
                    anchor,
                );
            }
            mir::Terminator::CallVirtual { receiver, call, .. }
            | mir::Terminator::TailCallVirtual { receiver, call, .. } => {
                let arguments = self.call_arguments_with_receiver_vec(*receiver, &call.arguments);
                let function = self.resolved_terminator_target(block_id, terminator);
                self.check_call_obligations(function, &call.signature, arguments, anchor);
            }
            mir::Terminator::CallDynamic { receiver, call, .. }
            | mir::Terminator::TailCallDynamic { receiver, call, .. } => {
                let arguments = self.call_arguments_with_receiver_vec(*receiver, &call.arguments);
                self.check_call_obligations(None, &call.signature, arguments, anchor);
            }
            mir::Terminator::CallIndirect { call, .. }
            | mir::Terminator::TailCallIndirect { call, .. } => {
                self.check_call_obligations(None, &call.signature, call.arguments.clone(), anchor);
            }
            _ => {}
        }
    }

    /// Return the resolved function target for one instruction callsite.
    fn resolved_instruction_target(
        &self,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
        instruction: &mir::Instruction,
    ) -> Option<mir::LocalNodeId<mir::Function>> {
        instruction
            .call_direct_target()
            .and_then(|target| target.function())
            .or_else(|| {
                self.tree
                    .metadata
                    .functions
                    .call(mir::CallSite::Instruction(instruction_id))
                    .and_then(|metadata| metadata.target)
            })
    }

    /// Return the resolved function target for one terminator callsite.
    fn resolved_terminator_target(
        &self,
        block_id: mir::LocalNodeId<mir::Block>,
        terminator: &mir::Terminator,
    ) -> Option<mir::LocalNodeId<mir::Function>> {
        terminator
            .call_direct_target()
            .and_then(|target| target.function())
            .or_else(|| {
                self.tree
                    .metadata
                    .functions
                    .call(mir::CallSite::Terminator(block_id))
                    .and_then(|metadata| metadata.target)
            })
    }

    /// Return borrow sources for one call terminator result.
    fn call_terminator_result_sources(
        &self,
        block_id: mir::LocalNodeId<mir::Block>,
        terminator: &mir::Terminator,
        flow: &FlowState,
    ) -> Option<BorrowSources> {
        match terminator {
            mir::Terminator::Call { function, call, .. } => Some(self.sources_for_call_result(
                function.function(),
                &call.signature,
                &call.arguments,
                flow,
            )),
            mir::Terminator::CallVirtual { receiver, call, .. } => {
                let arguments = self.call_arguments_with_receiver_vec(*receiver, &call.arguments);
                let function = self.resolved_terminator_target(block_id, terminator);
                Some(self.sources_for_call_result(function, &call.signature, &arguments, flow))
            }
            mir::Terminator::CallDynamic { receiver, call, .. } => {
                let arguments = self.call_arguments_with_receiver_vec(*receiver, &call.arguments);
                Some(self.sources_for_call_result(None, &call.signature, &arguments, flow))
            }
            mir::Terminator::CallIndirect { call, .. } => {
                Some(self.sources_for_call_result(None, &call.signature, &call.arguments, flow))
            }
            _ => None,
        }
    }

    /// Check callee obligations against call arguments.
    fn check_call_obligations(
        &mut self,
        function: Option<mir::LocalNodeId<mir::Function>>,
        signature: &mir::TypeReference,
        arguments: Vec<mir::ValueReference>,
        anchor: mir::LocalNodeIdAny,
    ) {
        let obligations = self.call_borrow_obligations(function, signature);

        // prove every callee obligation from the actual argument sources
        for obligation in obligations {
            match obligation {
                mir::BorrowObligation::SuspensionStable { lifetime } => {
                    self.check_stable_lifetime_arguments(&lifetime, &arguments, anchor);
                }
            }
        }
    }

    /// Return the borrow obligations for one call target.
    fn call_borrow_obligations(
        &self,
        function: Option<mir::LocalNodeId<mir::Function>>,
        signature: &mir::TypeReference,
    ) -> Vec<mir::BorrowObligation> {
        let mut obligations = self.signature_borrow_obligations(signature);

        // add concrete function obligations discovered by verification
        if let Some(function) = function {
            for obligation in &self.tree.get(function).borrow_obligations {
                if !obligations.contains(obligation) {
                    obligations.push(obligation.clone());
                }
            }
        }

        obligations
    }

    /// Return the borrow obligations encoded in one signature type.
    fn signature_borrow_obligations(
        &self,
        signature: &mir::TypeReference,
    ) -> Vec<mir::BorrowObligation> {
        let Some(signature) = signature.ty() else {
            return Vec::new();
        };
        let mir::Type::FunctionSignature {
            borrow_obligations, ..
        } = self.tree.get(signature)
        else {
            return Vec::new();
        };

        borrow_obligations.clone()
    }

    /// Define borrow sources for one call result.
    fn define_call_result_sources(
        &mut self,
        destination: Option<mir::ValueReference>,
        function: Option<mir::LocalNodeId<mir::Function>>,
        signature: &mir::TypeReference,
        arguments: &[mir::ValueReference],
    ) {
        let Some(destination) = destination else {
            return;
        };
        if !destination
            .value()
            .is_some_and(|value| self.value_can_carry_sources(value))
        {
            return;
        }

        let sources = self.sources_for_call_result(function, signature, arguments, &self.flow);
        self.define_sources(destination, sources);
    }

    /// Return borrow sources for one call result.
    fn sources_for_call_result(
        &self,
        function: Option<mir::LocalNodeId<mir::Function>>,
        signature: &mir::TypeReference,
        arguments: &[mir::ValueReference],
        flow: &FlowState,
    ) -> BorrowSources {
        let lifetime = function
            .and_then(|function| self.function_return_lifetime(function))
            .or_else(|| self.signature_return_lifetime(signature));

        let Some(lifetime) = lifetime else {
            return BorrowSources::none();
        };

        self.sources_from_callee_lifetime(&lifetime, arguments, flow)
    }

    /// Return the actual return lifetime inferred for one known function.
    fn function_return_lifetime(
        &self,
        function_id: mir::LocalNodeId<mir::Function>,
    ) -> Option<mir::Lifetime> {
        let function = self.tree.get(function_id);
        if !self
            .tree
            .type_reference_contains_borrowed_refs(&function.return_type)
        {
            return None;
        }

        self.function_lifetimes
            .get(&function_id)
            .cloned()
            .filter(|lifetime| !lifetime.is_empty())
            .or_else(|| {
                self.tree
                    .type_reference_lifetime(&function.return_type)
                    .filter(|lifetime| !lifetime.is_empty())
            })
            .or_else(|| {
                Some(self.tree.infer_function_return_lifetime(function_id))
                    .filter(|lifetime| !lifetime.is_empty())
            })
    }

    /// Return the declared return lifetime for one signature type.
    fn signature_return_lifetime(&self, signature: &mir::TypeReference) -> Option<mir::Lifetime> {
        let lifetime_args = signature.lifetimes();
        let signature = signature.ty()?;
        let mir::Type::FunctionSignature {
            parameters, result, ..
        } = self.tree.get(signature)
        else {
            return None;
        };

        if let Some(lifetime) = self
            .tree
            .type_reference_lifetime_with_lifetimes(result, lifetime_args)
        {
            return Some(lifetime);
        }
        if !self.tree.type_reference_contains_borrowed_refs(result) {
            return None;
        }

        let lifetime_slots = parameters
            .iter()
            .enumerate()
            .filter_map(|(index, parameter)| {
                self.type_reference_can_source_borrow(parameter)
                    .then_some(index as u32)
            });
        let lifetime = mir::Lifetime::slot_set(lifetime_slots);

        Some(lifetime).filter(|lifetime| !lifetime.is_empty())
    }

    /// Return whether one signature parameter can source a returned borrow.
    fn type_reference_can_source_borrow(&self, ty_ref: &mir::TypeReference) -> bool {
        let Some(ty) = ty_ref.ty() else {
            return false;
        };
        let ty_id = ty;
        let ty = self.tree.get(ty_id);

        matches!(
            ty.reference_kind(),
            Some(mir::ReferenceKind::Borrowed | mir::ReferenceKind::Managed)
        ) || self.tree.type_reference_contains_borrowed_refs(ty_ref)
    }

    /// Map callee lifetime origins to caller-side borrow sources.
    fn sources_from_callee_lifetime(
        &self,
        lifetime: &mir::Lifetime,
        arguments: &[mir::ValueReference],
        flow: &FlowState,
    ) -> BorrowSources {
        let mut sources = Vec::new();

        // map each external origin to the actual argument source
        for origin in &lifetime.terms {
            match origin {
                mir::LifetimeTerm::Static => {
                    sources.push(BorrowSource::Static);
                }
                mir::LifetimeTerm::Slot(index) => {
                    let Some(argument) = arguments.get(index.0 as usize) else {
                        continue;
                    };
                    let argument_sources = self.sources_for_value_in_flow(*argument, flow);
                    sources.extend(argument_sources.iter().cloned());
                }
            }
        }

        BorrowSources::new(sources)
    }

    /// Check one callee lifetime against actual argument sources.
    fn check_stable_lifetime_arguments(
        &mut self,
        lifetime: &mir::Lifetime,
        arguments: &[mir::ValueReference],
        anchor: mir::LocalNodeIdAny,
    ) {
        for origin in &lifetime.terms {
            match origin {
                mir::LifetimeTerm::Static => {}
                mir::LifetimeTerm::Slot(index) => {
                    let Some(argument) = arguments.get(index.0 as usize) else {
                        continue;
                    };

                    self.check_stable_argument_source(*argument, anchor);
                }
            }
        }
    }

    /// Check whether one argument can satisfy a suspension-stable source.
    fn check_stable_argument_source(
        &mut self,
        argument: mir::ValueReference,
        anchor: mir::LocalNodeIdAny,
    ) {
        let sources = self.sources_for_value(argument);
        match sources.suspension() {
            BorrowSuspension::Stable => {}
            BorrowSuspension::Requires(lifetimes) => {
                self.require_suspension_sources(lifetimes, anchor);
            }
            BorrowSuspension::Rejected => {
                let borrowed_at = argument
                    .value()
                    .map(|value| self.anchor_for_borrowed_reference(value, anchor))
                    .unwrap_or(anchor);
                let borrowed_at = self.context.anchor(self.tree, borrowed_at);
                self.emit_error(
                    VerifyError::ManagedBorrowAcrossSuspension {
                        anchor: self.context.anchor(self.tree, anchor),
                        borrowed_at: borrowed_at.clone(),
                    }
                    .label(borrowed_at, "managed borrow is used across suspension"),
                );
            }
        }
    }

    /// Return call arguments with a receiver in parameter slot zero.
    fn call_arguments_with_receiver(
        &self,
        receiver: mir::ValueReference,
        arguments: mir::ArgumentSlice,
    ) -> Vec<mir::ValueReference> {
        self.call_arguments_with_receiver_vec(receiver, self.tree.get_arguments(arguments))
    }

    /// Return call arguments with a receiver in parameter slot zero.
    fn call_arguments_with_receiver_vec(
        &self,
        receiver: mir::ValueReference,
        arguments: &[mir::ValueReference],
    ) -> Vec<mir::ValueReference> {
        let mut values = Vec::with_capacity(arguments.len() + 1);
        values.push(receiver);
        values.extend_from_slice(arguments);

        values
    }

    /// Return borrowed references that cross one suspension point.
    fn borrowed_references_across_suspension(
        &self,
        terminator: &mir::Terminator,
    ) -> Vec<mir::Value> {
        let mir::Terminator::Yield {
            value: yielded,
            resume,
        } = terminator
        else {
            return Vec::new();
        };

        let mut values = Vec::new();

        // check the yielded value itself
        if let Some(value) = yielded.value()
            && self.is_borrowed_value(value)
        {
            values.push(value);
        }

        // check parameter references carried into the continuation
        for parameter in &self.function.parameters {
            let Some(value) = parameter.value.value() else {
                continue;
            };
            if self.is_borrowed_value(value)
                && self.is_value_live_across_suspension(value, resume)
                && !values.contains(&value)
            {
                values.push(value);
            }
        }

        // check instruction results carried into the continuation
        for (index, _) in self.function.value_types.iter().enumerate() {
            let value = mir::Value::new(index as u32);
            if self.is_borrowed_value(value)
                && self.is_value_live_across_suspension(value, resume)
                && !values.contains(&value)
            {
                values.push(value);
            }
        }

        values
    }

    /// Return whether one value is a borrowed reference.
    fn is_borrowed_value(&self, value: mir::Value) -> bool {
        let Some(ty) = self.type_for_value(value) else {
            return false;
        };

        self.tree.get(ty).is_borrowed_reference()
    }

    /// Return whether one value crosses a suspension edge.
    fn is_value_live_across_suspension(
        &self,
        value: mir::Value,
        resume: &mir::BlockTarget,
    ) -> bool {
        if resume
            .arguments
            .iter()
            .any(|argument| argument.value() == Some(value))
        {
            return true;
        }

        let Some(block) = resume.block.block() else {
            return false;
        };

        self.liveness.is_value_live_in(block, value)
    }

    /// Return the diagnostic anchor for one exclusive reference.
    fn anchor_for_borrowed_reference(
        &self,
        value: mir::Value,
        fallback: mir::LocalNodeIdAny,
    ) -> mir::LocalNodeIdAny {
        if let Some(loan) = self
            .flow
            .loans
            .loans
            .iter()
            .find(|loan| loan.reference == value)
        {
            return loan.created_at;
        }

        let is_parameter = self
            .function
            .parameters
            .iter()
            .any(|parameter| parameter.value.value() == Some(value));
        if is_parameter {
            return self.function.entry.map(Into::into).unwrap_or(fallback);
        }

        fallback
    }

    /// Check whether changing a value invalidates active loans.
    fn check_value_change(
        &mut self,
        value: mir::ValueReference,
        anchor: mir::LocalNodeIdAny,
    ) -> bool {
        let place = self.place_for_value(value);

        self.check_place_change(&place, anchor)
    }

    /// Check whether changing a place invalidates active loans.
    fn check_place_change(&mut self, place: &mir::Place, anchor: mir::LocalNodeIdAny) -> bool {
        let Some(loan) = self
            .flow
            .loans
            .blocking_change(place, |left, right| self.aliases.may_alias(left, right))
            .cloned()
        else {
            return true;
        };

        // reject changes blocked by active loans
        let borrowed_at = self.context.anchor(self.tree, loan.created_at);
        self.emit_error(
            VerifyError::InvalidationOfBorrowedPlace {
                anchor: self.context.anchor(self.tree, anchor),
                borrowed_at: borrowed_at.clone(),
            }
            .label(borrowed_at, "borrow starts here"),
        );

        false
    }

    /// Expire loans whose reference is no longer live after one instruction.
    fn expire_instruction_loans(&mut self, block_id: mir::LocalNodeId<mir::Block>, index: usize) {
        self.flow.loans.retain_live(|reference| {
            self.liveness
                .is_value_live_after_instruction(block_id, index, reference, self.tree)
        });
    }

    /// Return whether a value remains live after entering one successor.
    fn is_value_live_in_successor(
        &self,
        block_id: mir::LocalNodeId<mir::Block>,
        value: mir::Value,
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
    fn is_move_only(&self, value: mir::ValueReference) -> bool {
        let Some(ty) = value.value().and_then(|value| self.type_for_value(value)) else {
            return false;
        };

        self.tree.get(ty).copy().is_no()
    }

    /// Return whether a value has custom drop glue.
    fn has_custom_drop(&self, value: mir::ValueReference) -> bool {
        let Some(ty) = value.value().and_then(|value| self.type_for_value(value)) else {
            return false;
        };

        self.tree
            .metadata
            .drop
            .drop_glue(ty)
            .is_some_and(|glue| !matches!(glue, mir::DropGlue::None))
    }

    /// Return whether one value has a union type.
    fn is_union_value(&self, value: mir::ValueReference) -> bool {
        let Some(ty) = value.value().and_then(|value| self.type_for_value(value)) else {
            return false;
        };

        matches!(self.tree.get(ty), mir::Type::Variant { .. })
    }

    /// Return whether one value is a borrowed reference-like value.
    fn is_borrowed_reference(&self, value: mir::ValueReference) -> bool {
        let Some(ty) = value.value().and_then(|value| self.type_for_value(value)) else {
            return false;
        };

        self.tree.get(ty).is_borrowed_reference()
    }

    /// Return access for one reference-like value.
    fn reference_access(&self, value: mir::ValueReference) -> Option<mir::Access> {
        let ty = value.value().and_then(|value| self.type_for_value(value))?;

        self.tree.get(ty).reference_access()
    }

    /// Propagate known borrow sources from one value to another.
    fn propagate_sources(&mut self, root: mir::ValueReference, destination: mir::ValueReference) {
        let Some(destination) = destination.value() else {
            return;
        };
        if !self.value_can_carry_sources(destination) {
            return;
        }

        let sources = self.sources_for_value(root);
        self.flow.borrows.insert(destination, sources);
    }

    /// Return whether one value can carry borrowed source metadata.
    fn value_can_carry_sources(&self, value: mir::Value) -> bool {
        let Some(ty) = self.type_for_value(value) else {
            return false;
        };
        let ty = mir::TypeReference::from(ty);

        self.tree.type_reference_lifetime(&ty).is_some()
            || self.tree.type_reference_contains_borrowed_refs(&ty)
    }

    /// Define borrow sources for one destination value.
    fn define_sources(&mut self, destination: mir::ValueReference, sources: BorrowSources) {
        let Some(destination) = destination.value() else {
            return;
        };

        self.flow.borrows.insert(destination, sources);
    }

    /// Return borrow sources for one value.
    fn sources_for_value(&self, value: mir::ValueReference) -> BorrowSources {
        self.sources_for_value_in_flow(value, &self.flow)
    }

    /// Return borrow sources for one value in a flow state.
    fn sources_for_value_in_flow(
        &self,
        value: mir::ValueReference,
        flow: &FlowState,
    ) -> BorrowSources {
        let Some(value) = value.value() else {
            return BorrowSources::none();
        };

        flow.borrows
            .get(value)
            .cloned()
            .unwrap_or_else(|| self.type_sources(value))
    }

    /// Return borrow sources implied by one value type.
    fn type_sources(&self, value: mir::Value) -> BorrowSources {
        let Some(ty) = self.type_for_value(value) else {
            return BorrowSources::none();
        };

        let ty = mir::TypeReference::from(ty);
        if let Some(lifetime) = self.tree.type_reference_lifetime(&ty) {
            return self.sources_from_lifetime(&lifetime);
        }

        BorrowSources::none()
    }

    /// Return borrow sources for one place.
    fn sources_for_place(&self, place: &mir::Place) -> BorrowSources {
        match place.origin {
            mir::PlaceOrigin::Local(_) => BorrowSources::one(BorrowSource::Owned),
            mir::PlaceOrigin::Global(_) => BorrowSources::one(BorrowSource::Static),
            mir::PlaceOrigin::Value(value) => self.sources_for_storage(value),
        }
    }

    /// Return borrow sources implied by a value used as storage.
    fn sources_for_storage(&self, value: mir::ValueReference) -> BorrowSources {
        let Some(value) = value.value() else {
            return BorrowSources::none();
        };

        // prefer flow sources from projections and propagated values
        if let Some(sources) = self.flow.borrows.get(value) {
            return sources.clone();
        }

        let Some(ty) = self.type_for_value(value) else {
            return BorrowSources::none();
        };

        let ty = self.storage_type(ty);
        let ty = self.tree.get(ty);
        match ty.reference_kind() {
            // keep managed storage alive through the managed handle
            Some(mir::ReferenceKind::Managed) => self.managed_source_for_type(ty, None),
            // keep owned storage alive through the unique handle
            Some(mir::ReferenceKind::Unique) => BorrowSources::one(BorrowSource::Owned),
            // preserve explicit lifetime sources for borrowed storage
            Some(mir::ReferenceKind::Borrowed) => {
                let Some(lifetime) = ty.reference_lifetime() else {
                    return BorrowSources::none();
                };
                if lifetime.is_empty() {
                    return BorrowSources::none();
                }

                self.sources_from_lifetime(lifetime)
            }
            _ => BorrowSources::none(),
        }
    }

    /// Return borrow sources implied by a storage-producing destination.
    fn sources_for_destination_storage(&self, destination: mir::ValueReference) -> BorrowSources {
        let Some(value) = destination.value() else {
            return BorrowSources::none();
        };
        let Some(ty) = self.type_for_value(value) else {
            return BorrowSources::none();
        };

        let ty = self.storage_type(ty);
        let ty = self.tree.get(ty);
        match ty.reference_kind() {
            Some(mir::ReferenceKind::Managed) => self.managed_source_for_type(ty, None),
            Some(mir::ReferenceKind::Unique | mir::ReferenceKind::Raw) => {
                BorrowSources::one(BorrowSource::Owned)
            }
            Some(mir::ReferenceKind::Borrowed) | None => BorrowSources::none(),
        }
    }

    /// Return a managed borrow source for one type.
    fn managed_source_for_type(&self, ty: &mir::Type, parameter: Option<u32>) -> BorrowSources {
        let Some(space) = Self::reference_space(ty).cloned() else {
            return BorrowSources::none();
        };

        BorrowSources::one(BorrowSource::Managed { space, parameter })
    }

    /// Return the initialized storage type represented by one type.
    fn storage_type(&self, ty: mir::LocalNodeId<mir::Type>) -> mir::LocalNodeId<mir::Type> {
        let mir::Type::Uninit { value } = self.tree.get(ty) else {
            return ty;
        };
        let Some(value) = value.ty() else {
            return ty;
        };

        value
    }

    /// Return the space for a reference-like type.
    fn reference_space(ty: &mir::Type) -> Option<&mir::Space> {
        match ty {
            mir::Type::Reference { space, .. }
            | mir::Type::Slice { space, .. }
            | mir::Type::TensorView { space, .. } => Some(space),
            _ => None,
        }
    }

    /// Return borrow sources from an explicit lifetime or default parameter.
    fn sources_from_lifetime_or_parameter(
        &self,
        lifetime: &mir::Lifetime,
        parameter: u32,
    ) -> BorrowSources {
        if lifetime.is_empty() {
            return BorrowSources::one(BorrowSource::Slot(parameter));
        }

        self.sources_from_lifetime(lifetime)
    }

    /// Return borrow sources from a MIR lifetime.
    fn sources_from_lifetime(&self, lifetime: &mir::Lifetime) -> BorrowSources {
        BorrowSources::new(lifetime.terms.iter().map(|origin| match origin {
            mir::LifetimeTerm::Static => BorrowSource::Static,
            mir::LifetimeTerm::Slot(index) => BorrowSource::Slot(index.0),
        }))
    }

    /// Return the explicitly declared return lifetime.
    fn explicit_return_lifetime(&self) -> Option<mir::Lifetime> {
        self.tree
            .type_reference_lifetime(&self.function.return_type)
    }

    /// Return the best known place for one value.
    fn place_for_value(&self, value: mir::ValueReference) -> mir::Place {
        let Some(value) = value.value() else {
            return mir::Place::value(value);
        };

        self.function
            .value_place(value)
            .cloned()
            .unwrap_or_else(|| mir::Place::value(value.into()))
    }

    /// Return the projected place for one base value.
    fn projected_place(
        &self,
        base: mir::ValueReference,
        projection: mir::Projection,
    ) -> mir::Place {
        let mut place = self.place_for_value(base);
        place.push(projection);

        place
    }

    /// Return the moved place for one projected move.
    fn place_moved_by_projection(
        &self,
        base: mir::ValueReference,
        projection: mir::Projection,
    ) -> mir::Place {
        let place = self.place_for_value(base);
        if self.is_union_value(base) {
            return place;
        }

        place.with_projection(projection)
    }

    /// Return the known type for one value.
    fn type_for_value(&self, value: mir::Value) -> Option<mir::LocalNodeId<mir::Type>> {
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
