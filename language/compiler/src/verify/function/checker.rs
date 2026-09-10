use std::sync::Arc;

use destack_mir::{
    Access, AliasTable, Block, Copy, Function, FunctionCache, InitializationTable, LivenessCursor,
    LivenessTable, Loan, LoanId, LocalNodeId, LocalNodeIdAny, MemoryEffectTable, MovePathId,
    MoveTable, OriginContext, OriginState, OriginTable, Place, PlaceOrigin, PlaceTable, Projection,
    ReferenceKind, RetentionTable, Tree, Type, TypeId, Value,
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
    pub(super) accesses: Arc<MemoryEffectTable>,
    /// Places derived by address values.
    pub(super) places: Arc<PlaceTable>,
    /// Dense independently movable paths, owned pointees included.
    pub(super) moves: Arc<MoveTable>,
    /// Solved borrow origin.
    pub(super) origin: Arc<OriginTable>,
    /// Verified ownership retention.
    retention: RetentionTable,
    /// Origin at the current operation.
    pub(super) state: OriginState,
    /// Loans live at the current operation.
    pub(super) active_loans: Vec<LoanId>,
    /// Loans rejected while emitting diagnostics.
    pub(super) rejected_loans: BitSet,
    /// Access errors already reported, one per loan and source anchor.
    reported: FxIndexSet<(LoanId, DiagnosticAnchor)>,
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
        let accesses =
            analyses.memory_effect(function, tree, verification.accesses, &verification.effects);
        let origin = analyses.origin(function, tree);
        let loan_count = origin.loans().len();

        Self {
            function,
            tree,
            verification,
            initialization,
            liveness,
            alias,
            accesses,
            places,
            moves,
            origin,
            state: OriginState::new(),
            active_loans: Vec::new(),
            rejected_loans: BitSet::new(loan_count),
            reported: FxIndexSet::default(),
            retention: RetentionTable::default(),
        }
    }

    /// Return the origin transfer context over this function's analyses.
    pub(super) fn context(&self) -> OriginContext<'_> {
        OriginContext::new(self.function, self.tree, &self.places, self.origin.loans())
    }

    /// Report one loan's access error at most once per anchor.
    pub(super) fn reports(&mut self, loan: LoanId, anchor: LocalNodeIdAny) -> bool {
        let anchor = self.verification.anchor(anchor);
        self.reported.insert((loan, anchor))
    }

    /// Check the function: moves, then the constructor's initialization, then borrows.
    pub(in crate::verify) fn check(mut self) -> RetentionTable {
        self.check_moves();
        self.check_initialization();
        self.check_borrows();

        self.retention
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
        let mut live = liveness.cursor(self.tree, block_id);

        // retain ownership live at block entry
        let entries = self.retention_entries(&live);
        self.retention.insert_block(block_id, entries);

        // activate loans live at block entry
        self.activate_loans(&live);

        // check and transfer each instruction in execution order
        for &instruction_id in &block.instructions {
            let instruction = self.tree.get(instruction_id);

            // activate loans live before the instruction
            self.activate_loans(&live);

            // check ownership and advance origin
            self.check_instruction(instruction_id, instruction);
            let cx =
                OriginContext::new(self.function, self.tree, &self.places, self.origin.loans());
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

        self.check_terminator(block_id, block.terminator, terminator);
    }

    /// Return live representations and the move-only owners they retain.
    fn retention_entries(&self, live: &LivenessCursor<'_>) -> Vec<MovePathId> {
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

    /// Return one structural field type.
    pub(super) fn field_type(&self, aggregate: Value, field: u32) -> TypeId {
        let ty = self.function.expect_value_type(aggregate);
        let ty = self.tree.represented(ty);

        self.tree
            .get(ty)
            .field_type(field, self.tree)
            .unwrap_or_else(|| unreachable!("aggregate has no field {field}"))
    }

    /// Return one fixed-array element type.
    pub(super) fn element_type(&self, aggregate: Value) -> TypeId {
        let ty = self.function.expect_value_type(aggregate);
        let ty = self.tree.represented(ty);
        let Type::FixedArray { element, .. } = self.tree.get(ty) else {
            unreachable!("element.set aggregate has no fixed-array type")
        };

        *element
    }

    /// Return the aggregate element type, selecting a newtype variant case by the stored value.
    pub(super) fn slot_type(&self, aggregate: Value, index: usize, value: Value) -> TypeId {
        let ty = self.function.expect_value_type(aggregate);
        let ty = self.tree.represented(ty);
        let aggregate = self.tree.get(ty);
        let slot = match aggregate {
            Type::Struct { fields, .. } => fields.get(index).map(|field| self.tree.get(*field).ty),
            Type::Tuple { elements, .. } => elements.get(index).copied(),
            Type::FixedArray { element, .. } => Some(*element),
            Type::Newtype { inner, .. } if index == 0 => {
                let inner = self.tree.represented(*inner);
                match self.tree.get(inner) {
                    Type::Variant { cases, .. } => {
                        let filled = TypeId::from(self.function.expect_value_type(value));
                        cases
                            .iter()
                            .map(|case| case.ty)
                            .find(|case| self.tree.same_representation(*case, filled))
                    }
                    _ => Some(inner),
                }
            }
            _ => None,
        };

        slot.unwrap_or_else(|| unreachable!("aggregate has no slot {index}"))
    }

    /// Derive loans retained by live values and places.
    fn activate_loans(&mut self, live: &LivenessCursor<'_>) {
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

    /// Return whether one value stores a variant, a store changing its case.
    pub(super) fn is_variant(&self, value: Value) -> bool {
        let ty = self.function.expect_value_type(value);

        matches!(self.tree.get(ty), Type::Variant { .. })
    }

    /// Return whether one value has a user drop hook.
    pub(super) fn has_drop_hook(&self, value: Value) -> bool {
        let ty = self.function.expect_value_type(value);

        self.verification.drops.has_hook(ty)
    }

    /// Return whether the loan writes to owned storage.
    pub(super) fn is_exclusive(&self, loan: &Loan) -> bool {
        loan.writes() && loan.place().is_some_and(|place| self.owns_place(place))
    }

    /// Return whether one address names owned storage.
    pub(super) fn addresses_owned_storage(&self, pointer: Value) -> bool {
        self.owns_place(self.places.get(pointer))
    }

    /// Return whether one place is owned storage.
    pub(super) fn owns_place(&self, place: &Place) -> bool {
        match (place.origin, place.path.first()) {
            (PlaceOrigin::Local(local), Some(Projection::Deref)) => {
                let ty = self.tree.get(local).ty;

                self.tree.get(ty).reference_kind() == Some(ReferenceKind::Unique)
            }
            (PlaceOrigin::Local(_) | PlaceOrigin::Global(_), _) => true,
            (PlaceOrigin::Value(value), _) => !matches!(
                self.function.reference_kind(value, self.tree),
                Some(ReferenceKind::Managed | ReferenceKind::Borrowed)
            ),
        }
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
