use std::borrow::Cow;

use destack_mir::{
    Access, AccessTarget, Block, Call, Instruction, Loan, LoanId, LocalNodeId, LocalNodeIdAny,
    MemoryAccessEffect, MemoryAddress, MemoryRegion, Place, PlaceOrigin, Projection, Reference,
    Storage, Terminator, Value,
};
use smallvec::SmallVec;

use destack_core::BitSet;

use crate::verify::VerifyError;

use super::checker::FunctionChecker;
use super::r#move::Movability;
use super::validate::stored_values;

/// What one access does to the storage it touches, deciding the diagnostic of a conflict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AccessKind {
    /// A new loan of the place.
    Borrow,
    /// A read of the place.
    Read,
    /// A write of the place, which may retag it or release the owned storage below it.
    Write,
    /// A move out of the place, leaving it uninitialized.
    Move,
}

impl FunctionChecker<'_, '_> {
    /// Check one instruction against the loans live before it.
    pub(super) fn check_instruction(
        &mut self,
        instruction_id: LocalNodeId<Instruction>,
        instruction: &Instruction,
    ) {
        let anchor = instruction_id.into_any();

        // check the access each new reference is granted, a cast addressing its argument's referent
        match instruction {
            Instruction::Address {
                destination, place, ..
            } => self.check_address_grant(*destination, place, anchor),
            Instruction::Cast {
                destination,
                argument,
                ..
            } if self.is_dereferenceable(*argument) => {
                let place = Place::value(*argument).with_projection(Projection::Deref);
                self.check_address_grant(*destination, &place, anchor);
            }
            _ => {}
        }
        if let Some(place) = instruction.store_place() {
            self.check_write_grant(place, anchor);
        }

        // check the lifetimes of stored borrows, leaving unchecked pointer stores to their author
        if !self.is_unchecked_store(instruction) {
            let stored = stored_values(self.tree, self.function_id, instruction)
                .unwrap_or_else(|message| unreachable!("a validated instruction has {message}"));
            for (value, destination) in stored {
                self.check_stored_borrows(value, destination, anchor);
            }
        }

        // check the lifetimes of call arguments
        if let Instruction::Call { call, .. } = instruction {
            let arguments = self.tree.get_values(call.arguments);
            self.check_call_outlives(&call.signature, arguments, anchor);
        }

        // check new loans, then every access against the loans live here
        let authorized = match instruction {
            Instruction::Address { destination, .. } | Instruction::Cast { destination, .. } => {
                self.check_issued_loans(*destination, anchor);
                Vec::new()
            }
            Instruction::Call { call, .. } => self.check_call_borrows(call, anchor),
            _ => Vec::new(),
        };
        let effects = self.effects.clone();
        self.check_effects(effects.instruction_effects(instruction_id), anchor);
        self.revoke(&authorized);

        // check each move out of a consumed value, an aggregate, or owned storage
        for value in instruction.consumes(self.tree) {
            if self.is_moved_on_use(value) {
                self.check_move(&Place::value(value), anchor);
            }
        }
        if let Some(place) = self.moved_place(instruction)
            && matches!(self.movability(&place), Movability::Owned { .. })
        {
            self.check_move(&place, anchor);
        }
    }

    /// Check one terminator against the loans live before it.
    pub(super) fn check_terminator(
        &mut self,
        block_id: LocalNodeId<Block>,
        terminator_id: LocalNodeId<Terminator>,
        terminator: &Terminator,
    ) {
        let anchor = terminator_id.into_any();

        // check the lifetimes of returned borrows and of call arguments
        if let Terminator::Return { value: Some(value) } = terminator {
            self.check_return(*value, anchor);
        }
        self.check_terminator_call_outlives(terminator, anchor);
        if let Terminator::TailCall { call } = terminator {
            self.check_tail_call(self.tree.get_values(call.arguments), anchor);
        }
        self.check_tail_call_return(terminator, anchor);

        // check every access against the loans live here
        let authorized = match terminator {
            Terminator::Invoke { call, .. } | Terminator::TailCall { call } => {
                self.check_call_borrows(call, anchor)
            }
            _ => Vec::new(),
        };
        let effects = self.effects.clone();
        self.check_effects(effects.terminator_effects(block_id), anchor);
        self.revoke(&authorized);

        // check each move out of a consumed value
        for value in terminator.consumes(self.tree) {
            if self.is_moved_on_use(value) {
                self.check_move(&Place::value(value), anchor);
            }
        }
    }

    /// Return whether one value is a reference or pointer an address may dereference.
    fn is_dereferenceable(&self, value: Value) -> bool {
        let ty = self.value_type(value);

        ty.reference_kind().is_some() || ty.is_pointer()
    }

    /// Return whether one instruction stores through a raw pointer, which its author vouches for.
    fn is_unchecked_store(&self, instruction: &Instruction) -> bool {
        let Instruction::Store { place, .. } = instruction else {
            return false;
        };

        place
            .reference_type(self.function_id, self.tree)
            .is_some_and(|ty| {
                let reference = self.tree.type_definition(ty);
                reference.is_pointer() || reference.reference_kind() == Some(Reference::Raw)
            })
    }

    /// Check every loan one new reference issues against the loans live here.
    fn check_issued_loans(&mut self, reference: Value, anchor: LocalNodeIdAny) {
        let origin = self.origin.clone();
        for loan_id in origin.loans().roots_of(reference) {
            // reject a reborrow of a rejected loan with it
            let loan = origin.loans().get(loan_id);
            if loan
                .parents()
                .iter()
                .any(|parent| self.rejected_loans.contains(parent.index()))
            {
                self.reject_borrow(reference);
            }
            if self.rejected_loans.contains(loan_id.index()) {
                continue;
            }

            self.check_loan(loan, false, anchor);
        }
    }

    /// Check the borrow each call argument makes, authorizing the loans the arguments hold.
    ///
    /// Return the authorized loans, which the caller revokes once the call's effects are checked.
    fn check_call_borrows(&mut self, call: &Call, anchor: LocalNodeIdAny) -> Vec<LoanId> {
        let Some((_, parameters, _)) = self.tree.get(call.signature).function_signature_parts()
        else {
            unreachable!("a validated call has a function signature")
        };
        let arguments = self.tree.get_values(call.arguments);
        let origin = self.origin.clone();
        let loans = origin.loans();
        let mut borrows: Vec<Cow<'_, Loan>> = Vec::new();

        // borrow each borrowed parameter's argument for the duration of the call
        for (parameter, &argument) in parameters.iter().zip(arguments) {
            let parameter_type = self.tree.get(parameter.ty);
            if parameter_type.reference_kind() != Some(Reference::Borrowed) {
                continue;
            }
            let access = parameter_type
                .reference_access()
                .unwrap_or_else(|| unreachable!("a borrowed parameter has no access"));

            // reuse the loans the argument issued, else borrow the places it may address
            let start = borrows.len();
            borrows.extend(
                loans
                    .roots_of(argument)
                    .map(|loan| loans.get(loan))
                    .filter(|loan| loan.place().is_some())
                    .map(Cow::Borrowed),
            );
            let is_issued = borrows.len() > start;
            if !is_issued {
                let carried = self.state.value(argument);
                for place in self.argument_places(argument) {
                    let loan = Loan::new(
                        place,
                        None,
                        access,
                        argument,
                        carried.loans().iter().copied(),
                        anchor,
                        &self.places,
                    );
                    borrows.push(Cow::Owned(loan));
                }
            }

            // reject a borrow excluding a borrow of an earlier argument
            let (earlier, current) = borrows.split_at(start);
            for borrow in current {
                let is_aliased = earlier.iter().any(|earlier| {
                    earlier.conflicts(
                        borrow,
                        &self.constants,
                        &self.places,
                        self.function_id,
                        self.tree,
                    )
                });
                if is_aliased {
                    self.verification
                        .emit_error(VerifyError::MutableArgumentAlias {
                            anchor: self.anchor(anchor),
                        });
                }

                // check each temporary borrow, which no address instruction checked
                if !is_issued {
                    self.check_loan(borrow, true, anchor);
                }
            }
        }

        // authorize the loans every argument holds, with their ancestry
        let mut authorized = Vec::new();
        for argument in arguments {
            authorized.extend_from_slice(self.state.value(*argument).loans());
        }
        loans.extend_parents(&mut authorized, &mut self.authorized_loans);

        authorized
    }

    /// Revoke the loans one operation authorized.
    fn revoke(&mut self, authorized: &[LoanId]) {
        for loan in authorized {
            self.authorized_loans.remove(loan.index());
        }
    }

    /// Check one new loan against shared storage and the loans live here.
    fn check_loan(&mut self, loan: &Loan, is_temporary: bool, anchor: LocalNodeIdAny) {
        let reference = loan.representation;

        // reject mutable access to shared storage, except a constructor filling it uninitialized
        let is_shared = loan
            .place()
            .and_then(|place| place.storage(self.function_id, self.tree))
            .is_some_and(Storage::is_shared);
        let is_uninit = !is_temporary && self.points_to_uninitialized(reference);
        if loan.writes() && is_shared && !is_uninit {
            self.verification
                .emit_error(VerifyError::MutableBorrowFromSharedStorage {
                    anchor: self.anchor(anchor),
                });
            if !is_temporary {
                self.reject_borrow(reference);
            }

            return;
        }

        // reject a live loan this one excludes
        let conflict = self.origin.loans().conflict(
            loan,
            &self.active_loans,
            &self.constants,
            &self.places,
            self.function_id,
            self.tree,
        );
        if let Some(conflict) = conflict {
            self.report_conflict(AccessKind::Borrow, conflict, anchor);
            if !is_temporary {
                self.reject_borrow(reference);
            }
        }
    }

    /// Check the memory effects of one operation against the loans live here.
    fn check_effects<'e>(
        &mut self,
        effects: impl Iterator<Item = &'e MemoryAccessEffect>,
        anchor: LocalNodeIdAny,
    ) {
        for effect in effects.filter(|effect| effect.reads || effect.writes) {
            let (kind, access) = if effect.writes {
                (AccessKind::Write, Access::Mutable)
            } else {
                (AccessKind::Read, Access::Readonly)
            };

            // select the touched storage and the references traversed to it
            let mut traversed = Vec::new();
            let target = match &effect.region {
                MemoryRegion::Address { location, .. } => match &location.address {
                    MemoryAddress::Place(place) => {
                        self.collect_traversed(place, &mut traversed);
                        AccessTarget::Place(self.places.resolve_place(place))
                    }
                    MemoryAddress::Dynamic { value, .. } => {
                        traversed.extend_from_slice(self.state.value(*value).loans());
                        AccessTarget::Place(self.places.get(*value).clone())
                    }
                },
                MemoryRegion::Local(local) => AccessTarget::Place(Place::local(*local)),
                MemoryRegion::Any { spaces } => AccessTarget::Spaces(*spaces),
                MemoryRegion::Place(_) => {
                    unreachable!("an operation's memory effect has no analysis place region")
                }
            };
            self.origin
                .loans()
                .extend_parents(&mut traversed, &mut self.traversed_loans);

            // leave accesses through a rejected borrow to its own diagnostic
            let is_rejected = traversed
                .iter()
                .any(|loan| self.rejected_loans.contains(loan.index()));
            if !is_rejected {
                self.check_access(&target, kind, access, anchor);
            }
            self.untraverse(&traversed);
        }
    }

    /// Check one move out of a place against the loans live here, except the loans it traverses.
    fn check_move(&mut self, place: &Place, anchor: LocalNodeIdAny) {
        let mut traversed = Vec::new();
        self.collect_traversed(place, &mut traversed);
        self.origin
            .loans()
            .extend_parents(&mut traversed, &mut self.traversed_loans);

        let target = AccessTarget::Place(self.places.resolve_place(place));
        self.check_access(&target, AccessKind::Move, Access::Exclusive, anchor);
        self.untraverse(&traversed);
    }

    /// Forget the loans one access traversed.
    fn untraverse(&mut self, traversed: &[LoanId]) {
        for loan in traversed {
            self.traversed_loans.remove(loan.index());
        }
    }

    /// Check one access against the loans live here, except the authorized and traversed ones.
    fn check_access(
        &mut self,
        target: &AccessTarget,
        kind: AccessKind,
        access: Access,
        anchor: LocalNodeIdAny,
    ) {
        let conflicting = self
            .active_loans
            .iter()
            .copied()
            .filter(|&loan_id| {
                let loan = self.origin.loans().get(loan_id);

                // skip authorized and traversed loans, and loans unexposed to an unrelated access
                !self.authorized_loans.contains(loan_id.index())
                    && !self.traversed_loans.contains(loan_id.index())
                    && (matches!(target, AccessTarget::Place(_)) || self.is_loan_exposed(loan))
                    && loan.forbids(
                        target,
                        access,
                        anchor,
                        &self.constants,
                        &self.places,
                        self.function_id,
                        self.tree,
                    )
            })
            .collect::<SmallVec<[LoanId; 4]>>();
        self.report_innermost(&conflicting, kind, anchor);
    }

    /// Report the innermost reborrow of each broken loan chain.
    fn report_innermost(
        &mut self,
        conflicting: &[LoanId],
        kind: AccessKind,
        anchor: LocalNodeIdAny,
    ) {
        if conflicting.is_empty() {
            return;
        }

        // report only the innermost reborrow of each chain
        let mut ancestors = BitSet::new(self.origin.loans().len());
        for &loan_id in conflicting {
            let mut parents = self.origin.loans().get(loan_id).parents().to_vec();
            self.origin
                .loans()
                .extend_parents(&mut parents, &mut ancestors);
        }
        for &loan_id in conflicting {
            if !ancestors.contains(loan_id.index()) {
                self.report_conflict(kind, loan_id, anchor);
            }
        }
    }

    /// Report one access breaking one loan, once per loan chain and anchor.
    fn report_conflict(&mut self, kind: AccessKind, loan: LoanId, anchor: LocalNodeIdAny) {
        if !self.reports(loan, anchor) {
            return;
        }
        let borrowed_at = self.anchor(self.origin.loans().get(loan).issued_at);
        let anchor = self.anchor(anchor);

        let error = match kind {
            AccessKind::Borrow => VerifyError::BorrowConflict {
                anchor,
                active_borrow: borrowed_at.clone(),
            },
            AccessKind::Read => VerifyError::UseOfExclusivelyBorrowedPlace {
                anchor,
                borrowed_at: borrowed_at.clone(),
            },
            AccessKind::Write | AccessKind::Move => VerifyError::InvalidationOfBorrowedPlace {
                anchor,
                borrowed_at: borrowed_at.clone(),
            },
        };
        self.verification
            .emit_error(error.label(borrowed_at, "borrow starts here"));
    }

    /// Return whether an unrelated call can access the borrowed storage at this point.
    fn is_loan_exposed(&self, loan: &Loan) -> bool {
        // leave an incoming borrow to its caller, whose aliases alone address its referent
        let Some(place) = loan.place() else {
            return false;
        };
        if matches!(place.origin, PlaceOrigin::Global(_))
            || place.may_be_retained(self.function_id, self.tree)
        {
            return true;
        }

        // find loans stored through an alias to this storage or to a unique pointer on its path
        let pointers = place
            .dereferences(self.function_id, self.tree)
            .filter(|(_, reference)| reference.is_unique_storage())
            .map(|(length, _)| Cow::Owned(place.prefix(length)));
        let escaped = self.state.escaped_loans();
        [Cow::Borrowed(place)]
            .into_iter()
            .chain(pointers)
            .any(|storage| {
                self.origin
                    .loans()
                    .blocking_change(&storage, escaped, |left, right| {
                        left.may_overlap(
                            right,
                            &self.constants,
                            &self.places,
                            self.function_id,
                            self.tree,
                        )
                    })
                    .is_some()
            })
    }

    /// Return the places a call argument's loans borrow, else the argument's own place.
    fn argument_places(&self, argument: Value) -> SmallVec<[Place; 2]> {
        let mut places: SmallVec<[Place; 2]> = self
            .state
            .value(argument)
            .loans()
            .iter()
            .filter_map(|loan| self.origin.loans().get(*loan).place().cloned())
            .collect();
        if places.is_empty() {
            places.push(self.places.get(argument).clone());
        }

        places
    }

    /// Collect the loans held by the reference at each dereference of one place.
    fn collect_traversed(&self, place: &Place, loans: &mut Vec<LoanId>) {
        for (length, _) in place.dereferences(self.function_id, self.tree) {
            let source = self.places.resolve_place(&place.prefix(length));
            loans.extend_from_slice(self.state.place(&self.context(), &source).loans());
        }
    }
}
