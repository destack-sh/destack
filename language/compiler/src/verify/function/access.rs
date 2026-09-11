use destack_mir::{
    Access, Block, Instruction, Lifetime, Loan, LoanId, LocalId, LocalNodeId, LocalNodeIdAny,
    MemoryAccessEffect, MemoryRegion, Place, PlaceOrigin, Projection, ReferenceKind, Storage,
    StorageSet, Terminator, Type, TypeId, Value,
};

use destack_core::BitSet;

use crate::verify::VerifyError;

use super::checker::FunctionChecker;

impl FunctionChecker<'_, '_> {
    /// Check one instruction against the current origin.
    pub(super) fn check_instruction(
        &mut self,
        instruction_id: LocalNodeId<Instruction>,
        instruction: &Instruction,
    ) {
        let anchor = instruction_id.into_any();

        // enforce every memory effect against active loans
        self.check_instruction_memory(instruction_id, instruction, anchor);

        // enforce instruction-specific ownership rules
        match instruction {
            Instruction::LocalGet { destination, local } if self.is_move_only(*destination) => {
                self.invalidate_local(*local, anchor);
            }
            // a move-only load out of owned storage moves the addressed place
            Instruction::Load {
                destination,
                pointer,
                ..
            } if self.is_move_only(*destination) && self.addresses_owned_storage(*pointer) => {
                let place = self.places.get(*pointer).clone();
                self.check_invalidation(&place, anchor);
            }
            Instruction::LocalSet { local, value } => {
                self.check_invalidation(&Place::local(*local), anchor);
                let ty = self.tree.get(*local).ty;
                self.check_write(*value, ty, anchor);
            }
            Instruction::Store { pointer, value } => {
                self.check_store(*pointer, *value, anchor);
            }
            Instruction::Aggregate {
                destination,
                values,
            } => {
                for (index, value) in self.tree.get_values(*values).iter().copied().enumerate() {
                    let ty = self.slot_type(*destination, index, value);
                    self.check_write(value, ty, anchor);
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
                self.check_borrow(*destination, anchor);
            }
            Instruction::FieldSet {
                aggregate,
                field,
                value,
                ..
            } => {
                let ty = self.field_type(*aggregate, *field);
                self.check_write(*value, ty, anchor);
            }
            Instruction::ElementSet {
                aggregate, value, ..
            } => {
                let ty = self.element_type(*aggregate);
                self.check_write(*value, ty, anchor);
            }
            Instruction::VariantNew {
                payload: Some(payload),
                case,
                result_type,
                ..
            } => {
                let result_type = self.tree.represented(*result_type);
                let Type::Variant { cases, .. } = self.tree.get(result_type) else {
                    unreachable!("variant.new result has no variant type")
                };
                let ty = cases
                    .get(*case as usize)
                    .map(|case| case.payload(self.tree))
                    .unwrap_or_else(|| unreachable!("variant.new case is out of range"));
                self.check_write(*payload, ty, anchor);
            }
            Instruction::GlobalAddr { destination, .. } => {
                self.check_borrow(*destination, anchor);
            }
            Instruction::Call { call, .. } => {
                let arguments = self.tree.get_values(call.arguments);
                self.check_call_outlives(&call.signature, arguments, anchor);
            }
            _ => {}
        }

        // invalidate loans rooted in consumed values
        self.invalidate_instruction(instruction, anchor);
    }

    /// Check one terminator against the current origin.
    pub(super) fn check_terminator(
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

        // reject frame origin transferred out of a replaced frame
        if let Terminator::TailCall { call } = terminator {
            self.check_tail_call(self.tree.get_values(call.arguments), anchor);
        }

        // enforce terminator memory effects against active loans
        self.check_terminator_memory(block_id, terminator, anchor);

        // check tail-call result retention against the function return lifetime
        self.check_tail_call_return(terminator, anchor);

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

        self.check_borrow(reference, anchor);
    }

    /// Check memory touched by one instruction.
    fn check_instruction_memory(
        &mut self,
        instruction_id: LocalNodeId<Instruction>,
        instruction: &Instruction,
        anchor: LocalNodeIdAny,
    ) {
        // reject writes through readonly safe references
        if let Some(pointer) = instruction.store_pointer() {
            self.check_reference_write(pointer, anchor);
        }

        let effects = self
            .accesses
            .instruction_effects(instruction_id)
            .cloned()
            .collect::<Vec<_>>();
        let mut authorized = BitSet::new(self.origin.loans().len());
        let carried = instruction
            .reads(self.tree)
            .into_iter()
            .flat_map(|value| self.state.value(value).loans().to_vec())
            .collect::<Vec<_>>();
        self.authorize_through(&mut authorized, carried);

        // check temporary parameter loans at calls
        if let Instruction::Call { call, .. } = instruction {
            let arguments = self.tree.get_values(call.arguments);
            for loan in self.check_call_access(&call.signature, arguments, anchor) {
                authorized.insert(loan.index());
            }
        }

        self.check_memory_effects(&effects, &authorized, anchor);
    }

    /// Authorize loans and every loan they reborrow through.
    fn authorize_through(&self, authorized: &mut BitSet, mut pending: Vec<LoanId>) {
        while let Some(loan) = pending.pop() {
            if !authorized.insert(loan.index()) {
                continue;
            }
            pending.extend(self.origin.loans().get(loan).parents().iter().copied());
        }
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
        let mut authorized = BitSet::new(self.origin.loans().len());
        for loan in self.check_call_access(&call.signature, arguments, anchor) {
            authorized.insert(loan.index());
        }

        let effects = self
            .accesses
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
                let loan = self.origin.loans().get(loan_id);

                // check every borrow for assignments and exclusive borrows for opaque effects
                let is_assignment = matches!(
                    effect.region,
                    MemoryRegion::Local(_) | MemoryRegion::Address { .. }
                );
                let is_conflicting = (effect.writes && is_assignment) || self.is_exclusive(loan);
                if !is_conflicting || authorized.contains(loan_id.index()) {
                    continue;
                }

                // skip opaque effects over loans confined to this frame
                if matches!(effect.region, MemoryRegion::Any { .. }) && !self.is_loan_exposed(loan)
                {
                    continue;
                }

                // report each loan once per anchor, over the effects that may touch it
                if !self.effect_may_touch_loan(effect, loan) {
                    continue;
                }
                if !self.reports(loan_id, anchor) {
                    continue;
                }

                let issued_at = self.origin.loans().get(loan_id).issued_at;
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

    /// Return whether an unrelated call can access the borrowed storage at this point.
    fn is_loan_exposed(&self, loan: &Loan) -> bool {
        // admit aliases to incoming borrows and global storage
        let Some(place) = loan.place() else {
            return true;
        };
        if matches!(place.origin, PlaceOrigin::Global(_)) || !self.owns_place(place) {
            return true;
        }

        // find loans already stored through an alias to this owned storage
        self.origin
            .loans()
            .blocking_change(place, self.state.escaped_loans(), |left, right| {
                left.may_overlap(right, &self.constants, self.function, self.tree)
            })
            .is_some()
    }

    /// Return whether one memory effect may touch an active loan.
    fn effect_may_touch_loan(&self, effect: &MemoryAccessEffect, loan: &Loan) -> bool {
        match &effect.region {
            MemoryRegion::Local(local) => loan.place().is_some_and(|place| {
                Place::local(*local).may_overlap(place, &self.constants, self.function, self.tree)
            }),
            MemoryRegion::Address { location, .. } => {
                let place = self.places.get(location.address.value());

                loan.place().is_some_and(|loan| {
                    place.may_overlap(loan, &self.constants, self.function, self.tree)
                })
            }
            MemoryRegion::Place(_) | MemoryRegion::Any { .. } => {
                // compare opaque effects with the reference's permitted memory spaces
                let spaces = self
                    .function
                    .reference_storage(loan.representation, self.tree)
                    .map_or(StorageSet::ANY, Storage::storage_set);

                !effect.region.spaces().is_disjoint(spaces)
            }
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

    /// Check every loan issued by one borrowed reference.
    fn check_borrow(&mut self, reference: Value, anchor: LocalNodeIdAny) {
        let origin = self.origin.clone();
        for loan in origin.loans().roots_of(reference) {
            self.check_loan(loan, anchor);
        }
    }

    /// Check one borrowed reference loan.
    fn check_loan(&mut self, loan_id: LoanId, anchor: LocalNodeIdAny) {
        let loan = self.origin.loans().get(loan_id);
        let reference = loan.representation;

        // derive every rejection before emitting diagnostics
        let storage = loan
            .place()
            .and_then(|place| place.storage(self.function, self.tree));
        let is_rejected = self.rejected_loans.contains(loan_id.index());

        // hold uninitialized storage exclusively while its constructor fills it
        let is_uninit = match self
            .tree
            .get(self.function.expect_value_type(loan.representation))
        {
            Type::Reference { pointee, .. } | Type::Pointer { pointee, .. } => {
                matches!(self.tree.get(*pointee), Type::Uninit { .. })
            }
            _ => false,
        };
        let is_shared_mutable = !is_rejected
            && !is_uninit
            && loan.access.can_write()
            && storage.is_some_and(Storage::is_shared);
        let conflict = if is_rejected || is_shared_mutable {
            None
        } else {
            self.origin.loans().conflict(
                loan,
                &self.active_loans,
                |left, right| left.may_overlap(right, &self.constants, self.function, self.tree),
                |loan| self.is_exclusive(loan),
            )
        };
        let conflict = match conflict {
            Some(conflict) if !self.reports(conflict, anchor) => None,
            other => other.map(|conflict| self.origin.loans().get(conflict).issued_at),
        };

        // reject mutable access to shared storage
        if is_shared_mutable {
            self.verification
                .emit_error(VerifyError::MutableBorrowFromSharedStorage {
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
        let pointer_type = self.tree.represented(pointer_type);
        let Type::Reference { pointee, .. } = self.tree.get(pointer_type) else {
            unreachable!("safe store pointer has no reference type")
        };

        self.check_write(value, *pointee, anchor);
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

        // create a loan per borrowed parameter, active for the duration of the call
        for (parameter, argument) in parameters.iter().zip(arguments.iter().copied()) {
            let parameter_type = self.tree.get(parameter.ty);
            if parameter_type.reference_kind() != Some(ReferenceKind::Borrowed) {
                continue;
            }
            let Some(access) = parameter_type.reference_access() else {
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

            // reuse every loan issued by this argument or create its temporary borrow
            let mut issued = self
                .origin
                .loans()
                .roots_of(argument)
                .map(|loan| self.origin.loans().get(loan))
                .filter(|loan| loan.place().is_some())
                .cloned()
                .collect::<Vec<_>>();
            let is_issued = !issued.is_empty();
            if !is_issued {
                issued.push(Loan::new(
                    self.places.get(argument).clone(),
                    None,
                    access,
                    argument,
                    self.state.value(argument).loans().to_vec(),
                    anchor,
                ));
            }

            // compare each possible referent with earlier arguments
            let previous_count = loans.len();
            for loan in issued {
                // reject mutable access to shared storage
                let storage = loan
                    .place()
                    .and_then(|place| place.storage(self.function, self.tree));
                if !is_issued && loan.writes() && storage.is_some_and(Storage::is_shared) {
                    self.verification
                        .emit_error(VerifyError::MutableBorrowFromSharedStorage {
                            anchor: self.verification.anchor(anchor),
                        });

                    continue;
                }

                // check loans already active before this call
                if !is_issued
                    && let Some(conflict) = self
                        .origin
                        .loans()
                        .conflict(
                            &loan,
                            &self.active_loans,
                            |left, right| {
                                left.may_overlap(right, &self.constants, self.function, self.tree)
                            },
                            |loan| self.is_exclusive(loan),
                        )
                        .map(|conflict| self.origin.loans().get(conflict).issued_at)
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
                if loans[..previous_count].iter().any(|active: &Loan| {
                    (self.is_exclusive(active) || self.is_exclusive(&loan))
                        && active
                            .place()
                            .zip(loan.place())
                            .is_some_and(|(left, right)| {
                                left.may_overlap(right, &self.constants, self.function, self.tree)
                            })
                }) {
                    self.verification
                        .emit_error(VerifyError::MutableArgumentAlias {
                            anchor: self.verification.anchor(anchor),
                        });
                }

                loans.push(loan);
            }
        }

        // authorize the active loans these argument loans already cover
        self.active_loans
            .iter()
            .copied()
            .filter(|active| {
                let active = self.origin.loans().get(*active);

                loans.iter().any(|loan| {
                    active
                        .place()
                        .zip(loan.place())
                        .is_some_and(|(left, right)| {
                            left.may_overlap(right, &self.constants, self.function, self.tree)
                        })
                })
            })
            .collect()
    }

    /// Check one value against its declared destination type.
    fn check_write(&mut self, value: Value, destination: TypeId, anchor: LocalNodeIdAny) {
        let bindings = self.state.value_borrows(&self.context(), value);
        if bindings.is_empty() {
            return;
        }

        let paths = self.tree.type_origin_paths(destination);
        let root = self.tree.type_lifetime(destination);

        // prove every stored borrow against its exact destination path
        for (path, origin) in bindings {
            let required = paths
                .iter()
                .find(|borrowed| borrowed.path == path)
                .map(|borrowed| borrowed.lifetime.clone())
                .or_else(|| path.is_root().then(|| root.clone()).flatten())
                .unwrap_or_else(Lifetime::empty);
            if origin.is_empty() || !origin.is_covered_by(&required, &self.function.lifetimes) {
                self.verification
                    .emit_error(VerifyError::BorrowOutlivesOrigin {
                        anchor: self.verification.anchor(anchor),
                    });

                return;
            }
        }
    }

    /// Check whether changing a place invalidates active loans.
    fn check_invalidation(&mut self, place: &Place, anchor: LocalNodeIdAny) {
        let Some(loan) =
            self.origin
                .loans()
                .blocking_change(place, &self.active_loans, |left, right| {
                    left.may_overlap(right, &self.constants, self.function, self.tree)
                })
        else {
            return;
        };
        if !self.reports(loan, anchor) {
            return;
        }

        // reject changes blocked by active loans
        let borrowed_at = self
            .verification
            .anchor(self.origin.loans().get(loan).issued_at);
        self.verification.emit_error(
            VerifyError::InvalidationOfBorrowedPlace {
                anchor: self.verification.anchor(anchor),
                borrowed_at: borrowed_at.clone(),
            }
            .label(borrowed_at, "borrow starts here"),
        );
    }
}
