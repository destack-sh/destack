use std::sync::Arc;

use destack_mir::{
    Block, ConstantTable, Function, FunctionCache, FunctionId, InitializationTable, LivenessCursor,
    LivenessTable, LoanId, LocalNodeId, LocalNodeIdAny, MemoryEffectTable, MovePathId, MoveTable,
    OriginContext, OriginState, OriginTable, PlaceOrigin, PlaceTable, RetentionTable, Tree, Type,
    Value, is_copy,
};

use destack_artifact::DiagnosticAnchor;
use destack_core::{BitSet, FxIndexSet};

use crate::verify::VerifyState;

/// Checker for one MIR function: its moves, its constructor initialization, and its borrows.
pub(in crate::verify) struct FunctionChecker<'a, 'b> {
    /// The function being verified.
    pub(super) function: &'a Function,
    /// The identity of the function being verified.
    pub(super) function_id: FunctionId,
    /// The MIR tree.
    pub(super) tree: &'a Tree,
    /// Module verification state.
    pub(super) verification: &'a mut VerifyState<'b>,
    /// Move-path initialization.
    pub(super) initialization: Arc<InitializationTable>,
    /// SSA value liveness.
    liveness: Arc<LivenessTable>,
    /// Constants used to compare structural projections.
    pub(super) constants: Arc<ConstantTable>,
    /// Memory effects for every operation.
    pub(super) effects: Arc<MemoryEffectTable>,
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
    /// The scratch loan set each activation fills and empties.
    included_loans: BitSet,
    /// The scratch loan set of the operation being checked: its arguments' loans with their ancestry.
    pub(super) authorized_loans: BitSet,
    /// The scratch loan set of the access being checked: the loans it traverses with their ancestry.
    pub(super) traversed_loans: BitSet,
    /// Access errors already reported, one per loan chain root and source anchor.
    reported: FxIndexSet<(LoanId, DiagnosticAnchor)>,
}

impl<'a, 'b> FunctionChecker<'a, 'b> {
    /// Create one function checker over the function's analyses.
    pub(in crate::verify) fn new(
        function_id: FunctionId,
        tree: &'a Tree,
        verification: &'a mut VerifyState<'b>,
        analyses: &mut FunctionCache,
    ) -> Self {
        let function = tree.get(function_id);
        let initialization = analyses.initialization(function_id, tree);
        let liveness = analyses.liveness(function_id, tree);
        let places = analyses.place(function_id, tree);
        let moves = analyses.moves(function_id, tree);
        let constants = analyses.constant(function_id, tree);
        let effects = analyses.memory_effect(function_id, tree, &verification.effects);
        let origin = analyses.origin(function_id, tree);
        let loan_count = origin.loans().len();

        Self {
            function,
            function_id,
            tree,
            verification,
            initialization,
            liveness,
            constants,
            effects,
            places,
            moves,
            origin,
            state: OriginState::new(),
            active_loans: Vec::new(),
            rejected_loans: BitSet::new(loan_count),
            included_loans: BitSet::new(loan_count),
            authorized_loans: BitSet::new(loan_count),
            traversed_loans: BitSet::new(loan_count),
            reported: FxIndexSet::default(),
            retention: RetentionTable::default(),
        }
    }

    /// Return the origin transfer context over this function's analyses.
    pub(super) fn context(&self) -> OriginContext<'_> {
        OriginContext::new(
            self.function_id,
            self.tree,
            &self.places,
            self.origin.loans(),
        )
    }

    /// Anchor one diagnostic at a node of the verified function.
    pub(super) fn anchor(&self, node: LocalNodeIdAny) -> DiagnosticAnchor {
        self.verification.anchor(node)
    }

    /// Report one loan chain's access error at most once per anchor.
    pub(super) fn reports(&mut self, loan: LoanId, anchor: LocalNodeIdAny) -> bool {
        // key the report by the least loan of the chain's ancestry, which parent cycles share
        let loans = self.origin.loans();
        let mut chain = vec![loan];
        loans.extend_parents(&mut chain, &mut BitSet::new(loans.len()));
        let Some(&root) = chain.iter().min() else {
            unreachable!("a loan chain holds its own loan");
        };
        let anchor = self.anchor(anchor);

        self.reported.insert((root, anchor))
    }

    /// Check the function's moves, constructor initialization, then borrows.
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

        // check and transfer each instruction in execution order
        for &instruction_id in &block.instructions {
            let instruction = self.tree.get(instruction_id);

            // activate loans live before the instruction
            self.activate_loans(&live);

            // check ownership and advance origin
            self.check_instruction(instruction_id, instruction);
            let cx = OriginContext::new(
                self.function_id,
                self.tree,
                &self.places,
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

    /// Derive loans retained by live values and places.
    fn activate_loans(&mut self, live: &LivenessCursor<'_>) {
        self.active_loans = self.state.active_loans(
            |value| live.contains_value(value),
            |place| {
                live.find_representation(place.origin, &self.places)
                    .is_some()
            },
        );

        // retain each parent loan while any of its reborrows remains live
        self.origin
            .loans()
            .extend_parents(&mut self.active_loans, &mut self.included_loans);
        for loan in &self.active_loans {
            self.included_loans.remove(loan.index());
        }

        // leave rejected loans to their own diagnostics
        self.active_loans
            .retain(|loan| !self.rejected_loans.contains(loan.index()));
    }

    /// Return whether using one value moves it, anything but Copy.
    pub(super) fn is_moved_on_use(&self, value: Value) -> bool {
        let ty = self.function.expect_value_type(value);

        !is_copy(self.tree, ty, &self.function.generics)
    }

    /// Return whether one reference addresses uninitialized storage.
    pub(super) fn points_to_uninitialized(&self, value: Value) -> bool {
        let ty = self.function.expect_value_type(value);
        let Type::Reference { pointee, .. } = self.tree.get(ty) else {
            return false;
        };

        matches!(self.tree.get(*pointee), Type::Uninit { .. })
    }

    /// Return the storage definition of one SSA value's type.
    pub(super) fn value_type(&self, value: Value) -> &'a Type {
        let ty = self.function.expect_value_type(value);

        self.tree.type_definition(self.tree.storage_type(ty))
    }

    /// Exclude every loan issued by a rejected reference from later diagnostics.
    pub(super) fn reject_borrow(&mut self, reference: Value) {
        for loan in self.origin.loans().roots_of(reference) {
            self.rejected_loans.insert(loan.index());
        }
    }
}
