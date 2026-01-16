use std::collections::{HashMap, HashSet};

use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::optimize::analyses::{
    AliasAnalysis, DominatorTree, MemoryAccess, MemoryAccessEffect, MemoryAccessId,
    MemoryAccessLocation, MemorySSA, OwnershipAnalysis,
};
use crate::optimize::common::{
    address_spaces_may_alias, alias_scopes_may_alias, apply_substitutions_in_function,
    can_substitute_value, location_sets_may_alias, memory_locations_compatible,
    resolve_substitution_chains, tbaa_tags_may_alias,
};
use crate::optimize::{
    AnalysisPreservation, FunctionPass, PipelineContext, TypeContext,
};

declare_pass! {
    /// Forward stored values to subsequent loads.
    ///
    /// This pass performs three optimizations:
    /// 1. **Store-to-load forwarding**: When a store is followed by a load from the same
    ///    location (with no intervening clobbers), replace the load with the stored value.
    /// 2. **Load-to-load forwarding**: When the same location is loaded twice with no
    ///    intervening clobbers, replace the second load with the first load's result.
    /// 3. **Cross-block forwarding**: Forward values across basic blocks when the store
    ///    or load dominates the use with no intervening clobbers.
    ///
    /// The pass handles:
    /// - Volatile and atomic operations (act as memory barriers)
    /// - Calls and intrinsics that may clobber memory
    /// - Aliasing through field and element access
    ///
    /// ```mir
    /// function @before() -> i32 {
    /// block0:
    ///     v0 = stack.alloc i32
    ///     v1 = iconst 42i32
    ///     store v0, v1
    ///     v2 = load v0       // forwarded from store
    ///     v3 = load v0       // forwarded from store (load-load)
    ///     v4 = iadd v2, v3
    ///     return v4
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function @after() -> i32 {
    /// block0:
    ///     v0 = stack.alloc i32
    ///     v1 = iconst 42i32
    ///     store v0, v1
    ///     v4 = iadd v1, v1
    ///     return v4
    /// }
    /// ```
    #[pass(id = "load-store-forward")]
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
    /// The memory location set for the access.
    location_set: mir::MemoryLocationSet,
    /// The address spaces for the access.
    address_spaces: Option<mir::AddressSpaceSet>,
    /// Alias scopes applied to the access.
    alias_scopes: Vec<mir::AliasScopeId>,
    /// No alias scopes applied to the access.
    noalias_scopes: Vec<mir::AliasScopeId>,
    /// Optional TBAA tag for the access.
    tbaa_tag: Option<mir::TbaaTagId>,
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
        let (aa, memory_ssa, dom_children, ownership) = {
            let analyses = ctx.function_analyses(function, tree);
            let domtree = analyses.get::<DominatorTree>();
            let aa = analyses.get::<AliasAnalysis>().clone();
            let memory_ssa = analyses.get::<MemorySSA>();
            let ownership = analyses.get::<OwnershipAnalysis>();
            let dom_children = build_dominator_children(function, &domtree);
            (aa, memory_ssa, dom_children, ownership)
        };

        // run load store forwarding
        let changed = run_load_store_forward(
            entry,
            function,
            tree,
            &aa,
            memory_ssa.as_ref(),
            &dom_children,
            ownership.as_ref(),
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

/// Core load-store forwarding logic. Returns true if changes were made.
#[allow(clippy::too_many_arguments)]
fn run_load_store_forward(
    entry: mir::LocalNodeId<mir::Block>,
    function: &mir::Function,
    tree: &mut mir::NodeTree,
    aa: &AliasAnalysis,
    memory_ssa: &MemorySSA,
    dom_children: &HashMap<mir::LocalNodeId<mir::Block>, Vec<mir::LocalNodeId<mir::Block>>>,
    ownership: &OwnershipAnalysis,
    type_context: TypeContext,
) -> bool {
    // run forwarding using dominator tree traversal
    let (substitutions, to_remove) = find_forwardable_loads(
        entry,
        tree,
        aa,
        memory_ssa,
        dom_children,
        ownership,
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
        let location = &use_effect.location;

        // skip unknown locations
        if matches!(location, MemoryAccessLocation::Unknown) {
            return None;
        }

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

                if !location_sets_may_alias(entry.location_set, use_effect.location_set) {
                    continue;
                }

                if !address_spaces_may_alias(&entry.address_spaces, &use_effect.address_spaces) {
                    continue;
                }

                if !tbaa_tags_may_alias(
                    &tree.memory_table.tbaa,
                    entry.tbaa_tag,
                    use_effect.tbaa_tag,
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
    ownership: &OwnershipAnalysis,
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
                    ownership,
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
    ownership: &OwnershipAnalysis,
    type_context: TypeContext,
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
                // resolve the memory def access
                let Some(def_access_id) = def_access_id(memory_ssa, instruction_id) else {
                    continue;
                };

                // read the def access data
                let MemoryAccess::Def(def_access) = memory_ssa.access(def_access_id) else {
                    continue;
                };

                // skip unknown or barrier accesses
                if def_access.effect.is_barrier
                    || matches!(def_access.effect.location, MemoryAccessLocation::Unknown)
                {
                    continue;
                }

                // record the available value
                available.insert(MemoryEntry {
                    clobber: def_access_id,
                    location: def_access.effect.location.clone(),
                    value: *value,
                    location_set: def_access.effect.location_set,
                    address_spaces: def_access.effect.address_spaces.clone(),
                    alias_scopes: def_access.effect.alias_scopes.clone(),
                    noalias_scopes: def_access.effect.noalias_scopes.clone(),
                    tbaa_tag: def_access.effect.tbaa_tag,
                });
            }

            mir::Instruction::Load { destination, .. } => {
                // resolve the memory use access
                let Some(use_access_id) = use_access_id(memory_ssa, instruction_id) else {
                    continue;
                };

                // read the use access data
                let MemoryAccess::Use(use_access) = memory_ssa.access(use_access_id) else {
                    continue;
                };

                // skip volatile or barrier reads
                if use_access.effect.is_volatile || use_access.effect.is_barrier {
                    continue;
                }

                // skip unknown locations
                if matches!(use_access.effect.location, MemoryAccessLocation::Unknown) {
                    continue;
                }

                // compute the clobbering access for this read
                let clobber = memory_ssa.clobbering_access_for_use(use_access_id, aa, tree);
                if matches!(memory_ssa.access(clobber), MemoryAccess::Phi(_)) {
                    continue;
                }

                // forward from an existing value when possible
                if let Some(existing) = available.get(clobber, &use_access.effect, aa, tree)
                    && can_substitute_value(
                        *destination,
                        existing,
                        ownership,
                        type_context.pointer_width_bits,
                        tree,
                    )
                {
                    substitutions.insert(*destination, existing);
                    to_remove.insert(instruction_id);
                } else {
                    available.insert(MemoryEntry {
                        clobber,
                        location: use_access.effect.location.clone(),
                        value: *destination,
                        location_set: use_access.effect.location_set,
                        address_spaces: use_access.effect.address_spaces.clone(),
                        alias_scopes: use_access.effect.alias_scopes.clone(),
                        noalias_scopes: use_access.effect.noalias_scopes.clone(),
                        tbaa_tag: use_access.effect.tbaa_tag,
                    });
                }
            }

            mir::Instruction::LocalGet { destination, .. } => {
                // resolve the memory use access
                let Some(use_access_id) = use_access_id(memory_ssa, instruction_id) else {
                    continue;
                };

                // read the use access data
                let MemoryAccess::Use(use_access) = memory_ssa.access(use_access_id) else {
                    continue;
                };

                // skip volatile or barrier reads
                if use_access.effect.is_volatile || use_access.effect.is_barrier {
                    continue;
                }

                // skip unknown locations
                if matches!(use_access.effect.location, MemoryAccessLocation::Unknown) {
                    continue;
                }

                // compute the clobbering access for this read
                let clobber = memory_ssa.clobbering_access_for_use(use_access_id, aa, tree);
                if matches!(memory_ssa.access(clobber), MemoryAccess::Phi(_)) {
                    continue;
                }

                // forward from an existing value when possible
                if let Some(existing) = available.get(clobber, &use_access.effect, aa, tree)
                    && can_substitute_value(
                        *destination,
                        existing,
                        ownership,
                        type_context.pointer_width_bits,
                        tree,
                    )
                {
                    substitutions.insert(*destination, existing);
                    to_remove.insert(instruction_id);
                } else {
                    available.insert(MemoryEntry {
                        clobber,
                        location: use_access.effect.location.clone(),
                        value: *destination,
                        location_set: use_access.effect.location_set,
                        address_spaces: use_access.effect.address_spaces.clone(),
                        alias_scopes: use_access.effect.alias_scopes.clone(),
                        noalias_scopes: use_access.effect.noalias_scopes.clone(),
                        tbaa_tag: use_access.effect.tbaa_tag,
                    });
                }
            }

            mir::Instruction::Intrinsic { intrinsic, .. } => {
                // clear on volatile or atomic barriers
                if is_memory_barrier(*intrinsic) {
                    available.clear();
                }
            }

            mir::Instruction::RawDrop { .. } | mir::Instruction::StackDrop { .. } => {
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

/// Check if an intrinsic acts as a memory barrier.
fn is_memory_barrier(intrinsic: mir::Intrinsic) -> bool {
    // match barrier intrinsics
    matches!(
        intrinsic,
        mir::Intrinsic::VolatileLoad
            | mir::Intrinsic::VolatileStore
            | mir::Intrinsic::AtomicLoad
            | mir::Intrinsic::AtomicStore
            | mir::Intrinsic::AtomicCas
            | mir::Intrinsic::AtomicFetchAdd
            | mir::Intrinsic::AtomicFetchSub
            | mir::Intrinsic::AtomicFetchAnd
            | mir::Intrinsic::AtomicFetchOr
            | mir::Intrinsic::AtomicFetchXor
            | mir::Intrinsic::AtomicFetchMin
            | mir::Intrinsic::AtomicFetchMax
            | mir::Intrinsic::AtomicFence
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Store then load from same pointer forwards the stored value.
    #[test]
    fn test_forward_simple_store_load() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = iconst 42i32
    store v0, v1
    v2 = load v0
    return v2
}"#;
        let expected = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = iconst 42i32
    store v0, v1
    return v1
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoadStoreForward);
        program.assert_output(expected);
    }

    /// Store to one pointer, load from different pointer: no forwarding.
    #[test]
    fn test_no_forward_different_pointers() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = stack.alloc i32
    v2 = iconst 42i32
    store v0, v2
    v3 = load v1
    return v3
}"#;
        let expected = input;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoadStoreForward);
        program.assert_output(expected);
    }

    /// Second store kills first, load forwards from second store.
    #[test]
    fn test_kill_on_clobbering_store() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = iconst 42i32
    v2 = iconst 100i32
    store v0, v1
    store v0, v2
    v3 = load v0
    return v3
}"#;
        let expected = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = iconst 42i32
    v2 = iconst 100i32
    store v0, v1
    store v0, v2
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoadStoreForward);
        program.assert_output(expected);
    }

    /// Multiple loads from same pointer all forward to stored value.
    #[test]
    fn test_forward_multiple_loads() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = iconst 42i32
    store v0, v1
    v2 = load v0
    v3 = load v0
    v4 = iadd v2, v3
    return v4
}"#;
        let expected = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = iconst 42i32
    store v0, v1
    v4 = iadd v1, v1
    return v4
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoadStoreForward);
        program.assert_output(expected);
    }

    /// Store to non-aliasing pointer does not kill available store.
    #[test]
    fn test_forward_through_non_aliasing_store() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = stack.alloc i32
    v2 = iconst 42i32
    v3 = iconst 100i32
    store v0, v2
    store v1, v3
    v4 = load v0
    return v4
}"#;
        let expected = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = stack.alloc i32
    v2 = iconst 42i32
    v3 = iconst 100i32
    store v0, v2
    store v1, v3
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoadStoreForward);
        program.assert_output(expected);
    }

    /// Store to field, load from same field forwards correctly.
    #[test]
    fn test_forward_field_access() {
        let input = r#"type @Point = { i32, i32 }
function @test() -> i32 {
block0:
    v0 = stack.alloc @Point
    v1 = field.addr v0, 0
    v2 = iconst 42i32
    store v1, v2
    v3 = load v1
    return v3
}"#;
        let expected = r#"type @Point = { i32, i32 }
function @test() -> i32 {
block0:
    v0 = stack.alloc @Point
    v1 = field.addr v0, 0
    v2 = iconst 42i32
    store v1, v2
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoadStoreForward);
        program.assert_output(expected);
    }

    /// Store to different fields: forward each independently.
    #[test]
    fn test_forward_different_fields() {
        let input = r#"type @Point = { i32, i32 }
function @test() -> i32 {
block0:
    v0 = stack.alloc @Point
    v1 = field.addr v0, 0
    v2 = field.addr v0, 1
    v3 = iconst 10i32
    v4 = iconst 20i32
    store v1, v3
    store v2, v4
    v5 = load v1
    v6 = load v2
    v7 = iadd v5, v6
    return v7
}"#;
        let expected = r#"type @Point = { i32, i32 }
function @test() -> i32 {
block0:
    v0 = stack.alloc @Point
    v1 = field.addr v0, 0
    v2 = field.addr v0, 1
    v3 = iconst 10i32
    v4 = iconst 20i32
    store v1, v3
    store v2, v4
    v7 = iadd v3, v4
    return v7
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoadStoreForward);
        program.assert_output(expected);
    }

    /// Load followed by another load from same location uses first result.
    #[test]
    fn test_load_to_load_forwarding() {
        let input = r#"function @test(v0: ref<raw i32>) -> i32 {
block0(v0: ref<raw i32>):
    v1 = load v0
    v2 = load v0
    v3 = iadd v1, v2
    return v3
}"#;
        let expected = r#"function @test(v0: ref<raw i32>) -> i32 {
block0(v0: ref<raw i32>):
    v1 = load v0
    v3 = iadd v1, v1
    return v3
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoadStoreForward);
        program.assert_output(expected);
    }

    /// Store between loads kills the first load's availability.
    #[test]
    fn test_load_load_killed_by_store() {
        let input = r#"function @test(v0: ref<raw i32>) -> i32 {
block0(v0: ref<raw i32>):
    v1 = load v0
    v2 = iconst 99i32
    store v0, v2
    v3 = load v0
    v4 = iadd v1, v3
    return v4
}"#;
        let expected = r#"function @test(v0: ref<raw i32>) -> i32 {
block0(v0: ref<raw i32>):
    v1 = load v0
    v2 = iconst 99i32
    store v0, v2
    v4 = iadd v1, v2
    return v4
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoadStoreForward);
        program.assert_output(expected);
    }

    /// Store in entry block forwards to dominated block.
    #[test]
    fn test_cross_block_forward_simple() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = iconst 42i32
    store v0, v1
    jump block1
block1:
    v2 = load v0
    return v2
}"#;
        let expected = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = iconst 42i32
    store v0, v1
    jump block1
block1:
    return v1
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoadStoreForward);
        program.assert_output(expected);
    }

    /// Store in entry forwards to both branches of a diamond.
    #[test]
    fn test_cross_block_forward_diamond() {
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = stack.alloc i32
    v2 = iconst 42i32
    store v1, v2
    branch v0, block1, block2
block1:
    v3 = load v1
    jump block3(v3)
block2:
    v4 = load v1
    jump block3(v4)
block3(v5: i32):
    return v5
}"#;
        let expected = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = stack.alloc i32
    v2 = iconst 42i32
    store v1, v2
    branch v0, block1, block2
block1:
    jump block3(v2)
block2:
    jump block3(v2)
block3(v5: i32):
    return v5
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoadStoreForward);
        program.assert_output(expected);
    }

    /// Store in one branch does not forward to sibling branch.
    #[test]
    fn test_no_forward_across_non_dominating_blocks() {
        let input = r#"function @test(v0: bool, v1: ref<raw i32>) -> i32 {
block0(v0: bool, v1: ref<raw i32>):
    branch v0, block1, block2
block1:
    v2 = iconst 42i32
    store v1, v2
    jump block3
block2:
    v3 = load v1
    jump block3
block3:
    v4 = iconst 0i32
    return v4
}"#;
        let expected = input;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoadStoreForward);
        program.assert_output(expected);
    }

    /// Deep dominator chain: store in entry reaches deeply nested block.
    #[test]
    fn test_cross_block_deep_chain() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = iconst 42i32
    store v0, v1
    jump block1
block1:
    jump block2
block2:
    jump block3
block3:
    v2 = load v0
    return v2
}"#;
        let expected = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = iconst 42i32
    store v0, v1
    jump block1
block1:
    jump block2
block2:
    jump block3
block3:
    return v1
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoadStoreForward);
        program.assert_output(expected);
    }

    /// Load in entry forwards to dominated blocks.
    #[test]
    fn test_cross_block_load_to_load() {
        let input = r#"function @test(v0: ref<raw i32>) -> i32 {
block0(v0: ref<raw i32>):
    v1 = load v0
    jump block1
block1:
    v2 = load v0
    v3 = iadd v1, v2
    return v3
}"#;
        let expected = r#"function @test(v0: ref<raw i32>) -> i32 {
block0(v0: ref<raw i32>):
    v1 = load v0
    jump block1
block1:
    v3 = iadd v1, v1
    return v3
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoadStoreForward);
        program.assert_output(expected);
    }

    /// Call with pointer argument may clobber: no forwarding.
    #[test]
    fn test_no_forward_after_call() {
        let input = r#"extern function @external(ref<raw i32>) -> void
function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = iconst 42i32
    store v0, v1
    call @external(v0)
    v2 = load v0
    return v2
}"#;
        let expected = input;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoadStoreForward);
        program.assert_output(expected);
    }

    /// Readnone calls do not block forwarding.
    #[test]
    fn test_forward_across_readnone_call() {
        let input = r#"extern function @external(ref<raw i32>) -> void
function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = iconst 42i32
    store v0, v1
    call @external(v0)
    v2 = load v0
    return v2
}"#;
        let expected = r#"extern function @external(ref<raw i32>) -> void
function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = iconst 42i32
    store v0, v1
    call @external(v0)
    return v1
}"#;

        let mut program = TestProgram::new(input);

        let function_id = program.entry_function_id();
        let (call_inst, callee) = program.first_call_in_entry(function_id);
        let signature = program.call_signature_for_callee(callee);

        let metadata = mir::CallMetadata::direct(callee, signature)
            .with_memory_effects(mir::MemoryEffect::none());
        program
            .tree
            .call_table
            .call_metadata_by_instruction_id
            .insert(call_inst, metadata);

        program.run_pass(&LoadStoreForward);
        program.assert_output(expected);
    }

    /// Call in dominator block kills forwarding to dominated blocks.
    #[test]
    fn test_call_kills_cross_block() {
        let input = r#"extern function @external(ref<raw i32>) -> void
function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = iconst 42i32
    store v0, v1
    call @external(v0)
    jump block1
block1:
    v2 = load v0
    return v2
}"#;
        let expected = input;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoadStoreForward);
        program.assert_output(expected);
    }

    /// Volatile load acts as memory barrier: kills all forwarding.
    #[test]
    fn test_volatile_load_is_barrier() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = stack.alloc i32
    v2 = iconst 42i32
    store v0, v2
    v3 = intrinsic.volatile.load(v1)
    v4 = load v0
    v5 = iadd v3, v4
    return v5
}"#;
        let expected = input;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoadStoreForward);
        program.assert_output(expected);
    }

    /// Volatile store acts as memory barrier.
    #[test]
    fn test_volatile_store_is_barrier() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = stack.alloc i32
    v2 = iconst 42i32
    v3 = iconst 99i32
    store v0, v2
    intrinsic.volatile.store(v1, v3)
    v4 = load v0
    return v4
}"#;
        let expected = input;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoadStoreForward);
        program.assert_output(expected);
    }

    /// Atomic load acts as memory barrier.
    #[test]
    fn test_atomic_load_is_barrier() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = stack.alloc i32
    v2 = iconst 42i32
    store v0, v2
    v3 = intrinsic.atomic.load(v1, acquire)
    v4 = load v0
    v5 = iadd v3, v4
    return v5
}"#;
        let expected = input;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoadStoreForward);
        program.assert_output(expected);
    }

    /// Atomic store acts as memory barrier.
    #[test]
    fn test_atomic_store_is_barrier() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = stack.alloc i32
    v2 = iconst 42i32
    v3 = iconst 99i32
    store v0, v2
    intrinsic.atomic.store(v1, v3, release)
    v4 = load v0
    return v4
}"#;
        let expected = input;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoadStoreForward);
        program.assert_output(expected);
    }

    /// Atomic fence acts as memory barrier.
    #[test]
    fn test_atomic_fence_is_barrier() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = iconst 42i32
    store v0, v1
    intrinsic.atomic.fence(seq_cst)
    v2 = load v0
    return v2
}"#;
        let expected = input;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoadStoreForward);
        program.assert_output(expected);
    }

    /// Scoped noalias metadata keeps stores from clobbering unrelated loads.
    #[test]
    fn test_forward_across_noalias_scope() {
        let input = r#"function @test(v0: ref<raw mut i32>, v1: ref<raw mut i32>) -> i32 {
block0(v0: ref<raw mut i32>, v1: ref<raw mut i32>):
    v2 = iconst 1i32
    store v0, v2
    v3 = iconst 2i32
    store v1, v3
    v4 = load v0
    return v4
}"#;
        let expected = r#"function @test(v0: ref<raw mut i32>, v1: ref<raw mut i32>) -> i32 {
block0(v0: ref<raw mut i32>, v1: ref<raw mut i32>):
    v2 = iconst 1i32
    store v0, v2
    v3 = iconst 2i32
    store v1, v3
    return v2
}"#;

        let mut program = TestProgram::new(input);

        // create alias scope metadata for the disjoint store
        let scope = program.create_alias_scope();

        // locate the relevant instructions
        let function_id = program.first_function_id();
        let instructions = program.entry_instructions(function_id);
        let store_v1 = instructions[3];
        let load_v0 = instructions[4];

        // attach scoped metadata to disambiguate the store
        program.insert_pointer_access(
            store_v1,
            mir::MemoryAccessKind::Write,
            mir::Value::new(1),
            Some(4),
            vec![scope],
            Vec::new(),
            None,
        );

        program.insert_pointer_access(
            load_v0,
            mir::MemoryAccessKind::Read,
            mir::Value::new(0),
            Some(4),
            Vec::new(),
            vec![scope],
            None,
        );

        program.run_pass(&LoadStoreForward);
        program.assert_output(expected);
    }

    /// TBAA tags disambiguate unrelated accesses.
    #[test]
    fn test_forward_across_tbaa_disjoint() {
        let input = r#"function @test(v0: ref<raw mut i32>, v1: ref<raw mut i32>) -> i32 {
block0(v0: ref<raw mut i32>, v1: ref<raw mut i32>):
    v2 = iconst 1i32
    store v0, v2
    v3 = iconst 2i32
    store v1, v3
    v4 = load v0
    return v4
}"#;
        let expected = r#"function @test(v0: ref<raw mut i32>, v1: ref<raw mut i32>) -> i32 {
block0(v0: ref<raw mut i32>, v1: ref<raw mut i32>):
    v2 = iconst 1i32
    store v0, v2
    v3 = iconst 2i32
    store v1, v3
    return v2
}"#;

        let mut program = TestProgram::new(input);

        // create disjoint tbaa tags
        let root = program.create_tbaa_node(None, false);
        let int_node = program.create_tbaa_node(Some(root), false);
        let float_node = program.create_tbaa_node(Some(root), false);
        let int_tag = program.create_tbaa_tag(root, int_node, 0, 4, false);
        let float_tag = program.create_tbaa_tag(root, float_node, 0, 4, false);

        // locate the relevant instructions
        let function_id = program.first_function_id();
        let instructions = program.entry_instructions(function_id);
        let store_v1 = instructions[3];
        let load_v0 = instructions[4];

        // attach disjoint tbaa tags to the store and load
        program.insert_pointer_access(
            store_v1,
            mir::MemoryAccessKind::Write,
            mir::Value::new(1),
            Some(4),
            Vec::new(),
            Vec::new(),
            Some(float_tag),
        );

        program.insert_pointer_access(
            load_v0,
            mir::MemoryAccessKind::Read,
            mir::Value::new(0),
            Some(4),
            Vec::new(),
            Vec::new(),
            Some(int_tag),
        );

        program.run_pass(&LoadStoreForward);
        program.assert_output(expected);
    }

    /// Disjoint TBAA offsets prevent clobbering stores from blocking forwarding.
    #[test]
    fn test_forward_across_tbaa_disjoint_offsets() {
        let input = r#"function @test(v0: ref<raw mut i32>, v1: ref<raw mut i32>) -> i32 {
block0(v0: ref<raw mut i32>, v1: ref<raw mut i32>):
    v2 = iconst 1i32
    store v0, v2
    v3 = iconst 2i32
    store v1, v3
    v4 = load v0
    return v4
}"#;
        let expected = r#"function @test(v0: ref<raw mut i32>, v1: ref<raw mut i32>) -> i32 {
block0(v0: ref<raw mut i32>, v1: ref<raw mut i32>):
    v2 = iconst 1i32
    store v0, v2
    v3 = iconst 2i32
    store v1, v3
    return v2
}"#;

        let mut program = TestProgram::new(input);

        // create tbaa tags with disjoint offsets
        let root = program.create_tbaa_node(None, false);
        let access = program.create_tbaa_node(Some(root), false);
        let tag_a = program.create_tbaa_tag(root, access, 0, 4, false);
        let tag_b = program.create_tbaa_tag(root, access, 8, 4, false);

        // locate the relevant instructions
        let function_id = program.first_function_id();
        let instructions = program.entry_instructions(function_id);
        let store_v1 = instructions[3];
        let load_v0 = instructions[4];

        // attach disjoint tbaa tags to the clobbering store and load
        program.insert_pointer_access(
            store_v1,
            mir::MemoryAccessKind::Write,
            mir::Value::new(1),
            Some(4),
            Vec::new(),
            Vec::new(),
            Some(tag_b),
        );

        program.insert_pointer_access(
            load_v0,
            mir::MemoryAccessKind::Read,
            mir::Value::new(0),
            Some(4),
            Vec::new(),
            Vec::new(),
            Some(tag_a),
        );

        program.run_pass(&LoadStoreForward);
        program.assert_output(expected);
    }

    /// Size mismatches prevent forwarding from matching pointers.
    #[test]
    fn test_no_forward_size_mismatch() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = iconst 1i32
    store v0, v1
    v2 = load v0
    return v2
}"#;
        let expected = input;

        let mut program = TestProgram::new(input);

        // locate the store and load
        let function_id = program.first_function_id();
        let instructions = program.entry_instructions(function_id);
        let store_v0 = instructions[2];
        let load_v0 = instructions[3];

        // attach mismatched sizes to block forwarding
        program.insert_pointer_access(
            store_v0,
            mir::MemoryAccessKind::Write,
            mir::Value::new(0),
            Some(8),
            Vec::new(),
            Vec::new(),
            None,
        );

        program.insert_pointer_access(
            load_v0,
            mir::MemoryAccessKind::Read,
            mir::Value::new(0),
            Some(4),
            Vec::new(),
            Vec::new(),
            None,
        );

        program.run_pass(&LoadStoreForward);
        program.assert_output(expected);
    }

    /// Transitive substitutions are resolved correctly.
    #[test]
    fn test_transitive_substitution() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = iconst 42i32
    store v0, v1
    v2 = load v0
    v3 = load v0
    v4 = iadd v2, v3
    return v4
}"#;
        let expected = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = iconst 42i32
    store v0, v1
    v4 = iadd v1, v1
    return v4
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoadStoreForward);
        program.assert_output(expected);
    }

    /// Empty function (import) is handled.
    #[test]
    fn test_skip_import_function() {
        let input = r#"extern function @external() -> void"#;
        let expected = input;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoadStoreForward);
        program.assert_output(expected);
    }

    /// No changes returns AnalysisPreservation::all().
    #[test]
    fn test_no_changes_preserves_all() {
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iadd v0, v0
    return v1
}"#;
        let expected = input;

        let mut program = TestProgram::new(input);
        program.run_pass(&LoadStoreForward);
        program.assert_output(expected);
    }
}
