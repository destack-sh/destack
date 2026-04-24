use std::collections::{HashMap, HashSet};

use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::optimize::analyses::{
    AliasAnalysis, DominatorTree, MemoryAccess, MemoryAccessEffect, MemoryAccessId,
    MemoryAccessLocation, MemorySSA,
};
use crate::optimize::common::{
    ValueTypeMap, address_spaces_may_alias, alias_scopes_may_alias,
    apply_substitutions_in_function, can_substitute_value, effect_is_trackable,
    memory_locations_compatible, resolve_substitution_chains, space_sets_may_alias,
    type_alias_tags_may_alias,
};
use crate::optimize::{AnalysisPreservation, FunctionPass, PipelineContext, TypeContext};

declare_pass! {
    /// Forward stored values to subsequent loads.
    ///
    /// This pass performs three optimizations:
    /// 1) **Store to load forwarding**: When a store is followed by a load from the same
    /// location (with no intervening clobbers), replace the load with the stored value.
    /// 2) **Load to load forwarding**: When the same location is loaded twice with no
    /// intervening clobbers, replace the second load with the first load's result.
    /// 3) **Cross block forwarding**: Forward values across basic blocks when the store
    /// or load dominates the use with no intervening clobbers.
    ///
    /// The pass handles:
    /// 1) Volatile and atomic operations that act as memory barriers.
    /// 2) Calls and intrinsics that may clobber memory.
    /// 3) Aliasing through field and element access.
    ///
    /// ```mir
    /// function before(): int32 {
    /// b0:
    ///     v0 = stack.alloc int32
    ///     v1 = 42int32
    ///     store v0, v1
    ///     v2 = load v0       // forwarded from store
    ///     v3 = load v0       // forwarded from store load to load
    ///     v4 = int.add v2, v3
    ///     return v4
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function after(): int32 {
    /// b0:
    ///     v0 = stack.alloc int32
    ///     v1 = 42int32
    ///     store v0, v1
    ///     v4 = int.add v1, v1
    ///     return v4
    /// }
    /// ```
    #[pass(id = "load-store-forward", requires(call_effects, memory_access_metadata))]
    pub LoadStoreForward,
    "Forward stored values to subsequent loads"
}

/// An available value tied to a MemorySSA clobber.
#[derive(Clone)]
struct MemoryEntry {
    /// The clobbering access id for the memory state.
    clobber: MemoryAccessId,
    /// The accessed memory location.
    location: MemoryAccessLocation,
    /// The available value.
    value: mir::Value,
    /// The memory space set for the access.
    space_set: mir::MemorySpaceSet,
    /// The address spaces for the access.
    address_spaces: Option<mir::AddressSpaceSet>,
    /// Alias scopes applied to the access.
    alias_scopes: Vec<mir::MemoryAliasScopeId>,
    /// No alias scopes applied to the access.
    noalias_scopes: Vec<mir::MemoryAliasScopeId>,
    /// Optional type-alias tag for the access.
    type_alias_tag: Option<mir::TypeAliasTagId>,
}

impl FunctionPass for LoadStoreForward {
    fn run(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::NodeTree,
        ctx: &PipelineContext<'_>,
    ) -> AnalysisPreservation {
        // skip empty functions
        let entry = match function.entry {
            Some(entry) => entry,
            None => return AnalysisPreservation::all(),
        };

        // get analyses
        let (aa, memory_ssa, dom_children) = {
            let analyses = ctx.function_analyses(function, tree);
            let domtree = analyses.get::<DominatorTree>();
            let aa = analyses.get::<AliasAnalysis>().clone();
            let memory_ssa = analyses.get::<MemorySSA>();
            let dom_children = build_dominator_children(function, &domtree);
            (aa, memory_ssa, dom_children)
        };

        let value_types = ValueTypeMap::new(function, tree);

        // run load store forwarding
        let changed = run_load_store_forward(
            entry,
            function,
            tree,
            &aa,
            memory_ssa.as_ref(),
            &dom_children,
            &value_types,
            ctx.type_context(),
        );

        // report analysis preservation
        if changed {
            AnalysisPreservation::none()
        } else {
            AnalysisPreservation::all()
        }
    }

    fn name(&self) -> &'static str {
        "LoadStoreForward"
    }

    fn id(&self) -> &'static str {
        "load-store-forward"
    }
}

/// Core load store forwarding logic. Returns true if changes were made.
#[allow(clippy::too_many_arguments)]
fn run_load_store_forward(
    entry: mir::LocalNodeId<mir::Block>,
    function: &mir::Function,
    tree: &mut mir::NodeTree,
    aa: &AliasAnalysis,
    memory_ssa: &MemorySSA,
    dom_children: &HashMap<mir::LocalNodeId<mir::Block>, Vec<mir::LocalNodeId<mir::Block>>>,
    value_types: &ValueTypeMap,
    type_context: TypeContext,
) -> bool {
    // run forwarding using dominator tree traversal
    let (substitutions, to_remove) = find_forwardable_loads(
        entry,
        tree,
        aa,
        memory_ssa,
        dom_children,
        value_types,
        type_context,
    );

    // nothing to do if no forwarding found
    if substitutions.is_empty() {
        return false;
    }

    // resolve transitive substitution chains
    let substitutions = resolve_substitution_chains(substitutions);

    // apply substitutions and remove forwarded loads
    apply_substitutions_in_function(function, tree, &substitutions, Some(&to_remove));

    true
}

/// Build a map from each block to its children in the dominator tree.
fn build_dominator_children(
    function: &mir::Function,
    domtree: &DominatorTree,
) -> HashMap<mir::LocalNodeId<mir::Block>, Vec<mir::LocalNodeId<mir::Block>>> {
    // prepare the child mapping
    let mut children: HashMap<_, Vec<_>> = HashMap::new();

    // initialize all blocks with empty children lists
    for &block_id in &function.blocks {
        children.insert(block_id, Vec::new());
    }

    // build parent to children mapping from idom relationships
    for &block_id in &function.blocks {
        if let Some(idom) = domtree.immediate_dominator(block_id) {
            children.get_mut(&idom).unwrap().push(block_id);
        }
    }

    children
}

/// Scoped table of available memory values.
///
/// Tracks values by MemorySSA clobbering access.
struct AvailableMemory {
    /// Stack of scopes, each holding memory entries.
    scopes: Vec<Vec<MemoryEntry>>,
}

impl AvailableMemory {
    /// Create a new empty scoped table with one scope.
    fn new() -> Self {
        Self {
            scopes: vec![Vec::new()],
        }
    }

    /// Push a new scope for entering a dominated block.
    fn push_scope(&mut self) {
        // push a new scope for this block
        self.scopes.push(Vec::new());
    }

    /// Pop the current scope when leaving a dominated block.
    fn pop_scope(&mut self) {
        // keep at least one scope
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    /// Look up an available value for the given clobber and location.
    fn get(
        &self,
        clobber: MemoryAccessId,
        use_effect: &MemoryAccessEffect,
        aa: &AliasAnalysis,
        tree: &mir::NodeTree,
    ) -> Option<mir::Value> {
        // skip untrackable effects
        if !effect_is_trackable(use_effect) {
            return None;
        }

        let location = &use_effect.location;

        // search from innermost to outermost scope
        for scope in self.scopes.iter().rev() {
            // scan entries from newest to oldest
            for entry in scope.iter().rev() {
                // skip entries with a different clobber
                if entry.clobber != clobber {
                    continue;
                }

                // disambiguate using alias scopes and tbaa tags
                if !alias_scopes_may_alias(
                    &entry.alias_scopes,
                    &entry.noalias_scopes,
                    &use_effect.alias_scopes,
                    &use_effect.noalias_scopes,
                ) {
                    continue;
                }

                if !space_sets_may_alias(entry.space_set, use_effect.space_set) {
                    continue;
                }

                if !address_spaces_may_alias(&entry.address_spaces, &use_effect.address_spaces) {
                    continue;
                }

                if !type_alias_tags_may_alias(
                    &tree.metadata.memory.type_alias,
                    entry.type_alias_tag,
                    use_effect.type_alias_tag,
                ) {
                    continue;
                }

                // compare matching locations
                match (&entry.location, location) {
                    (MemoryAccessLocation::Local(a), MemoryAccessLocation::Local(b)) => {
                        if a == b {
                            return Some(entry.value);
                        }
                    }
                    (MemoryAccessLocation::Pointer(a), MemoryAccessLocation::Pointer(b)) => {
                        if a.ptr == b.ptr {
                            if memory_locations_compatible(a, b) {
                                return Some(entry.value);
                            }

                            return None;
                        }

                        // consult alias analysis for derived pointers
                        let alias_result = aa.alias(a, b);
                        if alias_result.is_no_alias() {
                            continue;
                        }
                        if alias_result.is_must_alias() {
                            if memory_locations_compatible(a, b) {
                                return Some(entry.value);
                            }

                            return None;
                        }

                        return None;
                    }
                    _ => {}
                }
            }
        }

        None
    }

    /// Insert an available value in the current scope.
    fn insert(&mut self, entry: MemoryEntry) {
        // append to the current scope
        if let Some(scope) = self.scopes.last_mut() {
            scope.push(entry);
        }
    }

    /// Clear all tracked entries.
    fn clear(&mut self) {
        // clear every scope
        for scope in &mut self.scopes {
            scope.clear();
        }
    }
}

/// Find loads that can be forwarded using dominator tree traversal.
fn find_forwardable_loads(
    entry: mir::LocalNodeId<mir::Block>,
    tree: &mir::NodeTree,
    aa: &AliasAnalysis,
    memory_ssa: &MemorySSA,
    dom_children: &HashMap<mir::LocalNodeId<mir::Block>, Vec<mir::LocalNodeId<mir::Block>>>,
    value_types: &ValueTypeMap,
    type_context: TypeContext,
) -> (
    HashMap<mir::Value, mir::Value>,
    HashSet<mir::LocalNodeId<mir::Instruction>>,
) {
    // initialize substitution state
    let mut substitutions: HashMap<mir::Value, mir::Value> = HashMap::new();
    let mut to_remove: HashSet<mir::LocalNodeId<mir::Instruction>> = HashSet::new();
    let mut available = AvailableMemory::new();

    // work stack for dominator tree traversal
    enum Action {
        Enter(mir::LocalNodeId<mir::Block>),
        Leave,
    }

    // seed traversal with the entry block
    let mut stack = vec![Action::Enter(entry)];
    while let Some(action) = stack.pop() {
        match action {
            Action::Enter(block_id) => {
                // push scope for this block contributions
                available.push_scope();

                // process instructions in this block
                process_block(
                    block_id,
                    tree,
                    aa,
                    memory_ssa,
                    value_types,
                    type_context,
                    &mut available,
                    &mut substitutions,
                    &mut to_remove,
                );

                // schedule leave after all children are processed
                stack.push(Action::Leave);

                // schedule children in reverse so first child is processed first
                let children = dom_children.get(&block_id).cloned().unwrap_or_default();
                for child in children.into_iter().rev() {
                    stack.push(Action::Enter(child));
                }
            }
            Action::Leave => {
                // drop the current scope
                available.pop_scope();
            }
        }
    }

    (substitutions, to_remove)
}

/// Process a single block, tracking available values and finding forwardable loads.
#[allow(clippy::too_many_arguments)]
fn process_block(
    block_id: mir::LocalNodeId<mir::Block>,
    tree: &mir::NodeTree,
    aa: &AliasAnalysis,
    memory_ssa: &MemorySSA,
    value_types: &ValueTypeMap,
    _type_context: TypeContext,
    available: &mut AvailableMemory,
    substitutions: &mut HashMap<mir::Value, mir::Value>,
    to_remove: &mut HashSet<mir::LocalNodeId<mir::Instruction>>,
) {
    // read the block
    let block = tree.get(block_id);

    // scan instructions in the block
    for &instruction_id in &block.instructions {
        // read the instruction
        let instruction = tree.get(instruction_id);

        // update availability based on instruction kind
        match instruction {
            mir::Instruction::Store { value, .. } | mir::Instruction::LocalSet { value, .. } => {
                let Some(value) = value.value() else {
                    continue;
                };

                // resolve the memory def access
                let Some(def_access_id) = def_access_id(memory_ssa, instruction_id) else {
                    continue;
                };

                // read the def access data
                let MemoryAccess::Def(def_access) = memory_ssa.access(def_access_id) else {
                    continue;
                };

                // skip untrackable accesses
                if !effect_is_trackable(&def_access.effect) {
                    continue;
                }

                // record the available value
                available.insert(MemoryEntry {
                    clobber: def_access_id,
                    location: def_access.effect.location.clone(),
                    value,
                    space_set: def_access.effect.space_set,
                    address_spaces: def_access.effect.address_spaces.clone(),
                    alias_scopes: def_access.effect.alias_scopes.clone(),
                    noalias_scopes: def_access.effect.noalias_scopes.clone(),
                    type_alias_tag: def_access.effect.type_alias_tag,
                });
            }

            mir::Instruction::Load { destination, .. } => {
                let Some(destination) = destination.value() else {
                    continue;
                };

                // resolve the memory use access
                let Some(use_access_id) = use_access_id(memory_ssa, instruction_id) else {
                    continue;
                };

                // read the use access data
                let MemoryAccess::Use(use_access) = memory_ssa.access(use_access_id) else {
                    continue;
                };

                // skip untrackable reads
                if !effect_is_trackable(&use_access.effect) {
                    continue;
                }

                // compute the clobbering access for this read
                let clobber = memory_ssa.clobbering_access_for_use(use_access_id, aa, tree);
                let Some(clobber) = resolve_trivial_clobber(memory_ssa, clobber) else {
                    continue;
                };

                // forward from an existing value when possible
                if let Some(existing) = available.get(clobber, &use_access.effect, aa, tree)
                    && can_substitute_value(destination, existing, value_types, tree)
                {
                    substitutions.insert(destination, existing);
                    to_remove.insert(instruction_id);
                } else {
                    available.insert(MemoryEntry {
                        clobber,
                        location: use_access.effect.location.clone(),
                        value: destination,
                        space_set: use_access.effect.space_set,
                        address_spaces: use_access.effect.address_spaces.clone(),
                        alias_scopes: use_access.effect.alias_scopes.clone(),
                        noalias_scopes: use_access.effect.noalias_scopes.clone(),
                        type_alias_tag: use_access.effect.type_alias_tag,
                    });
                }
            }

            mir::Instruction::LocalGet { destination, .. } => {
                let Some(destination) = destination.value() else {
                    continue;
                };

                // resolve the memory use access
                let Some(use_access_id) = use_access_id(memory_ssa, instruction_id) else {
                    continue;
                };

                // read the use access data
                let MemoryAccess::Use(use_access) = memory_ssa.access(use_access_id) else {
                    continue;
                };

                // skip untrackable reads
                if !effect_is_trackable(&use_access.effect) {
                    continue;
                }

                // compute the clobbering access for this read
                let clobber = memory_ssa.clobbering_access_for_use(use_access_id, aa, tree);
                let Some(clobber) = resolve_trivial_clobber(memory_ssa, clobber) else {
                    continue;
                };

                // forward from an existing value when possible
                if let Some(existing) = available.get(clobber, &use_access.effect, aa, tree)
                    && can_substitute_value(destination, existing, value_types, tree)
                {
                    substitutions.insert(destination, existing);
                    to_remove.insert(instruction_id);
                } else {
                    available.insert(MemoryEntry {
                        clobber,
                        location: use_access.effect.location.clone(),
                        value: destination,
                        space_set: use_access.effect.space_set,
                        address_spaces: use_access.effect.address_spaces.clone(),
                        alias_scopes: use_access.effect.alias_scopes.clone(),
                        noalias_scopes: use_access.effect.noalias_scopes.clone(),
                        type_alias_tag: use_access.effect.type_alias_tag,
                    });
                }
            }

            mir::Instruction::Intrinsic { intrinsic, .. } => {
                // clear on volatile or atomic barriers
                if is_memory_barrier(*intrinsic) {
                    available.clear();
                }
            }
            mir::Instruction::AtomicLoad { .. }
            | mir::Instruction::AtomicStore { .. }
            | mir::Instruction::AtomicCompareExchange { .. }
            | mir::Instruction::AtomicRmw { .. }
            | mir::Instruction::AtomicFence { .. }
            | mir::Instruction::Barrier { .. } => {
                available.clear();
            }

            mir::Instruction::Dispose { .. }
            | mir::Instruction::AsyncDispose { .. }
            | mir::Instruction::Pin { .. }
            | mir::Instruction::Unpin { .. }
            | mir::Instruction::Drop { .. } => {
                // clear across destructor boundaries
                available.clear();
            }

            _ => {}
        }
    }
}

fn def_access_id(
    memory_ssa: &MemorySSA,
    instruction_id: mir::LocalNodeId<mir::Instruction>,
) -> Option<MemoryAccessId> {
    // find the first def access for the instruction
    let accesses = memory_ssa.accesses_for_instruction(instruction_id)?;
    for &access_id in accesses {
        if matches!(memory_ssa.access(access_id), MemoryAccess::Def(_)) {
            return Some(access_id);
        }
    }

    None
}

fn use_access_id(
    memory_ssa: &MemorySSA,
    instruction_id: mir::LocalNodeId<mir::Instruction>,
) -> Option<MemoryAccessId> {
    // find the first use access for the instruction
    let accesses = memory_ssa.accesses_for_instruction(instruction_id)?;
    for &access_id in accesses {
        if matches!(memory_ssa.access(access_id), MemoryAccess::Use(_)) {
            return Some(access_id);
        }
    }

    None
}

/// Resolve trivial memory phi nodes to a single clobbering access.
fn resolve_trivial_clobber(
    memory_ssa: &MemorySSA,
    access: MemoryAccessId,
) -> Option<MemoryAccessId> {
    let mut current = access;
    let mut visited = HashSet::new();

    loop {
        if !visited.insert(current) {
            return None;
        }

        let MemoryAccess::Phi(phi) = memory_ssa.access(current) else {
            return Some(current);
        };

        let mut incoming = phi.incoming.iter().map(|(_, access_id)| *access_id);
        let first = incoming.next()?;
        if incoming.all(|access_id| access_id == first) {
            current = first;
            continue;
        }

        return None;
    }
}

/// Check if an intrinsic acts as a memory barrier.
fn is_memory_barrier(intrinsic: mir::Intrinsic) -> bool {
    // match barrier intrinsics
    matches!(intrinsic, mir::Intrinsic::WriteBarrier)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Store then load from same pointer forwards the stored value.
    #[test]
    fn test_forward_simple_store_load() {
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 42int32
    store v0, v1
    v2: int32 = load v0
    return v2
}"#;
        let expected = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 42int32
    store v0, v1
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoadStoreForward);
        test.assert_output(expected);
    }

    /// Store to one pointer, load from different pointer: no forwarding.
    #[test]
    fn test_no_forward_different_pointers() {
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: int32 = 42int32
    store v0, v2
    v3: int32 = load v1
    return v3
}"#;
        let expected = input;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoadStoreForward);
        test.assert_output(expected);
    }

    /// Second store kills first, load forwards from second store.
    #[test]
    fn test_kill_on_clobbering_store() {
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 42int32
    v2: int32 = 100int32
    store v0, v1
    store v0, v2
    v3: int32 = load v0
    return v3
}"#;
        let expected = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 42int32
    v2: int32 = 100int32
    store v0, v1
    store v0, v2
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoadStoreForward);
        test.assert_output(expected);
    }

    /// Multiple loads from same pointer all forward to stored value.
    #[test]
    fn test_forward_multiple_loads() {
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 42int32
    store v0, v1
    v2: int32 = load v0
    v3: int32 = load v0
    v4: int32 = int.add v2, v3
    return v4
}"#;
        let expected = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 42int32
    store v0, v1
    v2: int32 = int.add v1, v1
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoadStoreForward);
        test.assert_output(expected);
    }

    /// Trivial memory phis forward through a merge.
    #[test]
    fn test_forward_through_trivial_memory_phi() {
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 7int32
    v2: boolean = true
    store v0, v1
    branch v2, b1, b2
b1:
    jump b3
b2:
    jump b3
b3:
    v3: int32 = load v0
    return v3
}"#;
        let expected = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 7int32
    v2: boolean = true
    store v0, v1
    branch v2, b1, b2
b1:
    jump b3
b2:
    jump b3
b3:
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoadStoreForward);
        test.assert_output(expected);
    }

    /// Trivial memory phis with multiple incoming edges are forwarded.
    #[test]
    fn test_forward_through_triple_memory_phi() {
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 7int32
    v2: uint32 = 0uint32
    store v0, v1
    switch v2, b1, 0 => b2, 1 => b3
b1:
    jump b4
b2:
    jump b4
b3:
    jump b4
b4:
    v3: int32 = load v0
    return v3
}"#;
        let expected = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 7int32
    v2: uint32 = 0uint32
    store v0, v1
    switch v2, b1, 0 => b2, 1 => b3
b1:
    jump b4
b2:
    jump b4
b3:
    jump b4
b4:
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoadStoreForward);
        test.assert_output(expected);
    }

    /// Store to non aliasing pointer does not kill available store.
    #[test]
    fn test_forward_through_non_aliasing_store() {
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: int32 = 42int32
    v3: int32 = 100int32
    store v0, v2
    store v1, v3
    v4: int32 = load v0
    return v4
}"#;
        let expected = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: int32 = 42int32
    v3: int32 = 100int32
    store v0, v2
    store v1, v3
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoadStoreForward);
        test.assert_output(expected);
    }

    /// Store to field, load from same field forwards correctly.
    #[test]
    fn test_forward_field_access() {
        let input = r#"
type Point {
    int32;
    int32;
}
function test(): int32 {
b0:
    v0: ref<Point, raw, space(stack)> = stack.alloc Point
    v1: ref<int32, borrowed> = field.address v0, 0
    v2: int32 = 42int32
    store v1, v2
    v3: int32 = load v1
    return v3
}"#;
        let expected = r#"
type Point {
    int32;
    int32;
}
function test(): int32 {
b0:
    v0: ref<Point, raw, space(stack)> = stack.alloc Point
    v1: ref<int32, borrowed> = field.address v0, 0
    v2: int32 = 42int32
    store v1, v2
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoadStoreForward);
        test.assert_output(expected);
    }

    /// Store to different fields: forward each independently.
    #[test]
    fn test_forward_different_fields() {
        let input = r#"
type Point {
    int32;
    int32;
}
function test(): int32 {
b0:
    v0: ref<Point, raw, space(stack)> = stack.alloc Point
    v1: ref<int32, borrowed> = field.address v0, 0
    v2: ref<int32, borrowed> = field.address v0, 1
    v3: int32 = 10int32
    v4: int32 = 20int32
    store v1, v3
    store v2, v4
    v5: int32 = load v1
    v6: int32 = load v2
    v7: int32 = int.add v5, v6
    return v7
}"#;
        let expected = r#"
type Point {
    int32;
    int32;
}
function test(): int32 {
b0:
    v0: ref<Point, raw, space(stack)> = stack.alloc Point
    v1: ref<int32, borrowed> = field.address v0, 0
    v2: ref<int32, borrowed> = field.address v0, 1
    v3: int32 = 10int32
    v4: int32 = 20int32
    store v1, v3
    store v2, v4
    v5: int32 = int.add v3, v4
    return v5
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoadStoreForward);
        test.assert_output(expected);
    }

    /// Load followed by another load from same location uses first result.
    #[test]
    fn test_load_to_load_forwarding() {
        let input = r#"
function test(v0: ref<int32, raw>): int32 {
b0(v0: ref<int32, raw>):
    v1: int32 = load v0
    v2: int32 = load v0
    v3: int32 = int.add v1, v2
    return v3
}"#;
        let expected = r#"
function test(v0: ref<int32, raw>): int32 {
b0(v0: ref<int32, raw>):
    v1: int32 = load v0
    v2: int32 = int.add v1, v1
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoadStoreForward);
        test.assert_output(expected);
    }

    /// Store between loads kills the first load's availability.
    #[test]
    fn test_load_load_killed_by_store() {
        let input = r#"
function test(v0: ref<int32, raw>): int32 {
b0(v0: ref<int32, raw>):
    v1: int32 = load v0
    v2: int32 = 99int32
    store v0, v2
    v3: int32 = load v0
    v4: int32 = int.add v1, v3
    return v4
}"#;
        let expected = r#"
function test(v0: ref<int32, raw>): int32 {
b0(v0: ref<int32, raw>):
    v1: int32 = load v0
    v2: int32 = 99int32
    store v0, v2
    v3: int32 = int.add v1, v2
    return v3
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoadStoreForward);
        test.assert_output(expected);
    }

    /// Store in entry block forwards to dominated block.
    #[test]
    fn test_cross_block_forward_simple() {
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 42int32
    store v0, v1
    jump b1
b1:
    v2: int32 = load v0
    return v2
}"#;
        let expected = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 42int32
    store v0, v1
    jump b1
b1:
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoadStoreForward);
        test.assert_output(expected);
    }

    /// Store in entry forwards to both branches of a diamond.
    #[test]
    fn test_cross_block_forward_diamond() {
        let input = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: int32 = 42int32
    store v1, v2
    branch v0, b1, b2
b1:
    v3: int32 = load v1
    jump b3(v3)
b2:
    v4: int32 = load v1
    jump b3(v4)
b3(v5: int32):
    return v5
}"#;
        let expected = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: int32 = 42int32
    store v1, v2
    branch v0, b1, b2
b1:
    jump b3(v2)
b2:
    jump b3(v2)
b3(v3: int32):
    return v3
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoadStoreForward);
        test.assert_output(expected);
    }

    /// Store in one branch does not forward to sibling branch.
    #[test]
    fn test_no_forward_across_non_dominating_blocks() {
        let input = r#"
function test(v0: boolean, v1: ref<int32, raw>): int32 {
b0(v0: boolean, v1: ref<int32, raw>):
    branch v0, b1, b2
b1:
    v2: int32 = 42int32
    store v1, v2
    jump b3
b2:
    v3: int32 = load v1
    jump b3
b3:
    v4: int32 = 0int32
    return v4
}"#;
        let expected = input;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoadStoreForward);
        test.assert_output(expected);
    }

    /// Deep dominator chain: store in entry reaches deeply nested block.
    #[test]
    fn test_cross_block_deep_chain() {
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 42int32
    store v0, v1
    jump b1
b1:
    jump b2
b2:
    jump b3
b3:
    v2: int32 = load v0
    return v2
}"#;
        let expected = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 42int32
    store v0, v1
    jump b1
b1:
    jump b2
b2:
    jump b3
b3:
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoadStoreForward);
        test.assert_output(expected);
    }

    /// Load in entry forwards to dominated blocks.
    #[test]
    fn test_cross_block_load_to_load() {
        let input = r#"
function test(v0: ref<int32, raw>): int32 {
b0(v0: ref<int32, raw>):
    v1: int32 = load v0
    jump b1
b1:
    v2: int32 = load v0
    v3: int32 = int.add v1, v2
    return v3
}"#;
        let expected = r#"
function test(v0: ref<int32, raw>): int32 {
b0(v0: ref<int32, raw>):
    v1: int32 = load v0
    jump b1
b1:
    v2: int32 = int.add v1, v1
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoadStoreForward);
        test.assert_output(expected);
    }

    /// Call with pointer argument may clobber: no forwarding.
    #[test]
    fn test_no_forward_after_call() {
        let input = r#"
extern function external(ref<int32, raw>): void
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 42int32
    store v0, v1
    call external(v0): (ref<int32, raw>) -> void
    v2: int32 = load v0
    return v2
}"#;
        let expected = input;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoadStoreForward);
        test.assert_output(expected);
    }

    /// Readnone calls do not block forwarding.
    #[test]
    fn test_forward_across_readnone_call() {
        let input = r#"
extern function external(ref<int32, raw>): void
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 42int32
    store v0, v1
    call external(v0): (ref<int32, raw>) -> void
    v2: int32 = load v0
    return v2
}"#;
        let expected = r#"
extern function external(ref<int32, raw>): void
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 42int32
    store v0, v1
    call external(v0): (ref<int32, raw>) -> void
    return v1
}"#;

        let mut test = TestProgram::new(input);

        let function_id = test.entry_function_id();
        let (call_inst, _callee) = test.first_call_in_entry(function_id);

        let instruction = test.tree.get_mut(call_inst);
        let Some(memory_effect) = instruction.call_memory_effect_mut() else {
            panic!("expected call instruction");
        };
        *memory_effect = Some(mir::MemoryEffect::none());

        test.run_pass(&LoadStoreForward);
        test.assert_output(expected);
    }

    /// Call in dominator block kills forwarding to dominated blocks.
    #[test]
    fn test_call_kills_cross_block() {
        let input = r#"
extern function external(ref<int32, raw>): void
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 42int32
    store v0, v1
    call external(v0): (ref<int32, raw>) -> void
    jump b1
b1:
    v2: int32 = load v0
    return v2
}"#;
        let expected = input;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoadStoreForward);
        test.assert_output(expected);
    }

    /// Volatile load only blocks forwarding for the accessed location.
    #[test]
    fn test_volatile_load_is_barrier() {
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: int32 = 42int32
    store v0, v2
    v3: int32 = load v1
    v4: int32 = load v0
    v5: int32 = int.add v3, v4
    return v5
}"#;
        let expected = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: int32 = 42int32
    store v0, v2
    v3: int32 = load v1
    v4: int32 = int.add v3, v2
    return v4
}"#;

        let mut test = TestProgram::new(input);
        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let block = test.tree.get(function.blocks[0]);
        let volatile_load = block.instructions[2];
        test.insert_pointer_access_with_options(
            volatile_load,
            mir::MemoryAccessKind::Read,
            mir::Value::new(1),
            Some(4),
            Vec::new(),
            Vec::new(),
            None,
            true,
            None,
        );
        test.run_pass(&LoadStoreForward);
        test.assert_output(expected);
    }

    /// Volatile store only blocks forwarding for the accessed location.
    #[test]
    fn test_volatile_store_is_barrier() {
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: int32 = 42int32
    v3: int32 = 99int32
    store v0, v2
    store v1, v3
    v4: int32 = load v0
    return v4
}"#;
        let expected = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: int32 = 42int32
    v3: int32 = 99int32
    store v0, v2
    store v1, v3
    return v2
}"#;

        let mut test = TestProgram::new(input);
        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let block = test.tree.get(function.blocks[0]);
        let volatile_store = block.instructions[3];
        test.insert_pointer_access_with_options(
            volatile_store,
            mir::MemoryAccessKind::Write,
            mir::Value::new(1),
            Some(4),
            Vec::new(),
            Vec::new(),
            None,
            true,
            None,
        );
        test.run_pass(&LoadStoreForward);
        test.assert_output(expected);
    }

    /// Atomic load acts as memory barrier.
    #[test]
    fn test_atomic_load_is_barrier() {
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: int32 = 42int32
    store v0, v2
    v3: int32 = atomic.load v1, acquire, device, device, any
    v4: int32 = load v0
    v5: int32 = int.add v3, v4
    return v5
}"#;
        let expected = input;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoadStoreForward);
        test.assert_output(expected);
    }

    /// Atomic store acts as memory barrier.
    #[test]
    fn test_atomic_store_is_barrier() {
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: ref<int32, raw, space(stack)> = stack.alloc int32
    v2: int32 = 42int32
    v3: int32 = 99int32
    store v0, v2
    atomic.store v1, v3, release, device, device, any
    v4: int32 = load v0
    return v4
}"#;
        let expected = input;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoadStoreForward);
        test.assert_output(expected);
    }

    /// Atomic fence acts as memory barrier.
    #[test]
    fn test_atomic_fence_is_barrier() {
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 42int32
    store v0, v1
    atomic.fence seq_cst, device, device, any
    v2: int32 = load v0
    return v2
}"#;
        let expected = input;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoadStoreForward);
        test.assert_output(expected);
    }

    /// Scoped noalias metadata keeps stores from clobbering unrelated loads.
    #[test]
    fn test_forward_across_noalias_scope() {
        let input = r#"
function test(v0: ref<int32, raw>, v1: ref<int32, raw>): int32 {
b0(v0: ref<int32, raw>, v1: ref<int32, raw>):
    v2: int32 = 1int32
    store v0, v2
    v3: int32 = 2int32
    store v1, v3
    v4: int32 = load v0
    return v4
}"#;
        let expected = r#"
function test(v0: ref<int32, raw>, v1: ref<int32, raw>): int32 {
b0(v0: ref<int32, raw>, v1: ref<int32, raw>):
    v2: int32 = 1int32
    store v0, v2
    v3: int32 = 2int32
    store v1, v3
    return v2
}"#;

        let mut test = TestProgram::new(input);

        // create alias scope metadata for the disjoint store
        let scope = test.create_alias_scope();

        // locate the relevant instructions
        let function_id = test.first_function_id();
        let instructions = test.entry_instructions(function_id);
        let store_v1 = instructions[3];
        let load_v0 = instructions[4];

        // attach scoped metadata to disambiguate the store
        test.insert_pointer_access(
            store_v1,
            mir::MemoryAccessKind::Write,
            mir::Value::new(1),
            Some(4),
            vec![scope],
            Vec::new(),
            None,
        );

        test.insert_pointer_access(
            load_v0,
            mir::MemoryAccessKind::Read,
            mir::Value::new(0),
            Some(4),
            Vec::new(),
            vec![scope],
            None,
        );

        test.run_pass(&LoadStoreForward);
        test.assert_output(expected);
    }

    /// TBAA tags disambiguate unrelated accesses.
    #[test]
    fn test_forward_across_tbaa_disjoint() {
        let input = r#"
function test(v0: ref<int32, raw>, v1: ref<int32, raw>): int32 {
b0(v0: ref<int32, raw>, v1: ref<int32, raw>):
    v2: int32 = 1int32
    store v0, v2
    v3: int32 = 2int32
    store v1, v3
    v4: int32 = load v0
    return v4
}"#;
        let expected = r#"
function test(v0: ref<int32, raw>, v1: ref<int32, raw>): int32 {
b0(v0: ref<int32, raw>, v1: ref<int32, raw>):
    v2: int32 = 1int32
    store v0, v2
    v3: int32 = 2int32
    store v1, v3
    return v2
}"#;

        let mut test = TestProgram::new(input);

        // create disjoint tbaa tags
        let root = test.create_type_alias_node(None, false);
        let int_node = test.create_type_alias_node(Some(root), false);
        let float_node = test.create_type_alias_node(Some(root), false);
        let int_tag = test.create_type_alias_tag(root, int_node, 0, 4, false);
        let float_tag = test.create_type_alias_tag(root, float_node, 0, 4, false);

        // locate the relevant instructions
        let function_id = test.first_function_id();
        let instructions = test.entry_instructions(function_id);
        let store_v1 = instructions[3];
        let load_v0 = instructions[4];

        // attach disjoint tbaa tags to the store and load
        test.insert_pointer_access(
            store_v1,
            mir::MemoryAccessKind::Write,
            mir::Value::new(1),
            Some(4),
            Vec::new(),
            Vec::new(),
            Some(float_tag),
        );

        test.insert_pointer_access(
            load_v0,
            mir::MemoryAccessKind::Read,
            mir::Value::new(0),
            Some(4),
            Vec::new(),
            Vec::new(),
            Some(int_tag),
        );

        test.run_pass(&LoadStoreForward);
        test.assert_output(expected);
    }

    /// Disjoint TBAA offsets prevent clobbering stores from blocking forwarding.
    #[test]
    fn test_forward_across_tbaa_disjoint_offsets() {
        let input = r#"
function test(v0: ref<int32, raw>, v1: ref<int32, raw>): int32 {
b0(v0: ref<int32, raw>, v1: ref<int32, raw>):
    v2: int32 = 1int32
    store v0, v2
    v3: int32 = 2int32
    store v1, v3
    v4: int32 = load v0
    return v4
}"#;
        let expected = r#"
function test(v0: ref<int32, raw>, v1: ref<int32, raw>): int32 {
b0(v0: ref<int32, raw>, v1: ref<int32, raw>):
    v2: int32 = 1int32
    store v0, v2
    v3: int32 = 2int32
    store v1, v3
    return v2
}"#;

        let mut test = TestProgram::new(input);

        // create tbaa tags with disjoint offsets
        let root = test.create_type_alias_node(None, false);
        let access = test.create_type_alias_node(Some(root), false);
        let tag_a = test.create_type_alias_tag(root, access, 0, 4, false);
        let tag_b = test.create_type_alias_tag(root, access, 8, 4, false);

        // locate the relevant instructions
        let function_id = test.first_function_id();
        let instructions = test.entry_instructions(function_id);
        let store_v1 = instructions[3];
        let load_v0 = instructions[4];

        // attach disjoint tbaa tags to the clobbering store and load
        test.insert_pointer_access(
            store_v1,
            mir::MemoryAccessKind::Write,
            mir::Value::new(1),
            Some(4),
            Vec::new(),
            Vec::new(),
            Some(tag_b),
        );

        test.insert_pointer_access(
            load_v0,
            mir::MemoryAccessKind::Read,
            mir::Value::new(0),
            Some(4),
            Vec::new(),
            Vec::new(),
            Some(tag_a),
        );

        test.run_pass(&LoadStoreForward);
        test.assert_output(expected);
    }

    /// Size mismatches prevent forwarding from matching pointers.
    #[test]
    fn test_no_forward_size_mismatch() {
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 1int32
    store v0, v1
    v2: int32 = load v0
    return v2
}"#;
        let expected = input;

        let mut test = TestProgram::new(input);

        // locate the store and load
        let function_id = test.first_function_id();
        let instructions = test.entry_instructions(function_id);
        let store_v0 = instructions[2];
        let load_v0 = instructions[3];

        // attach mismatched sizes to block forwarding
        test.insert_pointer_access(
            store_v0,
            mir::MemoryAccessKind::Write,
            mir::Value::new(0),
            Some(8),
            Vec::new(),
            Vec::new(),
            None,
        );

        test.insert_pointer_access(
            load_v0,
            mir::MemoryAccessKind::Read,
            mir::Value::new(0),
            Some(4),
            Vec::new(),
            Vec::new(),
            None,
        );

        test.run_pass(&LoadStoreForward);
        test.assert_output(expected);
    }

    /// Transitive substitutions are resolved correctly.
    #[test]
    fn test_transitive_substitution() {
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 42int32
    store v0, v1
    v2: int32 = load v0
    v3: int32 = load v0
    v4: int32 = int.add v2, v3
    return v4
}"#;
        let expected = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = 42int32
    store v0, v1
    v2: int32 = int.add v1, v1
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoadStoreForward);
        test.assert_output(expected);
    }

    /// Empty function (import) is handled.
    #[test]
    fn test_skip_import_function() {
        let input = r#"
extern function external(): void"#;
        let expected = input;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoadStoreForward);
        test.assert_output(expected);
    }

    /// No changes returns AnalysisPreservation::all().
    #[test]
    fn test_no_changes_preserves_all() {
        let input = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = int.add v0, v0
    return v1
}"#;
        let expected = input;

        let mut test = TestProgram::new(input);
        test.run_pass(&LoadStoreForward);
        test.assert_output(expected);
    }
}
