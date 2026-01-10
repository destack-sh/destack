use std::collections::{HashMap, HashSet};

use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::optimize::analyses::{AliasAnalysis, AliasResult, DominatorTree};
use crate::optimize::common::{
    MemoryLocation, instruction_substitute_uses, resolve_substitution_chains,
    terminator_substitute_uses,
};
use crate::optimize::{AnalysisPreservation, FunctionAnalyses, FunctionPass, PipelineContext};

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

/// An available value at a memory location.
#[derive(Clone, Copy)]
struct AvailableValue {
    /// The value stored or loaded.
    value: mir::Value,
}

impl FunctionPass for LoadStoreForward {
    fn run(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::NodeTree,
        _ctx: &PipelineContext<'_>,
    ) -> AnalysisPreservation {
        let entry = match function.entry {
            Some(entry) => entry,
            None => return AnalysisPreservation::all(),
        };

        // get analyses
        let (aa, dom_children) = {
            let analyses = FunctionAnalyses::new(function, tree);
            let domtree = analyses.get::<DominatorTree>();
            let aa = analyses.get::<AliasAnalysis>().clone();
            let dom_children = build_dominator_children(function, &domtree);
            (aa, dom_children)
        };

        // run load-store forwarding
        let changed = run_load_store_forward(entry, function, tree, &aa, &dom_children);
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
fn run_load_store_forward(
    entry: mir::LocalNodeId<mir::Block>,
    function: &mir::Function,
    tree: &mut mir::NodeTree,
    aa: &AliasAnalysis,
    dom_children: &HashMap<mir::LocalNodeId<mir::Block>, Vec<mir::LocalNodeId<mir::Block>>>,
) -> bool {
    // run forwarding using dominator tree traversal
    let (substitutions, to_remove) = find_forwardable_loads(entry, tree, aa, dom_children);

    // nothing to do if no forwarding found
    if substitutions.is_empty() {
        return false;
    }

    // resolve transitive substitution chains
    let substitutions = resolve_substitution_chains(substitutions);

    // apply substitutions and remove forwarded loads
    apply_substitutions(function, tree, &substitutions, &to_remove);

    true
}

/// Build a map from each block to its children in the dominator tree.
fn build_dominator_children(
    function: &mir::Function,
    domtree: &DominatorTree,
) -> HashMap<mir::LocalNodeId<mir::Block>, Vec<mir::LocalNodeId<mir::Block>>> {
    let mut children: HashMap<_, Vec<_>> = HashMap::new();

    // initialize all blocks with empty children lists
    for &block_id in &function.blocks {
        children.insert(block_id, Vec::new());
    }

    // build parent -> children mapping from idom relationships
    for &block_id in &function.blocks {
        if let Some(idom) = domtree.immediate_dominator(block_id) {
            children.get_mut(&idom).unwrap().push(block_id);
        }
    }

    children
}

/// Scoped table of available memory values.
///
/// Tracks both stored values and loaded values.
/// Supports push/pop for dominator tree traversal.
struct AvailableMemory {
    /// Stack of scopes, each mapping pointers to available values.
    scopes: Vec<HashMap<mir::Value, AvailableValue>>,
}

impl AvailableMemory {
    /// Create a new empty scoped table with one scope.
    fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
        }
    }

    /// Push a new scope for entering a dominated block.
    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    /// Pop the current scope when leaving a dominated block.
    fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    /// Look up an available value for the given pointer.
    ///
    /// Checks exact pointer match first, then uses alias analysis.
    fn get(&self, pointer: mir::Value, aa: &AliasAnalysis) -> Option<AvailableValue> {
        let load_loc = MemoryLocation::from_ptr(pointer);

        // search from innermost to outermost scope
        for scope in self.scopes.iter().rev() {
            // exact pointer match (fast path)
            if let Some(&av) = scope.get(&pointer) {
                return Some(av);
            }

            // alias analysis for derived pointers
            for (&stored_ptr, &av) in scope {
                let store_loc = MemoryLocation::from_ptr(stored_ptr);
                if aa.alias(&load_loc, &store_loc) == AliasResult::MustAlias {
                    return Some(av);
                }
            }
        }

        None
    }

    /// Insert an available value in the current scope.
    fn insert(&mut self, pointer: mir::Value, value: mir::Value) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(pointer, AvailableValue { value });
        }
    }

    /// Invalidate entries that may alias the given location.
    fn invalidate_may_alias(&mut self, pointer: mir::Value, aa: &AliasAnalysis) {
        let store_loc = MemoryLocation::from_ptr(pointer);

        for scope in &mut self.scopes {
            scope.retain(|&ptr, _| {
                let loc = MemoryLocation::from_ptr(ptr);
                aa.alias(&loc, &store_loc).is_no_alias()
            });
        }
    }

    /// Invalidate entries that may be clobbered by the given instruction.
    fn invalidate_clobbered(
        &mut self,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
        aa: &AliasAnalysis,
    ) {
        for scope in &mut self.scopes {
            scope.retain(|&ptr, _| {
                let loc = MemoryLocation::from_ptr(ptr);
                !aa.may_clobber(instruction_id, &loc)
            });
        }
    }

    /// Invalidate all entries (conservative for unknown effects).
    fn invalidate_all(&mut self) {
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
    dom_children: &HashMap<mir::LocalNodeId<mir::Block>, Vec<mir::LocalNodeId<mir::Block>>>,
) -> (
    HashMap<mir::Value, mir::Value>,
    HashSet<mir::LocalNodeId<mir::Instruction>>,
) {
    let mut substitutions: HashMap<mir::Value, mir::Value> = HashMap::new();
    let mut to_remove: HashSet<mir::LocalNodeId<mir::Instruction>> = HashSet::new();
    let mut available = AvailableMemory::new();

    // work stack for dominator tree traversal
    enum Action {
        Enter(mir::LocalNodeId<mir::Block>),
        Leave,
    }

    let mut stack = vec![Action::Enter(entry)];
    while let Some(action) = stack.pop() {
        match action {
            Action::Enter(block_id) => {
                // push scope for this block's contributions
                available.push_scope();

                // process instructions in this block
                process_block(
                    block_id,
                    tree,
                    aa,
                    &mut available,
                    &mut substitutions,
                    &mut to_remove,
                );

                // schedule Leave after all children are processed
                stack.push(Action::Leave);

                // schedule children in reverse so first child is processed first
                let children = dom_children.get(&block_id).cloned().unwrap_or_default();
                for child in children.into_iter().rev() {
                    stack.push(Action::Enter(child));
                }
            }
            Action::Leave => {
                available.pop_scope();
            }
        }
    }

    (substitutions, to_remove)
}

/// Process a single block, tracking available values and finding forwardable loads.
fn process_block(
    block_id: mir::LocalNodeId<mir::Block>,
    tree: &mir::NodeTree,
    aa: &AliasAnalysis,
    available: &mut AvailableMemory,
    substitutions: &mut HashMap<mir::Value, mir::Value>,
    to_remove: &mut HashSet<mir::LocalNodeId<mir::Instruction>>,
) {
    let block = tree.get(block_id);

    for &instruction_id in &block.instructions {
        let inst = tree.get(instruction_id);

        match inst {
            // store makes a value available
            mir::Instruction::Store { pointer, value } => {
                // invalidate any entries that may alias this store
                available.invalidate_may_alias(*pointer, aa);

                // record this store as available
                available.insert(*pointer, *value);
            }

            // load can be forwarded if we have an available value
            mir::Instruction::Load {
                destination,
                pointer,
            } => {
                if let Some(av) = available.get(*pointer, aa) {
                    // forward the available value
                    substitutions.insert(*destination, av.value);
                    to_remove.insert(instruction_id);
                } else {
                    // no available value; record this load's result as available
                    // (for load-to-load forwarding)
                    available.insert(*pointer, *destination);
                }
            }

            // calls may clobber memory
            mir::Instruction::Call { .. } | mir::Instruction::CallIndirect { .. } => {
                available.invalidate_clobbered(instruction_id, aa);
            }

            // intrinsics need special handling
            mir::Instruction::Intrinsic { intrinsic, .. } => {
                // volatile and atomic operations act as full memory barriers
                if is_memory_barrier(*intrinsic) {
                    available.invalidate_all();
                } else if intrinsic.has_memory_effects() {
                    available.invalidate_clobbered(instruction_id, aa);
                }
            }

            // drops may run destructors which can access any memory
            mir::Instruction::RawDrop { .. } | mir::Instruction::StackDrop { .. } => {
                available.invalidate_all();
            }

            // freeing memory invalidates any available value from that pointer
            mir::Instruction::RawFree { pointer } => {
                available.invalidate_may_alias(*pointer, aa);
            }

            _ => {}
        }
    }
}

/// Check if an intrinsic acts as a memory barrier.
fn is_memory_barrier(intrinsic: mir::Intrinsic) -> bool {
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

/// Apply value substitutions and remove forwarded loads.
fn apply_substitutions(
    function: &mir::Function,
    tree: &mut mir::NodeTree,
    substitutions: &HashMap<mir::Value, mir::Value>,
    to_remove: &HashSet<mir::LocalNodeId<mir::Instruction>>,
) {
    for &block_id in &function.blocks {
        let mut new_block = tree.get(block_id).clone();
        let mut modified = false;

        // substitute in instructions
        for &instruction_id in &new_block.instructions {
            if to_remove.contains(&instruction_id) {
                continue;
            }

            let inst = tree.get(instruction_id);
            let new_inst = instruction_substitute_uses(inst, substitutions);
            if &new_inst != inst {
                tree.replace(instruction_id, new_inst);
                modified = true;
            }
        }

        // substitute in terminator
        let new_term = terminator_substitute_uses(&new_block.terminator, substitutions);
        if new_term != new_block.terminator {
            new_block.terminator = new_term;
            modified = true;
        }

        // remove forwarded loads
        let orig_len = new_block.instructions.len();
        new_block.instructions.retain(|id| !to_remove.contains(id));
        if new_block.instructions.len() != orig_len {
            modified = true;
        }

        if modified {
            tree.replace(block_id, new_block);
        }
    }
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
        // v3 should forward from store (v2), not from first load
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
        // v3 in block2 cannot see store in block1 (not dominated)
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
        // volatile load kills forwarding of v2 to v4
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
        // both v2 and v3 should resolve to v1
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
