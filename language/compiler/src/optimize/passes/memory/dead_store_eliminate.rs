use std::collections::{HashMap, HashSet};

use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::optimize::{
    AliasAnalysis, AnalysisPreservation, ControlFlowGraph, FunctionAnalyses, FunctionPass,
    MemoryLocation, PipelineContext,
};

declare_pass! {
    /// Dead Store Elimination.
    ///
    /// Removes stores to memory locations that are never read:
    /// 1. Stores followed by another store to the same location (no intervening load)
    /// 2. Stores to locations that are never read before the function exits
    ///
    /// This pass uses alias analysis to determine when stores may alias with
    /// loads or other stores.
    ///
    /// ```mir
    /// // before DSE
    /// function @before() -> i32 {
    /// block0:
    ///     v0 = stack.alloc i32
    ///     v1 = iconst 1i32
    ///     store v0, v1        // dead: overwritten below
    ///     v2 = iconst 2i32
    ///     store v0, v2
    ///     v3 = load v0
    ///     return v3
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// // after DSE
    /// function @after() -> i32 {
    /// block0:
    ///     v0 = stack.alloc i32
    ///     v1 = iconst 1i32
    ///     // store removed
    ///     v2 = iconst 2i32
    ///     store v0, v2
    ///     v3 = load v0
    ///     return v3
    /// }
    /// ```
    #[pass(id = "dse")]
    pub DeadStoreEliminate,
    "Remove dead stores"
}

impl FunctionPass for DeadStoreEliminate {
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
        let (aa, cfg) = {
            let analyses = FunctionAnalyses::new(function, tree);
            (
                analyses.get::<AliasAnalysis>().clone(),
                analyses.get::<ControlFlowGraph>().clone(),
            )
        };

        // run dead store elimination
        let changed = run_dead_store_eliminate(function, tree, &aa, &cfg, entry);
        if changed {
            AnalysisPreservation::none()
        } else {
            AnalysisPreservation::all()
        }
    }

    fn name(&self) -> &'static str {
        "DeadStoreEliminate"
    }

    fn id(&self) -> &'static str {
        "dse"
    }
}

/// Core DSE logic. Returns true if changes were made.
fn run_dead_store_eliminate(
    function: &mut mir::Function,
    tree: &mut mir::NodeTree,
    aa: &AliasAnalysis,
    cfg: &ControlFlowGraph,
    entry: mir::LocalNodeId<mir::Block>,
) -> bool {
    // find dead stores
    let dead_stores = find_dead_stores(function, tree, aa, cfg, entry);

    if dead_stores.is_empty() {
        return false;
    }

    // remove dead stores
    for &block_id in &function.blocks {
        let block = tree.get_mut(block_id);
        block.instructions.retain(|id| !dead_stores.contains(id));
    }

    true
}

/// Find stores that are dead (never read before being overwritten or function exit).
fn find_dead_stores(
    function: &mir::Function,
    tree: &mir::NodeTree,
    aa: &AliasAnalysis,
    cfg: &ControlFlowGraph,
    entry: mir::LocalNodeId<mir::Block>,
) -> HashSet<mir::LocalNodeId<mir::Instruction>> {
    let mut dead_stores = HashSet::new();

    // phase 1: find locally dead stores (overwritten before any read within same block)
    for &block_id in &function.blocks {
        let local_dead = find_locally_dead_stores(block_id, tree, aa);
        dead_stores.extend(local_dead);
    }

    // phase 2: find cross-block dead stores
    // a store is dead if it's overwritten on ALL successor paths before any load
    let cross_block_dead = find_cross_block_dead_stores(function, tree, aa, cfg, entry);
    dead_stores.extend(cross_block_dead);

    // phase 3: find stores to stack allocations that are never read globally
    let unreachable_stores = find_unreachable_stores(function, tree, aa, cfg);
    dead_stores.extend(unreachable_stores);

    dead_stores
}

/// Information about stores that are "available" (not yet read) at a program point.
#[derive(Clone, Debug)]
struct AvailableStore {
    instruction: mir::LocalNodeId<mir::Instruction>,
    pointer: mir::Value,
}

/// Find cross-block dead stores using forward dataflow analysis.
///
/// A store is dead if on ALL successor paths:
/// 1. Another store to the same location overwrites it, AND
/// 2. No load from that location occurs before the overwrite
fn find_cross_block_dead_stores(
    function: &mir::Function,
    tree: &mir::NodeTree,
    aa: &AliasAnalysis,
    cfg: &ControlFlowGraph,
    entry: mir::LocalNodeId<mir::Block>,
) -> HashSet<mir::LocalNodeId<mir::Instruction>> {
    // compute stores available at exit of each block
    // "available" means: store that might still be live (not yet read)
    let available_at_exit = compute_available_stores(function, tree, aa, cfg, entry);

    // for each available store at block exit, check if it's overwritten on all successor paths
    let mut dead_stores = HashSet::new();

    for &block_id in &function.blocks {
        let block = tree.get(block_id);
        let successors: Vec<_> = block.terminator.successors().into_iter().collect();

        // skip blocks with no successors (return/unreachable)
        if successors.is_empty() {
            continue;
        }

        // get available stores at exit of this block
        let available = match available_at_exit.get(&block_id) {
            Some(stores) => stores,
            None => continue,
        };

        // for each available store, check if overwritten on ALL successor paths
        for store in available {
            let mut overwritten_on_all_paths = true;
            for &succ_id in &successors {
                if !is_store_overwritten_at_block_entry(store, succ_id, tree, aa) {
                    overwritten_on_all_paths = false;
                    break;
                }
            }
            if overwritten_on_all_paths {
                dead_stores.insert(store.instruction);
            }
        }
    }

    dead_stores
}

/// Compute stores that are "available" (not yet read) at exit of each block.
fn compute_available_stores(
    function: &mir::Function,
    tree: &mir::NodeTree,
    aa: &AliasAnalysis,
    _cfg: &ControlFlowGraph,
    _entry: mir::LocalNodeId<mir::Block>,
) -> HashMap<mir::LocalNodeId<mir::Block>, Vec<AvailableStore>> {
    let mut available_at_exit: HashMap<mir::LocalNodeId<mir::Block>, Vec<AvailableStore>> =
        HashMap::new();

    for &block_id in &function.blocks {
        let block = tree.get(block_id);
        let mut available: Vec<AvailableStore> = Vec::new();

        for &inst_id in &block.instructions {
            let inst = tree.get(inst_id);

            match inst {
                mir::Instruction::Store { pointer, .. } => {
                    // remove any stores that this store overwrites
                    available.retain(|s| {
                        // same pointer value definitely means same location
                        if s.pointer == *pointer {
                            return false; // overwritten
                        }
                        // check alias analysis for derived pointers
                        let s_loc = MemoryLocation::from_ptr(s.pointer);
                        let loc = MemoryLocation::from_ptr(*pointer);
                        !aa.alias(&s_loc, &loc).is_must_alias()
                    });

                    // add this store as available
                    available.push(AvailableStore {
                        instruction: inst_id,
                        pointer: *pointer,
                    });
                }

                mir::Instruction::Load { pointer, .. } => {
                    let loc = MemoryLocation::from_ptr(*pointer);

                    // remove any stores that may be read by this load
                    available.retain(|s| {
                        let s_loc = MemoryLocation::from_ptr(s.pointer);
                        !aa.alias(&s_loc, &loc).may_alias()
                    });
                }

                // calls may read memory (use mod-ref analysis)
                mir::Instruction::Call { .. } | mir::Instruction::CallIndirect { .. } => {
                    available.retain(|s| {
                        let store_loc = MemoryLocation::from_ptr(s.pointer);
                        !aa.get_mod_ref_info(inst_id, &store_loc).is_ref()
                    });
                }

                // drop/free may read memory via destructors (use mod-ref analysis)
                mir::Instruction::RawDrop { .. }
                | mir::Instruction::StackDrop { .. }
                | mir::Instruction::RawFree { .. } => {
                    available.retain(|s| {
                        let store_loc = MemoryLocation::from_ptr(s.pointer);
                        !aa.get_mod_ref_info(inst_id, &store_loc).is_ref()
                    });
                }

                _ => {}
            }
        }

        available_at_exit.insert(block_id, available);
    }

    available_at_exit
}

/// Check if a store is overwritten at the entry of a block.
///
/// A store is overwritten if the block contains a store to the same location
/// before any load from that location.
fn is_store_overwritten_at_block_entry(
    store: &AvailableStore,
    block_id: mir::LocalNodeId<mir::Block>,
    tree: &mir::NodeTree,
    aa: &AliasAnalysis,
) -> bool {
    let block = tree.get(block_id);
    let store_loc = MemoryLocation::from_ptr(store.pointer);

    for &instruction_id in &block.instructions {
        let instruction = tree.get(instruction_id);
        match instruction {
            mir::Instruction::Store { pointer, .. } => {
                // same pointer value definitely means same location
                if *pointer == store.pointer {
                    return true;
                }
                // check alias analysis for derived pointers
                let loc = MemoryLocation::from_ptr(*pointer);
                if aa.alias(&store_loc, &loc).is_must_alias() {
                    return true;
                }
            }

            mir::Instruction::Load { pointer, .. } => {
                let loc = MemoryLocation::from_ptr(*pointer);
                if aa.alias(&store_loc, &loc).may_alias() {
                    // store may be read - not overwritten
                    return false;
                }
            }

            // calls may read memory (use mod-ref analysis)
            mir::Instruction::Call { .. } | mir::Instruction::CallIndirect { .. } => {
                if aa.get_mod_ref_info(instruction_id, &store_loc).is_ref() {
                    return false;
                }
            }

            // drop/free may read memory (use mod-ref analysis)
            mir::Instruction::RawDrop { .. }
            | mir::Instruction::StackDrop { .. }
            | mir::Instruction::RawFree { .. } => {
                if aa.get_mod_ref_info(instruction_id, &store_loc).is_ref() {
                    return false;
                }
            }

            _ => {}
        }
    }

    // reached end of block without being overwritten or read
    false
}

/// Find stores within a single block that are overwritten before being read.
fn find_locally_dead_stores(
    block_id: mir::LocalNodeId<mir::Block>,
    tree: &mir::NodeTree,
    aa: &AliasAnalysis,
) -> HashSet<mir::LocalNodeId<mir::Instruction>> {
    let mut dead = HashSet::new();
    let block = tree.get(block_id);

    // track the last store to each memory location
    // map from pointer value to (instruction_id, is_dead)
    let mut last_stores: HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>> = HashMap::new();

    for &instruction_id in &block.instructions {
        let instruction = tree.get(instruction_id);
        match instruction {
            mir::Instruction::Store { pointer, .. } => {
                let loc = MemoryLocation::from_ptr(*pointer);

                // check if any previous store is overwritten by this store
                for (&prev_ptr, &prev_store) in last_stores.iter() {
                    // same pointer value definitely means same location
                    if prev_ptr == *pointer {
                        dead.insert(prev_store);
                    } else {
                        // check alias analysis for other cases
                        let prev_loc = MemoryLocation::from_ptr(prev_ptr);
                        if aa.alias(&loc, &prev_loc).is_must_alias() {
                            dead.insert(prev_store);
                        }
                    }
                }

                // record this store
                last_stores.insert(*pointer, instruction_id);
            }

            mir::Instruction::Load { pointer, .. } => {
                let loc = MemoryLocation::from_ptr(*pointer);

                // any store that may alias with this load is not dead
                last_stores.retain(|&ptr, _| {
                    let store_loc = MemoryLocation::from_ptr(ptr);
                    !aa.alias(&loc, &store_loc).may_alias()
                });
            }

            // calls may read/write memory - use mod-ref analysis
            mir::Instruction::Call { .. } | mir::Instruction::CallIndirect { .. } => {
                // remove stores that the call might read
                last_stores.retain(|&ptr, _| {
                    let store_loc = MemoryLocation::from_ptr(ptr);
                    !aa.get_mod_ref_info(instruction_id, &store_loc).is_ref()
                });
            }

            // drop/free may read memory - use mod-ref analysis
            mir::Instruction::RawDrop { .. }
            | mir::Instruction::StackDrop { .. }
            | mir::Instruction::RawFree { .. } => {
                last_stores.retain(|&ptr, _| {
                    let store_loc = MemoryLocation::from_ptr(ptr);
                    !aa.get_mod_ref_info(instruction_id, &store_loc).is_ref()
                });
            }

            _ => {}
        }
    }

    dead
}

/// Find stores to locations that are never read in the entire function.
///
/// This handles stores to stack allocations that escape the function unused.
fn find_unreachable_stores(
    function: &mir::Function,
    tree: &mir::NodeTree,
    aa: &AliasAnalysis,
    _cfg: &ControlFlowGraph,
) -> HashSet<mir::LocalNodeId<mir::Instruction>> {
    // collect all stack allocations
    let mut stack_allocs: HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>> = HashMap::new();

    for &block_id in &function.blocks {
        let block = tree.get(block_id);
        for &inst_id in &block.instructions {
            let inst = tree.get(inst_id);
            if let mir::Instruction::StackAlloc { destination, .. } = inst {
                stack_allocs.insert(*destination, inst_id);
            }
        }
    }

    // find stack allocations that may escape (used in calls, returned, etc.)
    let mut escaping: HashSet<mir::Value> = HashSet::new();

    for &block_id in &function.blocks {
        let block = tree.get(block_id);

        for &instruction_id in &block.instructions {
            let instruction = tree.get(instruction_id);
            match instruction {
                mir::Instruction::Call { .. } | mir::Instruction::CallIndirect { .. } => {
                    // any stack allocation passed to a call escapes
                    // arguments are stored externally, so we need to access them via get_arguments
                    if let Some(arg_slice) = instruction.argument_slice() {
                        for &arg in tree.get_arguments(arg_slice) {
                            if stack_allocs.contains_key(&arg) {
                                escaping.insert(arg);
                            }
                        }
                    }
                }

                // storing a pointer escapes it
                mir::Instruction::Store { value, .. } => {
                    if stack_allocs.contains_key(value) {
                        escaping.insert(*value);
                    }
                }

                _ => {}
            }
        }

        // check terminator for escapes
        match &block.terminator {
            mir::Terminator::Return { value: Some(v) } => {
                if stack_allocs.contains_key(v) {
                    escaping.insert(*v);
                }
            }
            mir::Terminator::Jump { arguments, .. } => {
                for &arg in arguments {
                    if stack_allocs.contains_key(&arg) {
                        escaping.insert(arg);
                    }
                }
            }
            mir::Terminator::Branch {
                then_arguments,
                else_arguments,
                ..
            } => {
                for &arg in then_arguments.iter().chain(else_arguments.iter()) {
                    if stack_allocs.contains_key(&arg) {
                        escaping.insert(arg);
                    }
                }
            }
            mir::Terminator::Switch {
                cases,
                default_arguments,
                ..
            } => {
                // check default arguments
                for &arg in default_arguments {
                    if stack_allocs.contains_key(&arg) {
                        escaping.insert(arg);
                    }
                }
                // check case arguments
                for case in cases {
                    for &arg in &case.arguments {
                        if stack_allocs.contains_key(&arg) {
                            escaping.insert(arg);
                        }
                    }
                }
            }
            mir::Terminator::Yield {
                value,
                resume_arguments,
                ..
            } => {
                // yielded value escapes
                if stack_allocs.contains_key(value) {
                    escaping.insert(*value);
                }
                // resume arguments escape
                for &arg in resume_arguments {
                    if stack_allocs.contains_key(&arg) {
                        escaping.insert(arg);
                    }
                }
            }
            mir::Terminator::Return { value: None } | mir::Terminator::Unreachable => {}
            mir::Terminator::TailCall { arguments, .. } => {
                for &arg in arguments {
                    if stack_allocs.contains_key(&arg) {
                        escaping.insert(arg);
                    }
                }
            }
            mir::Terminator::TailCallIndirect { callee, arguments } => {
                if stack_allocs.contains_key(callee) {
                    escaping.insert(*callee);
                }
                for &arg in arguments {
                    if stack_allocs.contains_key(&arg) {
                        escaping.insert(arg);
                    }
                }
            }
        }
    }

    // find loads from each stack allocation
    let mut has_load: HashSet<mir::Value> = HashSet::new();

    for &block_id in &function.blocks {
        let block = tree.get(block_id);
        for &inst_id in &block.instructions {
            let inst = tree.get(inst_id);

            if let mir::Instruction::Load { pointer, .. } = inst {
                // check if this load is from a stack allocation
                for &alloc_ptr in stack_allocs.keys() {
                    let alloc_loc = MemoryLocation::from_ptr(alloc_ptr);
                    let load_loc = MemoryLocation::from_ptr(*pointer);
                    if aa.alias(&alloc_loc, &load_loc).may_alias() {
                        has_load.insert(alloc_ptr);
                    }
                }
            }
        }
    }

    // stores to non-escaping stack allocations with no loads are dead
    let mut dead_stores = HashSet::new();
    for &block_id in &function.blocks {
        let block = tree.get(block_id);
        for &instruction_id in &block.instructions {
            let instruction = tree.get(instruction_id);
            if let mir::Instruction::Store { pointer, .. } = instruction {
                // check if storing to a non-escaping stack alloc with no loads
                for &alloc_ptr in stack_allocs.keys() {
                    if escaping.contains(&alloc_ptr) || has_load.contains(&alloc_ptr) {
                        continue;
                    }
                    let alloc_loc = MemoryLocation::from_ptr(alloc_ptr);
                    let store_loc = MemoryLocation::from_ptr(*pointer);
                    if aa.alias(&alloc_loc, &store_loc).may_alias() {
                        dead_stores.insert(instruction_id);
                    }
                }
            }
        }
    }

    dead_stores
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Store overwritten before being read is eliminated.
    ///
    /// When a store to a location is followed by another store to the same
    /// location without an intervening load, the first store is dead.
    #[test]
    fn test_remove_overwritten_store() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = iconst 1i32
    store v0, v1
    v2 = iconst 2i32
    store v0, v2
    v3 = load v0
    return v3
}"#;
        let expected = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = iconst 1i32
    v2 = iconst 2i32
    store v0, v2
    v3 = load v0
    return v3
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&DeadStoreEliminate);
        program.assert_output(expected);
    }

    /// Store followed by load is preserved.
    ///
    /// Both stores are read before being overwritten, so both are preserved.
    #[test]
    fn test_preserve_read_store() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = iconst 1i32
    store v0, v1
    v2 = load v0
    v3 = iconst 2i32
    store v0, v3
    v4 = load v0
    return v4
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&DeadStoreEliminate);
        program.assert_unchanged(input);
    }

    /// Store to allocation that is never read is eliminated.
    ///
    /// The stack allocation is never loaded from, so the store is dead.
    #[test]
    fn test_remove_store_to_unused_alloc() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = iconst 42i32
    store v0, v1
    v2 = iconst 0i32
    return v2
}"#;
        let expected = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = iconst 42i32
    v2 = iconst 0i32
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&DeadStoreEliminate);
        program.assert_output(expected);
    }

    /// Store to escaping allocation is preserved.
    ///
    /// When the allocation escapes (is passed to an external function),
    /// the store cannot be eliminated because the external function may read it.
    #[test]
    fn test_preserve_escaping_store() {
        let input = r#"extern function @external(ref<raw i32>) -> void
function @test() -> void {
block0:
    v0 = stack.alloc i32
    v1 = iconst 42i32
    store v0, v1
    call @external(v0)
    return
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&DeadStoreEliminate);
        program.assert_unchanged(input);
    }

    /// Multiple consecutive overwrites - all but last are eliminated.
    ///
    /// When a location is stored to multiple times before being read,
    /// only the final store is preserved.
    #[test]
    fn test_multiple_overwrites() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = iconst 1i32
    store v0, v1
    v2 = iconst 2i32
    store v0, v2
    v3 = iconst 3i32
    store v0, v3
    v4 = load v0
    return v4
}"#;
        let expected = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = iconst 1i32
    v2 = iconst 2i32
    v3 = iconst 3i32
    store v0, v3
    v4 = load v0
    return v4
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&DeadStoreEliminate);
        program.assert_output(expected);
    }

    /// No changes when no stores.
    ///
    /// Function without stores is unchanged.
    #[test]
    fn test_no_changes() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = iconst 42i32
    return v0
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&DeadStoreEliminate);
        program.assert_unchanged(input);
    }

    /// Stores to different locations are independent.
    ///
    /// Stores to different memory locations don't interfere with each other.
    #[test]
    fn test_different_locations() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = stack.alloc i32
    v2 = iconst 1i32
    store v0, v2
    v3 = iconst 2i32
    store v1, v3
    v4 = load v0
    v5 = load v1
    v6 = iadd v4, v5
    return v6
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&DeadStoreEliminate);
        program.assert_unchanged(input);
    }

    /// Store before call that may read is preserved.
    ///
    /// Calls conservatively clobber memory state (for now?), so stores before calls
    /// must be preserved if the location may be read by the callee.
    #[test]
    fn test_preserve_store_before_call() {
        let input = r#"extern function @read_value(ref<raw i32>) -> i32
function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = iconst 42i32
    store v0, v1
    v2 = call @read_value(v0)
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&DeadStoreEliminate);
        program.assert_unchanged(input);
    }

    /// Store after call but overwritten before read.
    ///
    /// Even after a call, dead store elimination applies to stores
    /// that are overwritten before being read.
    #[test]
    fn test_remove_after_call_overwritten() {
        let input = r#"extern function @side_effect() -> void
function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    call @side_effect()
    v1 = iconst 1i32
    store v0, v1
    v2 = iconst 2i32
    store v0, v2
    v3 = load v0
    return v3
}"#;
        let expected = r#"extern function @side_effect() -> void
function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    call @side_effect()
    v1 = iconst 1i32
    v2 = iconst 2i32
    store v0, v2
    v3 = load v0
    return v3
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&DeadStoreEliminate);
        program.assert_output(expected);
    }

    /// Store to returned allocation is preserved.
    ///
    /// If the allocation is returned, the store is visible to the caller.
    #[test]
    fn test_preserve_store_to_returned() {
        let input = r#"function @test() -> ref<raw i32> {
block0:
    v0 = stack.alloc i32
    v1 = iconst 42i32
    store v0, v1
    return v0
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&DeadStoreEliminate);
        program.assert_unchanged(input);
    }

    /// Cross-block dead store: store overwritten in successor block.
    ///
    /// When a store is followed by an unconditional jump to a block that
    /// overwrites the same location before any read, the first store is dead.
    #[test]
    fn test_cross_block_overwritten() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = iconst 1i32
    store v0, v1
    jump block1
block1:
    v2 = iconst 2i32
    store v0, v2
    v3 = load v0
    return v3
}"#;
        let expected = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = iconst 1i32
    jump block1
block1:
    v2 = iconst 2i32
    store v0, v2
    v3 = load v0
    return v3
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&DeadStoreEliminate);
        program.assert_output(expected);
    }

    /// Store preserved when read in one branch.
    ///
    /// If a store might be read on some control flow path, it must be preserved.
    #[test]
    fn test_preserve_cross_block_read_on_path() {
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    v1 = stack.alloc i32
    v2 = iconst 1i32
    store v1, v2
    branch v0, block1, block2
block1:
    v3 = load v1
    return v3
block2:
    v4 = iconst 2i32
    store v1, v4
    v5 = load v1
    return v5
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&DeadStoreEliminate);
        program.assert_unchanged(input);
    }

    /// Switch terminator: allocation passed as default argument escapes.
    ///
    /// When an allocation is passed as an argument in a switch default,
    /// it escapes and stores to it must be preserved.
    #[test]
    fn test_preserve_switch_escape() {
        let input = r#"function @test(v0: i32) -> void {
block0(v0: i32):
    v1 = stack.alloc i32
    v2 = iconst 42i32
    store v1, v2
    switch v0, block1(v1), 0 => block2
block1(v3: ref<raw i32>):
    return
block2:
    return
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&DeadStoreEliminate);
        program.assert_unchanged(input);
    }
}
