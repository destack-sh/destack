use std::sync::Arc;

use destack_core::BitSet;
use destack_mir::{
    Access, AliasTable, Block, CallSite, Function, FunctionBehavior, FunctionCache, Instruction,
    Lifetime, LiveSet, LivenessTable, Loan, LoanId, LocalId, LocalNodeId, LocalNodeIdAny,
    MemoryAccessEffect, MemoryLocation, MemoryRegion, MemoryTable, MovePathId, MoveTable, Path,
    Place, PlaceOrigin, PlaceTable, Projection, Provenance, ProvenanceState, ProvenanceTable,
    ReferenceKind, RetentionTable, Storage, Terminator, Tree, Type, TypeId, Value,
};

use crate::verify::{VerifyError, VerifyState};

/// Borrow checker for one MIR function.
pub(in crate::verify) struct BorrowChecker<'a, 'b> {
    /// The function being verified.
    function: &'a Function,
    /// The MIR tree.
    tree: &'a Tree,
    /// Module verification state.
    verification: &'a mut VerifyState<'b>,
    /// SSA value liveness.
    liveness: Arc<LivenessTable>,
    /// Alias relation for physical memory accesses.
    alias: Arc<AliasTable>,
    /// Memory effects for every operation.
    memory: Arc<MemoryTable>,
    /// Places derived by address values.
    places: Arc<PlaceTable>,
    /// Dense independently movable paths.
    moves: Arc<MoveTable>,
    /// Solved borrow provenance.
    provenance: Arc<ProvenanceTable>,
    /// Provenance at the current operation.
    state: ProvenanceState,
    /// Loans live at the current operation.
    active_loans: Vec<LoanId>,
    /// Loans rejected while emitting diagnostics.
    rejected_loans: BitSet,
    /// Verified ownership retention.
    retention: RetentionTable,
}

impl<'a, 'b> BorrowChecker<'a, 'b> {
    /// Create one function borrow checker.
    pub(in crate::verify) fn new(
        function: &'a Function,
        tree: &'a Tree,
        verification: &'a mut VerifyState<'b>,
        analyses: &mut FunctionCache,
    ) -> Self {
        let liveness = analyses.liveness(function, tree);
        let places = analyses.place(function, tree);
        let moves = analyses.moves(function, tree);
        let alias = analyses.alias(function, tree);
        let memory = analyses.memory(function, tree, verification.accesses, &verification.effects);
        let provenance = analyses.provenance(function, tree, &verification.resolution);
        let loan_count = provenance.loans().len();

        Self {
            function,
            tree,
            verification,
            liveness,
            alias,
            memory,
            places,
            moves,
            provenance,
            state: ProvenanceState::new(),
            active_loans: Vec::new(),
            rejected_loans: BitSet::new(loan_count),
            retention: RetentionTable::default(),
        }
    }

    /// Check borrow legality across the function.
    pub(in crate::verify) fn check(mut self) -> RetentionTable {
        // check each reachable block from its fixed provenance
        for &block_id in self.function.blocks() {
            let Some(entry) = self.provenance.entry(block_id).cloned() else {
                continue;
            };

            self.check_block(block_id, entry);
        }

        self.retention
    }

    /// Check one reachable block.
    fn check_block(&mut self, block_id: LocalNodeId<Block>, state: ProvenanceState) {
        self.state = state;
        let block = self.tree.get(block_id);
        let liveness = self.liveness.clone();
        let mut live = liveness.block(self.tree, block_id);

        // retain ownership live at block entry
        let entries = self.retention_entries(&live);
        self.retention.insert_block(block_id, entries);

        // check and transfer each instruction in execution order
        for &instruction_id in &block.instructions {
            let instruction = self.tree.get(instruction_id);

            // activate loans live before the instruction
            self.activate_loans(&live);

            // check ownership and advance provenance
            self.check_instruction(instruction_id, instruction);
            self.state.advance(
                instruction_id,
                self.function,
                self.tree,
                &self.places,
                &self.verification.resolution,
                self.provenance.loans(),
            );

            // advance liveness and retain ownership after the instruction
            live.advance(instruction, self.tree);
            let entries = self.retention_entries(&live);
            self.retention.insert_instruction(instruction_id, entries);
        }

        // check the terminating operation
        let terminator = self.tree.get(block.terminator);

        // activate loans live before the terminator
        self.activate_loans(&live);
        self.check_terminator(block_id, block.terminator, terminator);
    }

    /// Return live carriers and the move-only owners they retain.
    fn retention_entries(&self, live: &LiveSet<'_>) -> Vec<MovePathId> {
        let mut retention = Vec::new();

        // collect owners retained directly by SSA values
        for binding in self.state.bindings() {
            if !live.contains_value(binding.value) {
                continue;
            }

            let carrier = PlaceOrigin::Value(binding.value);
            for loan in binding.provenance.loans() {
                let Some(owner) = self.retained_owner(*loan, carrier) else {
                    continue;
                };
                if !retention.contains(&owner) {
                    retention.push(owner);
                }
            }
        }

        // collect owners retained by addressable places
        for binding in self.state.places() {
            let Some(carrier) = live.find_carrier(binding.place.origin, &self.places) else {
                continue;
            };

            for loan in binding.provenance.loans() {
                let Some(owner) = self.retained_owner(*loan, carrier) else {
                    continue;
                };
                if !retention.contains(&owner) {
                    retention.push(owner);
                }
            }
        }

        // retain owners escaped through aliasable storage scoped to the frame
        for &loan in self.state.escaped_loans() {
            let Some(owner) = self.owner(loan) else {
                continue;
            };
            if !retention.contains(&owner) {
                retention.push(owner);
            }
        }

        retention
    }

    /// Return the move-only owner retained by one loan.
    fn owner(&self, loan_id: LoanId) -> Option<MovePathId> {
        let loan = self.provenance.loans().get(loan_id);
        let place = loan.place()?;
        let path = self.moves.containing(place)?;

        Some(self.moves.root(path))
    }

    /// Return an owner retained beyond its direct carrier.
    fn retained_owner(&self, loan_id: LoanId, carrier: PlaceOrigin) -> Option<MovePathId> {
        let owner = self.owner(loan_id)?;
        let origin = self.moves.get(owner).place.origin;

        (carrier != origin).then_some(owner)
    }

    /// Check one instruction against the current provenance.
    fn check_instruction(
        &mut self,
        instruction_id: LocalNodeId<Instruction>,
        instruction: &Instruction,
    ) {
        let anchor = instruction_id.into_any();

        // reject managed borrows live across a parking call
        if instruction.call_dispatch().is_some() {
            let callsite = CallSite::Instruction(instruction_id);
            self.check_park(callsite, instruction.call_direct_target(), anchor);
        }

        // enforce every memory effect against active loans
        self.check_instruction_memory(instruction_id, instruction, anchor);

        // enforce instruction-specific ownership rules
        match instruction {
            Instruction::LocalGet { destination, local } if self.is_move_only(*destination) => {
                self.invalidate_local(*local, anchor);
            }
            Instruction::LocalSet { local, value } => {
                self.check_invalidation(&Place::local(*local), anchor);
                let ty = self.tree.get(*local).ty;
                self.check_write(*value, ty, &[], anchor);
            }
            Instruction::Store { pointer, value } => {
                self.check_store(*pointer, *value, anchor);
            }
            Instruction::Aggregate {
                destination,
                values,
            } => {
                for (index, value) in self.tree.get_values(*values).iter().copied().enumerate() {
                    let (ty, lifetimes) = self.slot_type(*destination, index);
                    self.check_write(value, ty, &lifetimes, anchor);
                }
            }
            Instruction::FieldAddr {
                destination,
                aggregate,
                ..
            } => {
                self.check_projection_loan(*destination, *aggregate, anchor);
            }
            Instruction::ElementAddr {
                destination, base, ..
            } => {
                self.check_projection_loan(*destination, *base, anchor);
            }
            Instruction::VariantPayloadAddr {
                destination,
                variant,
                ..
            } => {
                self.check_projection_loan(*destination, *variant, anchor);
            }
            Instruction::SliceView {
                destination,
                source,
                ..
            } => {
                self.check_projection_loan(*destination, *source, anchor);
            }
            Instruction::FieldGet {
                destination,
                aggregate,
                field,
            } => {
                let projection = Projection::Field { index: *field };
                self.invalidate_projection(*destination, *aggregate, projection, anchor);
            }
            Instruction::ElementGet {
                destination,
                aggregate,
                index,
            } => {
                let projection = Projection::Element { index: *index };
                self.invalidate_projection(*destination, *aggregate, projection, anchor);
            }
            Instruction::LocalAddr { destination, .. } => {
                self.check_loan(*destination, anchor);
            }
            Instruction::FieldSet {
                aggregate,
                field,
                value,
                ..
            } => {
                let (ty, lifetimes) = self.field_type(*aggregate, *field);
                self.check_write(*value, ty, &lifetimes, anchor);
            }
            Instruction::ElementSet {
                aggregate, value, ..
            } => {
                let (ty, lifetimes) = self.element_type(*aggregate);
                self.check_write(*value, ty, &lifetimes, anchor);
            }
            Instruction::VariantNew {
                payload: Some(payload),
                case,
                result_type,
                ..
            } => {
                let (result_type, lifetimes) = self.tree.split_lifetime_application(*result_type);
                let lifetimes = lifetimes.to_vec();
                let Type::Variant { cases, .. } = self.tree.get(result_type) else {
                    unreachable!("variant.new result has no variant type")
                };
                let ty = cases
                    .get(*case as usize)
                    .map(|case| case.ty)
                    .unwrap_or_else(|| unreachable!("variant.new case is out of range"));
                self.check_write(*payload, ty, &lifetimes, anchor);
            }
            Instruction::GlobalAddr { destination, .. } => {
                self.check_loan(*destination, anchor);
            }
            Instruction::Call { call, .. } => {
                let arguments = self.tree.get_values(call.arguments);
                self.check_call_outlives(&call.signature, arguments, anchor);
            }
            Instruction::TensorView {
                destination, view, ..
            } => {
                self.check_projection_loan(*destination, *view, anchor);
            }
            _ => {}
        }

        // invalidate loans rooted in consumed values
        self.invalidate_instruction(instruction, anchor);
    }

    /// Check one terminator against the current provenance.
    fn check_terminator(
        &mut self,
        block_id: LocalNodeId<Block>,
        terminator_id: LocalNodeId<Terminator>,
        terminator: &Terminator,
    ) {
        let anchor = terminator_id.into_any();

        // check returned retention against the function return lifetime
        if let Terminator::Return { value: Some(value) } = terminator {
            self.check_return(*value, anchor);
        }

        // check declared call lifetime relations
        self.check_terminator_call_outlives(terminator, anchor);

        // reject frame provenance transferred out of a replaced frame
        if let Terminator::TailCall { call } = terminator {
            self.check_tail_call(self.tree.get_values(call.arguments), anchor);
        }

        // enforce terminator memory effects against active loans
        self.check_terminator_memory(block_id, terminator, anchor);

        // reject managed borrows live across a parking call
        if terminator.call_dispatch().is_some() {
            let callsite = CallSite::Terminator(block_id);
            self.check_park(callsite, terminator.call_direct_target(), anchor);
        }

        // check tail-call result retention against the function return lifetime
        self.check_tail_call_return(block_id, terminator, anchor);

        // invalidate loans rooted in consumed terminator values
        for value in terminator.consumes(self.tree) {
            self.invalidate_value(value, anchor);
        }
    }

    /// Check loans invalidated by consuming one move-only value.
    fn invalidate_value(&mut self, value: Value, anchor: LocalNodeIdAny) {
        if !self.is_move_only(value) {
            return;
        }

        let place = self.places.get(value).clone();
        self.check_invalidation(&place, anchor);
    }

    /// Check loans invalidated by one instruction.
    fn invalidate_instruction(&mut self, instruction: &Instruction, anchor: LocalNodeIdAny) {
        for value in instruction.consumes(self.tree) {
            self.invalidate_value(value, anchor);
        }
    }

    /// Check loans invalidated by consuming one local.
    fn invalidate_local(&mut self, local: LocalId, anchor: LocalNodeIdAny) {
        let place = Place::local(local);
        self.check_invalidation(&place, anchor);
    }

    /// Check loans invalidated by consuming one projected place.
    fn invalidate_projection(
        &mut self,
        value: Value,
        base: Value,
        projection: Projection,
        anchor: LocalNodeIdAny,
    ) {
        if !self.is_move_only(value) {
            return;
        }

        // invalidate the complete variant or selected aggregate child
        let place = self.place_moved_by_projection(base, projection);
        self.check_invalidation(&place, anchor);
    }

    /// Check one projected loan.
    fn check_projection_loan(&mut self, reference: Value, root: Value, anchor: LocalNodeIdAny) {
        // propagate a rejected parent loan through the projection
        let is_parent_rejected = self
            .state
            .value(root)
            .loans()
            .iter()
            .any(|loan| self.rejected_loans.contains(loan.index()));
        if is_parent_rejected {
            self.reject_loan(reference);
        }

        // preserve readonly access through every safe projection
        let is_writable = self
            .reference_access(reference)
            .is_some_and(Access::can_write);
        let is_readonly_root = self
            .reference_access(root)
            .is_some_and(|access| !access.can_write());
        if is_writable && is_readonly_root {
            self.verification
                .emit_error(VerifyError::BorrowThroughReadonlyReference {
                    anchor: self.verification.anchor(anchor),
                });
            self.reject_loan(reference);
        }

        self.check_loan(reference, anchor);
    }

    /// Check memory touched by one instruction.
    fn check_instruction_memory(
        &mut self,
        instruction_id: LocalNodeId<Instruction>,
        instruction: &Instruction,
        anchor: LocalNodeIdAny,
    ) {
        // reject writes through readonly safe references
        if let Some(pointer) = Self::written_reference(instruction) {
            self.check_reference_write(pointer, anchor);
        }

        let effects = self
            .memory
            .instruction_effects(instruction_id)
            .cloned()
            .collect::<Vec<_>>();
        let mut authorized = BitSet::new(self.provenance.loans().len());
        for loan in instruction
            .reads(self.tree)
            .into_iter()
            .flat_map(|value| self.state.value(value).loans().to_vec())
        {
            authorized.insert(loan.index());
        }

        // check temporary parameter loans at calls
        if let Instruction::Call { call, .. } = instruction {
            let arguments = self.tree.get_values(call.arguments);
            for loan in self.check_call_access(&call.signature, arguments, anchor) {
                authorized.insert(loan.index());
            }
        }

        self.check_memory_effects(&effects, &authorized, anchor);
    }

    /// Check memory touched by one call terminator.
    fn check_terminator_memory(
        &mut self,
        block: LocalNodeId<Block>,
        terminator: &Terminator,
        anchor: LocalNodeIdAny,
    ) {
        let (Terminator::Invoke { call, .. } | Terminator::TailCall { call }) = terminator else {
            return;
        };
        let arguments = self.tree.get_values(call.arguments);
        let mut authorized = BitSet::new(self.provenance.loans().len());
        for loan in self.check_call_access(&call.signature, arguments, anchor) {
            authorized.insert(loan.index());
        }

        let effects = self
            .memory
            .terminator_effects(block)
            .cloned()
            .collect::<Vec<_>>();
        for loan in arguments
            .iter()
            .copied()
            .flat_map(|value| self.state.value(value).loans().to_vec())
        {
            authorized.insert(loan.index());
        }

        self.check_memory_effects(&effects, &authorized, anchor);
    }

    /// Check memory effects against active exclusive loans.
    fn check_memory_effects(
        &mut self,
        effects: &[MemoryAccessEffect],
        authorized: &BitSet,
        anchor: LocalNodeIdAny,
    ) {
        for effect in effects {
            for index in 0..self.active_loans.len() {
                let loan_id = self.active_loans[index];
                let loan = self.provenance.loans().get(loan_id);
                if !loan.is_exclusive() || authorized.contains(loan_id.index()) {
                    continue;
                }

                if !self.effect_may_touch_loan(effect, loan) {
                    continue;
                }

                let issued_at = loan.issued_at;
                let borrowed_at = self.verification.anchor(issued_at);
                if effect.writes {
                    self.verification.emit_error(
                        VerifyError::InvalidationOfBorrowedPlace {
                            anchor: self.verification.anchor(anchor),
                            borrowed_at: borrowed_at.clone(),
                        }
                        .label(borrowed_at, "borrow starts here"),
                    );
                } else if effect.reads {
                    self.verification.emit_error(
                        VerifyError::UseOfExclusivelyBorrowedPlace {
                            anchor: self.verification.anchor(anchor),
                            borrowed_at: borrowed_at.clone(),
                        }
                        .label(borrowed_at, "borrow starts here"),
                    );
                }
            }
        }
    }

    /// Return whether one memory effect may touch an active loan.
    fn effect_may_touch_loan(&self, effect: &MemoryAccessEffect, loan: &Loan) -> bool {
        match &effect.region {
            MemoryRegion::Local(local) => loan
                .place()
                .is_some_and(|place| self.alias.may_overlap(&Place::local(*local), place)),
            MemoryRegion::Address { location, .. } => {
                let place = self.places.get(location.address).clone();

                loan.place()
                    .is_some_and(|loan| self.alias.may_overlap(&place, loan))
            }
            MemoryRegion::Place(_) | MemoryRegion::Any { .. } => {
                let location = MemoryLocation::from_address(loan.carrier);

                effect.may_touch_location(&location, &self.alias)
            }
        }
    }

    /// Return the safe reference written by one instruction.
    fn written_reference(instruction: &Instruction) -> Option<Value> {
        match instruction {
            Instruction::Store { pointer, .. }
            | Instruction::TensorStore { view: pointer, .. }
            | Instruction::TensorFill { view: pointer, .. }
            | Instruction::AtomicStore { pointer, .. }
            | Instruction::AtomicCompareExchange { pointer, .. }
            | Instruction::AtomicRmw { pointer, .. } => Some(*pointer),
            Instruction::TensorCopy { target, .. } => Some(*target),
            _ => None,
        }
    }

    /// Check that one safe reference permits writes.
    fn check_reference_write(&mut self, pointer: Value, anchor: LocalNodeIdAny) {
        let is_writable = self
            .reference_access(pointer)
            .is_some_and(Access::can_write);
        if self.is_pointer(pointer) || is_writable {
            return;
        }

        self.verification
            .emit_error(VerifyError::WriteThroughReadonlyReference {
                anchor: self.verification.anchor(anchor),
            });
    }

    /// Check one borrowed reference loan.
    fn check_loan(&mut self, reference: Value, anchor: LocalNodeIdAny) {
        let Some(loan_id) = self.provenance.loans().root(reference) else {
            return;
        };
        let loan = self.provenance.loans().get(loan_id);

        // derive every rejection before emitting diagnostics
        let storage = loan.place().and_then(|place| self.alias.storage(place));
        let is_rejected = self.rejected_loans.contains(loan_id.index());
        let is_shared_exclusive =
            !is_rejected && loan.access.is_exclusive() && storage.is_some_and(Storage::is_shared);
        let conflict = if is_rejected || is_shared_exclusive {
            None
        } else {
            self.provenance
                .loans()
                .conflict(loan, &self.active_loans, |left, right| {
                    self.alias.may_overlap(left, right)
                })
                .map(|conflict| self.provenance.loans().get(conflict).issued_at)
        };

        // reject exclusive access to shared storage
        if is_shared_exclusive {
            self.verification
                .emit_error(VerifyError::ExclusiveBorrowFromSharedStorage {
                    anchor: self.verification.anchor(anchor),
                });
            self.reject_loan(reference);
        }

        // reject overlap with a loan already active here
        if let Some(conflict) = conflict {
            let active_borrow = self.verification.anchor(conflict);
            self.verification.emit_error(
                VerifyError::BorrowConflict {
                    anchor: self.verification.anchor(anchor),
                    active_borrow: active_borrow.clone(),
                }
                .label(active_borrow, "borrow starts here"),
            );

            self.reject_loan(reference);
        }
    }

    /// Check one value stored through a pointer.
    fn check_store(&mut self, pointer: Value, value: Value, anchor: LocalNodeIdAny) {
        if self.is_pointer(pointer) {
            return;
        }

        let pointer_type = self.function.expect_value_type(pointer);
        let (pointer_type, lifetimes) = self.tree.split_lifetime_application(pointer_type);
        let lifetimes = lifetimes.to_vec();
        let Type::Reference { pointee, .. } = self.tree.get(pointer_type) else {
            unreachable!("safe store pointer has no reference type")
        };

        self.check_write(value, *pointee, &lifetimes, anchor);
    }

    /// Check reference access granted to call arguments.
    fn check_call_access(
        &mut self,
        signature: &TypeId,
        arguments: &[Value],
        anchor: LocalNodeIdAny,
    ) -> Vec<LoanId> {
        let Some((_, parameters, _)) = self.tree.get(*signature).function_signature_parts() else {
            unreachable!("call has no function signature")
        };
        let mut loans = Vec::new();

        // create loans that remain active for the duration of the call
        for (parameter, argument) in parameters.iter().zip(arguments.iter().copied()) {
            let Some(access) = self.tree.get(parameter.ty).reference_access() else {
                continue;
            };
            let Some(argument_access) = self.reference_access(argument) else {
                continue;
            };

            // prevent calls from strengthening readonly references
            if access.can_write() && !argument_access.can_write() {
                self.verification
                    .emit_error(VerifyError::BorrowThroughReadonlyReference {
                        anchor: self.verification.anchor(anchor),
                    });
                continue;
            }

            let loan = Loan::new(
                self.places.get(argument).clone(),
                None,
                access,
                argument,
                self.state.value(argument).loans().to_vec(),
                anchor,
            );

            // reject exclusive access to shared storage
            let storage = loan.place().and_then(|place| self.alias.storage(place));
            if loan.is_exclusive() && storage.is_some_and(Storage::is_shared) {
                self.verification
                    .emit_error(VerifyError::ExclusiveBorrowFromSharedStorage {
                        anchor: self.verification.anchor(anchor),
                    });
                continue;
            }

            // check loans already active before this call
            if let Some(conflict) = self
                .provenance
                .loans()
                .conflict(&loan, &self.active_loans, |left, right| {
                    self.alias.may_overlap(left, right)
                })
                .map(|conflict| self.provenance.loans().get(conflict).issued_at)
            {
                let active_borrow = self.verification.anchor(conflict);
                self.verification.emit_error(
                    VerifyError::BorrowConflict {
                        anchor: self.verification.anchor(anchor),
                        active_borrow: active_borrow.clone(),
                    }
                    .label(active_borrow, "borrow starts here"),
                );
            }

            // check arguments whose temporary loans overlap each other
            if loans.iter().any(|active: &Loan| {
                (active.is_exclusive() || loan.is_exclusive())
                    && active
                        .place()
                        .zip(loan.place())
                        .is_some_and(|(left, right)| self.alias.may_overlap(left, right))
            }) {
                self.verification
                    .emit_error(VerifyError::ExclusiveArgumentAlias {
                        anchor: self.verification.anchor(anchor),
                    });
            }

            loans.push(loan);
        }

        // check memory through call arguments once
        self.active_loans
            .iter()
            .copied()
            .filter(|active| {
                let active = self.provenance.loans().get(*active);

                loans.iter().any(|loan| {
                    active
                        .place()
                        .zip(loan.place())
                        .is_some_and(|(left, right)| self.alias.may_overlap(left, right))
                })
            })
            .collect()
    }

    /// Check one value against its declared destination type.
    fn check_write(
        &mut self,
        value: Value,
        destination: TypeId,
        lifetimes: &[Lifetime],
        anchor: LocalNodeIdAny,
    ) {
        let bindings = self.state.value_bindings(value, self.function, self.tree);
        if bindings.is_empty() {
            return;
        }

        let paths = self
            .tree
            .type_borrowed_paths_with_lifetimes(destination, lifetimes, true);
        let root = self
            .tree
            .type_lifetime_with_lifetimes(destination, lifetimes);

        // prove every stored borrow against its exact destination path
        for (path, provenance) in bindings {
            let required = paths
                .iter()
                .find(|borrowed| borrowed.path == path)
                .map(|borrowed| borrowed.lifetime.clone())
                .or_else(|| path.is_root().then(|| root.clone()).flatten())
                .filter(|lifetime| !lifetime.is_empty())
                .unwrap_or_else(Lifetime::frame);
            if provenance.is_empty()
                || !provenance.is_covered_by(&required, &self.function.lifetimes)
            {
                self.verification
                    .emit_error(VerifyError::BorrowOutlivesOrigin {
                        anchor: self.verification.anchor(anchor),
                    });

                return;
            }
        }
    }

    /// Return one structural field type and its applied lifetimes.
    fn field_type(&self, aggregate: Value, field: u32) -> (TypeId, Vec<Lifetime>) {
        let ty = self.function.expect_value_type(aggregate);
        let (ty, lifetimes) = self.tree.split_lifetime_application(ty);
        let field = self
            .tree
            .get(ty)
            .field_type(field, self.tree)
            .unwrap_or_else(|| unreachable!("aggregate has no field {field}"));

        (field, lifetimes.to_vec())
    }

    /// Return one fixed-array element type and its applied lifetimes.
    fn element_type(&self, aggregate: Value) -> (TypeId, Vec<Lifetime>) {
        let ty = self.function.expect_value_type(aggregate);
        let (ty, lifetimes) = self.tree.split_lifetime_application(ty);
        let Type::FixedArray { element, .. } = self.tree.get(ty) else {
            unreachable!("element.set aggregate has no fixed-array type")
        };

        (*element, lifetimes.to_vec())
    }

    /// Return one aggregate slot type and its applied lifetimes.
    fn slot_type(&self, aggregate: Value, index: usize) -> (TypeId, Vec<Lifetime>) {
        let ty = self.function.expect_value_type(aggregate);
        let (ty, lifetimes) = self.tree.split_lifetime_application(ty);
        let aggregate = self.tree.get(ty);
        let slot = match aggregate {
            Type::Struct { fields, .. } => fields.get(index).map(|field| self.tree.get(*field).ty),
            Type::Tuple { elements, .. } => elements.get(index).copied(),
            Type::FixedArray { element, .. } => Some(*element),
            Type::Newtype { inner, .. } if index == 0 => Some(*inner),
            _ => None,
        }
        .unwrap_or_else(|| unreachable!("aggregate has no slot {index}"));

        (slot, lifetimes.to_vec())
    }

    /// Check one returned value.
    fn check_return(&mut self, value: Value, anchor: LocalNodeIdAny) {
        if !self
            .tree
            .type_contains_borrowed_refs(self.function.return_type)
        {
            return;
        }

        let bindings = self.state.value_bindings(value, self.function, self.tree);

        self.check_return_bindings(bindings, anchor);
    }

    /// Check tail-call result retention against the function return lifetime.
    fn check_tail_call_return(
        &mut self,
        block_id: LocalNodeId<Block>,
        terminator: &Terminator,
        anchor: LocalNodeIdAny,
    ) {
        let Terminator::TailCall { call } = terminator else {
            return;
        };
        let callsite = CallSite::Terminator(block_id);
        let function = self.resolved_target(callsite, terminator.call_direct_target());
        let arguments = self.tree.get_values(call.arguments);
        let bindings = self
            .state
            .call_result(function, call.signature, arguments, self.tree);

        self.check_return_bindings(bindings, anchor);
    }

    /// Check returned borrow bindings against declared return lifetimes.
    fn check_return_bindings(&mut self, bindings: Vec<(Path, Provenance)>, anchor: LocalNodeIdAny) {
        let return_type = self.function.return_type;
        let return_paths = self.tree.type_borrowed_paths(return_type);
        let return_lifetime = self.tree.type_lifetime(return_type);

        for (path, provenance) in bindings {
            // require proven provenance outside the current frame
            let is_unproven = !provenance.has_region() || provenance.has_local_region();

            // check the matching explicit return lifetime when one exists
            let required = return_paths
                .iter()
                .find(|borrowed| borrowed.path == path)
                .map(|borrowed| borrowed.lifetime.clone());
            let required = if required.is_none() && path.is_root() {
                return_lifetime.clone()
            } else {
                required
            };
            let is_uncovered = match required {
                Some(required) => !provenance.is_covered_by(&required, &self.function.lifetimes),
                None => true,
            };

            if is_unproven || is_uncovered {
                self.verification
                    .emit_error(VerifyError::BorrowOutlivesOrigin {
                        anchor: self.verification.anchor(anchor),
                    });

                return;
            }
        }
    }

    /// Check declared lifetime relations for one terminator call.
    fn check_terminator_call_outlives(&mut self, terminator: &Terminator, anchor: LocalNodeIdAny) {
        match terminator {
            Terminator::Invoke { call, .. } | Terminator::TailCall { call } => {
                self.check_call_outlives(
                    &call.signature,
                    self.tree.get_values(call.arguments),
                    anchor,
                );
            }
            _ => {}
        }
    }

    /// Check managed loans live across one parking call.
    fn check_park(
        &mut self,
        callsite: CallSite,
        direct: Option<LocalNodeId<Function>>,
        anchor: LocalNodeIdAny,
    ) {
        let target = self.resolved_target(callsite, direct);
        if !self.call_may_park(callsite, target) {
            return;
        }

        // select managed loans already live at the call
        let loans = self
            .active_loans
            .iter()
            .copied()
            .filter(|loan| self.loan_is_managed(*loan))
            .collect::<Vec<_>>();

        self.emit_park_errors(&loans, anchor);
    }

    /// Return whether one call may park the current fiber.
    fn call_may_park(&self, callsite: CallSite, target: Option<LocalNodeId<Function>>) -> bool {
        let call = self.verification.effects.call(callsite);
        if let Some(call) = call
            && call.behavior != FunctionBehavior::unknown()
        {
            return call.behavior.park.may_park();
        }

        if let Some(effect) = target.and_then(|target| self.verification.effects.function(target)) {
            effect.behavior.park.may_park()
        } else {
            true
        }
    }

    /// Return whether one loan borrows managed storage.
    fn loan_is_managed(&self, loan: LoanId) -> bool {
        let loan = self.provenance.loans().get(loan);
        let Some(place) = loan.place() else {
            return false;
        };
        let PlaceOrigin::Value(value) = place.origin else {
            return false;
        };
        let ty = self.function.expect_value_type(value);

        self.tree.get(ty).reference_kind() == Some(ReferenceKind::Managed)
    }

    /// Emit managed borrow errors for one parking call.
    fn emit_park_errors(&mut self, loans: &[LoanId], anchor: LocalNodeIdAny) {
        let borrowed_at = loans
            .iter()
            .map(|loan| self.provenance.loans().get(*loan))
            .map(|loan| loan.issued_at)
            .collect::<Vec<_>>();

        for borrowed_at in borrowed_at {
            let borrowed_at = self.verification.anchor(borrowed_at);
            self.verification.emit_error(
                VerifyError::ManagedBorrowAcrossPark {
                    anchor: self.verification.anchor(anchor),
                    borrowed_at: borrowed_at.clone(),
                }
                .label(borrowed_at, "borrow starts here"),
            );
        }
    }

    /// Return one callsite's statically resolved function.
    fn resolved_target(
        &self,
        callsite: CallSite,
        direct: Option<LocalNodeId<Function>>,
    ) -> Option<LocalNodeId<Function>> {
        direct.or_else(|| self.verification.resolution.target(callsite))
    }

    /// Check declared lifetime relations against call arguments.
    fn check_call_outlives(
        &mut self,
        signature: &TypeId,
        arguments: &[Value],
        anchor: LocalNodeIdAny,
    ) {
        let Some((lifetimes, parameters, _)) = self.tree.get(*signature).function_signature_parts()
        else {
            unreachable!("call has no function signature")
        };

        // prove every declared outlives bound from the actual argument provenance
        for (slot, parameter) in lifetimes.iter().enumerate() {
            for target in &parameter.outlives {
                let longer = self.state.map_lifetime(
                    &Lifetime::slot(slot as u32),
                    lifetimes,
                    parameters,
                    arguments,
                    self.tree,
                );
                let shorter = self.state.map_lifetime(
                    &Lifetime::slot(target.0),
                    lifetimes,
                    parameters,
                    arguments,
                    self.tree,
                );

                // defer result-only slots to result mapping
                if shorter.is_empty() {
                    continue;
                }
                if !longer.outlives(&shorter, &self.function.lifetimes) {
                    self.verification
                        .emit_error(VerifyError::BorrowOutlivesOrigin {
                            anchor: self.verification.anchor(anchor),
                        });
                }
            }
        }
    }

    /// Check values transferred from a replaced caller frame.
    fn check_tail_call(&mut self, arguments: &[Value], anchor: LocalNodeIdAny) {
        for argument in arguments.iter().copied() {
            let bindings = self
                .state
                .value_bindings(argument, self.function, self.tree);
            let escaping = bindings
                .iter()
                .filter(|(_, provenance)| provenance.has_local_region())
                .collect::<Vec<_>>();
            if escaping.is_empty() {
                continue;
            }

            self.verification
                .emit_error(VerifyError::BorrowOutlivesOrigin {
                    anchor: self.verification.anchor(anchor),
                });
            for (_, provenance) in escaping {
                for loan in provenance.loans() {
                    self.rejected_loans.insert(loan.index());
                }
            }
        }
    }

    /// Check whether changing a place invalidates active loans.
    fn check_invalidation(&mut self, place: &Place, anchor: LocalNodeIdAny) {
        let Some(loan) = self
            .provenance
            .loans()
            .blocking_change(place, &self.active_loans, |left, right| {
                self.alias.may_overlap(left, right)
            })
            .map(|loan| self.provenance.loans().get(loan).issued_at)
        else {
            return;
        };

        // reject changes blocked by active loans
        let borrowed_at = self.verification.anchor(loan);
        self.verification.emit_error(
            VerifyError::InvalidationOfBorrowedPlace {
                anchor: self.verification.anchor(anchor),
                borrowed_at: borrowed_at.clone(),
            }
            .label(borrowed_at, "borrow starts here"),
        );
    }

    /// Derive loans retained by live values and places.
    fn activate_loans(&mut self, live: &LiveSet<'_>) {
        self.active_loans = self.state.active_loans(
            |value| live.contains_value(value),
            |place| live.find_carrier(place.origin, &self.places).is_some(),
        );
        self.active_loans
            .retain(|loan| !self.rejected_loans.contains(loan.index()));
    }

    /// Return whether a value is move-only.
    fn is_move_only(&self, value: Value) -> bool {
        let ty = self.function.expect_value_type(value);

        self.tree.get(ty).copy(self.tree).is_no()
    }

    /// Return whether one value has a variant type.
    fn is_variant_value(&self, value: Value) -> bool {
        let ty = self.function.expect_value_type(value);

        matches!(self.tree.get(ty), Type::Variant { .. })
    }

    /// Return access for one reference-like value.
    fn reference_access(&self, value: Value) -> Option<Access> {
        let ty = self.function.expect_value_type(value);

        self.tree.get(ty).reference_access()
    }

    /// Return whether one value has an unchecked pointer type.
    fn is_pointer(&self, value: Value) -> bool {
        let ty = self.function.expect_value_type(value);

        self.tree.get(ty).is_pointer()
    }

    /// Return the moved place for one projected move.
    fn place_moved_by_projection(&self, base: Value, projection: Projection) -> Place {
        let place = self.places.get(base).clone();
        if self.is_variant_value(base) {
            return place;
        }

        place.with_projection(projection)
    }

    /// Exclude one rejected loan from later diagnostics.
    fn reject_loan(&mut self, reference: Value) {
        let Some(loan) = self.provenance.loans().root(reference) else {
            return;
        };

        self.rejected_loans.insert(loan.index());
    }
}
