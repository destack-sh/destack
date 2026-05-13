use std::collections::{HashMap, VecDeque};

use crate::common::mir::terminator_arguments_for_successor;
use crate::verify::value::{instruction_consumes, instruction_uses, terminator_consumes};
use crate::verify::{VerifyError, VerifyState};
use destack_artifact::DiagnosticBuilder;
use destack_mir as mir;

use super::alias::PlaceAlias;
use super::borrow::{BorrowRoot, BorrowRoots};
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
    /// Whether this pass should emit diagnostics.
    is_diagnostics_enabled: bool,
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
            is_diagnostics_enabled: true,
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
        let predecessor = self.tree.get(predecessor);
        let terminator = self.tree.get(predecessor.terminator);
        let arguments = terminator_arguments_for_successor(terminator, successor);
        let successor_block = self.tree.get(successor);

        // bind edge arguments to successor block parameters
        for (parameter, argument) in successor_block.parameters.iter().zip(arguments) {
            let (Some(parameter), Some(argument)) = (parameter.value.value(), argument.value())
            else {
                continue;
            };

            flow.bind(argument, parameter);
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
        self.is_diagnostics_enabled = is_diagnostics_enabled;

        let block = self.tree.get(block_id);

        // apply instruction reads, moves, borrows, and definitions
        for (index, &instruction_id) in block.instructions.iter().enumerate() {
            let instruction = self.tree.get(instruction_id);
            self.transfer_instruction(block_id, index, instruction_id, instruction);
            self.expire_instruction_loans(block_id, index);
        }

        // apply terminator returns, branches, and consumes
        self.transfer_terminator(block.terminator, self.tree.get(block.terminator));

        self.flow.clone()
    }

    /// Return initial flow state for the function entry.
    fn initial_state(&self) -> FlowState {
        let mut flow = FlowState::new();

        // seed parameter borrow roots from declared types
        for (index, parameter) in self.function.parameters.iter().enumerate() {
            let Some(value) = parameter.value.value() else {
                continue;
            };

            let roots = self.parameter_roots(index as u32, parameter.ty);
            flow.borrows.insert(value, roots);
        }

        flow
    }

    /// Return the borrow roots implied by one parameter type.
    fn parameter_roots(&self, index: u32, ty: mir::TypeReference) -> BorrowRoots {
        let Some(ty) = ty.ty() else {
            return BorrowRoots::none();
        };
        let ty_ref = mir::TypeReference::Type(ty);
        let ty = self.tree.get(ty);

        match ty.reference_kind() {
            // root managed parameters for their own lifetime
            Some(mir::ReferenceKind::Managed) => BorrowRoots::one(BorrowRoot::Parameter(index)),
            // prefer explicit borrowed lifetimes over parameter defaults
            Some(mir::ReferenceKind::Borrowed) => {
                let Some(lifetime) = ty.reference_lifetime() else {
                    return BorrowRoots::none();
                };

                self.roots_from_lifetime_or_parameter(lifetime, index)
            }
            _ => {
                // derive aggregate roots from contained borrowed references
                if let Some(lifetime) = self.tree.type_reference_lifetime(ty_ref) {
                    return self.roots_from_lifetime(&lifetime);
                }
                if self.tree.type_reference_contains_borrowed_refs(ty_ref) {
                    return BorrowRoots::one(BorrowRoot::Parameter(index));
                }

                BorrowRoots::none()
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

        // update loans and borrow roots from instruction forms
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
                    mir::PlaceProjection::Field { index: *index },
                    anchor,
                );
                self.propagate_roots(*aggregate, *destination);
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
                    mir::PlaceProjection::Element { index: *index },
                    anchor,
                );
                self.propagate_roots(*array, *destination);
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
                self.propagate_roots(*aggregate, *destination);
            }
            mir::Instruction::ElementSet {
                destination,
                array,
                value,
                ..
            } => {
                self.check_aggregate_set(*array, *value, anchor);
                self.propagate_roots(*array, *destination);
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
                self.propagate_roots(*pointer, *destination);
            }
            mir::Instruction::StackAlloc { destination, .. }
            | mir::Instruction::New { destination, .. }
            | mir::Instruction::NewSlice { destination, .. }
            | mir::Instruction::RawAlloc { destination, .. } => {
                self.define_roots(
                    *destination,
                    BorrowRoots::one(BorrowRoot::Local(self.place_for_value(*destination))),
                );
            }
            mir::Instruction::GlobalAddr { destination, .. } => {
                self.define_roots(*destination, BorrowRoots::one(BorrowRoot::Static));
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
                let projection = mir::PlaceProjection::Field { index: *index };
                self.check_projection_use(*aggregate, projection, anchor);
            }
            mir::Instruction::ElementGet { array, index, .. } => {
                let projection = mir::PlaceProjection::Element { index: *index };
                self.check_projection_use(*array, projection, anchor);
            }
            mir::Instruction::ElementAddr { array, index, .. } => {
                let projection = mir::PlaceProjection::Index { index: *index };
                self.check_projection_use(*array, projection, anchor);
                self.check_value_use(*index, anchor);
            }
            mir::Instruction::Slice {
                source,
                start,
                length,
                ..
            } => {
                let projection = mir::PlaceProjection::Range {
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
        projection: mir::PlaceProjection,
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
        projection: mir::PlaceProjection,
        anchor: mir::LocalNodeIdAny,
    ) {
        if !self.is_move_only(value) {
            return;
        }

        // reject partial moves through custom drop glue
        if !self.is_union_value(base) && self.has_custom_drop(base) {
            self.emit_error(VerifyError::PartialMoveOfDropType {
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
            self.propagate_roots(root, reference);
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
            let existing_loan = self.context.anchor(self.tree, conflict.created_at);
            self.emit_error(
                VerifyError::ConflictingLoan {
                    anchor: self.context.anchor(self.tree, anchor),
                    existing_loan: existing_loan.clone(),
                }
                .label(existing_loan, "active loan is here"),
            );

            self.flow.invalidate_borrow(reference);
            return false;
        }

        self.flow.loans.insert(loan);

        let roots = self.roots_for_place(&place);
        self.flow.borrows.insert(reference, roots);

        true
    }

    /// Check a pointer store for access and borrow escape.
    fn check_store(
        &mut self,
        pointer: mir::ValueReference,
        value: mir::ValueReference,
        anchor: mir::LocalNodeIdAny,
    ) {
        // reject writes through readonly references
        if self
            .reference_access(pointer)
            .is_some_and(|access| !access.can_write())
        {
            self.emit_error(VerifyError::ReadonlyWrite {
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
                VerifyError::ChangeOfBorrowedPlace {
                    anchor: self.context.anchor(self.tree, anchor),
                    borrowed_at: borrowed_at.clone(),
                }
                .label(borrowed_at, "loan is here"),
            );
        }

        // reject local borrows stored into escaping locations
        let value_roots = self.roots_for_value(value);
        let target_roots = self.roots_for_place(&pointer_place);
        if value_roots.has_local_root() && target_roots.is_escaping() {
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
        let aggregate_roots = self.roots_for_value(aggregate);
        let value_roots = self.roots_for_value(value);

        // reject local borrows embedded into escaping aggregates
        if value_roots.has_local_root() && aggregate_roots.is_escaping() {
            self.emit_error(VerifyError::BorrowOutlivesOrigin {
                anchor: self.context.anchor(self.tree, anchor),
            });
        }
    }

    /// Check one returned value.
    fn check_return(&mut self, value: mir::ValueReference, anchor: mir::LocalNodeIdAny) {
        let roots = self.roots_for_value(value);
        if roots.is_empty() {
            return;
        }

        // reject frame-local borrows escaping the function
        if roots.has_local_root() {
            self.emit_error(VerifyError::BorrowOutlivesOrigin {
                anchor: self.context.anchor(self.tree, anchor),
            });
            return;
        }

        // apply explicit return lifetimes to nonlocal roots
        let Some(required) = self.explicit_return_lifetime() else {
            return;
        };

        if !roots.is_covered_by(required) {
            self.emit_error(VerifyError::BorrowOutlivesOrigin {
                anchor: self.context.anchor(self.tree, anchor),
            });
        }
    }

    /// Check live borrows at one suspension point.
    fn check_suspension(&mut self, terminator: &mir::Terminator, anchor: mir::LocalNodeIdAny) {
        let exclusive_loans = self
            .flow
            .loans
            .loans
            .iter()
            .filter(|loan| loan.access.is_exclusive())
            .cloned()
            .collect::<Vec<_>>();
        let mut reported = Vec::new();

        // reject each exclusive loan that remains live
        for loan in exclusive_loans {
            reported.push(loan.reference);

            let borrowed_at = self.context.anchor(self.tree, loan.created_at);
            self.emit_error(
                VerifyError::ExclusiveLoanAcrossSuspension {
                    anchor: self.context.anchor(self.tree, anchor),
                    borrowed_at: borrowed_at.clone(),
                }
                .label(borrowed_at, "exclusive loan is here"),
            );
        }

        // reject exclusive borrowed parameters or propagated references
        for value in self.exclusive_references_across_suspension(terminator) {
            if reported.contains(&value) {
                continue;
            }

            let borrowed_at = self.context.anchor(
                self.tree,
                self.anchor_for_exclusive_reference(value, anchor),
            );
            self.emit_error(
                VerifyError::ExclusiveLoanAcrossSuspension {
                    anchor: self.context.anchor(self.tree, anchor),
                    borrowed_at: borrowed_at.clone(),
                }
                .label(borrowed_at, "exclusive reference is live here"),
            );
        }
    }

    /// Return exclusive borrowed references that cross one suspension point.
    fn exclusive_references_across_suspension(
        &self,
        terminator: &mir::Terminator,
    ) -> Vec<mir::Value> {
        let mir::Terminator::Yield { resume, .. } = terminator else {
            return Vec::new();
        };

        let mut values = Vec::new();

        // check parameter references carried into the continuation
        for parameter in &self.function.parameters {
            let Some(value) = parameter.value.value() else {
                continue;
            };
            if self.is_exclusive_borrowed_value(value)
                && self.is_value_live_across_suspension(value, resume)
            {
                values.push(value);
            }
        }

        // check instruction results carried into the continuation
        for (index, _) in self.function.value_types.iter().enumerate() {
            let value = mir::Value::new(index as u32);
            if self.is_exclusive_borrowed_value(value)
                && self.is_value_live_across_suspension(value, resume)
                && !values.contains(&value)
            {
                values.push(value);
            }
        }

        values
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

    /// Return whether one value is an exclusive borrowed reference.
    fn is_exclusive_borrowed_value(&self, value: mir::Value) -> bool {
        let Some(ty) = self.type_for_value(value) else {
            return false;
        };
        let ty = self.tree.get(ty);

        ty.is_borrowed_reference()
            && ty
                .reference_access()
                .is_some_and(|access| access.is_exclusive())
    }

    /// Return the diagnostic anchor for one exclusive reference.
    fn anchor_for_exclusive_reference(
        &self,
        value: mir::Value,
        fallback: mir::LocalNodeIdAny,
    ) -> mir::LocalNodeIdAny {
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
            VerifyError::ChangeOfBorrowedPlace {
                anchor: self.context.anchor(self.tree, anchor),
                borrowed_at: borrowed_at.clone(),
            }
            .label(borrowed_at, "loan is here"),
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

        matches!(self.tree.get(ty), mir::Type::Union { .. })
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

    /// Propagate known borrow roots from one value to another.
    fn propagate_roots(&mut self, root: mir::ValueReference, destination: mir::ValueReference) {
        let Some(destination) = destination.value() else {
            return;
        };

        let roots = self.roots_for_value(root);
        self.flow.borrows.insert(destination, roots);
    }

    /// Define borrow roots for one destination value.
    fn define_roots(&mut self, destination: mir::ValueReference, roots: BorrowRoots) {
        let Some(destination) = destination.value() else {
            return;
        };

        self.flow.borrows.insert(destination, roots);
    }

    /// Return borrow roots for one value.
    fn roots_for_value(&self, value: mir::ValueReference) -> BorrowRoots {
        let Some(value) = value.value() else {
            return BorrowRoots::none();
        };

        self.flow
            .borrows
            .get(value)
            .cloned()
            .unwrap_or_else(|| self.type_roots(value))
    }

    /// Return borrow roots implied by one value type.
    fn type_roots(&self, value: mir::Value) -> BorrowRoots {
        let Some(ty) = self.type_for_value(value) else {
            return BorrowRoots::none();
        };

        if let Some(lifetime) = self.tree.type_reference_lifetime(ty.into()) {
            return self.roots_from_lifetime(&lifetime);
        }

        BorrowRoots::none()
    }

    /// Return borrow roots for one place.
    fn roots_for_place(&self, place: &mir::Place) -> BorrowRoots {
        match place.origin {
            mir::PlaceOrigin::Local(_) => BorrowRoots::one(BorrowRoot::Local(place.clone())),
            mir::PlaceOrigin::Global(_) => BorrowRoots::one(BorrowRoot::Static),
            mir::PlaceOrigin::Value(value) => self.roots_for_storage(value),
        }
    }

    /// Return borrow roots implied by a value used as storage.
    fn roots_for_storage(&self, value: mir::ValueReference) -> BorrowRoots {
        let Some(value) = value.value() else {
            return BorrowRoots::none();
        };

        // prefer flow roots from projections and propagated values
        if let Some(roots) = self.flow.borrows.get(value) {
            return roots.clone();
        }

        let Some(ty) = self.type_for_value(value) else {
            return BorrowRoots::none();
        };

        let ty = self.tree.get(ty);
        match ty.reference_kind() {
            // keep managed and unique storage alive through the value
            Some(mir::ReferenceKind::Managed | mir::ReferenceKind::Unique) => {
                BorrowRoots::one(BorrowRoot::Local(self.place_for_value(value.into())))
            }
            // preserve explicit lifetime roots for borrowed storage
            Some(mir::ReferenceKind::Borrowed) => {
                let Some(lifetime) = ty.reference_lifetime() else {
                    return BorrowRoots::none();
                };
                if lifetime.is_empty() {
                    return BorrowRoots::none();
                }

                self.roots_from_lifetime(lifetime)
            }
            _ => BorrowRoots::none(),
        }
    }

    /// Return borrow roots from an explicit lifetime or default parameter.
    fn roots_from_lifetime_or_parameter(
        &self,
        lifetime: &mir::Lifetime,
        parameter: u32,
    ) -> BorrowRoots {
        if lifetime.is_empty() {
            return BorrowRoots::one(BorrowRoot::Parameter(parameter));
        }

        self.roots_from_lifetime(lifetime)
    }

    /// Return borrow roots from a MIR lifetime.
    fn roots_from_lifetime(&self, lifetime: &mir::Lifetime) -> BorrowRoots {
        BorrowRoots::new(lifetime.origins.iter().map(|origin| match origin {
            mir::LifetimeOrigin::Static => BorrowRoot::Static,
            mir::LifetimeOrigin::Parameter(index) => BorrowRoot::Parameter(*index),
        }))
    }

    /// Return the explicitly declared return lifetime.
    fn explicit_return_lifetime(&self) -> Option<&mir::Lifetime> {
        self.tree
            .type_reference_lifetime(self.function.return_type)
            .map(|_| &self.function.return_lifetime)
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
        projection: mir::PlaceProjection,
    ) -> mir::Place {
        let mut place = self.place_for_value(base);
        place.push(projection);

        place
    }

    /// Return the moved place for one projected move.
    fn place_moved_by_projection(
        &self,
        base: mir::ValueReference,
        projection: mir::PlaceProjection,
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
