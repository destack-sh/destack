use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use destack_compiler_macros::declare_pass;
use destack_mir as mir;
use destack_source::{ModuleId, TargetId};
use mir::{Instruction, Mutability, ReferenceKind, Type, Value};

use crate::optimize::common::ValueTypeMap;
use crate::optimize::{
    AliasAnalysis, AnalysisPreservation, BorrowAnalysis, BorrowMap, DiagnosticEmitter,
    FunctionPass, LifetimeAnalysis, LivenessAnalysis, MemoryLocation, PipelineContext,
    ResolvedLifetime,
};
use crate::{OptimizeError, OptimizeWarning};

declare_pass! {
    /// Verify borrow rules for references.
    ///
    /// Tracks active borrows created by `field.address` and `element.address`, and detects:
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
    /// If v3 = field.address v2 and v2 = field.address v0, then v3's provenance is {v2, v0}.
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
    value_types: ValueTypeMap,
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
            value_types: ValueTypeMap::new(function, tree),
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

    /// Determine mutability for a derived reference based on source type.
    ///
    /// For field.address/element.address, the result inherits mutability from source.
    fn derive_mutability(&self, source: Value) -> bool {
        let ty_id = self.value_types.require_value_type(source);
        let ty = self.tree.get(ty_id);
        let Type::Reference { mutability, .. } = ty else {
            panic!("expected reference type for {source:?}, found {ty:?}");
        };

        *mutability == Mutability::Mutable
    }

    /// Determine mutability directly from a reference type.
    fn mutability_from_reference_type(&self, ty_id: mir::LocalNodeId<Type>) -> bool {
        let ty = self.tree.get(ty_id);
        match ty {
            Type::Reference { mutability, .. } | Type::TensorView { mutability, .. } => {
                *mutability == Mutability::Mutable
            }
            _ => {
                panic!("expected reference type for {ty_id:?}, found {ty:?}");
            }
        }
    }

    /// Check if a value has a borrowed reference type.
    fn is_borrowed_reference_value(&self, value: Value) -> bool {
        let ty_id = self.value_types.require_value_type(value);
        let ty = self.tree.get(ty_id);
        ty.is_borrowed_reference()
    }

    /// Check if a reference type is borrowed.
    fn is_borrowed_reference_type(&self, ty_id: mir::LocalNodeId<Type>) -> bool {
        let ty = self.tree.get(ty_id);
        ty.is_borrowed_reference()
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
            .into_anchored(self.module_id, self.target_id)
    }

    /// Build provenance chain for a new borrow.
    ///
    /// If the origin has borrows, we include its provenance chain in ours.
    fn build_provenance(&self, origin: Value) -> HashSet<Value> {
        let mut provenance = HashSet::new();

        // accumulate provenance from all borrows that reference this origin
        for borrow in self.active_borrows.values() {
            if borrow.reference == origin {
                if let Some(parent_origin) = borrow.origin {
                    provenance.insert(parent_origin);
                }
                provenance.extend(borrow.provenance.iter().copied());
            }
        }

        provenance
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

    /// Register a borrow with explicit provenance.
    fn add_borrow_with_provenance(
        &mut self,
        reference: Value,
        origin: Value,
        is_mutable: bool,
        at: mir::LocalNodeId<Instruction>,
        provenance: HashSet<Value>,
    ) {
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

    /// Collect locals borrowed by a reference value.
    fn local_borrows_for_reference(
        &self,
        reference: Value,
    ) -> HashMap<mir::LocalNodeId<mir::Local>, bool> {
        let mut locals = HashMap::new();

        for (local, ids) in &self.local_borrows {
            for id in ids {
                let Some(borrow) = self.active_borrows.get(id) else {
                    continue;
                };
                if borrow.reference == reference {
                    locals
                        .entry(*local)
                        .and_modify(|is_mutable| *is_mutable |= borrow.is_mutable)
                        .or_insert(borrow.is_mutable);
                }
            }
        }

        locals
    }

    /// Propagate borrows from an existing reference to a new reference.
    fn propagate_reference_borrows(
        &mut self,
        destination: Value,
        source: Value,
        at: mir::LocalNodeId<Instruction>,
    ) {
        let mut origins: HashMap<Value, (bool, HashSet<Value>)> = HashMap::new();

        for borrow in self.active_borrows.values() {
            if borrow.reference != source {
                continue;
            }

            let Some(origin) = borrow.origin else {
                continue;
            };

            let entry = origins
                .entry(origin)
                .or_insert((borrow.is_mutable, borrow.provenance.clone()));

            if borrow.is_mutable {
                entry.0 = true;
            }
            entry.1.extend(borrow.provenance.iter().copied());
        }

        for (origin, (is_mutable, provenance)) in origins {
            self.add_borrow_with_provenance(destination, origin, is_mutable, at, provenance);
        }

        let locals = self.local_borrows_for_reference(source);
        for (local, is_mutable) in locals {
            self.add_local_borrow(destination, local, is_mutable, at);
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
    /// - Two field.address to different fields of same struct: NoAlias (no conflict)
    /// - Two field.address to same field: MayAlias/MustAlias (conflict if mutable)
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
        let ty_id = self.value_types.require_value_type(value);
        let ty = self.tree.get(ty_id);
        let kind = match ty {
            Type::Reference { kind, .. } | Type::TensorView { kind, .. } => kind,
            _ => return,
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
                let Some(return_type) = callee.return_type.ty() else {
                    return;
                };
                let return_ty = self.tree.get(return_type);
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
        let target_id = *ctx.target_id();

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

        // process blocks
        for &block_id in &function.blocks {
            let block = tree.get(block_id);

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
                    .any(|param| param.value.value() == Some(borrow.reference));
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
        // field.address creates a borrow of the aggregate
        Instruction::FieldAddr {
            destination,
            aggregate,
            result_type,
            ..
        } => {
            let Some(result_type) = result_type.ty() else {
                return;
            };
            let Some(destination) = destination.value() else {
                return;
            };
            let Some(aggregate) = aggregate.value() else {
                return;
            };

            if !checker.is_borrowed_reference_type(result_type) {
                return;
            }

            // derive mutability from reference type
            let is_mutable = checker.mutability_from_reference_type(result_type);
            checker.check_new_borrow(destination, aggregate, is_mutable, instruction_id, context);
        }

        // element.address creates a borrow of the array
        Instruction::ElementAddr {
            destination,
            array,
            result_type,
            ..
        } => {
            let Some(result_type) = result_type.ty() else {
                return;
            };
            let Some(destination) = destination.value() else {
                return;
            };
            let Some(array) = array.value() else {
                return;
            };

            if !checker.is_borrowed_reference_type(result_type) {
                return;
            }

            // derive mutability from reference type
            let is_mutable = checker.mutability_from_reference_type(result_type);
            checker.check_new_borrow(destination, array, is_mutable, instruction_id, context);
        }

        // store: moves value, invalidates borrows
        Instruction::Store { pointer, value } => {
            let Some(value) = value.value() else {
                return;
            };
            let Some(pointer) = pointer.value() else {
                return;
            };

            // check if value being stored has active borrows (move-while-borrowed)
            checker.check_move_while_borrowed(value, instruction_id, context);
            // storing invalidates other borrows of the same location
            checker.check_mutation_through_reference(pointer, instruction_id, context);
        }

        // local.set mutates the local
        Instruction::LocalSet { local, value } => {
            let Some(value) = value.value() else {
                return;
            };
            let Some(local) = local.local() else {
                return;
            };

            // check if the value being set has active borrows
            checker.check_move_while_borrowed(value, instruction_id, context);
            // check if the local is currently borrowed (mutation through borrow)
            checker.check_local_set_while_borrowed(local, instruction_id, context);
        }

        // local.get loads a value without creating a borrow
        Instruction::LocalGet { .. } => {}

        // local.address creates a borrow of a local
        Instruction::LocalAddr {
            destination,
            local,
            result_type,
        } => {
            let Some(result_type) = result_type.ty() else {
                return;
            };
            let Some(destination) = destination.value() else {
                return;
            };
            let Some(local) = local.local() else {
                return;
            };

            // only references participate in borrow checking
            if !checker.is_borrowed_reference_type(result_type) {
                return;
            }

            let is_mutable = checker.mutability_from_reference_type(result_type);
            checker.check_local_borrow_conflict(local, is_mutable, instruction_id, context);
            checker.add_local_borrow(destination, local, is_mutable, instruction_id);
        }

        // drop invalidates any borrows from this value
        Instruction::Drop { value } => {
            let Some(value) = value.value() else {
                return;
            };

            checker.check_drop_while_borrowed(value, instruction_id, context);
        }

        // raw.free invalidates borrows
        Instruction::RawFree { pointer } => {
            let Some(pointer) = pointer.value() else {
                return;
            };

            checker.check_drop_while_borrowed(pointer, instruction_id, context);
        }

        // pinning does not create or end borrows on its own
        Instruction::Pin { .. } | Instruction::Unpin { .. } => {}

        // load through a reference
        Instruction::Load { .. } => {
            // borrow validity is checked via liveness-based expiry in expire_dead_borrows
        }

        // call: arguments are moved (for owned types)
        Instruction::Call {
            destination,
            function,
            call,
            ..
        } => {
            let args = checker.tree.get_arguments(call.arguments);
            for &arg in args {
                let Some(arg) = arg.value() else {
                    continue;
                };

                checker.check_move_while_borrowed(arg, instruction_id, context);
            }
            if let Some(dest) = destination.and_then(|value| value.value())
                && let Some(function) = function.function()
            {
                let args: Vec<_> = args.iter().filter_map(|arg| arg.value()).collect();

                // track borrows created by the call based on callee's lifetime bounds
                checker.track_call_return_borrows(dest, function, &args, instruction_id);
            }
        }

        Instruction::CallVirtual { call, .. } | Instruction::CallInterface { call, .. } => {
            // callee is used, not moved
            let args = checker.tree.get_arguments(call.arguments);
            for &arg in args {
                let Some(arg) = arg.value() else {
                    continue;
                };

                checker.check_move_while_borrowed(arg, instruction_id, context);
            }
        }
        Instruction::CallIndirect { call, .. } => {
            // callee is used, not moved
            let args = checker.tree.get_arguments(call.arguments);
            for &arg in args {
                let Some(arg) = arg.value() else {
                    continue;
                };

                checker.check_move_while_borrowed(arg, instruction_id, context);
            }
        }

        // field.set moves value into aggregate
        Instruction::FieldSet { value, .. } => {
            let Some(value) = value.value() else {
                return;
            };

            checker.check_move_while_borrowed(value, instruction_id, context);
        }

        // element.set moves value into array
        Instruction::ElementSet { value, .. } => {
            let Some(value) = value.value() else {
                return;
            };

            checker.check_move_while_borrowed(value, instruction_id, context);
        }

        // struct/tuple/array construction moves all fields
        Instruction::Struct { fields, .. } => {
            let field_values = checker.tree.get_arguments(*fields);
            for &field in field_values {
                let Some(field) = field.value() else {
                    continue;
                };

                checker.check_move_while_borrowed(field, instruction_id, context);
            }
        }

        Instruction::Tuple { elements, .. } => {
            let element_values = checker.tree.get_arguments(*elements);
            for &elem in element_values {
                let Some(elem) = elem.value() else {
                    continue;
                };

                checker.check_move_while_borrowed(elem, instruction_id, context);
            }
        }

        Instruction::Array { elements, .. } => {
            let element_values = checker.tree.get_arguments(*elements);
            for &elem in element_values {
                let Some(elem) = elem.value() else {
                    continue;
                };

                checker.check_move_while_borrowed(elem, instruction_id, context);
            }
        }

        // cast preserves borrow provenance for references
        Instruction::Cast {
            destination,
            argument,
            ..
        } => {
            let Some(destination) = destination.value() else {
                return;
            };
            let Some(argument) = argument.value() else {
                return;
            };

            if !checker.is_borrowed_reference_value(destination) {
                return;
            }

            checker.propagate_reference_borrows(destination, argument, instruction_id);
        }

        // select between references merges borrow provenance
        Instruction::Select {
            destination,
            then_value,
            else_value,
            ..
        } => {
            let Some(destination) = destination.value() else {
                return;
            };
            let Some(then_value) = then_value.value() else {
                return;
            };
            let Some(else_value) = else_value.value() else {
                return;
            };

            if !checker.is_borrowed_reference_value(destination) {
                return;
            }

            checker.propagate_reference_borrows(destination, then_value, instruction_id);
            if then_value != else_value {
                checker.propagate_reference_borrows(destination, else_value, instruction_id);
            }
        }

        // allocations produce references
        Instruction::New { .. }
        | Instruction::RawAlloc { .. }
        | Instruction::StackAlloc { .. }
        | Instruction::NewSlice { .. } => {}

        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::OptimizeError;
    use crate::optimize::PipelineOptions;
    use crate::optimize::common::tests::TestProgram;

    /// Build strict borrow mode options for verification tests.
    fn strict_options() -> PipelineOptions {
        PipelineOptions {
            strict_borrow_mode: true,
            ..Default::default()
        }
    }

    /// Simple function with no borrows passes verification.
    #[test]
    fn test_verify_no_borrows() {
        let input = r#"
function test(): int32 {
b0:
    v0: int32 = 42int32
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
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 42int32
    store v0, v1
    v2: int32 = load v0
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&BorrowCheck);
        test.assert_no_errors();
    }

    /// Sequential loads from same pointer is valid.
    #[test]
    fn test_verify_sequential_loads() {
        let input = r#"
function test(v0: ref<int32, raw>): int32 {
b0(v0: ref<int32, raw>):
    v1: int32 = load v0
    v2: int32 = load v0
    v3: int32 = int.add v1, v2
    return v3
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&BorrowCheck);
        test.assert_no_errors();
    }

    /// Store through pointer is valid.
    #[test]
    fn test_verify_store_no_conflict() {
        let input = r#"
function test(v0: ref<int32, raw>, v1: int32): void {
b0(v0: ref<int32, raw>, v1: int32):
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
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: ref<int32, borrowed> = field.address v0, 0
    drop v0
    v2: int32 = load v1
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&BorrowCheck);
        test.assert_error(|e| matches!(e, OptimizeError::DropWhileBorrowed { .. }));
    }

    /// raw.free while borrowed is detected.
    #[test]
    fn test_detect_raw_free_while_borrowed() {
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw> = raw.alloc int32
    v1: ref<int32, borrowed> = field.address v0, 0
    raw.free v0
    v2: int32 = load v1
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&BorrowCheck);
        test.assert_error(|e| matches!(e, OptimizeError::DropWhileBorrowed { .. }));
    }

    /// local.set without existing borrows is valid.
    #[test]
    fn test_verify_local_set_no_borrow() {
        let input = r#"
function test(): int32 {
    local local0: int32, owned
b0:
    v0: int32 = 42int32
    local.set local0, v0
    v1: int32 = local.get local0
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&BorrowCheck);
        test.assert_no_errors();
    }

    /// local.set while borrowed is rejected.
    #[test]
    fn test_detect_local_set_while_borrowed() {
        let input = r#"
function test(): void {
    local local0: int32, owned
b0:
    v0: int32 = 1int32
    local.set local0, v0
    v1: ref<int32, borrowed, space(frame)> = local.address local0
    v2: int32 = 2int32
    local.set local0, v2
    v3: int32 = load v1
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&BorrowCheck);
        test.assert_error(|e| matches!(e, OptimizeError::LocalSetWhileBorrowed { .. }));
    }

    /// Multiple shared local borrows do not conflict.
    #[test]
    fn test_verify_local_shared_borrows_no_conflict() {
        let input = r#"
function test(): void {
    local local0: int32, owned
b0:
    v0: int32 = 0int32
    local.set local0, v0
    v1: ref<int32, borrowed, readonly, space(frame)> = local.address local0
    v2: ref<int32, borrowed, readonly, space(frame)> = local.address local0
    v3: int32 = load v1
    v4: int32 = load v2
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&BorrowCheck);
        test.assert_no_errors();
    }

    /// local borrows propagate through block parameters.
    #[test]
    fn test_detect_local_borrow_propagates_across_blocks() {
        let input = r#"
function test(): void {
    local local0: int32, owned
b0:
    v0: int32 = 1int32
    local.set local0, v0
    v1: ref<int32, borrowed, space(frame)> = local.address local0
    jump b1(v1)
b1(v2: ref<int32, borrowed, space(stack)>):
    v3: int32 = 2int32
    local.set local0, v3
    v4: int32 = load v2
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&BorrowCheck);
        test.assert_error(|e| matches!(e, OptimizeError::LocalSetWhileBorrowed { .. }));
    }

    /// Multiple mutable local borrows conflict.
    #[test]
    fn test_detect_local_borrow_conflict() {
        let input = r#"
function test(): void {
    local local0: int32, owned
b0:
    v0: int32 = 0int32
    local.set local0, v0
    v1: ref<int32, borrowed, space(frame)> = local.address local0
    v2: ref<int32, borrowed, space(frame)> = local.address local0
    v3: int32 = load v1
    v4: int32 = load v2
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&BorrowCheck);
        test.assert_error(|e| matches!(e, OptimizeError::ConflictingBorrow { .. }));
    }

    /// Multiple parameters with no conflicts is valid.
    #[test]
    fn test_verify_multiple_params_no_conflict() {
        let input = r#"
function test(v0: ref<int32, raw>, v1: ref<int32, raw>): int32 {
b0(v0: ref<int32, raw>, v1: ref<int32, raw>):
    v2: int32 = load v0
    v3: int32 = load v1
    v4: int32 = int.add v2, v3
    return v4
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&BorrowCheck);
        test.assert_no_errors();
    }

    /// Drop while borrowed in strict mode emits error.
    #[test]
    fn test_strict_mode_drop_while_borrowed() {
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 42int32
    store v0, v1
    v2: ref<int32, borrowed> = field.address v0, 0
    drop v0
    v3: int32 = load v2
    return v3
}"#;

        let options = strict_options();

        let mut test = TestProgram::new(input);
        test.run_pass_with_options(&BorrowCheck, options);
        test.assert_error(|e| matches!(e, OptimizeError::DropWhileBorrowed { .. }));
    }

    /// Drop while borrowed is always an error, even in lenient mode.
    #[test]
    fn test_detect_drop_while_borrowed_lenient_mode() {
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 42int32
    store v0, v1
    v2: ref<int32, borrowed> = field.address v0, 0
    drop v0
    v3: int32 = load v2
    return v3
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&BorrowCheck);
        test.assert_error(|e| matches!(e, OptimizeError::DropWhileBorrowed { .. }));
    }

    /// Single field access is valid.
    #[test]
    fn test_verify_single_field_access() {
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 42int32
    store v0, v1
    v2: int32 = load v0
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&BorrowCheck);
        test.assert_no_errors();
    }

    /// Load through reference is valid.
    #[test]
    fn test_verify_load() {
        let input = r#"
function test(v0: ref<int32, raw>): int32 {
b0(v0: ref<int32, raw>):
    v1: int32 = load v0
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&BorrowCheck);
        test.assert_no_errors();
    }

    /// Multiple sequential stores to same location are valid.
    #[test]
    fn test_verify_multiple_stores_sequential() {
        let input = r#"
function test(v0: ref<int32, raw>, v1: int32, v2: int32): void {
b0(v0: ref<int32, raw>, v1: int32, v2: int32):
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
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 42int32
    store v0, v1
    v2: ref<int32, borrowed> = field.address v0, 0
    v3: int32 = load v2
    drop v0
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
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 42int32
    store v0, v1
    v2: ref<int32, borrowed> = field.address v0, 0
    drop v0
    v3: int32 = load v2
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
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 42int32
    store v0, v1
    v2: int32 = 0int32
    v3: ref<int32, borrowed> = element.address v0, v2
    v4: int32 = load v3
    drop v0
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
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 42int32
    store v0, v1
    v2: ref<int32, borrowed> = field.address v0, 0
    v3: int32 = 10int32
    drop v0
    v4: int32 = load v2
    return v4
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&BorrowCheck);
        // v2 is still live at drop v0 (used in v4 = load v2)
        test.assert_error(|e| matches!(e, OptimizeError::DropWhileBorrowed { .. }));
    }

    /// Chained field.address tracks transitive provenance.
    ///
    /// v3 = field.address v2 where v2 = field.address v0 means dropping v0 while
    /// v3 is live should be an error.
    #[test]
    fn test_detect_transitive_borrow_chain() {
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 42int32
    store v0, v1
    v2: ref<int32, borrowed> = field.address v0, 0
    v3: ref<int32, borrowed> = field.address v2, 0
    drop v0
    v4: int32 = load v3
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
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 42int32
    store v0, v1
    v2: ref<int32, borrowed> = field.address v0, 0
    v3: ref<int32, borrowed> = field.address v2, 0
    v4: ref<int32, borrowed> = field.address v3, 0
    drop v0
    v5: int32 = load v4
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
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 42int32
    store v0, v1
    v2: ref<int32, borrowed> = field.address v0, 0
    v3: ref<int32, borrowed> = field.address v2, 0
    v4: int32 = load v3
    drop v0
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
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 42int32
    store v0, v1
    v2: ref<int32, borrowed> = field.address v0, 0
    v3: ref<int32, borrowed> = field.address v2, 0
    drop v2
    v4: int32 = load v3
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
        let input = r#"
function test(v0: ref<int32, borrowed>): int32 {
b0(v0: ref<int32, borrowed>):
    v1: ref<int32, borrowed> = field.address v0, 0
    v2: ref<int32, borrowed> = field.address v0, 0
    v3: int32 = load v1
    v4: int32 = load v2
    v5: int32 = int.add v3, v4
    return v5
}"#;

        let options = strict_options();

        let mut test = TestProgram::new(input);
        test.run_pass_with_options(&BorrowCheck, options);
        // v2 = field.address while v1 still borrows v0 - both are mutable since v0 is mut
        // mutable borrow while existing mutable borrow is conflict
        test.assert_error(|e| matches!(e, OptimizeError::ConflictingBorrow { .. }));
    }

    /// Shared borrows don't conflict with each other.
    #[test]
    fn test_verify_multiple_shared_borrows() {
        let input = r#"
function test(v0: ref<int32, borrowed>): int32 {
b0(v0: ref<int32, borrowed>):
    v1: ref<int32, borrowed> = field.address v0, 0
    v2: ref<int32, borrowed> = field.address v0, 0
    v3: int32 = load v1
    v4: int32 = load v2
    v5: int32 = int.add v3, v4
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
        let input = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: int32 = 42int32
    store v1, v2
    v3: ref<int32, borrowed> = field.address v1, 0
    branch v0, b1, b2
b1:
    jump b3
b2:
    jump b3
b3:
    drop v1
    v4: int32 = load v3
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
        let input = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: ref<int32, raw, space(stack)> = stack.alloc int32
    v3: int32 = 42int32
    store v1, v3
    store v2, v3
    branch v0, b1, b2
b1:
    v4: ref<int32, borrowed> = field.address v1, 0
    jump b3(v4)
b2:
    v5: ref<int32, borrowed> = field.address v2, 0
    jump b3(v5)
b3(v6: ref<int32, raw>):
    drop v1
    v7: int32 = load v6
    drop v2
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
        let input = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: int32 = 42int32
    store v1, v2
    branch v0, b1, b2
b1:
    v3: ref<int32, borrowed> = field.address v1, 0
    v4: int32 = load v3
    jump b3(v4)
b2:
    v5: ref<int32, borrowed> = field.address v1, 0
    v6: int32 = load v5
    jump b3(v6)
b3(v7: int32):
    drop v1
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
        let input = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    store v1, v0
    jump b1
b1:
    v2: ref<int32, borrowed> = field.address v1, 0
    v3: int32 = load v2
    v4: int32 = 0int32
    v5: boolean = int.gt.s v3, v4
    branch v5, b2, b3
b2:
    v6: int32 = 1int32
    v7: int32 = int.sub v3, v6
    store v1, v7
    jump b1
b3:
    v8: int32 = load v1
    drop v1
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
        let input = r#"
function test(v0: int32): ref<int32, raw> {
b0(v0: int32):
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    store v1, v0
    v2: ref<int32, borrowed> = field.address v1, 0
    jump b1(v2)
b1(v3: ref<int32, raw>):
    v4: int32 = load v3
    v5: int32 = 0int32
    v6: boolean = int.gt.s v4, v5
    branch v6, b2, b3
b2:
    jump b1(v3)
b3:
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
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: int32 = 1int32
    v3: int32 = 2int32
    store v0, v2
    store v1, v3
    v4: ref<int32, borrowed> = field.address v0, 0
    v5: ref<int32, borrowed> = field.address v1, 0
    v6: int32 = load v4
    v7: int32 = load v5
    v8: int32 = int.add v6, v7
    drop v0
    drop v1
    return v8
}"#;

        let options = strict_options();

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
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: int32 = 1int32
    store v0, v2
    store v1, v2
    v3: ref<int32, borrowed> = field.address v0, 0
    v4: int32 = 99int32
    store v1, v4
    v5: int32 = load v3
    drop v0
    drop v1
    return v5
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&BorrowCheck);
        // store to v1 doesn't invalidate borrow of v0 - different allocations
        test.assert_no_errors();
    }

    /// Concurrent mutable borrows of different allocations are valid.
    ///
    /// Even in strict modeable borrows to different allocations don't conflict
    /// because alias analysis proves they refer to different memory.
    #[test]
    fn test_alias_concurrent_mutable_borrows_different_allocs() {
        let input = r#"
function test(v0: ref<int32, borrowed>, v1: ref<int32, borrowed>): int32 {
b0(v0: ref<int32, borrowed>, v1: ref<int32, borrowed>):
    v2: ref<int32, borrowed> = field.address v0, 0
    v3: ref<int32, borrowed> = field.address v1, 0
    v4: int32 = 42int32
    store v2, v4
    store v3, v4
    v5: int32 = load v0
    v6: int32 = load v1
    v7: int32 = int.add v5, v6
    return v7
}"#;

        let options = strict_options();

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
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: int32 = 42int32
    store v0, v2
    store v1, v2
    v3: ref<int32, borrowed> = field.address v0, 0
    drop v1
    v4: int32 = load v3
    drop v0
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
        let input = r#"
global g1: int32 = 42int32
function test(): int32 {
b0:
    v0: ref<int32, raw, space(global)> = global.address g1
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: int32 = 1int32
    store v1, v2
    v3: ref<int32, borrowed> = field.address v0, 0
    v4: ref<int32, borrowed> = field.address v1, 0
    v5: int32 = load v3
    v6: int32 = load v4
    drop v1
    v7: int32 = int.add v5, v6
    return v7
}"#;

        let options = strict_options();

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
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, managed> = new int32
    v1: ref<int32, raw> = raw.alloc int32
    v2: int32 = 42int32
    store v0, v2
    store v1, v2
    v3: ref<int32, borrowed> = field.address v0, 0
    v4: ref<int32, borrowed> = field.address v1, 0
    v5: int32 = load v3
    v6: int32 = load v4
    raw.free v1
    v7: int32 = int.add v5, v6
    return v7
}"#;

        let options = strict_options();

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
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: ref<int32, raw, space(stack)> = stack.alloc int32
    v3: int32 = 1int32
    v4: int32 = 2int32
    v5: int32 = 3int32
    store v0, v3
    store v1, v4
    store v2, v5
    v6: ref<int32, borrowed> = field.address v0, 0
    v7: ref<int32, borrowed> = field.address v1, 0
    v8: ref<int32, borrowed> = field.address v2, 0
    v9: int32 = load v6
    v10: int32 = load v7
    v11: int32 = load v8
    v12: int32 = int.add v9, v10
    v13: int32 = int.add v12, v11
    drop v0
    drop v1
    drop v2
    return v13
}"#;

        let options = strict_options();

        let mut test = TestProgram::new(input);
        test.run_pass_with_options(&BorrowCheck, options);
        // all three allocations are independent - no conflicts
        test.assert_no_errors();
    }

    /// Same allocation, same field access - detects conflict for mutable borrows.
    ///
    /// When two field.address instructions access the same allocation at the same field,
    /// mutable borrows conflict because they may alias the same memory.
    #[test]
    fn test_alias_same_allocation_same_field_conflict() {
        // in strict mode with mutable ref, same field access conflicts
        let input_mut = r#"
function test(v0: ref<int32, borrowed>): int32 {
b0(v0: ref<int32, borrowed>):
    v1: ref<int32, borrowed> = field.address v0, 0
    v2: ref<int32, borrowed> = field.address v0, 0
    v3: int32 = load v1
    v4: int32 = load v2
    v5: int32 = int.add v3, v4
    return v5
}"#;

        let options = strict_options();

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
        let input = r#"
function test(v0: ref<int32, borrowed>): int32 {
b0(v0: ref<int32, borrowed>):
    v1: ref<int32, borrowed> = field.address v0, 0
    v2: int32 = 99int32
    store v0, v2
    v3: int32 = load v1
    return v3
}"#;

        let options = strict_options();

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
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 42int32
    store v0, v1
    v2: ref<int32, borrowed> = field.address v0, 0
    v3: ref<int32, borrowed> = field.address v0, 0
    drop v0
    v4: int32 = load v2
    v5: int32 = load v3
    v6: int32 = int.add v4, v5
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
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 42int32
    store v0, v1
    v2: ref<int32, raw> = cast.bit v0 -> ref<int32, raw>
    v3: ref<int32, borrowed> = field.address v2, 0
    drop v0
    v4: int32 = load v3
    return v4
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&BorrowCheck);
        // v3 borrows from v2 which is a cast of v0
        // dropping v0 while v3 is live should be detected
        test.assert_error(|e| matches!(e, OptimizeError::DropWhileBorrowed { .. }));
    }

    /// Select between two borrows preserves both origins.
    #[test]
    fn test_select_merges_borrow_origins() {
        let input = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: ref<int32, raw, space(stack)> = stack.alloc int32
    v3: int32 = 10int32
    v4: int32 = 20int32
    store v1, v3
    store v2, v4
    v5: ref<int32, borrowed> = field.address v1, 0
    v6: ref<int32, borrowed> = field.address v2, 0
    v7: ref<int32, borrowed> = select v0, v5, v6
    drop v1
    v8: int32 = load v7
    return v8
}"#;

        let options = strict_options();

        let mut test = TestProgram::new(input);
        test.run_pass_with_options(&BorrowCheck, options);
        // v7 may borrow from v1, so dropping v1 is invalid
        test.assert_error(|e| matches!(e, OptimizeError::DropWhileBorrowed { .. }));
    }

    /// Cast propagation carries borrow info across blocks.
    #[test]
    fn test_cast_propagates_borrow_across_blocks() {
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 42int32
    store v0, v1
    v2: ref<int32, borrowed> = field.address v0, 0
    jump b1(v2)
b1(v3: ref<int32, borrowed>):
    v4: ref<int32, borrowed> = cast.bit v3 -> ref<int32, borrowed>
    drop v0
    v5: int32 = load v4
    return v5
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&BorrowCheck);
        // v4 borrows from v0 through v3, so dropping v0 is invalid
        test.assert_error(|e| matches!(e, OptimizeError::DropWhileBorrowed { .. }));
    }

    /// Local borrows propagate through select.
    #[test]
    fn test_select_propagates_local_borrows() {
        let input = r#"
function test(v0: boolean): int32 {
    local local0: int32, owned
b0(v0: boolean):
    v1: int32 = 42int32
    local.set local0, v1
    v2: ref<int32, borrowed, space(frame)> = local.address local0
    v3: ref<int32, borrowed> = select v0, v2, v2
    v4: int32 = 7int32
    local.set local0, v4
    v5: int32 = load v3
    return v5
}"#;

        let options = strict_options();

        let mut test = TestProgram::new(input);
        test.run_pass_with_options(&BorrowCheck, options);
        // local0 is borrowed via v3, so the local.set should error
        test.assert_error(|e| matches!(e, OptimizeError::LocalSetWhileBorrowed { .. }));
    }

    /// Raw references do not participate in borrow checking.
    #[test]
    fn test_raw_reference_skips_borrow_tracking() {
        let input = r#"
function test(): int32 {
    local local0: int32, owned
b0:
    v0: int32 = 1int32
    local.set local0, v0
    v1: ref<int32, raw, space(frame)> = local.address local0
    v2: int32 = 2int32
    local.set local0, v2
    v3: int32 = load v1
    return v3
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&BorrowCheck);
        test.assert_no_errors();
    }

    /// Call to function with inferred single-param lifetime tracks borrow correctly.
    ///
    /// When calling a function that returns a borrowed reference and has one
    /// borrowed parameter, the return value borrows from that argument.
    #[test]
    fn test_track_call_borrow_from_single_param() {
        let input = r#"
function identity(v0: ref<int32, borrowed>): ref<int32, borrowed> {
b0(v0: ref<int32, borrowed>):
    return v0
}
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 42int32
    store v0, v1
    v2: ref<int32, borrowed> = field.address v0, 0
    v3: ref<int32, borrowed> = call identity(v2): (ref<int32, borrowed>) -> ref<int32, borrowed>
    drop v0
    v4: int32 = load v3
    return v4
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&BorrowCheck);

        // v3 = call identity(v2) returns a borrow of v2 which borrows from v0
        // dropping v0 while v3 is live should be an error
        test.assert_error(|e| matches!(e, OptimizeError::DropWhileBorrowed { .. }));
    }

    /// Call result with static lifetime doesn't track borrow from arguments.
    ///
    /// When a function has explicit static lifetime, the return value doesn't
    /// borrow from any arguments.
    #[test]
    fn test_track_call_static_lifetime_no_borrow() {
        let input = r#"
function getStatic(v0: ref<int32, borrowed>): ref<int32, borrowed> {
b0(v0: ref<int32, borrowed>):
    return v0
}
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 42int32
    store v0, v1
    v2: ref<int32, borrowed> = field.address v0, 0
    v3: ref<int32, borrowed> = call getStatic(v2): (ref<int32, borrowed>) -> ref<int32, borrowed>
    drop v0
    v4: int32 = load v3
    return v4
}"#;

        let mut test = TestProgram::new(input);
        test.set_function_lifetime("getStatic", mir::BorrowRegion::Static);
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
        let input = r#"
function pickFirst(v0: ref<int32, borrowed>, v1: ref<int32, borrowed>): ref<int32, borrowed> {
b0(v0: ref<int32, borrowed>, v1: ref<int32, borrowed>):
    return v0
}
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: int32 = 42int32
    store v0, v2
    store v1, v2
    v3: ref<int32, borrowed> = field.address v0, 0
    v4: ref<int32, borrowed> = field.address v1, 0
    v5: ref<int32, borrowed> = call pickFirst(v3, v4): (ref<int32, borrowed>, ref<int32, borrowed>) -> ref<int32, borrowed>
    drop v1
    v6: int32 = load v5
    drop v0
    return v6
}"#;

        let mut test = TestProgram::new(input);
        test.set_function_lifetime("pickFirst", mir::BorrowRegion::param(0));
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
        let input = r#"
function pickAny(v0: ref<int32, borrowed>, v1: ref<int32, borrowed>): ref<int32, borrowed> {
b0(v0: ref<int32, borrowed>, v1: ref<int32, borrowed>):
    return v0
}
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: int32 = 42int32
    store v0, v2
    store v1, v2
    v3: ref<int32, borrowed> = field.address v0, 0
    v4: ref<int32, borrowed> = field.address v1, 0
    v5: ref<int32, borrowed> = call pickAny(v3, v4): (ref<int32, borrowed>, ref<int32, borrowed>) -> ref<int32, borrowed>
    drop v1
    v6: int32 = load v5
    drop v0
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
        let input = r#"
function deref(v0: ref<int32, borrowed>): int32 {
b0(v0: ref<int32, borrowed>):
    v1: int32 = load v0
    return v1
}
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 42int32
    store v0, v1
    v2: ref<int32, borrowed> = field.address v0, 0
    v3: int32 = call deref(v2): (ref<int32, borrowed>) -> int32
    drop v0
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
        let input = r#"
function identity(v0: ref<int32, borrowed>): ref<int32, borrowed> {
b0(v0: ref<int32, borrowed>):
    return v0
}
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 42int32
    store v0, v1
    v2: ref<int32, borrowed> = field.address v0, 0
    v3: ref<int32, borrowed> = call identity(v2): (ref<int32, borrowed>) -> ref<int32, borrowed>
    v4: int32 = load v3
    drop v0
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
        let input = r#"
function pickEither(v0: ref<int32, borrowed>, v1: ref<int32, borrowed>): ref<int32, borrowed> {
b0(v0: ref<int32, borrowed>, v1: ref<int32, borrowed>):
    return v0
}
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: int32 = 42int32
    store v0, v2
    store v1, v2
    v3: ref<int32, borrowed> = field.address v0, 0
    v4: ref<int32, borrowed> = field.address v1, 0
    v5: ref<int32, borrowed> = call pickEither(v3, v4): (ref<int32, borrowed>, ref<int32, borrowed>) -> ref<int32, borrowed>
    drop v0
    v6: int32 = load v5
    drop v1
    return v6
}"#;

        let mut test = TestProgram::new(input);

        // explicit lifetime: borrows from both param 0 and param 1
        test.set_function_lifetime("pickEither", mir::BorrowRegion::params([0, 1]));
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
        let input = r#"
function pickSecond(v0: ref<int32, borrowed>, v1: ref<int32, borrowed>): ref<int32, borrowed> {
b0(v0: ref<int32, borrowed>, v1: ref<int32, borrowed>):
    return v1
}

function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: int32 = 42int32
    store v0, v2
    store v1, v2
    v3: ref<int32, borrowed> = field.address v0, 0
    v4: ref<int32, borrowed> = field.address v1, 0
    v5: ref<int32, borrowed> = call pickSecond(v3, v4): (ref<int32, borrowed>, ref<int32, borrowed>) -> ref<int32, borrowed>
    drop v0
    v6: int32 = load v5
    drop v1
    return v6
}"#;

        let mut test = TestProgram::new(input);

        // explicit lifetime: borrows only from param 1
        test.set_function_lifetime("pickSecond", mir::BorrowRegion::param(1));
        test.run_pass(&BorrowCheck);

        // v5 borrows from v4 (param 1), not v3 (param 0)
        // dropping v0 is okay, v1 must stay live until after v5 is used
        test.assert_no_errors();
    }

    /// Call returning mutable borrowed reference propagates mutability.
    ///
    /// When a function returns ref<T, borrowed>, the returned borrow
    /// should be tracked as mutable and conflict with other borrows.
    #[test]
    fn test_track_call_mutable_return_conflicts() {
        let input = r#"
function getMut(v0: ref<int32, borrowed>): ref<int32, borrowed> {
b0(v0: ref<int32, borrowed>):
    return v0
}
function test(v0: ref<int32, borrowed>): int32 {
b0(v0: ref<int32, borrowed>):
    v1: ref<int32, borrowed> = call getMut(v0): (ref<int32, borrowed>) -> ref<int32, borrowed>
    v2: ref<int32, borrowed> = field.address v0, 0
    v3: int32 = load v1
    v4: int32 = load v2
    v5: int32 = int.add v3, v4
    return v5
}"#;

        let options = strict_options();

        let mut test = TestProgram::new(input);
        test.run_pass_with_options(&BorrowCheck, options);

        // v1 = call getMut(v0) returns a mutable borrow from v0
        // v2 = field.address v0, 0 creates another mutable borrow of v0
        // these should conflict in strict mode
        test.assert_error(|e| matches!(e, OptimizeError::ConflictingBorrow { .. }));
    }

    /// Call returning shared borrowed reference allows concurrent shared borrows.
    ///
    /// When a function returns ref<T, borrowed> (shared), the returned borrow
    /// should be tracked as shared and not conflict with other shared borrows.
    #[test]
    fn test_track_call_shared_return_no_conflict() {
        let input = r#"
function getShared(v0: ref<int32, borrowed, readonly>): ref<int32, borrowed, readonly> {
b0(v0: ref<int32, borrowed, readonly>):
    return v0
}
function test(v0: ref<int32, borrowed, readonly>): int32 {
b0(v0: ref<int32, borrowed, readonly>):
    v1: ref<int32, borrowed, readonly> = call getShared(v0): (ref<int32, borrowed, readonly>) -> ref<int32, borrowed, readonly>
    v2: ref<int32, borrowed, readonly> = field.address v0, 0
    v3: int32 = load v1
    v4: int32 = load v2
    v5: int32 = int.add v3, v4
    return v5
}"#;

        let options = strict_options();

        let mut test = TestProgram::new(input);
        test.run_pass_with_options(&BorrowCheck, options);

        // v1 = call getShared(v0) returns a shared borrow from v0
        // v2 = field.address v0, 0 creates another shared borrow of v0
        // shared + shared is okay
        test.assert_no_errors();
    }

    /// Mutable borrow from call conflicts with subsequent mutable field.address.
    ///
    /// Tests that mutability is correctly propagated through the call and
    /// detected when creating another mutable borrow.
    #[test]
    fn test_track_call_mutable_borrow_then_field_addr_conflict() {
        let input = r#"
function getMutRef(v0: ref<int32, borrowed>): ref<int32, borrowed> {
b0(v0: ref<int32, borrowed>):
    v1: ref<int32, borrowed> = field.address v0, 0
    return v1
}
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 42int32
    store v0, v1
    v2: ref<int32, borrowed> = call getMutRef(v0): (ref<int32, borrowed>) -> ref<int32, borrowed>
    v3: ref<int32, borrowed> = field.address v0, 0
    v4: int32 = load v2
    v5: int32 = load v3
    v6: int32 = int.add v4, v5
    drop v0
    return v6
}"#;

        let options = strict_options();

        let mut test = TestProgram::new(input);
        test.run_pass_with_options(&BorrowCheck, options);

        // v2 is a mutable borrow returned from call
        // v3 = field.address v0 creates another borrow from v0
        // since v0 is stack.alloc (not mut ref param), the field.address creates
        // a mutable borrow (conservative default), so this should conflict
        test.assert_error(|e| matches!(e, OptimizeError::ConflictingBorrow { .. }));
    }
}
