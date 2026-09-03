use std::sync::Arc;

use destack_mir::{
    Access, AliasTable, Block, EscapeTable, Function, FunctionCache, Lifetime, LiveSet,
    LivenessTable, LoanId, LocalNodeId, LocalNodeIdAny, LoopTable, MemoryTable, MovePathId,
    MoveTable, OriginContext, OriginState, OriginTable, Place, PlaceOrigin, PlaceTable, Point,
    Projection, RetentionTable, SafepointTable, Terminator, Tree, Type, TypeId, Value,
};

use destack_artifact::DiagnosticAnchor;
use destack_core::{BitSet, FxIndexSet};

use crate::verify::VerifyState;

/// Borrow checker for one MIR function.
pub(in crate::verify) struct BorrowChecker<'a, 'b> {
    /// The function being verified.
    pub(super) function: &'a Function,
    /// The MIR tree.
    pub(super) tree: &'a Tree,
    /// Module verification state.
    pub(super) verification: &'a mut VerifyState<'b>,
    /// SSA value liveness.
    liveness: Arc<LivenessTable>,
    /// Alias relation for physical memory accesses.
    pub(super) alias: Arc<AliasTable>,
    /// Memory effects for every operation.
    pub(super) memory: Arc<MemoryTable>,
    /// Places derived by address values.
    pub(super) places: Arc<PlaceTable>,
    /// Dense independently movable paths.
    moves: Arc<MoveTable>,
    /// Solved borrow origin.
    pub(super) origin: Arc<OriginTable>,
    /// Whole-function escape decisions.
    pub(super) escape: Arc<EscapeTable>,
    /// Verified ownership retention.
    retention: RetentionTable,
    /// The loops of the function.
    loops: Arc<LoopTable>,
    /// Origin at the current operation.
    pub(super) state: OriginState,
    /// Loans live at the current operation.
    pub(super) active_loans: Vec<LoanId>,
    /// Loans rejected while emitting diagnostics.
    pub(super) rejected_loans: BitSet,
    /// Access errors already reported, one per loan and source anchor.
    reported: FxIndexSet<(LoanId, DiagnosticAnchor)>,
    /// The safepoints of this function.
    pub(super) safepoints: SafepointTable,
}

/// What one verified function hands to elaboration.
pub(in crate::verify) struct BorrowVerdict {
    /// Verified ownership retention.
    pub(in crate::verify) retention: RetentionTable,
    /// The safepoints of every verified function.
    ///
    /// A parking call carries its pins.
    /// A loop header or tail call outside every managed borrow polls.
    pub(in crate::verify) safepoints: SafepointTable,
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
        let origin = analyses.origin(function, tree, &verification.resolution);
        let escape = analyses.escape(function, tree);
        let loops = analyses.loops(function, tree);
        let loan_count = origin.loans().len();

        Self {
            function,
            tree,
            verification,
            liveness,
            alias,
            memory,
            places,
            moves,
            origin,
            escape,
            state: OriginState::new(),
            active_loans: Vec::new(),
            rejected_loans: BitSet::new(loan_count),
            reported: FxIndexSet::default(),
            retention: RetentionTable::default(),
            loops,
            safepoints: SafepointTable::default(),
        }
    }

    /// Return the origin transfer context over this function's analyses.
    pub(super) fn context(&self) -> OriginContext<'_> {
        OriginContext::new(
            self.function,
            self.tree,
            &self.places,
            &self.verification.resolution,
            self.origin.loans(),
        )
    }

    /// Report one loan's access error at most once per anchor.
    pub(super) fn reports(&mut self, loan: LoanId, anchor: LocalNodeIdAny) -> bool {
        let anchor = self.verification.anchor(anchor);
        self.reported.insert((loan, anchor))
    }

    /// Check borrow legality across the function, answering what elaboration needs.
    pub(in crate::verify) fn check(mut self) -> BorrowVerdict {
        // check each reachable block from its fixed origin
        for &block_id in self.function.blocks() {
            let Some(entry) = self.origin.entry(block_id).cloned() else {
                continue;
            };

            self.check_block(block_id, entry);
        }

        BorrowVerdict {
            retention: self.retention,
            safepoints: self.safepoints,
        }
    }

    /// Check one reachable block.
    fn check_block(&mut self, block_id: LocalNodeId<Block>, state: OriginState) {
        self.state = state;
        let block = self.tree.get(block_id);
        let liveness = self.liveness.clone();
        let mut live = liveness.block(self.tree, block_id);

        // retain ownership live at block entry
        let entries = self.retention_entries(&live);
        self.retention.insert_block(block_id, entries);

        // poll the runtime at a loop header outside every managed borrow
        self.activate_loans(&live);
        let is_header = self
            .loops
            .loops()
            .iter()
            .any(|entry| entry.header == block_id);
        if is_header && !self.holds_local_managed_loan() {
            let point = match block.instructions.first() {
                Some(&instruction) => Point::Instruction(instruction),
                None => Point::Terminator(block_id),
            };
            self.safepoints.insert(point, Vec::new());
        }

        // check and transfer each instruction in execution order
        for &instruction_id in &block.instructions {
            let instruction = self.tree.get(instruction_id);

            // activate loans live before the instruction
            self.activate_loans(&live);

            // check ownership and advance origin
            self.check_instruction(instruction_id, instruction);
            let cx = OriginContext::new(
                self.function,
                self.tree,
                &self.places,
                &self.verification.resolution,
                self.origin.loans(),
            );
            self.state.advance(&cx, instruction_id);

            // advance liveness and retain ownership after the instruction
            live.advance(instruction, self.tree);
            let entries = self.retention_entries(&live);
            self.retention.insert_instruction(instruction_id, entries);
        }

        // check the terminating operation
        let terminator = self.tree.get(block.terminator);

        // activate loans live before the terminator
        self.activate_loans(&live);

        // poll the runtime before a tail call outside every managed borrow
        if matches!(terminator, Terminator::TailCall { .. }) && !self.holds_local_managed_loan() {
            self.safepoints
                .insert(Point::Terminator(block_id), Vec::new());
        }

        self.check_terminator(block_id, block.terminator, terminator);
    }

    /// Return live representations and the move-only owners they retain.
    fn retention_entries(&self, live: &LiveSet<'_>) -> Vec<MovePathId> {
        let mut retention = Vec::new();

        // collect owners retained directly by SSA values
        for binding in self.state.bindings() {
            if !live.contains_value(binding.value) {
                continue;
            }

            let representation = PlaceOrigin::Value(binding.value);
            for loan in binding.origin.loans() {
                let Some(owner) = self.retained_owner(*loan, representation) else {
                    continue;
                };
                if !retention.contains(&owner) {
                    retention.push(owner);
                }
            }
        }

        // collect owners retained by addressable places
        for binding in self.state.places() {
            let Some(representation) = live.find_representation(binding.place.origin, &self.places)
            else {
                continue;
            };

            for loan in binding.origin.loans() {
                let Some(owner) = self.retained_owner(*loan, representation) else {
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
        let loan = self.origin.loans().get(loan_id);
        let place = loan.place()?;
        let path = self.moves.containing(place)?;

        Some(self.moves.root(path))
    }

    /// Return an owner retained beyond its direct representation.
    fn retained_owner(&self, loan_id: LoanId, representation: PlaceOrigin) -> Option<MovePathId> {
        let owner = self.owner(loan_id)?;
        let origin = self.moves.get(owner).place.origin;

        (representation != origin).then_some(owner)
    }

    /// Return one structural field type and its applied lifetimes.
    pub(super) fn field_type(&self, aggregate: Value, field: u32) -> (TypeId, Vec<Lifetime>) {
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
    pub(super) fn element_type(&self, aggregate: Value) -> (TypeId, Vec<Lifetime>) {
        let ty = self.function.expect_value_type(aggregate);
        let (ty, lifetimes) = self.tree.split_lifetime_application(ty);
        let Type::FixedArray { element, .. } = self.tree.get(ty) else {
            unreachable!("element.set aggregate has no fixed-array type")
        };

        (*element, lifetimes.to_vec())
    }

    /// Return one aggregate slot type and its applied lifetimes.
    pub(super) fn slot_type(&self, aggregate: Value, index: usize) -> (TypeId, Vec<Lifetime>) {
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

    /// Return one callsite's statically resolved function.
    pub(super) fn resolved_target(
        &self,
        callsite: Point,
        direct: Option<LocalNodeId<Function>>,
    ) -> Option<LocalNodeId<Function>> {
        direct.or_else(|| self.verification.resolution.target(callsite))
    }

    /// Derive loans retained by live values and places.
    fn activate_loans(&mut self, live: &LiveSet<'_>) {
        self.active_loans = self.state.active_loans(
            |value| live.contains_value(value),
            |place| {
                live.find_representation(place.origin, &self.places)
                    .is_some()
            },
        );
        self.active_loans
            .retain(|loan| !self.rejected_loans.contains(loan.index()));
    }

    /// Return whether a value is move-only.
    pub(super) fn is_move_only(&self, value: Value) -> bool {
        let ty = self.function.expect_value_type(value);

        self.tree.get(ty).copy(self.tree).is_no()
    }

    /// Return whether one value has a variant type.
    fn is_variant_value(&self, value: Value) -> bool {
        let ty = self.function.expect_value_type(value);

        matches!(self.tree.get(ty), Type::Variant { .. })
    }

    /// Return access for one reference-like value.
    pub(super) fn reference_access(&self, value: Value) -> Option<Access> {
        let ty = self.function.expect_value_type(value);

        self.tree.get(ty).reference_access()
    }

    /// Return whether one value has an unchecked pointer type.
    pub(super) fn is_pointer(&self, value: Value) -> bool {
        let ty = self.function.expect_value_type(value);

        self.tree.get(ty).is_pointer()
    }

    /// Return the moved place for one projected move.
    pub(super) fn place_moved_by_projection(&self, base: Value, projection: Projection) -> Place {
        let place = self.places.get(base).clone();
        if self.is_variant_value(base) {
            return place;
        }

        place.with_projection(projection)
    }

    /// Exclude one rejected loan from later diagnostics.
    pub(super) fn reject_loan(&mut self, reference: Value) {
        let Some(loan) = self.origin.loans().root(reference) else {
            return;
        };

        self.rejected_loans.insert(loan.index());
    }
}
