use std::sync::Arc;

use destack_mir::{
    Access, AliasTable, Block, Copy, EscapeTable, Function, FunctionCache, InitializationTable,
    Lifetime, LiveSet, LivenessTable, LoanId, LocalNodeId, LocalNodeIdAny, LoopTable, MemoryTable,
    MovePathId, MoveTable, OriginContext, OriginState, OriginTable, Place, PlaceOrigin, PlaceTable,
    Point, Projection, RetentionTable, SafepointKind, SafepointTable, Terminator, Tree, Type,
    TypeId, Value,
};

use destack_artifact::DiagnosticAnchor;
use destack_core::{BitSet, FxIndexSet};

use crate::verify::VerifyState;

/// Checker for one MIR function: its moves, its constructor initialization, and its borrows.
pub(in crate::verify) struct FunctionChecker<'a, 'b> {
    /// The function being verified.
    pub(super) function: &'a Function,
    /// The MIR tree.
    pub(super) tree: &'a Tree,
    /// Module verification state.
    pub(super) verification: &'a mut VerifyState<'b>,
    /// Move-path initialization.
    pub(super) initialization: Arc<InitializationTable>,
    /// SSA value liveness.
    liveness: Arc<LivenessTable>,
    /// Alias relation for physical memory accesses.
    pub(super) alias: Arc<AliasTable>,
    /// Memory effects for every operation.
    pub(super) memory: Arc<MemoryTable>,
    /// Places derived by address values.
    pub(super) places: Arc<PlaceTable>,
    /// Dense independently movable paths, owned pointees included.
    pub(super) moves: Arc<MoveTable>,
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
pub(in crate::verify) struct FunctionVerdict {
    /// Verified ownership retention.
    pub(in crate::verify) retention: RetentionTable,
    /// The safepoints of every verified function.
    ///
    /// A parking call holds its live handles past the call.
    /// A loop header or tail call polls and holds its live handles past the poll.
    pub(in crate::verify) safepoints: SafepointTable,
}

impl<'a, 'b> FunctionChecker<'a, 'b> {
    /// Create one function checker over the function's analyses.
    pub(in crate::verify) fn new(
        function: &'a Function,
        tree: &'a Tree,
        verification: &'a mut VerifyState<'b>,
        analyses: &mut FunctionCache,
    ) -> Self {
        let initialization = analyses.initialization(function, tree);
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
            initialization,
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

    /// Check the function, answering what elaboration needs: moves first, so a moved value
    /// reports before the borrows it breaks, then the constructor's initialization, then borrows.
    pub(in crate::verify) fn check(mut self) -> FunctionVerdict {
        self.check_moves();
        self.check_initialization();
        self.check_borrows();

        FunctionVerdict {
            retention: self.retention,
            safepoints: self.safepoints,
        }
    }

    /// Check borrow legality across the function.
    fn check_borrows(&mut self) {
        // check each reachable block from its fixed origin
        for &block_id in self.function.blocks() {
            let Some(entry) = self.origin.entry(block_id).cloned() else {
                continue;
            };

            self.check_block(block_id, entry);
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

        // activate loans live at block entry
        self.activate_loans(&live);

        // poll the runtime at a loop header
        let is_header = self
            .loops
            .loops()
            .iter()
            .any(|entry| entry.header == block_id);
        if is_header {
            let point = match block.instructions.first() {
                Some(&instruction) => Point::Instruction(instruction),
                None => Point::Terminator(block_id),
            };
            let handles = self.live_handles();
            self.safepoints.insert(point, SafepointKind::Poll, handles);
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

        // poll the runtime before a tail call
        if matches!(terminator, Terminator::TailCall { .. }) {
            let handles = self.live_handles();
            self.safepoints
                .insert(Point::Terminator(block_id), SafepointKind::Poll, handles);
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

        Copy::decide(self.tree, ty, &self.function.generics).is_no()
    }

    /// Return whether one value stores a variant, whose case a store may change.
    pub(super) fn is_variant(&self, value: Value) -> bool {
        let ty = self.function.expect_value_type(value);

        matches!(self.tree.get(ty), Type::Variant { .. })
    }

    /// Return whether one value has a user drop hook.
    pub(super) fn has_drop_hook(&self, value: Value) -> bool {
        let ty = self.function.expect_value_type(value);

        self.verification.drops.has_hook(ty)
    }

    /// Return whether one reference grants exclusive access.
    pub(super) fn is_exclusive(&self, value: Value) -> bool {
        let ty = self.function.expect_value_type(value);

        self.tree.get(ty).reference_access() == Some(Access::Exclusive)
    }

    /// Return whether one reference addresses uninitialized storage.
    pub(super) fn points_to_uninitialized(&self, value: Value) -> bool {
        let ty = self.function.expect_value_type(value);
        let Type::Reference { pointee, .. } = self.tree.get(ty) else {
            return false;
        };

        matches!(self.tree.get(*pointee), Type::Uninit { .. })
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
        if self.is_variant(base) {
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
