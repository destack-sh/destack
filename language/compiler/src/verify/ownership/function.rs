use std::collections::{HashMap, VecDeque};

use crate::verify::{VerifyError, VerifyState};
use destack_artifact::DiagnosticBuilder;
use destack_mir as mir;

use super::alias::PlaceAlias;
use super::borrow::{BorrowSource, BorrowSources};
use super::flow::FlowState;
use super::loan::Loan;
use super::r#move::{MoveState, MoveUse};

/// Ownership verifier for one function.
pub(super) struct FunctionVerifyState<'a, 'b> {
    /// The function being verified.
    function: &'a mir::Function,
    /// The MIR tree.
    tree: &'a mir::Tree,
    /// Verification context.
    context: &'a mut VerifyState<'b>,
    /// SSA value liveness.
    liveness: mir::FunctionLiveness,
    /// Place alias relation.
    aliases: PlaceAlias,
    /// Diagnostics produced by the active transfer.
    diagnostics: Vec<DiagnosticBuilder<VerifyError>>,
    /// Current flow state.
    flow: FlowState,
}

impl<'a, 'b> FunctionVerifyState<'a, 'b> {
    /// Create a function verifier.
    pub(super) fn new(
        function: &'a mir::Function,
        tree: &'a mir::Tree,
        context: &'a mut VerifyState<'b>,
    ) -> Self {
        Self {
            function,
            tree,
            context,
            liveness: mir::FunctionLiveness::build(function, tree),
            aliases: PlaceAlias::new(function, tree),
            diagnostics: Vec::new(),
            flow: FlowState::new(),
        }
    }

    /// Verify one function.
    pub(super) fn check(mut self) {
        let entries = self.solve_entries();

        // replay blocks with fixed entry states
        self.replay_blocks(entries);
    }

    /// Replay reachable blocks with diagnostics enabled.
    fn replay_blocks(&mut self, entries: HashMap<mir::LocalNodeId<mir::Block>, FlowState>) {
        for &block_id in self.function.blocks() {
            let Some(entry) = entries.get(&block_id).cloned() else {
                continue;
            };

            self.transfer_block(block_id, entry);
            self.flush_diagnostics();
        }
    }

    /// Solve fixed-point entry flow for every reachable block.
    fn solve_entries(&mut self) -> HashMap<mir::LocalNodeId<mir::Block>, FlowState> {
        let Some(entry) = self.function.entry() else {
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
            let exit_state = self.transfer_block(block_id, entry_state);
            self.discard_diagnostics();
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
            for successor in terminator.successors(self.tree) {
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
        if Some(block_id) == self.function.entry() {
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
        let arguments = terminator.successor_arguments(self.tree, successor);
        let parameters = terminator.successor_parameters(self.tree, successor);

        // bind edge arguments to successor block parameters
        for (parameter, argument) in parameters.iter().zip(arguments.iter().copied()) {
            flow.bind(argument, parameter.value);
        }

        // bind the implicit invoke result when the continuation receives one
        if terminator.has_successor_result(successor) {
            let bindings = self.invoke_result_sources(predecessor_id, terminator, &flow);
            let parameter = self
                .tree
                .get(successor)
                .parameters
                .first()
                .map(|parameter| parameter.value);
            if let (Some(bindings), Some(parameter)) = (bindings, parameter) {
                flow.borrows.insert_bindings(parameter, bindings);
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
    ) -> FlowState {
        self.flow = flow;
        self.diagnostics.clear();

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
        for parameter in &self.function.parameters {
            for (path, sources) in self.parameter_source_bindings(parameter.ty) {
                flow.borrows.insert_at(parameter.value, path, sources);
            }
        }

        flow
    }

    /// Return borrow source bindings implied by one parameter type.
    fn parameter_source_bindings(&self, ty: mir::TypeId) -> Vec<(mir::Path, BorrowSources)> {
        let ty_id = ty;
        let ty = self.tree.get(ty_id);

        match ty.reference_kind() {
            Some(mir::ReferenceKind::Managed) => {
                vec![(mir::Path::root(), self.managed_source_for_type(ty))]
            }
            Some(mir::ReferenceKind::Borrowed) => {
                let Some(lifetime) = ty.reference_lifetime() else {
                    return Vec::new();
                };

                vec![(mir::Path::root(), self.sources_from_lifetime(lifetime))]
            }
            _ => {
                let borrowed_paths = self.tree.type_borrowed_source_paths(ty_id);
                if borrowed_paths.is_empty() {
                    return Vec::new();
                }

                let mut bindings = Vec::new();
                let mut root_sources = BorrowSources::none();

                // derive aggregate sources from contained borrowed references
                for borrowed_path in borrowed_paths {
                    let sources = self.sources_from_lifetime(&borrowed_path.lifetime);
                    root_sources = root_sources.merge(&sources);
                    bindings.push((borrowed_path.path, sources));
                }
                bindings.push((mir::Path::root(), root_sources));

                bindings
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

        // reject another operation while aggregate decomposition is incomplete
        if !matches!(
            instruction,
            mir::Instruction::FieldGet { .. } | mir::Instruction::ElementGet { .. }
        ) {
            self.reject_partial_moves(anchor);
        }

        // check uses against initialized places
        self.check_instruction_uses(instruction, anchor);

        // record places defined by this instruction
        self.flow.apply_instruction(instruction);

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
                destination, base, ..
            } => {
                self.create_projection_loan(*destination, *base, anchor);
            }
            mir::Instruction::SliceView {
                destination,
                source,
                ..
            } => {
                self.create_projection_loan(*destination, *source, anchor);
            }
            mir::Instruction::FieldGet {
                destination,
                aggregate,
                field,
                ..
            } => {
                let projection = mir::Projection::Field { index: *field };
                self.move_projection_value(*destination, *aggregate, projection.clone(), anchor);
                self.propagate_projection_sources(*aggregate, projection, *destination);
            }
            mir::Instruction::ElementGet {
                destination,
                aggregate,
                index,
            } => {
                let projection = mir::Projection::Element { index: *index };
                self.move_projection_value(*destination, *aggregate, projection.clone(), anchor);
                self.propagate_projection_sources(*aggregate, projection, *destination);
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
                field,
                ..
            } => {
                let projection = mir::Projection::Field { index: *field };
                self.check_aggregate_set(*aggregate, *value, anchor);
                self.propagate_sources(*aggregate, *destination);
                self.propagate_sources_to_path(*value, *destination, projection);
            }
            mir::Instruction::ElementSet {
                destination,
                aggregate,
                value,
                index,
            } => {
                let projection = mir::Projection::Element { index: *index };
                self.check_aggregate_set(*aggregate, *value, anchor);
                self.propagate_sources(*aggregate, *destination);
                self.propagate_sources_to_path(*value, *destination, projection);
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
            mir::Instruction::NewZeroed { destination, .. }
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
            mir::Instruction::Call { call, .. } => {
                let arguments = self.tree.get_values(call.arguments).to_vec();
                let function = self.resolved_instruction_target(instruction_id, instruction);
                self.check_call_outlives(&call.signature, &arguments, anchor);
                self.define_call_result_sources(
                    instruction.destination(),
                    function,
                    &call.signature,
                    &arguments,
                );
            }
            _ => {}
        }

        // move values consumed by the instruction
        for value in instruction.consumes(self.tree) {
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

        // reject control flow while an aggregate remains partially moved
        self.reject_partial_moves(anchor);

        // check terminator uses against initialized values
        for value in terminator.uses(self.tree) {
            self.check_value_use(value, anchor);
        }

        // check returned borrows against the function return lifetime
        if let mir::Terminator::Return { value: Some(value) } = terminator {
            self.check_return(*value, anchor);
        }

        // check declared call lifetime relations
        self.check_terminator_call_outlives(terminator, anchor);

        // check tail-call result borrows against the function return lifetime
        self.check_tail_call_return(block_id, terminator, anchor);

        // move terminator arguments consumed by the current function
        for value in terminator.consumes(self.tree) {
            self.move_value(value, anchor);
        }
    }

    /// Define the instruction destination as initialized.
    fn define_destination(&mut self, instruction: &mir::Instruction) {
        let Some(destination) = instruction.destination() else {
            return;
        };

        let place = self.place_for_value(destination);
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
                aggregate, field, ..
            }
            | mir::Instruction::FieldAddr {
                aggregate, field, ..
            } => {
                let projection = mir::Projection::Field { index: *field };
                self.check_projection_use(*aggregate, projection, anchor);
            }
            mir::Instruction::ElementGet {
                aggregate, index, ..
            } => {
                let projection = mir::Projection::Element { index: *index };
                self.check_projection_use(*aggregate, projection, anchor);
            }
            mir::Instruction::ElementAddr { base, index, .. } => {
                let projection = mir::Projection::Index { index: *index };
                self.check_projection_use(*base, projection, anchor);
                self.check_value_use(*index, anchor);
            }
            mir::Instruction::SliceView {
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
                for value in instruction.reads(self.tree) {
                    self.check_value_use(value, anchor);
                }
            }
        }
    }

    /// Check one projected place use.
    fn check_projection_use(
        &mut self,
        base: mir::Value,
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
    fn check_value_use(&mut self, value: mir::Value, anchor: mir::LocalNodeIdAny) {
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
        let moved_at = self.context.anchor(moved.at);
        match moved.state {
            // report places moved by every predecessor
            MoveState::Moved => {
                self.emit_error(
                    VerifyError::UseAfterMove {
                        anchor: self.context.anchor(anchor),
                        moved_at: moved_at.clone(),
                    }
                    .label(moved_at, "value moved here"),
                );
            }
            // report places moved by at least one predecessor
            MoveState::MaybeMoved => {
                self.emit_error(
                    VerifyError::MaybeUseAfterMove {
                        anchor: self.context.anchor(anchor),
                        moved_at: moved_at.clone(),
                    }
                    .label(moved_at, "value moved on this path"),
                );
            }
        }
    }

    /// Move one value when it is move-only.
    fn move_value(&mut self, value: mir::Value, anchor: mir::LocalNodeIdAny) {
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
        value: mir::Value,
        base: mir::Value,
        projection: mir::Projection,
        anchor: mir::LocalNodeIdAny,
    ) {
        if !self.is_move_only(value) {
            return;
        }

        // reject moves out of values whose hook requires the complete receiver
        if self.has_drop_hook(base) {
            self.emit_error(VerifyError::MoveOutOfDrop {
                anchor: self.context.anchor(anchor),
            });
            return;
        }

        // move the complete variant or selected aggregate child
        let parent = self.place_for_value(base);
        let place = self.place_moved_by_projection(base, projection);
        if !self.check_place_change(&place, anchor) {
            return;
        }
        if !self.flow.moves.move_place(place, anchor) || self.is_variant_value(base) {
            return;
        }

        // begin or advance complete aggregate decomposition
        if self.flow.moves.step_decomposition(&parent, anchor) {
            return;
        }

        let child_count = self.decomposition_child_count(base);
        if child_count == 0 {
            self.flow.moves.move_place(parent, anchor);
        } else {
            self.flow
                .moves
                .begin_decomposition(parent, child_count, anchor);
        }
    }

    /// Reject an aggregate decomposition before another operation.
    fn reject_partial_moves(&mut self, anchor: mir::LocalNodeIdAny) {
        let Some(moved) = self.flow.moves.collapse_partial_moves() else {
            return;
        };
        let moved_at = self.context.anchor(moved.at);

        self.emit_error(
            VerifyError::PartialMove {
                anchor: self.context.anchor(anchor),
                moved_at: moved_at.clone(),
            }
            .label(moved_at, "aggregate decomposition begins here"),
        );
    }

    /// Create a loan whose projected place is recorded on the reference.
    fn create_projection_loan(
        &mut self,
        reference: mir::Value,
        root: mir::Value,
        anchor: mir::LocalNodeIdAny,
    ) {
        if self.flow.is_invalid_borrow(root) {
            return;
        }

        let place = self.place_for_value(reference);
        self.create_loan_from_place(reference, place, anchor, Some(root));
    }

    /// Create one active loan.
    fn create_loan_from_place(
        &mut self,
        reference: mir::Value,
        place: mir::Place,
        anchor: mir::LocalNodeIdAny,
        parent: Option<mir::Value>,
    ) -> bool {
        if !self.is_borrowed_reference(reference) {
            return false;
        }
        if self.flow.is_invalid_borrow(reference) {
            return false;
        }

        let access = self
            .reference_access(reference)
            .expect("borrowed reference must have access");
        let sources = self.sources_for_place(&place);
        if !sources.allows_borrow(access) {
            self.emit_error(VerifyError::ExclusiveBorrowFromSharedManaged {
                anchor: self.context.anchor(anchor),
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
            let active_borrow = self.context.anchor(conflict.created_at);
            self.emit_error(
                VerifyError::BorrowConflict {
                    anchor: self.context.anchor(anchor),
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
    fn check_store(&mut self, pointer: mir::Value, value: mir::Value, anchor: mir::LocalNodeIdAny) {
        // reject readonly reference writes
        if self
            .reference_access(pointer)
            .is_some_and(|access| !access.can_write())
        {
            self.emit_error(VerifyError::WriteThroughReadonlyReference {
                anchor: self.context.anchor(anchor),
            });
        }

        // reject writes blocked by exclusive loans
        let pointer_place = self.place_for_value(pointer);
        if let Some(loan) = self
            .flow
            .loans
            .blocking_write(&pointer_place, Some(pointer), |left, right| {
                self.aliases.may_alias(left, right)
            })
            .cloned()
        {
            let borrowed_at = self.context.anchor(loan.created_at);
            self.emit_error(
                VerifyError::InvalidationOfBorrowedPlace {
                    anchor: self.context.anchor(anchor),
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
                anchor: self.context.anchor(anchor),
            });
        }
    }

    /// Check a structural aggregate update for borrow escape.
    fn check_aggregate_set(
        &mut self,
        aggregate: mir::Value,
        value: mir::Value,
        anchor: mir::LocalNodeIdAny,
    ) {
        let aggregate_sources = self.sources_for_value(aggregate);
        let value_sources = self.sources_for_value(value);

        // reject local borrows embedded into escaping aggregates
        if value_sources.has_function_local_source() && aggregate_sources.is_escaping() {
            self.emit_error(VerifyError::BorrowOutlivesOrigin {
                anchor: self.context.anchor(anchor),
            });
        }
    }

    /// Check one returned value.
    fn check_return(&mut self, value: mir::Value, anchor: mir::LocalNodeIdAny) {
        let bindings = self.source_bindings_for_value(value);

        self.check_return_bindings(bindings, anchor);
    }

    /// Check tail-call result borrows against the function return lifetime.
    fn check_tail_call_return(
        &mut self,
        block_id: mir::LocalNodeId<mir::Block>,
        terminator: &mir::Terminator,
        anchor: mir::LocalNodeIdAny,
    ) {
        let mir::Terminator::TailCall { call } = terminator else {
            return;
        };
        let function = self.resolved_terminator_target(block_id, terminator);
        let arguments = self.tree.get_values(call.arguments);
        let bindings =
            self.call_result_source_bindings(function, &call.signature, arguments, &self.flow);

        self.check_return_bindings(bindings, anchor);
    }

    /// Check returned borrow bindings against declared return lifetimes.
    fn check_return_bindings(
        &mut self,
        bindings: Vec<(mir::Path, BorrowSources)>,
        anchor: mir::LocalNodeIdAny,
    ) {
        for (path, sources) in bindings {
            // returns need proven sources that are not frame-local
            let unsourced = sources.is_empty() || sources.has_function_local_source();

            // check the matching explicit return lifetime when one exists
            let uncovered = match self.return_lifetime_for_path(&path) {
                Some(required) => {
                    let required = self.widen_through_outlives(required);
                    !sources.is_covered_by(&required)
                }
                None => true,
            };

            if unsourced || uncovered {
                self.emit_error(VerifyError::BorrowOutlivesOrigin {
                    anchor: self.context.anchor(anchor),
                });

                return;
            }
        }
    }

    /// Check declared lifetime relations for one terminator call.
    fn check_terminator_call_outlives(
        &mut self,
        terminator: &mir::Terminator,
        anchor: mir::LocalNodeIdAny,
    ) {
        match terminator {
            mir::Terminator::Invoke { call, .. } | mir::Terminator::TailCall { call } => {
                self.check_call_outlives(
                    &call.signature,
                    self.tree.get_values(call.arguments),
                    anchor,
                );
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
        instruction.call_direct_target().or_else(|| {
            self.context
                .effects
                .call(mir::CallSite::Instruction(instruction_id))
                .and_then(|tables| tables.target)
        })
    }

    /// Return the resolved function target for one terminator callsite.
    fn resolved_terminator_target(
        &self,
        block_id: mir::LocalNodeId<mir::Block>,
        terminator: &mir::Terminator,
    ) -> Option<mir::LocalNodeId<mir::Function>> {
        terminator.call_direct_target().or_else(|| {
            self.context
                .effects
                .call(mir::CallSite::Terminator(block_id))
                .and_then(|tables| tables.target)
        })
    }

    /// Return borrow sources for one invoke result.
    fn invoke_result_sources(
        &self,
        block_id: mir::LocalNodeId<mir::Block>,
        terminator: &mir::Terminator,
        flow: &FlowState,
    ) -> Option<Vec<(mir::Path, BorrowSources)>> {
        let mir::Terminator::Invoke { call, .. } = terminator else {
            return None;
        };
        let function = self.resolved_terminator_target(block_id, terminator);
        let arguments = self.tree.get_values(call.arguments);

        Some(self.call_result_source_bindings(function, &call.signature, arguments, flow))
    }

    /// Check declared lifetime relations against call arguments.
    fn check_call_outlives(
        &mut self,
        signature: &mir::TypeId,
        arguments: &[mir::Value],
        anchor: mir::LocalNodeIdAny,
    ) {
        let parameter_types = self.call_parameter_types(signature);

        // prove every declared outlives row from the actual argument sources
        let rows = self.signature_outlives_rows(signature);
        for (slot, target) in rows {
            let longer = self.sources_from_callee_lifetime(
                &mir::Lifetime::slot(slot),
                &parameter_types,
                &arguments,
                &self.flow,
            );
            let shorter = self.sources_from_callee_lifetime(
                &mir::Lifetime::slot(target.0),
                &parameter_types,
                &arguments,
                &self.flow,
            );
            // result-only slots bind no arguments and discharge through
            //  the widened result mapping instead
            if shorter.is_empty() {
                continue;
            }
            if !longer.outlives(&shorter) {
                self.emit_error(VerifyError::BorrowOutlivesOrigin {
                    anchor: self.context.anchor(anchor),
                });
            }
        }
    }

    /// Return the declared outlives rows encoded in one signature type.
    fn signature_outlives_rows(&self, signature: &mir::TypeId) -> Vec<(u32, mir::LifetimeSlot)> {
        let mir::Type::FunctionSignature { lifetimes, .. } = self.tree.get(*signature) else {
            return Vec::new();
        };

        let mut rows = Vec::new();
        for (slot, parameter) in lifetimes.iter().enumerate() {
            for target in &parameter.outlives {
                rows.push((slot as u32, *target));
            }
        }

        rows
    }

    /// Define borrow sources for one call result.
    fn define_call_result_sources(
        &mut self,
        destination: Option<mir::Value>,
        function: Option<mir::LocalNodeId<mir::Function>>,
        signature: &mir::TypeId,
        arguments: &[mir::Value],
    ) {
        let Some(destination) = destination else {
            return;
        };
        if !self.value_can_carry_sources(destination) {
            return;
        }

        let bindings = self.call_result_source_bindings(function, signature, arguments, &self.flow);
        self.define_source_bindings(destination, bindings);
    }

    /// Return borrow source bindings for one call result.
    fn call_result_source_bindings(
        &self,
        function: Option<mir::LocalNodeId<mir::Function>>,
        signature: &mir::TypeId,
        arguments: &[mir::Value],
        flow: &FlowState,
    ) -> Vec<(mir::Path, BorrowSources)> {
        let paths = function
            .map(|function| self.function_return_borrowed_paths(function))
            .filter(|paths| !paths.is_empty())
            .unwrap_or_else(|| self.signature_return_borrowed_paths(signature));
        if !paths.is_empty() {
            let parameter_types = self.call_parameter_types(signature);

            return paths
                .into_iter()
                .map(|borrowed_path| {
                    let lifetime = self.widen_with_signature(borrowed_path.lifetime, signature);
                    let sources = self.sources_from_callee_lifetime(
                        &lifetime,
                        &parameter_types,
                        arguments,
                        flow,
                    );

                    (borrowed_path.path, sources)
                })
                .collect();
        }

        let lifetime = function
            .and_then(|function| self.function_return_lifetime(function))
            .or_else(|| self.signature_return_lifetime(signature));
        let Some(lifetime) = lifetime else {
            return Vec::new();
        };
        let parameter_types = self.call_parameter_types(signature);
        let lifetime = self.widen_with_signature(lifetime, signature);
        let sources =
            self.sources_from_callee_lifetime(&lifetime, &parameter_types, arguments, flow);

        vec![(mir::Path::root(), sources)]
    }

    /// Return call parameter types from the lowered signature.
    fn call_parameter_types(&self, signature: &mir::TypeId) -> Vec<mir::TypeId> {
        let mir::Type::FunctionSignature { parameters, .. } = self.tree.get(*signature) else {
            return Vec::new();
        };

        parameters.iter().map(|parameter| parameter.ty).collect()
    }

    /// Return borrowed return paths for one known function.
    fn function_return_borrowed_paths(
        &self,
        function_id: mir::LocalNodeId<mir::Function>,
    ) -> Vec<mir::BorrowedPath> {
        self.tree
            .type_borrowed_paths(self.tree.get(function_id).return_type)
    }

    /// Return borrowed return paths for one signature type.
    fn signature_return_borrowed_paths(&self, signature: &mir::TypeId) -> Vec<mir::BorrowedPath> {
        let mir::Type::FunctionSignature { result, .. } = self.tree.get(*signature) else {
            return Vec::new();
        };

        self.tree.type_borrowed_paths(*result)
    }

    /// Return the return lifetime encoded in one known function.
    fn function_return_lifetime(
        &self,
        function_id: mir::LocalNodeId<mir::Function>,
    ) -> Option<mir::Lifetime> {
        let function = self.tree.get(function_id);
        if !self.tree.type_contains_borrowed_refs(function.return_type) {
            return None;
        }

        self.tree
            .type_lifetime(function.return_type)
            .filter(|lifetime| !lifetime.is_empty())
    }

    /// Return the declared return lifetime for one signature type.
    fn signature_return_lifetime(&self, signature: &mir::TypeId) -> Option<mir::Lifetime> {
        let mir::Type::FunctionSignature { result, .. } = self.tree.get(*signature) else {
            return None;
        };

        self.tree
            .type_lifetime(*result)
            .filter(|lifetime| !lifetime.is_empty())
    }

    /// Map callee lifetime origins to caller-side borrow sources.
    fn sources_from_callee_lifetime(
        &self,
        lifetime: &mir::Lifetime,
        parameter_types: &[mir::TypeId],
        arguments: &[mir::Value],
        flow: &FlowState,
    ) -> BorrowSources {
        let mut sources = BorrowSources::none();

        // map static origins directly
        for origin in &lifetime.terms {
            if matches!(origin, mir::LifetimeTerm::Static) {
                sources = sources.merge(&BorrowSources::one(BorrowSource::Static));
            }
        }

        // map lifetime slots through parameter path facts
        for (index, ty) in parameter_types.iter().enumerate() {
            let Some(argument) = arguments.get(index) else {
                continue;
            };

            for (path, callee_sources) in self.parameter_source_bindings(*ty) {
                if !callee_sources.is_covered_by(lifetime) {
                    continue;
                }

                let argument_sources = self.sources_for_value_path_in_flow(*argument, &path, flow);
                sources = sources.merge(&argument_sources);
            }
        }

        sources
    }

    /// Check whether changing a value invalidates active loans.
    fn check_value_change(&mut self, value: mir::Value, anchor: mir::LocalNodeIdAny) -> bool {
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
        let borrowed_at = self.context.anchor(loan.created_at);
        self.emit_error(
            VerifyError::InvalidationOfBorrowedPlace {
                anchor: self.context.anchor(anchor),
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
            .any(|parameter| parameter.value == value);
        if !is_parameter {
            return false;
        }

        let is_used_by_instruction = block.instructions.iter().any(|instruction_id| {
            self.tree
                .get(*instruction_id)
                .reads(self.tree)
                .contains(&value)
        });
        let terminator = self.tree.get(block.terminator);
        let is_used_by_terminator = terminator.uses(self.tree).contains(&value);

        is_used_by_instruction || is_used_by_terminator
    }

    /// Return whether a value is move-only.
    fn is_move_only(&self, value: mir::Value) -> bool {
        let Some(ty) = self.function.value_type(value) else {
            return false;
        };

        self.tree.get(ty).copy(self.tree).is_no()
    }

    /// Return the direct move-only child count for one aggregate.
    fn decomposition_child_count(&self, value: mir::Value) -> u64 {
        let Some(ty) = self.function.value_type(value) else {
            return 0;
        };
        let ty = self.tree.repr_type(ty);

        match self.tree.get(ty) {
            mir::Type::Struct { fields, .. } => fields
                .iter()
                .filter(|field| {
                    self.tree
                        .get(self.tree.get(**field).ty)
                        .copy(self.tree)
                        .is_no()
                })
                .count() as u64,
            mir::Type::Tuple { elements, .. } => elements
                .iter()
                .filter(|element| self.tree.get(**element).copy(self.tree).is_no())
                .count() as u64,
            mir::Type::FixedArray {
                element, length, ..
            } => {
                if self.tree.get(*element).copy(self.tree).is_no() {
                    *length
                } else {
                    0
                }
            }
            mir::Type::Newtype { inner, .. } => {
                u64::from(self.tree.get(*inner).copy(self.tree).is_no())
            }
            _ => 0,
        }
    }

    /// Return whether a value has a user-authored drop hook.
    fn has_drop_hook(&self, value: mir::Value) -> bool {
        let Some(ty) = self.function.value_type(value) else {
            return false;
        };

        self.context.drops.hook(ty).is_some()
    }

    /// Return whether one value has a variant type.
    fn is_variant_value(&self, value: mir::Value) -> bool {
        let Some(ty) = self.function.value_type(value) else {
            return false;
        };

        matches!(self.tree.get(ty), mir::Type::Variant { .. })
    }

    /// Return whether one value is a borrowed reference-like value.
    fn is_borrowed_reference(&self, value: mir::Value) -> bool {
        let Some(ty) = self.function.value_type(value) else {
            return false;
        };

        self.tree.get(ty).is_borrowed_reference()
    }

    /// Return access for one reference-like value.
    fn reference_access(&self, value: mir::Value) -> Option<mir::Access> {
        let ty = self.function.value_type(value)?;

        self.tree.get(ty).reference_access()
    }

    /// Propagate known borrow sources from one value to another.
    fn propagate_sources(&mut self, root: mir::Value, destination: mir::Value) {
        let place = self.place_for_value(root);

        self.propagate_sources_from_place(&place, destination);
    }

    /// Propagate known borrow sources from one projected value to another.
    fn propagate_projection_sources(
        &mut self,
        root: mir::Value,
        projection: mir::Projection,
        destination: mir::Value,
    ) {
        let place = self.projected_place(root, projection);

        self.propagate_sources_from_place(&place, destination);
    }

    /// Propagate known borrow sources into one destination path.
    fn propagate_sources_to_path(
        &mut self,
        root: mir::Value,
        destination: mir::Value,
        projection: mir::Projection,
    ) {
        let base_path = mir::Path::root().with_projection(projection);
        for (path, sources) in self.source_bindings_for_value(root) {
            let path = base_path.clone().with_path(&path);
            self.flow
                .borrows
                .merge_sources_at(destination, &mir::Path::root(), &sources);
            self.flow.borrows.insert_at(destination, path, sources);
        }
    }

    /// Propagate known borrow sources from one place to another value.
    fn propagate_sources_from_place(&mut self, place: &mir::Place, destination: mir::Value) {
        if !self.value_can_carry_sources(destination) {
            return;
        }

        let root_sources = self.sources_for_place(place);
        self.flow.borrows.insert(destination, root_sources);

        let Some(destination_type) = self.function.value_type(destination) else {
            return;
        };
        let destination_type = mir::TypeId::from(destination_type);

        // propagate nested borrowed paths without collapsing sibling fields
        for borrowed_path in self.tree.type_borrowed_source_paths(destination_type) {
            let path = place.path.clone().with_path(&borrowed_path.path);
            let place = mir::Place {
                origin: place.origin,
                path,
            };
            let sources = self.sources_for_place(&place);

            self.flow
                .borrows
                .insert_at(destination, borrowed_path.path, sources);
        }
    }

    /// Return whether one value can carry borrowed source metadata.
    fn value_can_carry_sources(&self, value: mir::Value) -> bool {
        let Some(ty) = self.function.value_type(value) else {
            return false;
        };
        let ty = mir::TypeId::from(ty);

        self.tree.type_lifetime(ty).is_some() || self.tree.type_contains_borrowed_refs(ty)
    }

    /// Define borrow sources for one destination value.
    fn define_sources(&mut self, destination: mir::Value, sources: BorrowSources) {
        self.flow.borrows.insert(destination, sources);
    }

    /// Define borrow source bindings for one destination value.
    fn define_source_bindings(
        &mut self,
        destination: mir::Value,
        bindings: Vec<(mir::Path, BorrowSources)>,
    ) {
        self.flow.borrows.insert_bindings(destination, bindings);
    }

    /// Return borrow source bindings for one value.
    fn source_bindings_for_value(&self, value: mir::Value) -> Vec<(mir::Path, BorrowSources)> {
        let Some(ty) = self.function.value_type(value) else {
            return Vec::new();
        };

        let ty = mir::TypeId::from(ty);
        let borrowed_paths = self.tree.type_borrowed_source_paths(ty);
        if borrowed_paths.is_empty() {
            let sources = self.sources_for_value(value);
            if sources.is_empty() {
                return Vec::new();
            }

            return vec![(mir::Path::root(), sources)];
        }

        borrowed_paths
            .into_iter()
            .map(|borrowed_path| {
                let sources = self.sources_for_value_path(value, &borrowed_path.path);

                (borrowed_path.path, sources)
            })
            .collect()
    }

    /// Return borrow sources for one value.
    fn sources_for_value(&self, value: mir::Value) -> BorrowSources {
        self.sources_for_value_in_flow(value, &self.flow)
    }

    /// Return borrow sources for one value path.
    fn sources_for_value_path(&self, value: mir::Value, path: &mir::Path) -> BorrowSources {
        self.sources_for_value_path_in_flow(value, path, &self.flow)
    }

    /// Return borrow sources for one value in a flow state.
    fn sources_for_value_in_flow(&self, value: mir::Value, flow: &FlowState) -> BorrowSources {
        self.sources_for_value_path_in_flow(value, &mir::Path::root(), flow)
    }

    /// Return borrow sources for one value path in a flow state.
    fn sources_for_value_path_in_flow(
        &self,
        value: mir::Value,
        path: &mir::Path,
        flow: &FlowState,
    ) -> BorrowSources {
        flow.borrows
            .get_at(value, path)
            .cloned()
            .unwrap_or_else(|| {
                if path.is_root() {
                    self.type_sources(value)
                } else {
                    BorrowSources::none()
                }
            })
    }

    /// Return borrow sources implied by one value type.
    fn type_sources(&self, value: mir::Value) -> BorrowSources {
        let Some(ty) = self.function.value_type(value) else {
            return BorrowSources::none();
        };

        let ty = mir::TypeId::from(ty);
        if let Some(lifetime) = self.tree.type_lifetime(ty) {
            return self.sources_from_lifetime(&lifetime);
        }

        BorrowSources::none()
    }

    /// Return borrow sources for one place.
    fn sources_for_place(&self, place: &mir::Place) -> BorrowSources {
        match place.origin {
            mir::PlaceOrigin::Local(_) => BorrowSources::one(BorrowSource::Owned),
            mir::PlaceOrigin::Global(_) => BorrowSources::one(BorrowSource::Static),
            mir::PlaceOrigin::Value(value) => self.sources_for_storage_path(value, &place.path),
        }
    }

    /// Return borrow sources implied by a value path used as storage.
    fn sources_for_storage_path(&self, value: mir::Value, path: &mir::Path) -> BorrowSources {
        // prefer flow sources from projections and propagated values
        if let Some(sources) = self.flow.borrows.get_at(value, path) {
            return sources.clone();
        }
        if !path.is_root()
            && let Some(sources) = self.flow.borrows.get_at(value, &mir::Path::root())
        {
            return sources.clone();
        }

        let Some(ty) = self.function.value_type(value) else {
            return BorrowSources::none();
        };

        let ty = self.storage_type(ty);
        let ty = self.tree.get(ty);
        match ty.reference_kind() {
            // keep projected reference storage tied to the reference root
            Some(mir::ReferenceKind::Managed | mir::ReferenceKind::Borrowed) => self
                .flow
                .borrows
                .get_at(value, &mir::Path::root())
                .cloned()
                .unwrap_or_else(|| self.reference_source_for_type(ty)),
            // keep owned storage alive through the unique handle
            Some(mir::ReferenceKind::Unique) => BorrowSources::one(BorrowSource::Owned),
            Some(mir::ReferenceKind::Raw) => BorrowSources::none(),
            _ => BorrowSources::none(),
        }
    }

    /// Return borrow sources implied by a storage-producing destination.
    fn sources_for_destination_storage(&self, destination: mir::Value) -> BorrowSources {
        let Some(ty) = self.function.value_type(destination) else {
            return BorrowSources::none();
        };

        let ty = self.storage_type(ty);
        let ty = self.tree.get(ty);
        match ty.reference_kind() {
            Some(mir::ReferenceKind::Managed) => self.managed_source_for_type(ty),
            Some(mir::ReferenceKind::Unique | mir::ReferenceKind::Raw) => {
                BorrowSources::one(BorrowSource::Owned)
            }
            Some(mir::ReferenceKind::Borrowed) | None => BorrowSources::none(),
        }
    }

    /// Return a managed borrow source for one type.
    fn managed_source_for_type(&self, ty: &mir::Type) -> BorrowSources {
        let Some(space) = Self::reference_space(ty).cloned() else {
            return BorrowSources::none();
        };

        let lifetime = ty
            .reference_lifetime()
            .filter(|lifetime| !lifetime.is_empty())
            .cloned();

        BorrowSources::one(BorrowSource::Managed { space, lifetime })
    }

    /// Return the borrow source for a reference-like type.
    fn reference_source_for_type(&self, ty: &mir::Type) -> BorrowSources {
        match ty.reference_kind() {
            Some(mir::ReferenceKind::Managed) => self.managed_source_for_type(ty),
            Some(mir::ReferenceKind::Borrowed) => {
                let Some(lifetime) = ty.reference_lifetime() else {
                    return BorrowSources::none();
                };
                if lifetime.is_empty() {
                    return BorrowSources::none();
                }

                self.sources_from_lifetime(lifetime)
            }
            Some(mir::ReferenceKind::Unique) => BorrowSources::one(BorrowSource::Owned),
            Some(mir::ReferenceKind::Raw) | None => BorrowSources::none(),
        }
    }

    /// Return the initialized storage type represented by one type.
    fn storage_type(&self, ty: mir::LocalNodeId<mir::Type>) -> mir::LocalNodeId<mir::Type> {
        let mir::Type::Uninit { value } = self.tree.get(ty) else {
            return ty;
        };

        *value
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

    /// Return borrow sources from a MIR lifetime.
    fn sources_from_lifetime(&self, lifetime: &mir::Lifetime) -> BorrowSources {
        BorrowSources::new(lifetime.terms.iter().map(|origin| match origin {
            mir::LifetimeTerm::Static => BorrowSource::Static,
            mir::LifetimeTerm::Slot(slot) => BorrowSource::Lifetime(*slot),
        }))
    }

    /// Widen one required lifetime with every declared slot that outlives it.
    fn widen_through_outlives(&self, required: mir::Lifetime) -> mir::Lifetime {
        Self::widen_with_lifetimes(required, &self.function.lifetimes)
    }

    /// Widen one lifetime through the outlives rows of a declared slot list.
    fn widen_with_lifetimes(
        required: mir::Lifetime,
        lifetimes: &[mir::LifetimeParameter],
    ) -> mir::Lifetime {
        let mut terms = required.terms.clone();

        // close over declared rows: a covered slot admits its outliving slots
        let mut index = 0;
        while index < terms.len() {
            if let mir::LifetimeTerm::Slot(target) = terms[index] {
                for (slot, parameter) in lifetimes.iter().enumerate() {
                    if parameter.outlives.contains(&target) {
                        let term = mir::LifetimeTerm::Slot(mir::LifetimeSlot(slot as u32));
                        if !terms.contains(&term) {
                            terms.push(term);
                        }
                    }
                }
            }
            index += 1;
        }

        mir::Lifetime::new(terms)
    }

    /// Widen one callee lifetime through the signature's declared rows.
    fn widen_with_signature(
        &self,
        required: mir::Lifetime,
        signature: &mir::TypeId,
    ) -> mir::Lifetime {
        let mir::Type::FunctionSignature { lifetimes, .. } = self.tree.get(*signature) else {
            return required;
        };

        Self::widen_with_lifetimes(required, lifetimes)
    }

    /// Return the explicitly declared return lifetime.
    fn explicit_return_lifetime(&self) -> Option<mir::Lifetime> {
        self.tree.type_lifetime(self.function.return_type)
    }

    /// Return the return lifetime declared for one borrowed path.
    fn return_lifetime_for_path(&self, path: &mir::Path) -> Option<mir::Lifetime> {
        let borrowed_paths = self.tree.type_borrowed_paths(self.function.return_type);
        if let Some(borrowed_path) = borrowed_paths
            .into_iter()
            .find(|borrowed_path| &borrowed_path.path == path)
        {
            return Some(borrowed_path.lifetime);
        }

        if path.is_root() {
            self.explicit_return_lifetime()
        } else {
            None
        }
    }

    /// Return the best known place for one value.
    fn place_for_value(&self, value: mir::Value) -> mir::Place {
        self.flow.place_for_value(value)
    }

    /// Return the projected place for one base value.
    fn projected_place(&self, base: mir::Value, projection: mir::Projection) -> mir::Place {
        let mut place = self.place_for_value(base);
        place.push(projection);

        place
    }

    /// Return the moved place for one projected move.
    fn place_moved_by_projection(
        &self,
        base: mir::Value,
        projection: mir::Projection,
    ) -> mir::Place {
        let place = self.place_for_value(base);
        if self.is_variant_value(base) {
            return place;
        }

        place.with_projection(projection)
    }

    /// Emit one error when diagnostics are enabled.
    fn emit_error(&mut self, error: impl Into<DiagnosticBuilder<VerifyError>>) {
        self.diagnostics.push(error.into());
    }

    /// Flush active transfer diagnostics into verify state.
    fn flush_diagnostics(&mut self) {
        for diagnostic in self.diagnostics.drain(..) {
            self.context.emit_error(diagnostic);
        }
    }

    /// Discard active transfer diagnostics.
    fn discard_diagnostics(&mut self) {
        self.diagnostics.clear();
    }

}
