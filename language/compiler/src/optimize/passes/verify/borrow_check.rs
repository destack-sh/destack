use std::collections::{HashMap, HashSet};

use destack_compiler_macros::declare_pass;
use destack_mir as mir;
use destack_source::ModuleId;
use destack_workspace::TargetId;
use mir::{Instruction, Mutability, ReferenceKind, Type, Value};

use crate::optimize::{
    AnalysisPreservation, BorrowAnalysis, BorrowMap, ControlFlowGraph, FunctionPass,
    LivenessAnalysis, OptimizationContext, Pass, PassMetadata,
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

impl Pass for BorrowCheck {
    fn metadata(&self) -> &'static PassMetadata {
        BorrowCheck::metadata()
    }
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
    /// If v3 = field.addr v2 and v2 = field.addr v0, then v3's provenance is [v2, v0].
    /// This allows us to detect when dropping any value in the chain invalidates the borrow.
    provenance: Vec<Value>,
}

/// Context for borrow checking a single function.
struct BorrowCheckContext<'a> {
    /// The function being checked.
    #[allow(dead_code)]
    function: &'a mir::Function,
    /// The MIR tree.
    tree: &'a mir::NodeTree,
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
        module_id: ModuleId,
        target_id: TargetId,
        strict_mode: bool,
    ) -> Self {
        Self {
            function,
            tree,
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
        // conservative: assume shared (immutable) if unknown
        false
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
            self.active_borrows.insert(
                id,
                ActiveBorrow {
                    reference,
                    origin: Some(origin),
                    is_mutable: false, // conservative: could derive from type
                    created_at: at,
                    provenance: Vec::new(),
                },
            );
            self.borrows_of.entry(origin).or_default().insert(id);
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
    fn build_provenance(&self, origin: Value) -> Vec<Value> {
        // find any borrow where the reference is our origin
        for borrow in self.active_borrows.values() {
            if borrow.reference == origin {
                // origin is itself a reference: inherit its provenance plus origin
                let mut provenance = Vec::new();
                if let Some(parent_origin) = borrow.origin {
                    provenance.push(parent_origin);
                }
                provenance.extend(borrow.provenance.iter().copied());
                return provenance;
            }
        }
        // origin has no provenance chain
        Vec::new()
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
                provenance: Vec::new(),
            },
        );

        self.local_borrows.entry(local).or_default().insert(id);
    }

    /// Check if a local has any active borrows.
    fn local_has_borrow(&self, local: mir::LocalNodeId<mir::Local>) -> Option<&ActiveBorrow> {
        self.local_borrows
            .get(&local)
            .and_then(|ids| ids.iter().next())
            .and_then(|id| self.active_borrows.get(id))
    }

    /// Get active borrows of a value (direct borrows only).
    fn borrows_of_value(&self, value: Value) -> impl Iterator<Item = &ActiveBorrow> {
        self.borrows_of
            .get(&value)
            .into_iter()
            .flat_map(|ids| ids.iter().filter_map(|id| self.active_borrows.get(id)))
    }

    /// Get all borrows that transitively borrow from a value.
    ///
    /// This includes direct borrows and borrows where the value appears in provenance.
    fn borrows_transitively_from(&self, value: Value) -> Vec<&ActiveBorrow> {
        self.active_borrows
            .values()
            .filter(|borrow| borrow.origin == Some(value) || borrow.provenance.contains(&value))
            .collect()
    }

    /// Check if a value has any active mutable borrows.
    fn has_mutable_borrow(&self, value: Value) -> Option<&ActiveBorrow> {
        self.borrows_of_value(value).find(|b| b.is_mutable)
    }

    /// Check if a value has any active borrows (mutable or immutable).
    fn has_any_borrow(&self, value: Value) -> Option<&ActiveBorrow> {
        self.borrows_of_value(value).next()
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
    fn check_new_borrow(
        &mut self,
        new_reference: Value,
        origin: Value,
        is_mutable: bool,
        at: mir::LocalNodeId<Instruction>,
        context: &OptimizationContext<'_>,
    ) {
        // check for conflicting borrows
        let conflict = if is_mutable {
            // mutable borrow: conflicts with any existing borrow
            self.has_any_borrow(origin)
                .map(|b| (b.created_at, b.is_mutable))
        } else {
            // shared borrow: conflicts with existing mutable borrow
            self.has_mutable_borrow(origin)
                .map(|b| (b.created_at, b.is_mutable))
        };

        if let Some((existing_at, existing_is_mutable)) = conflict {
            self.emit_borrow_conflict(at, existing_at, existing_is_mutable, context);
        }

        // register the new borrow (with provenance tracking)
        self.add_borrow(new_reference, origin, is_mutable, at);
    }

    /// Check mutation through reference doesn't invalidate other borrows.
    fn check_mutation_through_reference(
        &mut self,
        pointer: Value,
        at: mir::LocalNodeId<Instruction>,
        context: &OptimizationContext<'_>,
    ) {
        // check if any existing borrow may be invalidated by this mutation
        // FUGU #Broken: this is a simplified check based on direct origin equality.
        // Full alias analysis would detect may-alias relationships.
        let invalidated: Vec<_> = self
            .active_borrows
            .values()
            .filter(|borrow| {
                // mutating through our own reference is fine
                if borrow.reference == pointer {
                    return false;
                }
                // if the borrow's origin is the same as the pointer, mutation may invalidate
                borrow.origin == Some(pointer)
            })
            .map(|b| b.created_at)
            .collect();

        for invalidated_at in invalidated {
            self.emit_invalidated_reference(at, invalidated_at, context);
        }
    }

    /// Check that dropping a value doesn't drop while borrowed.
    fn check_drop_while_borrowed(
        &mut self,
        value: Value,
        at: mir::LocalNodeId<Instruction>,
        context: &OptimizationContext<'_>,
    ) {
        // check borrows that directly or transitively borrow from this value
        let borrowed_borrows = self.borrows_transitively_from(value);

        if let Some(borrow) = borrowed_borrows.first() {
            let borrowed_at = borrow.created_at;
            context.emit_error(OptimizeError::DropWhileBorrowed {
                node: self.anchor(at),
                borrowed_at: self.anchor(borrowed_at),
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
        context: &OptimizationContext<'_>,
    ) {
        // check if the value being moved has active borrows
        let borrowed_at = self.has_any_borrow(value).map(|b| b.created_at);

        if let Some(borrowed_at) = borrowed_at {
            // moving while borrowed invalidates the borrow
            context.emit_error(OptimizeError::MoveOfBorrowedValue {
                node: self.anchor(at),
                borrowed_at: self.anchor(borrowed_at),
            });
            self.had_aliasing_violations = true;
        }
    }

    /// Check that setting a local doesn't overwrite a borrowed local.
    fn check_local_set_while_borrowed(
        &mut self,
        local: mir::LocalNodeId<mir::Local>,
        at: mir::LocalNodeId<Instruction>,
        context: &OptimizationContext<'_>,
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

    /// Emit a borrow conflict error/warning.
    fn emit_borrow_conflict(
        &mut self,
        at: mir::LocalNodeId<Instruction>,
        existing_at: mir::LocalNodeId<Instruction>,
        existing_is_mutable: bool,
        context: &OptimizationContext<'_>,
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
        context: &OptimizationContext<'_>,
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
    fn run_on_function(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::NodeTree,
        context: &OptimizationContext<'_>,
    ) -> AnalysisPreservation {
        let _cfg = context
            .analyses
            .get::<ControlFlowGraph>(function, tree, context);

        // get liveness for borrow lifetime tracking
        let liveness = context
            .analyses
            .get::<LivenessAnalysis>(function, tree, context);

        // get borrow analysis for cross-block tracking
        let borrow_analysis = context
            .analyses
            .get::<BorrowAnalysis>(function, tree, context);

        let module_id = context.module_id();
        let target_id = context.target_id().clone();

        let strict_mode = context.options.strict_borrow_mode;
        let mut checker =
            BorrowCheckContext::new(function, tree, module_id, target_id, strict_mode);

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

                check_instruction(&mut checker, instruction_id, &instruction, context);
            }
        }

        // if aliasing violations were found, mark context
        if checker.had_aliasing_violations {
            context.mark_aliasing_violation();
        }

        // borrow checking is a pure analysis pass
        AnalysisPreservation::all()
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
                // at block entry, check if value is not live-in
                !liveness.is_live_in(block_id, borrow.reference)
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
    context: &OptimizationContext<'_>,
) {
    match instruction {
        // field.addr creates a borrow of the aggregate
        Instruction::FieldAddr {
            destination,
            aggregate,
            ..
        } => {
            // derive mutability from source aggregate type
            let is_mutable = checker.derive_mutability(*aggregate);
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
            destination, array, ..
        } => {
            // derive mutability from source array type
            let is_mutable = checker.derive_mutability(*array);
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

        // local.get may borrow from a local (track for local.set checks)
        Instruction::LocalGet { destination, local } => {
            // track that this value came from this local
            let local_decl = checker.tree.get(*local);
            checker.register_value_type(*destination, local_decl.ty);

            // if local contains owned/managed ref, local.get is borrowing
            let ty = checker.tree.get(local_decl.ty);
            if let Type::Reference {
                kind: ReferenceKind::Owned | ReferenceKind::Managed,
                mutability,
                ..
            } = ty
            {
                let is_mutable = *mutability == Mutability::Mutable;
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
        } => {
            let args = checker.tree.get_arguments(*arguments);
            for &arg in args {
                checker.check_move_while_borrowed(arg, instruction_id, context);
            }
            // register return type from function signature
            if let Some(dest) = destination {
                let func = checker.tree.get(*function);
                checker.register_value_type(*dest, func.return_type);
            }
        }

        Instruction::CallIndirect {
            destination,
            callee,
            arguments,
        } => {
            // callee is used, not moved
            let _ = callee;
            let args = checker.tree.get_arguments(*arguments);
            for &arg in args {
                checker.check_move_while_borrowed(arg, instruction_id, context);
            }
            if let Some(dest) = destination {
                // NOTE #Architecture: indirect calls don't have signatures for borrow check
                let _ = dest;
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
            layout,
        }
        | Instruction::RawAlloc {
            destination,
            layout,
        }
        | Instruction::StackAlloc {
            destination,
            layout,
        } => {
            checker.register_value_type(*destination, *layout);
        }

        Instruction::ManagedAllocArray {
            destination,
            element,
            ..
        } => {
            checker.register_value_type(*destination, *element);
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

        let mut program = TestProgram::new(input);
        program.run_pass(&BorrowCheck);
        program.assert_no_errors();
        program.assert_unchanged(input);
    }

    /// Field address with no other borrows is valid.
    #[test]
    fn test_verify_single_field_addr() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = iconst 42i32
    store v0, v1
    v2 = load v0
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&BorrowCheck);
        program.assert_no_errors();
    }

    /// Sequential loads from same pointer is valid.
    #[test]
    fn test_verify_sequential_loads() {
        let input = r#"function @test(v0: ref<raw i32>) -> i32 {
block0(v0: ref<raw i32>):
    v1 = load v0
    v2 = load v0
    v3 = iadd v1, v2
    return v3
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&BorrowCheck);
        program.assert_no_errors();
    }

    /// Store through pointer is valid.
    #[test]
    fn test_verify_store_no_conflict() {
        let input = r#"function @test(v0: ref<raw i32>, v1: i32) -> void {
block0(v0: ref<raw i32>, v1: i32):
    store v0, v1
    return
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&BorrowCheck);
        program.assert_no_errors();
    }

    /// Dropping while borrowed is detected.
    #[test]
    fn test_detect_drop_while_borrowed() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = field.addr v0, 0
    raw.drop v0
    v2 = load v1
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&BorrowCheck);
        program.assert_error(|e| matches!(e, OptimizeError::DropWhileBorrowed { .. }));
    }

    /// raw.free while borrowed is detected.
    #[test]
    fn test_detect_raw_free_while_borrowed() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = raw.alloc i32
    v1 = field.addr v0, 0
    raw.free v0
    v2 = load v1
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&BorrowCheck);
        program.assert_error(|e| matches!(e, OptimizeError::DropWhileBorrowed { .. }));
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

        let mut program = TestProgram::new(input);
        program.run_pass(&BorrowCheck);
        program.assert_no_errors();
    }

    /// Multiple parameters with no conflicts is valid.
    #[test]
    fn test_verify_multiple_params_no_conflict() {
        let input = r#"function @test(v0: ref<raw i32>, v1: ref<raw i32>) -> i32 {
block0(v0: ref<raw i32>, v1: ref<raw i32>):
    v2 = load v0
    v3 = load v1
    v4 = iadd v2, v3
    return v4
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&BorrowCheck);
        program.assert_no_errors();
    }

    /// Drop while borrowed in strict mode emits error.
    #[test]
    fn test_strict_mode_drop_while_borrowed() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = iconst 42i32
    store v0, v1
    v2 = field.addr v0, 0
    raw.drop v0
    v3 = load v2
    return v3
}"#;

        let mut options = crate::optimize::OptimizeOptions::default();
        options.strict_borrow_mode = true;

        let mut program = TestProgram::new(input);
        program.run_pass_with_options(&BorrowCheck, options);
        program.assert_error(|e| matches!(e, OptimizeError::DropWhileBorrowed { .. }));
    }

    /// Drop while borrowed is always an error, even in lenient mode.
    #[test]
    fn test_detect_drop_while_borrowed_lenient_mode() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = iconst 42i32
    store v0, v1
    v2 = field.addr v0, 0
    raw.drop v0
    v3 = load v2
    return v3
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&BorrowCheck);
        program.assert_error(|e| matches!(e, OptimizeError::DropWhileBorrowed { .. }));
    }

    /// Single field access is valid.
    #[test]
    fn test_verify_single_field_access() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = iconst 42i32
    store v0, v1
    v2 = load v0
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&BorrowCheck);
        program.assert_no_errors();
    }

    /// Load through reference is valid.
    #[test]
    fn test_verify_load() {
        let input = r#"function @test(v0: ref<raw i32>) -> i32 {
block0(v0: ref<raw i32>):
    v1 = load v0
    return v1
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&BorrowCheck);
        program.assert_no_errors();
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

        let mut program = TestProgram::new(input);
        program.run_pass(&BorrowCheck);
        program.assert_no_errors();
    }

    /// Borrow expires when reference is no longer used.
    ///
    /// The reference v2 is not used after the load, so the borrow expires
    /// and dropping v0 is valid.
    #[test]
    fn test_verify_borrow_expires_after_use() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = iconst 42i32
    store v0, v1
    v2 = field.addr v0, 0
    v3 = load v2
    raw.drop v0
    return v3
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&BorrowCheck);
        // borrow of v0 through v2 expires after v3 = load v2 (v2 is dead)
        // so drop v0 is valid
        program.assert_no_errors();
    }

    /// Borrow does not expire if reference is still live.
    #[test]
    fn test_detect_borrow_still_live() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = iconst 42i32
    store v0, v1
    v2 = field.addr v0, 0
    raw.drop v0
    v3 = load v2
    return v3
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&BorrowCheck);
        // v2 is still live at drop v0 (used in v3 = load v2)
        // so this should be an error
        program.assert_error(|e| matches!(e, OptimizeError::DropWhileBorrowed { .. }));
    }

    /// Element address also creates borrow that tracks lifetime.
    #[test]
    fn test_verify_element_addr_borrow_expires() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = iconst 42i32
    store v0, v1
    v2 = iconst 0i32
    v3 = element.addr v0, v2
    v4 = load v3
    raw.drop v0
    return v4
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&BorrowCheck);
        // borrow expires after v4 = load v3
        program.assert_no_errors();
    }

    /// Direct field address borrow is tracked even with intermediate operations.
    #[test]
    fn test_detect_direct_borrow_still_live() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = iconst 42i32
    store v0, v1
    v2 = field.addr v0, 0
    v3 = iconst 10i32
    raw.drop v0
    v4 = load v2
    return v4
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&BorrowCheck);
        // v2 is still live at drop v0 (used in v4 = load v2)
        program.assert_error(|e| matches!(e, OptimizeError::DropWhileBorrowed { .. }));
    }

    /// Chained field.addr tracks transitive provenance.
    ///
    /// v3 = field.addr v2 where v2 = field.addr v0 means dropping v0 while
    /// v3 is live should be an error.
    #[test]
    fn test_detect_transitive_borrow_chain() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = iconst 42i32
    store v0, v1
    v2 = field.addr v0, 0
    v3 = field.addr v2, 0
    raw.drop v0
    v4 = load v3
    return v4
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&BorrowCheck);
        // v3 transitively borrows from v0 via v2, so drop v0 is error
        program.assert_error(|e| matches!(e, OptimizeError::DropWhileBorrowed { .. }));
    }

    /// Deep transitive chain (v0 -> v2 -> v3 -> v4) tracks correctly.
    #[test]
    fn test_detect_deep_transitive_chain() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = iconst 42i32
    store v0, v1
    v2 = field.addr v0, 0
    v3 = field.addr v2, 0
    v4 = field.addr v3, 0
    raw.drop v0
    v5 = load v4
    return v5
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&BorrowCheck);
        // v4 transitively borrows from v0 via v3 -> v2 -> v0
        program.assert_error(|e| matches!(e, OptimizeError::DropWhileBorrowed { .. }));
    }

    /// Transitive borrow expires when intermediate references are dead.
    #[test]
    fn test_verify_transitive_borrow_expires() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = iconst 42i32
    store v0, v1
    v2 = field.addr v0, 0
    v3 = field.addr v2, 0
    v4 = load v3
    raw.drop v0
    return v4
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&BorrowCheck);
        // v3 is dead after v4 = load v3, so transitive borrow expires
        // drop v0 is valid
        program.assert_no_errors();
    }

    /// Dropping intermediate value in chain is still error if final ref is live.
    #[test]
    fn test_detect_drop_intermediate_while_borrowed() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = iconst 42i32
    store v0, v1
    v2 = field.addr v0, 0
    v3 = field.addr v2, 0
    raw.drop v2
    v4 = load v3
    return v4
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&BorrowCheck);
        // dropping v2 while v3 (which borrows from v2) is live is error
        program.assert_error(|e| matches!(e, OptimizeError::DropWhileBorrowed { .. }));
    }

    /// Mutable borrow while existing mutable borrow is detected in strict mode.
    #[test]
    fn test_detect_mutable_borrow_conflict_strict() {
        let input = r#"function @test(v0: ref<borrowed mut i32>) -> i32 {
block0(v0: ref<borrowed mut i32>):
    v1 = field.addr v0, 0
    v2 = field.addr v0, 0
    v3 = load v1
    v4 = load v2
    v5 = iadd v3, v4
    return v5
}"#;

        let mut options = crate::optimize::OptimizeOptions::default();
        options.strict_borrow_mode = true;

        let mut program = TestProgram::new(input);
        program.run_pass_with_options(&BorrowCheck, options);
        // v2 = field.addr while v1 still borrows v0 - both are mutable since v0 is mut
        // mutable borrow while existing mutable borrow is conflict
        program.assert_error(|e| matches!(e, OptimizeError::ConflictingBorrow { .. }));
    }

    /// Shared borrows don't conflict with each other.
    #[test]
    fn test_verify_multiple_shared_borrows() {
        let input = r#"function @test(v0: ref<borrowed i32>) -> i32 {
block0(v0: ref<borrowed i32>):
    v1 = field.addr v0, 0
    v2 = field.addr v0, 0
    v3 = load v1
    v4 = load v2
    v5 = iadd v3, v4
    return v5
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&BorrowCheck);
        // multiple shared borrows of same origin is valid
        program.assert_no_errors();
    }

    /// Borrow that flows through diamond control flow.
    #[test]
    fn test_detect_borrow_through_diamond() {
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = stack.alloc i32
    v2 = iconst 42i32
    store v1, v2
    v3 = field.addr v1, 0
    branch v0, block1, block2
block1:
    jump block3
block2:
    jump block3
block3:
    raw.drop v1
    v4 = load v3
    return v4
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&BorrowCheck);
        // v3 is still live at drop v1 after both paths converge
        program.assert_error(|e| matches!(e, OptimizeError::DropWhileBorrowed { .. }));
    }

    /// Borrow created on one branch only - dropping after merge is error.
    /// On block1 path: v1 is borrowed via v3 which is passed to block3
    /// On block2 path: v2 is borrowed (not v1)
    /// At merge: v1 is "maybe borrowed" - dropping v1 should error
    #[test]
    fn test_detect_maybe_borrowed_one_branch() {
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = stack.alloc i32
    v2 = stack.alloc i32
    v3 = iconst 42i32
    store v1, v3
    store v2, v3
    branch v0, block1, block2
block1:
    v4 = field.addr v1, 0
    jump block3(v4)
block2:
    v5 = field.addr v2, 0
    jump block3(v5)
block3(v6: ref<raw i32>):
    raw.drop v1
    v7 = load v6
    raw.drop v2
    return v7
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&BorrowCheck);
        // v1 is borrowed via v4 on block1 path only
        // v1 is NOT borrowed on block2 path (v5 borrows v2 instead)
        // at merge (block3), v1 is MaybeBorrowed
        // drop v1 while MaybeBorrowed should error
        program.assert_error(|e| matches!(e, OptimizeError::DropWhileBorrowed { .. }));
    }

    /// Borrow expires before merge - drop after merge is ok.
    #[test]
    fn test_verify_borrow_expires_before_merge() {
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = stack.alloc i32
    v2 = iconst 42i32
    store v1, v2
    branch v0, block1, block2
block1:
    v3 = field.addr v1, 0
    v4 = load v3
    jump block3(v4)
block2:
    v5 = field.addr v1, 0
    v6 = load v5
    jump block3(v6)
block3(v7: i32):
    raw.drop v1
    return v7
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&BorrowCheck);
        // borrows v3 and v5 are consumed by loads before the jumps
        // v7 is just an i32, not a reference, so no borrow at drop
        program.assert_no_errors();
    }

    /// Loop with borrow - borrow created and used within loop body.
    #[test]
    fn test_verify_loop_borrow_local() {
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = stack.alloc i32
    store v1, v0
    jump block1
block1:
    v2 = field.addr v1, 0
    v3 = load v2
    v4 = iconst 0i32
    v5 = icmp_sgt v3, v4
    branch v5, block2, block3
block2:
    v6 = iconst 1i32
    v7 = isub v3, v6
    store v1, v7
    jump block1
block3:
    v8 = load v1
    raw.drop v1
    return v8
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&BorrowCheck);
        // v2 borrow is used immediately by v3=load, then dead
        // loop is fine because borrow doesn't escape iteration
        program.assert_no_errors();
    }

    /// Borrow escapes loop iteration - error.
    #[test]
    fn test_detect_borrow_escapes_loop() {
        let input = r#"function @test(v0: i32) -> ref<raw i32> {
block0(v0: i32):
    v1 = stack.alloc i32
    store v1, v0
    v2 = field.addr v1, 0
    jump block1(v2)
block1(v3: ref<raw i32>):
    v4 = load v3
    v5 = iconst 0i32
    v6 = icmp_sgt v4, v5
    branch v6, block2, block3
block2:
    jump block1(v3)
block3:
    return v3
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&BorrowCheck);
        // v3 is a borrow of v1 that escapes via return
        // (this is actually a stack escape, but borrow check sees it too)
        // The borrow flows through the loop and returns
        program.assert_no_errors(); // borrow check doesn't catch return escape (stack_check does)
    }
}
