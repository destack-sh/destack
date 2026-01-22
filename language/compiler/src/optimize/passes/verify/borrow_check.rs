use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use destack_compiler_macros::declare_pass;
use destack_mir as mir;
use destack_source::ModuleId;
use destack_workspace::TargetId;
use mir::{Instruction, Mutability, ReferenceKind, Type, Value};

use crate::optimize::{
    AliasAnalysis, AnalysisPreservation, BorrowAnalysis, BorrowMap, DiagnosticEmitter,
    FunctionPass, LifetimeAnalysis, LivenessAnalysis, MemoryLocation, PipelineContext,
    ResolvedLifetime,
};
use crate::{OptimizeError, OptimizeWarning};

declare_pass! {
    /// Verify borrow rules for references.
    ///
    /// Tracks active borrows created by `field.addr` and `element.addr`, and detects:
    /// - Drop while borrowed: `drop` or `raw.free` of a value with active borrows (always error)
    /// - Conflicting borrows: new borrow conflicts with existing (strict mode: error, lenient: warning)
    /// - Invalidated references: `store` through a pointer may invalidate other borrows
    ///
    /// Borrows are tracked with liveness-based expiry: a borrow expires when its
    /// reference value is no longer live. Provenance chains track transitive borrows
    /// so that dropping an origin value invalidates all derived borrows.
    #[pass(id = "borrow-check")]
    pub BorrowCheck,
    "Verify borrow rules"
}

/// Unique identifier for a borrow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct BorrowId(u32);

/// An active borrow being tracked.
#[derive(Debug, Clone)]
struct ActiveBorrow {
    /// The value holding the reference.
    reference: Value,
    /// The direct value being borrowed from (the origin).
    ///
    /// None for borrows from local variables (tracked via local_borrows map).
    origin: Option<Value>,
    /// Whether this is a mutable borrow.
    is_mutable: bool,
    /// Where the borrow was created.
    created_at: mir::LocalNodeId<Instruction>,
    /// Provenance chain: all values this borrow transitively borrows from.
    ///
    /// If v3 = field.addr v2 and v2 = field.addr v0, then v3's provenance is {v2, v0}.
    /// This allows us to detect when dropping any value in the chain invalidates the borrow.
    provenance: HashSet<Value>,
}

/// Context for borrow checking a single function.
#[allow(dead_code)]
struct BorrowCheckContext<'a> {
    /// The function being checked.
    function: &'a mir::Function,
    /// The MIR tree.
    tree: &'a mir::NodeTree,
    /// Alias analysis for may-alias queries.
    alias_analysis: Arc<AliasAnalysis>,
    /// Lifetime analysis for cross-function borrow tracking.
    lifetime_analysis: Arc<LifetimeAnalysis>,
    /// Currently active borrows by ID.
    active_borrows: HashMap<BorrowId, ActiveBorrow>,
    /// Map from borrowed-from value to borrow IDs.
    borrows_of: HashMap<Value, HashSet<BorrowId>>,
    /// Next borrow ID to allocate.
    next_borrow_id: u32,
    /// The module being checked.
    module_id: ModuleId,
    /// The target being checked.
    target_id: TargetId,
    /// Whether we're in strict mode.
    strict_mode: bool,
    /// Whether any aliasing violations were found.
    had_aliasing_violations: bool,
    /// Known types for values (for mutability detection).
    value_types: HashMap<Value, mir::LocalNodeId<Type>>,
    /// Map from local to borrow IDs that borrow from it.
    local_borrows: HashMap<mir::LocalNodeId<mir::Local>, HashSet<BorrowId>>,
    /// The BorrowAnalysis entry state for the current block.
    ///
    /// Used to detect MaybeBorrowed state from control flow merges.
    entry_borrow_state: Option<BorrowMap>,
}

impl<'a> BorrowCheckContext<'a> {
    fn new(
        function: &'a mir::Function,
        tree: &'a mir::NodeTree,
        alias_analysis: Arc<AliasAnalysis>,
        lifetime_analysis: Arc<LifetimeAnalysis>,
        module_id: ModuleId,
        target_id: TargetId,
        strict_mode: bool,
    ) -> Self {
        Self {
            function,
            tree,
            alias_analysis,
            lifetime_analysis,
            active_borrows: HashMap::new(),
            borrows_of: HashMap::new(),
            next_borrow_id: 0,
            module_id,
            target_id,
            strict_mode,
            had_aliasing_violations: false,
            value_types: HashMap::new(),
            local_borrows: HashMap::new(),
            entry_borrow_state: None,
        }
    }

    /// Allocate a new borrow ID.
    fn alloc_borrow_id(&mut self) -> BorrowId {
        let id = BorrowId(self.next_borrow_id);
        self.next_borrow_id += 1;
        id
    }

    /// Register a value's type.
    fn register_value_type(&mut self, value: Value, ty_id: mir::LocalNodeId<Type>) {
        self.value_types.insert(value, ty_id);
    }

    /// Determine mutability for a derived reference based on source type.
    ///
    /// For field.addr/element.addr, the result inherits mutability from source.
    fn derive_mutability(&self, source: Value) -> bool {
        if let Some(&ty_id) = self.value_types.get(&source) {
            let ty = self.tree.get(ty_id);
            if let Type::Reference { mutability, .. } = ty {
                return *mutability == Mutability::Mutable;
            }
        }

        // conservative: assume mutable if type unknown
        // this catches more potential conflicts than assuming shared
        true
    }

    /// Determine mutability directly from a reference type.
    fn mutability_from_reference_type(&self, ty_id: mir::LocalNodeId<Type>) -> bool {
        let ty = self.tree.get(ty_id);
        if let Type::Reference { mutability, .. } = ty {
            return *mutability == Mutability::Mutable;
        }

        // conservative: assume mutable if type unknown
        true
    }

    /// Reset borrows for a new block and seed from BorrowAnalysis.
    ///
    /// This ensures borrows created in predecessor blocks are visible.
    fn reset_and_seed_from_analysis(&mut self, borrow_state: &BorrowMap) {
        // clear existing borrows
        self.active_borrows.clear();
        self.borrows_of.clear();
        self.local_borrows.clear();
        self.next_borrow_id = 0;

        // store entry state for MaybeBorrowed detection
        self.entry_borrow_state = Some(borrow_state.clone());

        // seed from BorrowAnalysis - use active_borrows which has reference info
        for (reference, origin, at) in borrow_state.active_borrows() {
            let id = self.alloc_borrow_id();
            // derive mutability from reference type
            let is_mutable = self.derive_mutability(reference);
            self.active_borrows.insert(
                id,
                ActiveBorrow {
                    reference,
                    origin: Some(origin),
                    is_mutable,
                    created_at: at,
                    provenance: HashSet::new(),
                },
            );
            self.borrows_of.entry(origin).or_default().insert(id);
        }

        // seed local borrows from analysis
        for (reference, local, at) in borrow_state.active_local_borrows() {
            let id = self.alloc_borrow_id();
            let is_mutable = self.derive_mutability(reference);
            self.active_borrows.insert(
                id,
                ActiveBorrow {
                    reference,
                    origin: None,
                    is_mutable,
                    created_at: at,
                    provenance: HashSet::new(),
                },
            );
            self.local_borrows.entry(local).or_default().insert(id);
        }
    }

    /// Create an anchored node ID for diagnostics.
    fn anchor(&self, instruction_id: mir::LocalNodeId<Instruction>) -> mir::AnchoredGlobalNodeId {
        instruction_id
            .into_any()
            .into_anchored(self.module_id, self.target_id.clone())
    }

    /// Build provenance chain for a new borrow.
    ///
    /// If the origin has borrows, we include its provenance chain in ours.
    fn build_provenance(&self, origin: Value) -> HashSet<Value> {
        // find any borrow where the reference is our origin
        for borrow in self.active_borrows.values() {
            if borrow.reference == origin {
                // origin is itself a reference: inherit its provenance plus origin
                let mut provenance = HashSet::new();
                if let Some(parent_origin) = borrow.origin {
                    provenance.insert(parent_origin);
                }
                provenance.extend(borrow.provenance.iter().copied());
                return provenance;
            }
        }
        // origin has no provenance chain
        HashSet::new()
    }

    /// Register a new borrow.
    fn add_borrow(
        &mut self,
        reference: Value,
        origin: Value,
        is_mutable: bool,
        at: mir::LocalNodeId<Instruction>,
    ) {
        let provenance = self.build_provenance(origin);
        let id = self.alloc_borrow_id();

        self.active_borrows.insert(
            id,
            ActiveBorrow {
                reference,
                origin: Some(origin),
                is_mutable,
                created_at: at,
                provenance,
            },
        );

        // track in borrows_of for direct origin
        self.borrows_of.entry(origin).or_default().insert(id);
    }

    /// Register a borrow from a local variable.
    fn add_local_borrow(
        &mut self,
        reference: Value,
        local: mir::LocalNodeId<mir::Local>,
        is_mutable: bool,
        at: mir::LocalNodeId<Instruction>,
    ) {
        let id = self.alloc_borrow_id();

        self.active_borrows.insert(
            id,
            ActiveBorrow {
                reference,
                origin: None, // local borrows tracked via local_borrows map
                is_mutable,
                created_at: at,
                provenance: HashSet::new(),
            },
        );

        self.local_borrows.entry(local).or_default().insert(id);
    }

    /// Check whether a new local borrow conflicts with existing borrows.
    fn check_local_borrow_conflict(
        &mut self,
        local: mir::LocalNodeId<mir::Local>,
        is_mutable: bool,
        at: mir::LocalNodeId<Instruction>,
        context: &impl DiagnosticEmitter,
    ) {
        let Some(borrow_ids) = self.local_borrows.get(&local) else {
            return;
        };

        let conflict = borrow_ids.iter().find_map(|id| {
            let borrow = self.active_borrows.get(id)?;
            if is_mutable || borrow.is_mutable {
                Some((borrow.created_at, borrow.is_mutable))
            } else {
                None
            }
        });

        if let Some((existing_at, existing_is_mutable)) = conflict {
            self.had_aliasing_violations = true;
            context.emit_error(OptimizeError::ConflictingBorrow {
                node: self.anchor(at),
                existing_borrow: self.anchor(existing_at),
                existing_is_mutable,
            });
        }
    }

    /// Check if a local has any active borrows.
    fn local_has_borrow(&self, local: mir::LocalNodeId<mir::Local>) -> Option<&ActiveBorrow> {
        self.local_borrows
            .get(&local)
            .and_then(|ids| ids.iter().next())
            .and_then(|id| self.active_borrows.get(id))
    }

    /// Remove a borrow by ID.
    fn remove_borrow(&mut self, id: BorrowId) {
        if let Some(borrow) = self.active_borrows.remove(&id) {
            // remove from borrows_of
            if let Some(origin) = borrow.origin
                && let Some(ids) = self.borrows_of.get_mut(&origin)
            {
                ids.remove(&id);
            }
        }

        // remove from local_borrows
        for ids in self.local_borrows.values_mut() {
            ids.remove(&id);
        }
    }

    /// Check creating a new borrow for conflicts.
    ///
    /// Uses alias analysis to detect may-alias relationships between reference
    /// values. This enables more precise
    /// conflict detection:
    /// - Two field.addr to different fields of same struct: NoAlias (no conflict)
    /// - Two field.addr to same field: MayAlias/MustAlias (conflict if mutable)
    /// - Unrelated allocations: NoAlias (no conflict)
    fn check_new_borrow(
        &mut self,
        new_reference: Value,
        origin: Value,
        is_mutable: bool,
        at: mir::LocalNodeId<Instruction>,
        context: &impl DiagnosticEmitter,
    ) {
        let new_loc = MemoryLocation::from_ptr(new_reference);

        // find conflicting borrows using alias analysis
        let conflict = self.active_borrows.values().find_map(|borrow| {
            // local borrows only guard local.set
            borrow.origin?;

            let borrow_loc = MemoryLocation::from_ptr(borrow.reference);

            // check if locations may alias
            if !self.alias_analysis.alias(&new_loc, &borrow_loc).may_alias() {
                return None; // no alias, no conflict
            }

            // aliasing exists - check borrow rules
            if is_mutable {
                // mutable borrow conflicts with any existing borrow of same location
                Some((borrow.created_at, borrow.is_mutable))
            } else if borrow.is_mutable {
                // shared borrow conflicts with existing mutable borrow
                Some((borrow.created_at, borrow.is_mutable))
            } else {
                None // shared + shared is ok
            }
        });

        if let Some((existing_at, existing_is_mutable)) = conflict {
            self.report_borrow_conflict(at, existing_at, existing_is_mutable, context);
        }

        // register the new borrow (with provenance tracking)
        self.add_borrow(new_reference, origin, is_mutable, at);
    }

    /// Check mutation through reference doesn't invalidate other borrows.
    ///
    /// Uses alias analysis to detect may-alias relationships between the store
    /// location and borrow origins. This catches cases where different pointers
    /// may refer to overlapping memory.
    fn check_mutation_through_reference(
        &mut self,
        pointer: Value,
        at: mir::LocalNodeId<Instruction>,
        context: &impl DiagnosticEmitter,
    ) {
        let store_loc = MemoryLocation::from_ptr(pointer);

        // check if any existing borrow may be invalidated by this mutation
        let invalidated: Vec<_> = self
            .active_borrows
            .values()
            .filter(|borrow| {
                // mutating through our own reference is fine
                if borrow.reference == pointer {
                    return false;
                }

                // check if the borrow's location may alias the store location
                // for regular borrows, use origin; for local borrows, use reference
                let borrow_ptr = borrow.origin.unwrap_or(borrow.reference);
                let borrow_loc = MemoryLocation::from_ptr(borrow_ptr);
                self.alias_analysis
                    .alias(&store_loc, &borrow_loc)
                    .may_alias()
            })
            .map(|b| b.created_at)
            .collect();

        for invalidated_at in invalidated {
            self.emit_invalidated_reference(at, invalidated_at, context);
        }
    }

    /// Check that dropping a value doesn't drop while borrowed.
    ///
    /// Uses alias analysis to check if the dropped value may alias any borrow
    /// origin. This catches cases where different pointers refer to the same
    /// or overlapping memory.
    fn check_drop_while_borrowed(
        &mut self,
        value: Value,
        at: mir::LocalNodeId<Instruction>,
        context: &impl DiagnosticEmitter,
    ) {
        let drop_loc = MemoryLocation::from_ptr(value);

        // check borrows that may alias the dropped value
        let conflicting_borrow = self.active_borrows.values().find(|borrow| {
            // local borrows are handled by local.set checks
            if borrow.origin.is_none() {
                return false;
            }

            // check direct transitive borrow relationship (provenance chain)
            if borrow.origin == Some(value) || borrow.provenance.contains(&value) {
                return true;
            }

            // check if the borrow's location may alias the dropped value
            // for regular borrows, use origin; for local borrows, use reference
            let borrow_ptr = borrow.origin.unwrap_or(borrow.reference);
            let borrow_loc = MemoryLocation::from_ptr(borrow_ptr);
            self.alias_analysis
                .alias(&drop_loc, &borrow_loc)
                .may_alias()
        });

        if let Some(borrow) = conflicting_borrow {
            context.emit_error(OptimizeError::DropWhileBorrowed {
                node: self.anchor(at),
                borrowed_at: self.anchor(borrow.created_at),
            });
            self.had_aliasing_violations = true;
            return;
        }

        // also check entry state for MaybeBorrowed from control flow merges
        if let Some(entry_state) = &self.entry_borrow_state {
            let borrow_state = entry_state.get(value);
            if let Some(borrowed_at) = borrow_state.borrow_location() {
                context.emit_error(OptimizeError::DropWhileBorrowed {
                    node: self.anchor(at),
                    borrowed_at: self.anchor(borrowed_at),
                });
                self.had_aliasing_violations = true;
            }
        }
    }

    /// Check that moving a value doesn't move while it has active borrows.
    fn check_move_while_borrowed(
        &mut self,
        value: Value,
        at: mir::LocalNodeId<Instruction>,
        context: &impl DiagnosticEmitter,
    ) {
        let Some(&ty_id) = self.value_types.get(&value) else {
            return;
        };

        let ty = self.tree.get(ty_id);
        let Type::Reference { kind, .. } = ty else {
            return;
        };

        if matches!(kind, ReferenceKind::Raw | ReferenceKind::Borrowed) {
            return;
        }

        // build the move location
        let move_loc = MemoryLocation::from_ptr(value);

        // check for any active borrow that aliases the move
        let conflicting_borrow = self.active_borrows.values().find(|borrow| {
            // local borrows only guard local.set
            if borrow.origin.is_none() {
                return false;
            }

            // check direct provenance links
            if borrow.origin == Some(value) || borrow.provenance.contains(&value) {
                return true;
            }

            // check aliasing between the move and the borrow
            let borrow_ptr = borrow.origin.unwrap_or(borrow.reference);
            let borrow_loc = MemoryLocation::from_ptr(borrow_ptr);
            self.alias_analysis
                .alias(&move_loc, &borrow_loc)
                .may_alias()
        });

        // check for conflicts with active borrows
        if let Some(borrow) = conflicting_borrow {
            context.emit_error(OptimizeError::MoveOfBorrowedValue {
                node: self.anchor(at),
                borrowed_at: self.anchor(borrow.created_at),
            });
            self.had_aliasing_violations = true;
            return;
        }

        // check for conflicts with entry borrow state
        if let Some(entry_state) = &self.entry_borrow_state {
            let borrow_state = entry_state.get(value);
            if let Some(borrowed_at) = borrow_state.borrow_location() {
                context.emit_error(OptimizeError::MoveOfBorrowedValue {
                    node: self.anchor(at),
                    borrowed_at: self.anchor(borrowed_at),
                });
                self.had_aliasing_violations = true;
            }
        }
    }

    /// Check that setting a local doesn't overwrite a borrowed local.
    fn check_local_set_while_borrowed(
        &mut self,
        local: mir::LocalNodeId<mir::Local>,
        at: mir::LocalNodeId<Instruction>,
        context: &impl DiagnosticEmitter,
    ) {
        let borrowed_at = self.local_has_borrow(local).map(|b| b.created_at);

        if let Some(borrowed_at) = borrowed_at {
            // setting a borrowed local invalidates the borrow
            context.emit_error(OptimizeError::LocalSetWhileBorrowed {
                node: self.anchor(at),
                borrowed_at: self.anchor(borrowed_at),
            });
            self.had_aliasing_violations = true;
        }
    }

    /// Track borrows created by a function call based on lifetime analysis.
    ///
    /// When a function returns a borrowed reference (or aggregate containing references),
    /// the returned value may borrow from some of the arguments based on the callee's
    /// lifetime bounds.
    fn track_call_return_borrows(
        &mut self,
        destination: Value,
        callee_id: mir::LocalNodeId<mir::Function>,
        arguments: &[Value],
        at: mir::LocalNodeId<Instruction>,
    ) {
        // clone to avoid borrow conflict with self
        let lifetime = self.lifetime_analysis.get(callee_id).clone();
        match lifetime {
            // no borrowed references in return: nothing to track
            ResolvedLifetime::None => {}

            // static lifetime: return borrows from global/static data, not arguments
            ResolvedLifetime::Static => {}

            // return borrows from specific parameters
            ResolvedLifetime::Parameters(param_indices) => {
                // determine mutability from callee's return type
                let callee = self.tree.get(callee_id);
                let return_ty = self.tree.get(callee.return_type);
                let is_mutable = return_ty.is_mutable_borrowed_reference();

                for param_idx in param_indices {
                    if let Some(&arg) = arguments.get(param_idx as usize) {
                        // returned value borrows from this argument
                        // if argument is a reference, we inherit its provenance
                        let provenance = self.build_provenance(arg);
                        let id = self.alloc_borrow_id();

                        self.active_borrows.insert(
                            id,
                            ActiveBorrow {
                                reference: destination,
                                origin: Some(arg),
                                is_mutable,
                                created_at: at,
                                provenance,
                            },
                        );

                        self.borrows_of.entry(arg).or_default().insert(id);
                    }
                }
            }
        }
    }

    /// Emit a borrow conflict error/warning.
    fn report_borrow_conflict(
        &mut self,
        at: mir::LocalNodeId<Instruction>,
        existing_at: mir::LocalNodeId<Instruction>,
        existing_is_mutable: bool,
        context: &impl DiagnosticEmitter,
    ) {
        self.had_aliasing_violations = true;

        if self.strict_mode {
            context.emit_error(OptimizeError::ConflictingBorrow {
                node: self.anchor(at),
                existing_borrow: self.anchor(existing_at),
                existing_is_mutable,
            });
        } else {
            context.emit_warning(OptimizeWarning::PotentialAliasingViolation {
                node: self.anchor(at),
                existing_borrow: self.anchor(existing_at),
            });
        }
    }

    /// Emit an invalidated reference error/warning.
    fn emit_invalidated_reference(
        &mut self,
        at: mir::LocalNodeId<Instruction>,
        invalidated_at: mir::LocalNodeId<Instruction>,
        context: &impl DiagnosticEmitter,
    ) {
        self.had_aliasing_violations = true;

        if self.strict_mode {
            context.emit_error(OptimizeError::InvalidatedReference {
                node: self.anchor(at),
                invalidated_by: self.anchor(invalidated_at),
            });
        } else {
            context.emit_warning(OptimizeWarning::PotentialInvalidatedReference {
                node: self.anchor(invalidated_at),
                mutation_at: self.anchor(at),
            });
        }
    }
}

impl FunctionPass for BorrowCheck {
    fn run(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::NodeTree,
        ctx: &PipelineContext<'_>,
    ) -> AnalysisPreservation {
        // get function-level analyses
        let (liveness, borrow_analysis, alias_analysis) = {
            let analyses = ctx.function_analyses(function, tree);
            (
                analyses.get::<LivenessAnalysis>().clone(),
                analyses.get::<BorrowAnalysis>().clone(),
                analyses.get::<AliasAnalysis>().clone(),
            )
        };

        // get module-level lifetime analysis
        let lifetime_analysis = ctx.module_analyses(tree).get::<LifetimeAnalysis>().clone();

        let module_id = ctx.module_id();
        let target_id = ctx.target_id().clone();

        let strict_mode = ctx.options.strict_borrow_mode;
        let mut checker = BorrowCheckContext::new(
            function,
            tree,
            alias_analysis,
            lifetime_analysis,
            module_id,
            target_id,
            strict_mode,
        );

        // register function parameter types
        for param in &function.parameters {
            checker.register_value_type(param.value, param.ty);
        }

        // process blocks
        for &block_id in &function.blocks {
            let block = tree.get(block_id);

            // register block parameter types
            for param in &block.parameters {
                checker.register_value_type(param.value, param.ty);
            }

            // seed borrows from BorrowAnalysis at block entry
            if let Some(entry_state) = borrow_analysis.state_at_entry(block_id) {
                checker.reset_and_seed_from_analysis(entry_state);
            }

            // process each instruction
            for (inst_idx, &instruction_id) in block.instructions.iter().enumerate() {
                let instruction = tree.get(instruction_id).clone();

                // expire borrows whose references are no longer live
                expire_dead_borrows(&mut checker, block_id, inst_idx, &liveness, tree);

                check_instruction(&mut checker, instruction_id, &instruction, ctx);
            }
        }

        // if aliasing violations were found, mark context
        if checker.had_aliasing_violations {
            ctx.mark_aliasing_violation();
        }

        // borrow checking is a pure analysis pass
        AnalysisPreservation::all()
    }

    fn name(&self) -> &'static str {
        "BorrowCheck"
    }

    fn id(&self) -> &'static str {
        "borrow-check"
    }
}

/// Expire borrows whose reference values are no longer live.
fn expire_dead_borrows(
    checker: &mut BorrowCheckContext<'_>,
    block_id: mir::LocalNodeId<mir::Block>,
    instruction_index: usize,
    liveness: &LivenessAnalysis,
    tree: &mir::NodeTree,
) {
    // collect expired borrow IDs
    let expired: Vec<BorrowId> = checker
        .active_borrows
        .iter()
        .filter(|(_, borrow)| {
            if instruction_index == 0 {
                let block = tree.get(block_id);
                let is_block_param = block
                    .parameters
                    .iter()
                    .any(|param| param.value == borrow.reference);
                !liveness.is_live_in(block_id, borrow.reference) && !is_block_param
            } else {
                // check if value is dead after previous instruction
                !liveness.is_live_after_instruction(
                    block_id,
                    instruction_index - 1,
                    borrow.reference,
                    tree,
                )
            }
        })
        .map(|(id, _)| *id)
        .collect();

    // remove expired borrows
    for id in expired {
        checker.remove_borrow(id);
    }
}

/// Check a single instruction for borrow violations.
fn check_instruction(
    checker: &mut BorrowCheckContext<'_>,
    instruction_id: mir::LocalNodeId<Instruction>,
    instruction: &Instruction,
    context: &impl DiagnosticEmitter,
) {
    match instruction {
        // field.addr creates a borrow of the aggregate
        Instruction::FieldAddr {
            destination,
            aggregate,
            result_type,
            ..
        } => {
            // track reference type for later borrow lookups
            checker.register_value_type(*destination, *result_type);

            // derive mutability from reference type
            let is_mutable = checker.mutability_from_reference_type(*result_type);
            checker.check_new_borrow(
                *destination,
                *aggregate,
                is_mutable,
                instruction_id,
                context,
            );
        }

        // element.addr creates a borrow of the array
        Instruction::ElementAddr {
            destination,
            array,
            result_type,
            ..
        } => {
            // track reference type for later borrow lookups
            checker.register_value_type(*destination, *result_type);

            // derive mutability from reference type
            let is_mutable = checker.mutability_from_reference_type(*result_type);
            checker.check_new_borrow(*destination, *array, is_mutable, instruction_id, context);
        }

        // store: moves value, invalidates borrows
        Instruction::Store { pointer, value } => {
            // check if value being stored has active borrows (move-while-borrowed)
            checker.check_move_while_borrowed(*value, instruction_id, context);
            // storing invalidates other borrows of the same location
            checker.check_mutation_through_reference(*pointer, instruction_id, context);
        }

        // local.set mutates the local
        Instruction::LocalSet { local, value } => {
            // check if the value being set has active borrows
            checker.check_move_while_borrowed(*value, instruction_id, context);
            // check if the local is currently borrowed (mutation through borrow)
            checker.check_local_set_while_borrowed(*local, instruction_id, context);
        }

        // local.get loads a value without creating a borrow
        Instruction::LocalGet { destination, local } => {
            // track the value type for later use
            let local_decl = checker.tree.get(*local);
            checker.register_value_type(*destination, local_decl.ty);
        }

        // local.addr creates a borrow of a local
        Instruction::LocalAddr {
            destination,
            local,
            result_type,
        } => {
            // track reference type for later borrow lookups
            checker.register_value_type(*destination, *result_type);

            // only references participate in borrow checking
            let ty = checker.tree.get(*result_type);
            if matches!(ty, Type::Reference { .. }) {
                let is_mutable = checker.mutability_from_reference_type(*result_type);
                checker.check_local_borrow_conflict(*local, is_mutable, instruction_id, context);
                checker.add_local_borrow(*destination, *local, is_mutable, instruction_id);
            }
        }

        // raw.drop invalidates any borrows from this value
        Instruction::RawDrop { value } => {
            checker.check_drop_while_borrowed(*value, instruction_id, context);
        }

        // stack.drop invalidates any borrows from this value
        Instruction::StackDrop { value } => {
            checker.check_drop_while_borrowed(*value, instruction_id, context);
        }

        // raw.free invalidates borrows
        Instruction::RawFree { pointer } => {
            checker.check_drop_while_borrowed(*pointer, instruction_id, context);
        }

        // load through a reference
        Instruction::Load {
            destination,
            pointer,
            ..
        } => {
            // borrow validity is checked via liveness-based expiry in expire_dead_borrows
            // propagate reference type info if loading a reference
            if let Some(&ty_id) = checker.value_types.get(pointer) {
                let ty = checker.tree.get(ty_id);
                if let Type::Reference { pointee, .. } = ty {
                    checker.register_value_type(*destination, *pointee);
                }
            }
        }

        // call: arguments are moved (for owned types)
        Instruction::Call {
            destination,
            function,
            arguments,
            ..
        } => {
            let args = checker.tree.get_arguments(*arguments);
            for &arg in args {
                checker.check_move_while_borrowed(arg, instruction_id, context);
            }
            // register return type from function signature
            if let Some(dest) = destination {
                let func = checker.tree.get(*function);
                checker.register_value_type(*dest, func.return_type);

                // track borrows created by the call based on callee's lifetime bounds
                checker.track_call_return_borrows(*dest, *function, args, instruction_id);
            }
        }

        Instruction::CallVirtual {
            destination,
            arguments,
            signature,
            ..
        }
        | Instruction::CallInterface {
            destination,
            arguments,
            signature,
            ..
        }
        | Instruction::CallIndirect {
            destination,
            callee: _,
            arguments,
            signature,
            ..
        } => {
            // callee is used, not moved
            let args = checker.tree.get_arguments(*arguments);
            for &arg in args {
                checker.check_move_while_borrowed(arg, instruction_id, context);
            }
            if let Some(dest) = destination {
                let signature_type = checker.tree.get(*signature);
                if let Type::FunctionPointer { result, .. } = signature_type {
                    checker.register_value_type(*dest, *result);
                }
            }
        }

        // field.set moves value into aggregate
        Instruction::FieldSet {
            destination,
            aggregate,
            value,
            ..
        } => {
            checker.check_move_while_borrowed(*value, instruction_id, context);
            // propagate type from aggregate
            if let Some(&ty_id) = checker.value_types.get(aggregate) {
                checker.register_value_type(*destination, ty_id);
            }
        }

        // element.set moves value into array
        Instruction::ElementSet {
            destination,
            array,
            value,
            ..
        } => {
            checker.check_move_while_borrowed(*value, instruction_id, context);
            if let Some(&ty_id) = checker.value_types.get(array) {
                checker.register_value_type(*destination, ty_id);
            }
        }

        // struct/tuple/array construction moves all fields
        Instruction::Struct {
            destination,
            ty,
            fields,
        } => {
            let field_values = checker.tree.get_arguments(*fields);
            for &field in field_values {
                checker.check_move_while_borrowed(field, instruction_id, context);
            }
            checker.register_value_type(*destination, *ty);
        }

        Instruction::Tuple {
            destination,
            ty,
            elements,
        } => {
            let element_values = checker.tree.get_arguments(*elements);
            for &elem in element_values {
                checker.check_move_while_borrowed(elem, instruction_id, context);
            }
            checker.register_value_type(*destination, *ty);
        }

        Instruction::Array {
            destination,
            ty,
            elements,
        } => {
            let element_values = checker.tree.get_arguments(*elements);
            for &elem in element_values {
                checker.check_move_while_borrowed(elem, instruction_id, context);
            }
            checker.register_value_type(*destination, *ty);
        }

        // cast preserves type info
        Instruction::Cast {
            destination,
            to_type,
            ..
        } => {
            checker.register_value_type(*destination, *to_type);
        }

        // allocations produce references
        Instruction::ManagedAlloc {
            destination,
            result_type,
            ..
        }
        | Instruction::RawAlloc {
            destination,
            result_type,
            ..
        }
        | Instruction::StackAlloc {
            destination,
            result_type,
            ..
        }
        | Instruction::ManagedAllocArray {
            destination,
            result_type,
            ..
        } => {
            checker.register_value_type(*destination, *result_type);
        }

        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::OptimizeError;
    use crate::optimize::common::tests::TestProgram;

    /// Simple function with no borrows passes verification.
    #[test]
    fn test_verify_no_borrows() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst 42i32
    return v0
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&BorrowCheck);
        test.assert_no_errors();
        test.assert_unchanged(input);
    }

    /// Field address with no other borrows is valid.
    #[test]
    fn test_verify_single_field_addr() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v1 = iconst 42i32
    store v0, v1
    v2 = load v0 -> i32
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&BorrowCheck);
        test.assert_no_errors();
    }

    /// Sequential loads from same pointer is valid.
    #[test]
    fn test_verify_sequential_loads() {
        let input = r#"function @test(v0: ref<raw i32>) -> i32 {
block0(v0: ref<raw i32>):
    v1 = load v0 -> i32
    v2 = load v0 -> i32
    v3 = iadd v1, v2
    return v3
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&BorrowCheck);
        test.assert_no_errors();
    }

    /// Store through pointer is valid.
    #[test]
    fn test_verify_store_no_conflict() {
        let input = r#"function @test(v0: ref<raw i32>, v1: i32) -> void {
block0(v0: ref<raw i32>, v1: i32):
    store v0, v1
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&BorrowCheck);
        test.assert_no_errors();
    }

    /// Dropping while borrowed is detected.
    #[test]
    fn test_detect_drop_while_borrowed() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v1 = field.addr v0, 0 -> ref<borrowed i32>
    raw.drop v0
    v2 = load v1 -> i32
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&BorrowCheck);
        test.assert_error(|e| matches!(e, OptimizeError::DropWhileBorrowed { .. }));
    }

    /// raw.free while borrowed is detected.
    #[test]
    fn test_detect_raw_free_while_borrowed() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = raw.alloc i32 -> ref<raw i32>
    v1 = field.addr v0, 0 -> ref<borrowed i32>
    raw.free v0
    v2 = load v1 -> i32
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&BorrowCheck);
        test.assert_error(|e| matches!(e, OptimizeError::DropWhileBorrowed { .. }));
    }

    /// local.set without existing borrows is valid.
    #[test]
    fn test_verify_local_set_no_borrow() {
        let input = r#"function @test() -> i32 {
local0: i32
block0:
    v0 = iconst 42i32
    local.set local0, v0
    v1 = local.get local0
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&BorrowCheck);
        test.assert_no_errors();
    }

    /// local.set while borrowed is rejected.
    #[test]
    fn test_detect_local_set_while_borrowed() {
        let input = r#"function @test() -> void {
local0: i32 ; owned, mut
block0:
    v0 = iconst 1i32
    local.set local0, v0
    v1 = local.addr local0 -> ref<borrowed addrspace(stack) i32>
    v2 = iconst 2i32
    local.set local0, v2
    v3 = load v1 -> i32
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&BorrowCheck);
        test.assert_error(|e| matches!(e, OptimizeError::LocalSetWhileBorrowed { .. }));
    }

    /// Multiple shared local borrows do not conflict.
    #[test]
    fn test_verify_local_shared_borrows_no_conflict() {
        let input = r#"function @test() -> void {
local0: i32 ; owned, mut
block0:
    v0 = iconst 0i32
    local.set local0, v0
    v1 = local.addr local0 -> ref<borrowed addrspace(stack) i32>
    v2 = local.addr local0 -> ref<borrowed addrspace(stack) i32>
    v3 = load v1 -> i32
    v4 = load v2 -> i32
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&BorrowCheck);
        test.assert_no_errors();
    }

    /// local borrows propagate through block parameters.
    #[test]
    fn test_detect_local_borrow_propagates_across_blocks() {
        let input = r#"function @test() -> void {
local0: i32 ; owned, mut
block0:
    v0 = iconst 1i32
    local.set local0, v0
    v1 = local.addr local0 -> ref<borrowed addrspace(stack) i32>
    jump block1(v1)
block1(v2: ref<borrowed addrspace(stack) i32>):
    v3 = iconst 2i32
    local.set local0, v3
    v4 = load v2 -> i32
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&BorrowCheck);
        test.assert_error(|e| matches!(e, OptimizeError::LocalSetWhileBorrowed { .. }));
    }

    /// Multiple mutable local borrows conflict.
    #[test]
    fn test_detect_local_borrow_conflict() {
        let input = r#"function @test() -> void {
local0: i32 ; owned, mut
block0:
    v0 = iconst 0i32
    local.set local0, v0
    v1 = local.addr local0 -> ref<borrowed addrspace(stack) mut i32>
    v2 = local.addr local0 -> ref<borrowed addrspace(stack) mut i32>
    v3 = load v1 -> i32
    v4 = load v2 -> i32
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&BorrowCheck);
        test.assert_error(|e| matches!(e, OptimizeError::ConflictingBorrow { .. }));
    }

    /// Multiple parameters with no conflicts is valid.
    #[test]
    fn test_verify_multiple_params_no_conflict() {
        let input = r#"function @test(v0: ref<raw i32>, v1: ref<raw i32>) -> i32 {
block0(v0: ref<raw i32>, v1: ref<raw i32>):
    v2 = load v0 -> i32
    v3 = load v1 -> i32
    v4 = iadd v2, v3
    return v4
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&BorrowCheck);
        test.assert_no_errors();
    }

    /// Drop while borrowed in strict mode emits error.
    #[test]
    fn test_strict_mode_drop_while_borrowed() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v1 = iconst 42i32
    store v0, v1
    v2 = field.addr v0, 0 -> ref<borrowed i32>
    raw.drop v0
    v3 = load v2 -> i32
    return v3
}"#;

        let mut options = crate::optimize::PipelineOptions::default();
        options.strict_borrow_mode = true;

        let mut test = TestProgram::new(input);
        test.run_pass_with_options(&BorrowCheck, options);
        test.assert_error(|e| matches!(e, OptimizeError::DropWhileBorrowed { .. }));
    }

    /// Drop while borrowed is always an error, even in lenient mode.
    #[test]
    fn test_detect_drop_while_borrowed_lenient_mode() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v1 = iconst 42i32
    store v0, v1
    v2 = field.addr v0, 0 -> ref<borrowed i32>
    raw.drop v0
    v3 = load v2 -> i32
    return v3
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&BorrowCheck);
        test.assert_error(|e| matches!(e, OptimizeError::DropWhileBorrowed { .. }));
    }

    /// Single field access is valid.
    #[test]
    fn test_verify_single_field_access() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v1 = iconst 42i32
    store v0, v1
    v2 = load v0 -> i32
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&BorrowCheck);
        test.assert_no_errors();
    }

    /// Load through reference is valid.
    #[test]
    fn test_verify_load() {
        let input = r#"function @test(v0: ref<raw i32>) -> i32 {
block0(v0: ref<raw i32>):
    v1 = load v0 -> i32
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&BorrowCheck);
        test.assert_no_errors();
    }

    /// Multiple sequential stores to same location are valid.
    #[test]
    fn test_verify_multiple_stores_sequential() {
        let input = r#"function @test(v0: ref<raw i32>, v1: i32, v2: i32) -> void {
block0(v0: ref<raw i32>, v1: i32, v2: i32):
    store v0, v1
    store v0, v2
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&BorrowCheck);
        test.assert_no_errors();
    }

    /// Borrow expires when reference is no longer used.
    ///
    /// The reference v2 is not used after the load, so the borrow expires
    /// and dropping v0 is valid.
    #[test]
    fn test_verify_borrow_expires_after_use() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v1 = iconst 42i32
    store v0, v1
    v2 = field.addr v0, 0 -> ref<borrowed i32>
    v3 = load v2 -> i32
    raw.drop v0
    return v3
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&BorrowCheck);
        // borrow of v0 through v2 expires after v3 = load v2 (v2 is dead)
        // so drop v0 is valid
        test.assert_no_errors();
    }

    /// Borrow does not expire if reference is still live.
    #[test]
    fn test_detect_borrow_still_live() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v1 = iconst 42i32
    store v0, v1
    v2 = field.addr v0, 0 -> ref<borrowed i32>
    raw.drop v0
    v3 = load v2 -> i32
    return v3
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&BorrowCheck);
        // v2 is still live at drop v0 (used in v3 = load v2)
        // so this should be an error
        test.assert_error(|e| matches!(e, OptimizeError::DropWhileBorrowed { .. }));
    }

    /// Element address also creates borrow that tracks lifetime.
    #[test]
    fn test_verify_element_addr_borrow_expires() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v1 = iconst 42i32
    store v0, v1
    v2 = iconst 0i32
    v3 = element.addr v0, v2 -> ref<borrowed i32>
    v4 = load v3 -> i32
    raw.drop v0
    return v4
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&BorrowCheck);
        // borrow expires after v4 = load v3
        test.assert_no_errors();
    }

    /// Direct field address borrow is tracked even with intermediate operations.
    #[test]
    fn test_detect_direct_borrow_still_live() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v1 = iconst 42i32
    store v0, v1
    v2 = field.addr v0, 0 -> ref<borrowed i32>
    v3 = iconst 10i32
    raw.drop v0
    v4 = load v2 -> i32
    return v4
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&BorrowCheck);
        // v2 is still live at drop v0 (used in v4 = load v2)
        test.assert_error(|e| matches!(e, OptimizeError::DropWhileBorrowed { .. }));
    }

    /// Chained field.addr tracks transitive provenance.
    ///
    /// v3 = field.addr v2 where v2 = field.addr v0 means dropping v0 while
    /// v3 is live should be an error.
    #[test]
    fn test_detect_transitive_borrow_chain() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v1 = iconst 42i32
    store v0, v1
    v2 = field.addr v0, 0 -> ref<borrowed i32>
    v3 = field.addr v2, 0 -> ref<borrowed i32>
    raw.drop v0
    v4 = load v3 -> i32
    return v4
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&BorrowCheck);
        // v3 transitively borrows from v0 via v2, so drop v0 is error
        test.assert_error(|e| matches!(e, OptimizeError::DropWhileBorrowed { .. }));
    }

    /// Deep transitive chain (v0 -> v2 -> v3 -> v4) tracks correctly.
    #[test]
    fn test_detect_deep_transitive_chain() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v1 = iconst 42i32
    store v0, v1
    v2 = field.addr v0, 0 -> ref<borrowed i32>
    v3 = field.addr v2, 0 -> ref<borrowed i32>
    v4 = field.addr v3, 0 -> ref<borrowed i32>
    raw.drop v0
    v5 = load v4 -> i32
    return v5
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&BorrowCheck);
        // v4 transitively borrows from v0 via v3 -> v2 -> v0
        test.assert_error(|e| matches!(e, OptimizeError::DropWhileBorrowed { .. }));
    }

    /// Transitive borrow expires when intermediate references are dead.
    #[test]
    fn test_verify_transitive_borrow_expires() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v1 = iconst 42i32
    store v0, v1
    v2 = field.addr v0, 0 -> ref<borrowed i32>
    v3 = field.addr v2, 0 -> ref<borrowed i32>
    v4 = load v3 -> i32
    raw.drop v0
    return v4
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&BorrowCheck);
        // v3 is dead after v4 = load v3, so transitive borrow expires
        // drop v0 is valid
        test.assert_no_errors();
    }

    /// Dropping intermediate value in chain is still error if final ref is live.
    #[test]
    fn test_detect_drop_intermediate_while_borrowed() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v1 = iconst 42i32
    store v0, v1
    v2 = field.addr v0, 0 -> ref<borrowed i32>
    v3 = field.addr v2, 0 -> ref<borrowed i32>
    raw.drop v2
    v4 = load v3 -> i32
    return v4
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&BorrowCheck);
        // dropping v2 while v3 (which borrows from v2) is live is error
        test.assert_error(|e| matches!(e, OptimizeError::DropWhileBorrowed { .. }));
    }

    /// Mutable borrow while existing mutable borrow is detected in strict mode.
    #[test]
    fn test_detect_mutable_borrow_conflict_strict() {
        let input = r#"function @test(v0: ref<borrowed mut i32>) -> i32 {
block0(v0: ref<borrowed mut i32>):
    v1 = field.addr v0, 0 -> ref<borrowed mut i32>
    v2 = field.addr v0, 0 -> ref<borrowed mut i32>
    v3 = load v1 -> i32
    v4 = load v2 -> i32
    v5 = iadd v3, v4
    return v5
}"#;

        let mut options = crate::optimize::PipelineOptions::default();
        options.strict_borrow_mode = true;

        let mut test = TestProgram::new(input);
        test.run_pass_with_options(&BorrowCheck, options);
        // v2 = field.addr while v1 still borrows v0 - both are mutable since v0 is mut
        // mutable borrow while existing mutable borrow is conflict
        test.assert_error(|e| matches!(e, OptimizeError::ConflictingBorrow { .. }));
    }

    /// Shared borrows don't conflict with each other.
    #[test]
    fn test_verify_multiple_shared_borrows() {
        let input = r#"function @test(v0: ref<borrowed i32>) -> i32 {
block0(v0: ref<borrowed i32>):
    v1 = field.addr v0, 0 -> ref<borrowed i32>
    v2 = field.addr v0, 0 -> ref<borrowed i32>
    v3 = load v1 -> i32
    v4 = load v2 -> i32
    v5 = iadd v3, v4
    return v5
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&BorrowCheck);
        // multiple shared borrows of same origin is valid
        test.assert_no_errors();
    }

    /// Borrow that flows through diamond control flow.
    #[test]
    fn test_detect_borrow_through_diamond() {
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v2 = iconst 42i32
    store v1, v2
    v3 = field.addr v1, 0 -> ref<borrowed i32>
    branch v0, block1, block2
block1:
    jump block3
block2:
    jump block3
block3:
    raw.drop v1
    v4 = load v3 -> i32
    return v4
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&BorrowCheck);
        // v3 is still live at drop v1 after both paths converge
        test.assert_error(|e| matches!(e, OptimizeError::DropWhileBorrowed { .. }));
    }

    /// Borrow created on one branch only - dropping after merge is error.
    /// On block1 path: v1 is borrowed via v3 which is passed to block3
    /// On block2 path: v2 is borrowed (not v1)
    /// At merge: v1 is "maybe borrowed" - dropping v1 should error
    #[test]
    fn test_detect_maybe_borrowed_one_branch() {
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v2 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v3 = iconst 42i32
    store v1, v3
    store v2, v3
    branch v0, block1, block2
block1:
    v4 = field.addr v1, 0 -> ref<borrowed i32>
    jump block3(v4)
block2:
    v5 = field.addr v2, 0 -> ref<borrowed i32>
    jump block3(v5)
block3(v6: ref<raw i32>):
    raw.drop v1
    v7 = load v6 -> i32
    raw.drop v2
    return v7
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&BorrowCheck);
        // v1 is borrowed via v4 on block1 path only
        // v1 is NOT borrowed on block2 path (v5 borrows v2 instead)
        // at merge (block3), v1 is MaybeBorrowed
        // drop v1 while MaybeBorrowed should error
        test.assert_error(|e| matches!(e, OptimizeError::DropWhileBorrowed { .. }));
    }

    /// Borrow expires before merge - drop after merge is ok.
    #[test]
    fn test_verify_borrow_expires_before_merge() {
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v2 = iconst 42i32
    store v1, v2
    branch v0, block1, block2
block1:
    v3 = field.addr v1, 0 -> ref<borrowed i32>
    v4 = load v3 -> i32
    jump block3(v4)
block2:
    v5 = field.addr v1, 0 -> ref<borrowed i32>
    v6 = load v5 -> i32
    jump block3(v6)
block3(v7: i32):
    raw.drop v1
    return v7
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&BorrowCheck);
        // borrows v3 and v5 are consumed by loads before the jumps
        // v7 is just an i32, not a reference, so no borrow at drop
        test.assert_no_errors();
    }

    /// Loop with borrow - borrow created and used within loop body.
    #[test]
    fn test_verify_loop_borrow_local() {
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    store v1, v0
    jump block1
block1:
    v2 = field.addr v1, 0 -> ref<borrowed i32>
    v3 = load v2 -> i32
    v4 = iconst 0i32
    v5 = icmp_sgt v3, v4
    branch v5, block2, block3
block2:
    v6 = iconst 1i32
    v7 = isub v3, v6
    store v1, v7
    jump block1
block3:
    v8 = load v1 -> i32
    raw.drop v1
    return v8
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&BorrowCheck);
        // v2 borrow is used immediately by v3=load, then dead
        // loop is fine because borrow doesn't escape iteration
        test.assert_no_errors();
    }

    /// Borrow escapes loop iteration - error.
    #[test]
    fn test_detect_borrow_escapes_loop() {
        let input = r#"function @test(v0: i32) -> ref<raw i32> {
block0(v0: i32):
    v1 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    store v1, v0
    v2 = field.addr v1, 0 -> ref<borrowed i32>
    jump block1(v2)
block1(v3: ref<raw i32>):
    v4 = load v3 -> i32
    v5 = iconst 0i32
    v6 = icmp_sgt v4, v5
    branch v6, block2, block3
block2:
    jump block1(v3)
block3:
    return v3
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&BorrowCheck);
        // v3 is a borrow of v1 that escapes via return
        // (this is actually a stack escape, but borrow check sees it too)
        // The borrow flows through the loop and returns
        test.assert_no_errors(); // borrow check doesn't catch return escape (stack_check does)
    }

    /// Different allocations don't alias - borrows from separate allocations don't conflict.
    ///
    /// Even with mutable borrows, references to different allocations are independent
    /// because alias analysis proves they cannot refer to the same memory.
    #[test]
    fn test_alias_different_allocations_no_conflict() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v1 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v2 = iconst 1i32
    v3 = iconst 2i32
    store v0, v2
    store v1, v3
    v4 = field.addr v0, 0 -> ref<borrowed i32>
    v5 = field.addr v1, 0 -> ref<borrowed i32>
    v6 = load v4 -> i32
    v7 = load v5 -> i32
    v8 = iadd v6, v7
    raw.drop v0
    raw.drop v1
    return v8
}"#;

        let mut options = crate::optimize::PipelineOptions::default();
        options.strict_borrow_mode = true;

        let mut test = TestProgram::new(input);
        test.run_pass_with_options(&BorrowCheck, options);
        // v4 borrows v0, v5 borrows v1 - different allocations don't conflict
        test.assert_no_errors();
    }

    /// Store through one pointer doesn't invalidate borrow through unrelated pointer.
    ///
    /// Alias analysis proves the pointers cannot refer to overlapping memory,
    /// so storing through one doesn't invalidate borrows of the other.
    #[test]
    fn test_alias_store_unrelated_pointers_no_conflict() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v1 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v2 = iconst 1i32
    store v0, v2
    store v1, v2
    v3 = field.addr v0, 0 -> ref<borrowed i32>
    v4 = iconst 99i32
    store v1, v4
    v5 = load v3 -> i32
    raw.drop v0
    raw.drop v1
    return v5
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&BorrowCheck);
        // store to v1 doesn't invalidate borrow of v0 - different allocations
        test.assert_no_errors();
    }

    /// Concurrent mutable borrows of different allocations are valid.
    ///
    /// Even in strict mode, mutable borrows to different allocations don't conflict
    /// because alias analysis proves they refer to different memory.
    #[test]
    fn test_alias_concurrent_mutable_borrows_different_allocs() {
        let input = r#"function @test(v0: ref<borrowed mut i32>, v1: ref<borrowed mut i32>) -> i32 {
block0(v0: ref<borrowed mut i32>, v1: ref<borrowed mut i32>):
    v2 = field.addr v0, 0 -> ref<borrowed i32>
    v3 = field.addr v1, 0 -> ref<borrowed i32>
    v4 = iconst 42i32
    store v2, v4
    store v3, v4
    v5 = load v0 -> i32
    v6 = load v1 -> i32
    v7 = iadd v5, v6
    return v7
}"#;

        let mut options = crate::optimize::PipelineOptions::default();
        options.strict_borrow_mode = true;

        let mut test = TestProgram::new(input);
        test.run_pass_with_options(&BorrowCheck, options);
        // noalias params in strict mode means v0 and v1 don't alias
        // so mutable borrows from both don't conflict
        test.assert_no_errors();
    }

    /// Drop of unrelated allocation while another allocation is borrowed is valid.
    ///
    /// Alias analysis proves the dropped value doesn't alias the borrowed value's origin.
    #[test]
    fn test_alias_drop_unrelated_allocation_while_other_borrowed() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v1 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v2 = iconst 42i32
    store v0, v2
    store v1, v2
    v3 = field.addr v0, 0 -> ref<borrowed i32>
    raw.drop v1
    v4 = load v3 -> i32
    raw.drop v0
    return v4
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&BorrowCheck);
        // dropping v1 while v0 is borrowed via v3 is fine - different allocations
        test.assert_no_errors();
    }

    /// Global address doesn't alias stack allocation.
    ///
    /// Alias analysis proves globals and stack allocations are in different
    /// address spaces.
    #[test]
    fn test_alias_global_vs_stack_no_conflict() {
        let input = r#"global @g1: i32 = 42i32
function @test() -> i32 {
block0:
    v0 = global.addr @g1 -> ref<raw addrspace(global) i32>
    v1 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v2 = iconst 1i32
    store v1, v2
    v3 = field.addr v0, 0 -> ref<borrowed i32>
    v4 = field.addr v1, 0 -> ref<borrowed i32>
    v5 = load v3 -> i32
    v6 = load v4 -> i32
    raw.drop v1
    v7 = iadd v5, v6
    return v7
}"#;

        let mut options = crate::optimize::PipelineOptions::default();
        options.strict_borrow_mode = true;

        let mut test = TestProgram::new(input);
        test.run_pass_with_options(&BorrowCheck, options);
        // global and stack allocation don't alias - no conflict
        test.assert_no_errors();
    }

    /// Managed and raw allocations don't alias each other.
    ///
    /// Different allocation types in the same function are independent.
    #[test]
    fn test_alias_managed_vs_raw_no_conflict() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = managed.alloc i32 -> ref<managed i32>
    v1 = raw.alloc i32 -> ref<raw i32>
    v2 = iconst 42i32
    store v0, v2
    store v1, v2
    v3 = field.addr v0, 0 -> ref<borrowed i32>
    v4 = field.addr v1, 0 -> ref<borrowed i32>
    v5 = load v3 -> i32
    v6 = load v4 -> i32
    raw.free v1
    v7 = iadd v5, v6
    return v7
}"#;

        let mut options = crate::optimize::PipelineOptions::default();
        options.strict_borrow_mode = true;

        let mut test = TestProgram::new(input);
        test.run_pass_with_options(&BorrowCheck, options);
        // managed and raw allocations don't alias - no conflict
        test.assert_no_errors();
    }

    /// Multiple sequential allocations in same block don't alias.
    ///
    /// Each allocation instruction creates a fresh allocation that cannot
    /// alias any previous allocation.
    #[test]
    fn test_alias_sequential_allocations_no_conflict() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v1 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v2 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v3 = iconst 1i32
    v4 = iconst 2i32
    v5 = iconst 3i32
    store v0, v3
    store v1, v4
    store v2, v5
    v6 = field.addr v0, 0 -> ref<borrowed i32>
    v7 = field.addr v1, 0 -> ref<borrowed i32>
    v8 = field.addr v2, 0 -> ref<borrowed i32>
    v9 = load v6 -> i32
    v10 = load v7 -> i32
    v11 = load v8 -> i32
    v12 = iadd v9, v10
    v13 = iadd v12, v11
    raw.drop v0
    raw.drop v1
    raw.drop v2
    return v13
}"#;

        let mut options = crate::optimize::PipelineOptions::default();
        options.strict_borrow_mode = true;

        let mut test = TestProgram::new(input);
        test.run_pass_with_options(&BorrowCheck, options);
        // all three allocations are independent - no conflicts
        test.assert_no_errors();
    }

    /// Same allocation, same field access - detects conflict for mutable borrows.
    ///
    /// When two field.addr instructions access the same allocation at the same field,
    /// mutable borrows conflict because they may alias the same memory.
    #[test]
    fn test_alias_same_allocation_same_field_conflict() {
        // in strict mode with mutable ref, same field access conflicts
        let input_mut = r#"function @test(v0: ref<borrowed mut i32>) -> i32 {
block0(v0: ref<borrowed mut i32>):
    v1 = field.addr v0, 0 -> ref<borrowed mut i32>
    v2 = field.addr v0, 0 -> ref<borrowed mut i32>
    v3 = load v1 -> i32
    v4 = load v2 -> i32
    v5 = iadd v3, v4
    return v5
}"#;

        let mut options = crate::optimize::PipelineOptions::default();
        options.strict_borrow_mode = true;

        let mut test = TestProgram::new(input_mut);
        test.run_pass_with_options(&BorrowCheck, options);
        // v1 and v2 both borrow from v0 mutably - same allocation, same field
        test.assert_error(|e| matches!(e, OptimizeError::ConflictingBorrow { .. }));
    }

    /// Store through aliasing pointer invalidates borrow.
    ///
    /// When a store may alias an existing borrow's origin, the borrow is invalidated.
    #[test]
    fn test_alias_store_invalidates_aliasing_borrow() {
        let input = r#"function @test(v0: ref<borrowed mut i32>) -> i32 {
block0(v0: ref<borrowed mut i32>):
    v1 = field.addr v0, 0 -> ref<borrowed i32>
    v2 = iconst 99i32
    store v0, v2
    v3 = load v1 -> i32
    return v3
}"#;

        let mut options = crate::optimize::PipelineOptions::default();
        options.strict_borrow_mode = true;

        let mut test = TestProgram::new(input);
        test.run_pass_with_options(&BorrowCheck, options);
        // store to v0 may alias v1's origin (v0), so it invalidates the borrow
        // this should emit an error in strict mode
        test.assert_error(|e| matches!(e, OptimizeError::InvalidatedReference { .. }));
    }

    /// Drop of aliasing value while borrowed is detected.
    ///
    /// Alias analysis detects when a dropped value may alias an existing borrow origin.
    #[test]
    fn test_alias_drop_aliasing_value_detected() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v1 = iconst 42i32
    store v0, v1
    v2 = field.addr v0, 0 -> ref<borrowed i32>
    v3 = field.addr v0, 0 -> ref<borrowed i32>
    raw.drop v0
    v4 = load v2 -> i32
    v5 = load v3 -> i32
    v6 = iadd v4, v5
    return v6
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&BorrowCheck);
        // dropping v0 while v2 and v3 borrow from it is detected
        test.assert_error(|e| matches!(e, OptimizeError::DropWhileBorrowed { .. }));
    }

    /// Cast doesn't break alias tracking.
    ///
    /// Pointer casts preserve provenance for alias analysis.
    #[test]
    fn test_alias_cast_preserves_provenance() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v1 = iconst 42i32
    store v0, v1
    v2 = bitcast v0 -> ref<raw i32>
    v3 = field.addr v2, 0 -> ref<borrowed i32>
    raw.drop v0
    v4 = load v3 -> i32
    return v4
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&BorrowCheck);
        // v3 borrows from v2 which is a cast of v0
        // dropping v0 while v3 is live should be detected
        test.assert_error(|e| matches!(e, OptimizeError::DropWhileBorrowed { .. }));
    }

    /// Call to function with inferred single-param lifetime tracks borrow correctly.
    ///
    /// When calling a function that returns a borrowed reference and has one
    /// borrowed parameter, the return value borrows from that argument.
    #[test]
    fn test_track_call_borrow_from_single_param() {
        let input = r#"function @identity(v0: ref<borrowed i32>) -> ref<borrowed i32> {
block0(v0: ref<borrowed i32>):
    return v0
}

function @test() -> i32 {
block0:
    v0 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v1 = iconst 42i32
    store v0, v1
    v2 = field.addr v0, 0 -> ref<borrowed i32>
    v3 = call @identity(v2) -> fn(ref<borrowed i32>) -> ref<borrowed i32>
    raw.drop v0
    v4 = load v3 -> i32
    return v4
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&BorrowCheck);

        // v3 = call @identity(v2) -> fn(ref<borrowed i32>) -> ref<borrowed i32> returns a borrow of v2 which borrows from v0
        // dropping v0 while v3 is live should be an error
        test.assert_error(|e| matches!(e, OptimizeError::DropWhileBorrowed { .. }));
    }

    /// Call result with static lifetime doesn't track borrow from arguments.
    ///
    /// When a function has explicit static lifetime, the return value doesn't
    /// borrow from any arguments.
    #[test]
    fn test_track_call_static_lifetime_no_borrow() {
        let input = r#"function @getStatic(v0: ref<borrowed i32>) -> ref<borrowed i32> {
block0(v0: ref<borrowed i32>):
    return v0
}

function @test() -> i32 {
block0:
    v0 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v1 = iconst 42i32
    store v0, v1
    v2 = field.addr v0, 0 -> ref<borrowed i32>
    v3 = call @getStatic(v2) -> fn(ref<borrowed i32>) -> ref<borrowed i32>
    raw.drop v0
    v4 = load v3 -> i32
    return v4
}"#;

        let mut test = TestProgram::new(input);
        test.set_function_lifetime("getStatic", mir::Lifetime::Static);
        test.run_pass(&BorrowCheck);

        // with static lifetime, v3 doesn't borrow from v2
        // so dropping v0 while v3 is live is okay (from borrow perspective)
        // (the load v4 would be use-after-free, but borrow check doesn't catch that, see move-check)
        test.assert_no_errors();
    }

    /// Call with explicit param lifetime tracks only specified params.
    ///
    /// When a function has explicit lifetime annotation specifying which params
    /// the return borrows from, only those params are tracked.
    #[test]
    fn test_track_call_explicit_param_lifetime() {
        let input = r#"function @pickFirst(v0: ref<borrowed i32>, v1: ref<borrowed i32>) -> ref<borrowed i32> {
block0(v0: ref<borrowed i32>, v1: ref<borrowed i32>):
    return v0
}

function @test() -> i32 {
block0:
    v0 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v1 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v2 = iconst 42i32
    store v0, v2
    store v1, v2
    v3 = field.addr v0, 0 -> ref<borrowed i32>
    v4 = field.addr v1, 0 -> ref<borrowed i32>
    v5 = call @pickFirst(v3, v4) -> fn(ref<borrowed i32>, ref<borrowed i32>) -> ref<borrowed i32>
    raw.drop v1
    v6 = load v5 -> i32
    raw.drop v0
    return v6
}"#;

        let mut test = TestProgram::new(input);
        test.set_function_lifetime("pickFirst", mir::Lifetime::param(0));
        test.run_pass(&BorrowCheck);

        // v5 only borrows from v3 (param 0), not v4 (param 1)
        // so dropping v1 while v5 is live is okay (v5 doesn't borrow from v4 which borrows v1)
        test.assert_no_errors();
    }

    /// Call with conservative multi-param inference tracks all borrowed params.
    ///
    /// When a function has multiple borrowed params and no explicit annotation,
    /// the return may borrow from all of them.
    #[test]
    fn test_track_call_conservative_multi_param() {
        let input = r#"function @pick_any(v0: ref<borrowed i32>, v1: ref<borrowed i32>) -> ref<borrowed i32> {
block0(v0: ref<borrowed i32>, v1: ref<borrowed i32>):
    return v0
}

function @test() -> i32 {
block0:
    v0 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v1 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v2 = iconst 42i32
    store v0, v2
    store v1, v2
    v3 = field.addr v0, 0 -> ref<borrowed i32>
    v4 = field.addr v1, 0 -> ref<borrowed i32>
    v5 = call @pick_any(v3, v4) -> fn(ref<borrowed i32>, ref<borrowed i32>) -> ref<borrowed i32>
    raw.drop v1
    v6 = load v5 -> i32
    raw.drop v0
    return v6
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&BorrowCheck);

        // with conservative inference, v5 may borrow from both v3 and v4
        // dropping v1 while v5 is live is an error (v5 may borrow from v4 which borrows v1)
        test.assert_error(|e| matches!(e, OptimizeError::DropWhileBorrowed { .. }));
    }

    /// Call returning non-borrowed type doesn't track any borrows.
    ///
    /// When a function returns a non-borrowed type (like i32), no borrow
    /// tracking is needed for the call.
    #[test]
    fn test_track_call_no_borrow_for_value_return() {
        let input = r#"function @deref(v0: ref<borrowed i32>) -> i32 {
block0(v0: ref<borrowed i32>):
    v1 = load v0 -> i32
    return v1
}

function @test() -> i32 {
block0:
    v0 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v1 = iconst 42i32
    store v0, v1
    v2 = field.addr v0, 0 -> ref<borrowed i32>
    v3 = call @deref(v2) -> fn(ref<borrowed i32>) -> i32
    raw.drop v0
    return v3
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&BorrowCheck);

        // v3 is an i32, not a reference, so no borrow tracking needed
        // dropping v0 after v2 is consumed by the call is okay
        test.assert_no_errors();
    }

    /// Borrow expires after call return is consumed.
    ///
    /// The borrow from a call return expires when the returned reference
    /// is no longer live, allowing the original value to be dropped.
    #[test]
    fn test_track_call_borrow_expires_after_use() {
        let input = r#"function @identity(v0: ref<borrowed i32>) -> ref<borrowed i32> {
block0(v0: ref<borrowed i32>):
    return v0
}

function @test() -> i32 {
block0:
    v0 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v1 = iconst 42i32
    store v0, v1
    v2 = field.addr v0, 0 -> ref<borrowed i32>
    v3 = call @identity(v2) -> fn(ref<borrowed i32>) -> ref<borrowed i32>
    v4 = load v3 -> i32
    raw.drop v0
    return v4
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&BorrowCheck);

        // v3 borrow expires after v4 = load v3 (v3 is dead)
        // dropping v0 is okay
        test.assert_no_errors();
    }

    /// Explicit multi-param lifetime tracks all specified parameters.
    ///
    /// When a function has explicit lifetime annotation for multiple params,
    /// the return may borrow from any of them.
    #[test]
    fn test_track_call_explicit_multi_param_lifetime() {
        let input = r#"function @pickEither(v0: ref<borrowed i32>, v1: ref<borrowed i32>) -> ref<borrowed i32> {
block0(v0: ref<borrowed i32>, v1: ref<borrowed i32>):
    return v0
}

function @test() -> i32 {
block0:
    v0 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v1 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v2 = iconst 42i32
    store v0, v2
    store v1, v2
    v3 = field.addr v0, 0 -> ref<borrowed i32>
    v4 = field.addr v1, 0 -> ref<borrowed i32>
    v5 = call @pickEither(v3, v4) -> fn(ref<borrowed i32>, ref<borrowed i32>) -> ref<borrowed i32>
    raw.drop v0
    v6 = load v5 -> i32
    raw.drop v1
    return v6
}"#;

        let mut test = TestProgram::new(input);

        // explicit lifetime: borrows from both param 0 and param 1
        test.set_function_lifetime("pickEither", mir::Lifetime::params([0, 1]));
        test.run_pass(&BorrowCheck);

        // v5 may borrow from v3 (param 0), dropping v0 while v5 is live is an error
        test.assert_error(|e| matches!(e, OptimizeError::DropWhileBorrowed { .. }));
    }

    /// Explicit lifetime param(1) allows dropping param 0's origin.
    ///
    /// When a function explicitly states return borrows from param 1,
    /// dropping param 0's origin is safe.
    #[test]
    fn test_track_call_explicit_second_param_lifetime() {
        let input = r#"function @pickSecond(v0: ref<borrowed i32>, v1: ref<borrowed i32>) -> ref<borrowed i32> {
block0(v0: ref<borrowed i32>, v1: ref<borrowed i32>):
    return v1
}

function @test() -> i32 {
block0:
    v0 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v1 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v2 = iconst 42i32
    store v0, v2
    store v1, v2
    v3 = field.addr v0, 0 -> ref<borrowed i32>
    v4 = field.addr v1, 0 -> ref<borrowed i32>
    v5 = call @pickSecond(v3, v4) -> fn(ref<borrowed i32>, ref<borrowed i32>) -> ref<borrowed i32>
    raw.drop v0
    v6 = load v5 -> i32
    raw.drop v1
    return v6
}"#;

        let mut test = TestProgram::new(input);

        // explicit lifetime: borrows only from param 1
        test.set_function_lifetime("pickSecond", mir::Lifetime::param(1));
        test.run_pass(&BorrowCheck);

        // v5 borrows from v4 (param 1), not v3 (param 0)
        // dropping v0 is okay, v1 must stay live until after v5 is used
        test.assert_no_errors();
    }

    /// Call returning mutable borrowed reference propagates mutability.
    ///
    /// When a function returns ref<borrowed mut T>, the returned borrow
    /// should be tracked as mutable and conflict with other borrows.
    #[test]
    fn test_track_call_mutable_return_conflicts() {
        let input = r#"function @getMut(v0: ref<borrowed mut i32>) -> ref<borrowed mut i32> {
block0(v0: ref<borrowed mut i32>):
    return v0
}

function @test(v0: ref<borrowed mut i32>) -> i32 {
block0(v0: ref<borrowed mut i32>):
    v1 = call @getMut(v0) -> fn(ref<borrowed mut i32>) -> ref<borrowed mut i32>
    v2 = field.addr v0, 0 -> ref<borrowed i32>
    v3 = load v1 -> i32
    v4 = load v2 -> i32
    v5 = iadd v3, v4
    return v5
}"#;

        let mut options = crate::optimize::PipelineOptions::default();
        options.strict_borrow_mode = true;

        let mut test = TestProgram::new(input);
        test.run_pass_with_options(&BorrowCheck, options);

        // v1 = call @getMut(v0) -> fn(ref<borrowed mut i32>) -> ref<borrowed mut i32> returns a mutable borrow from v0
        // v2 = field.addr v0, 0 creates another mutable borrow of v0
        // these should conflict in strict mode
        test.assert_error(|e| matches!(e, OptimizeError::ConflictingBorrow { .. }));
    }

    /// Call returning shared borrowed reference allows concurrent shared borrows.
    ///
    /// When a function returns ref<borrowed T> (shared), the returned borrow
    /// should be tracked as shared and not conflict with other shared borrows.
    #[test]
    fn test_track_call_shared_return_no_conflict() {
        let input = r#"function @getShared(v0: ref<borrowed i32>) -> ref<borrowed i32> {
block0(v0: ref<borrowed i32>):
    return v0
}

function @test(v0: ref<borrowed i32>) -> i32 {
block0(v0: ref<borrowed i32>):
    v1 = call @getShared(v0) -> fn(ref<borrowed i32>) -> ref<borrowed i32>
    v2 = field.addr v0, 0 -> ref<borrowed i32>
    v3 = load v1 -> i32
    v4 = load v2 -> i32
    v5 = iadd v3, v4
    return v5
}"#;

        let mut options = crate::optimize::PipelineOptions::default();
        options.strict_borrow_mode = true;

        let mut test = TestProgram::new(input);
        test.run_pass_with_options(&BorrowCheck, options);

        // v1 = call @getShared(v0) -> fn(ref<borrowed i32>) -> ref<borrowed i32> returns a shared borrow from v0
        // v2 = field.addr v0, 0 creates another shared borrow of v0
        // shared + shared is okay
        test.assert_no_errors();
    }

    /// Mutable borrow from call conflicts with subsequent mutable field.addr.
    ///
    /// Tests that mutability is correctly propagated through the call and
    /// detected when creating another mutable borrow.
    #[test]
    fn test_track_call_mutable_borrow_then_field_addr_conflict() {
        let input = r#"function @getMutRef(v0: ref<borrowed mut i32>) -> ref<borrowed mut i32> {
block0(v0: ref<borrowed mut i32>):
    v1 = field.addr v0, 0 -> ref<borrowed i32>
    return v1
}

function @test() -> i32 {
block0:
    v0 = stack.alloc i32 -> ref<raw addrspace(stack) i32>
    v1 = iconst 42i32
    store v0, v1
    v2 = call @getMutRef(v0) -> fn(ref<borrowed mut i32>) -> ref<borrowed mut i32>
    v3 = field.addr v0, 0 -> ref<borrowed i32>
    v4 = load v2 -> i32
    v5 = load v3 -> i32
    v6 = iadd v4, v5
    raw.drop v0
    return v6
}"#;

        let mut options = crate::optimize::PipelineOptions::default();
        options.strict_borrow_mode = true;

        let mut test = TestProgram::new(input);
        test.run_pass_with_options(&BorrowCheck, options);

        // v2 is a mutable borrow returned from call
        // v3 = field.addr v0 creates another borrow from v0
        // since v0 is stack.alloc (not mut ref param), the field.addr creates
        // a mutable borrow (conservative default), so this should conflict
        test.assert_error(|e| matches!(e, OptimizeError::ConflictingBorrow { .. }));
    }
}
