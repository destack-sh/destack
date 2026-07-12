use std::collections::HashSet;

use crate::optimize::declare_pass;
use destack_mir as mir;

use crate::optimize::{FunctionPass, MirOptimized, PipelineContext};
use destack_mir::{
    AliasAnalysis, ByteRange, MemoryAccessId, MemoryNode, MemoryPlace, MemoryRegion,
    MemoryRegionBuilder, MemorySSA, Mutation, PostDominatorTree, RangeRelation, TargetLayout,
    ValueDefinitions, ValueTypes,
};

declare_pass! {
    /// Dead Store Elimination.
    ///
    /// Removes stores to memory locations that are never read:
    /// 1. Stores overwritten on all paths before any read
    /// 2. Stores to non escaping stack locations that are never read
    ///
    /// This pass uses MemorySSA, alias analysis, and post dominance to
    /// identify clobbering stores and preserve externally visible writes.
    ///
    /// ```mir
    /// // before DSE
    /// function before(): int32 {
    /// b0:
    ///     v0 = frame.alloc.zeroed int32
    ///     v1 = 1int32
    ///     store v0, v1        // dead: overwritten below
    ///     v2 = 2int32
    ///     store v0, v2
    ///     v3 = load v0
    ///     return v3
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// // after DSE
    /// function after(): int32 {
    /// b0:
    ///     v0 = frame.alloc.zeroed int32
    ///     v1 = 1int32
    ///     // store removed
    ///     v2 = 2int32
    ///     store v0, v2
    ///     v3 = load v0
    ///     return v3
    /// }
    /// ```
    #[pass(id = "eliminate-dead-stores")]
    pub EliminateDeadStores,
    "Remove dead stores"
}

impl FunctionPass for EliminateDeadStores {
    /// Run dead store elimination on the function.
    fn run(
        &self,
        function: &mut mir::Function,
        optimized: &mut MirOptimized,
        ctx: &PipelineContext<'_>,
        analyses: &mir::FunctionAnalysisCache,
    ) -> Mutation {
        let tree = &mut optimized.tree;
        let effects = &mut optimized.effects;

        // skip empty functions
        let _entry = match function.entry() {
            Some(entry) => entry,
            None => return Mutation::NONE,
        };

        // get analyses
        let aa = analyses.get::<AliasAnalysis>(function, tree).clone();
        let memory_ssa = analyses.get::<MemorySSA>(function, tree);
        let cfg = analyses.get::<mir::ControlFlowGraph>(function, tree);
        let postdom = PostDominatorTree::build(function, tree, &cfg);

        let value_types = analyses.get::<ValueTypes>(function, tree);

        // run dead store elimination
        let changed = run_eliminate_dead_stores(
            function,
            tree,
            effects,
            &aa,
            memory_ssa.as_ref(),
            &value_types,
            &postdom,
            ctx.target_layout(),
        );

        // report what this pass changed
        if changed {
            Mutation::VALUE
        } else {
            Mutation::NONE
        }
    }

    /// Return the pass name.
    fn name(&self) -> &'static str {
        "EliminateDeadStores"
    }

    /// Return the pass id.
    fn id(&self) -> &'static str {
        "dse"
    }
}

/// Core DSE logic. Returns true if changes were made.
fn run_eliminate_dead_stores(
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    effects: &mir::EffectTable,
    aa: &AliasAnalysis,
    memory_ssa: &MemorySSA,
    value_types: &ValueTypes,
    postdom: &PostDominatorTree,
    target_layout: TargetLayout,
) -> bool {
    // collect store candidates
    let store_candidates = collect_store_candidates(function, tree, memory_ssa);

    // exit early when there are no stores
    if store_candidates.is_empty() {
        return false;
    }

    // collect memory definitions
    let def_accesses = collect_def_accesses(function, tree, memory_ssa);

    // collect live definitions from memory reads
    let live_defs = collect_live_defs(function, tree, memory_ssa, aa);

    // collect non escaping stack allocations
    let value_definitions = ValueDefinitions::build(function, tree);
    let non_escaping_frame_allocs =
        value_definitions.non_escaping_frame_allocs(function, tree, effects);
    let frame_alloc_reads = collect_frame_alloc_reads(
        function,
        memory_ssa,
        &value_definitions,
        &non_escaping_frame_allocs,
        tree,
    );

    // determine dead stores
    let mut dead_stores = HashSet::new();

    // scan store candidates for removal
    for store in store_candidates {
        // skip volatile or barrier stores
        if store.is_volatile || store.is_barrier {
            continue;
        }

        // skip stores that are read
        if live_defs.contains(&store.access) {
            continue;
        }

        // skip imprecise regions
        if matches!(store.region, MemoryRegion::Any { .. }) {
            continue;
        }

        // remove stores to non escaping stack memory
        if store_is_non_escaping_stack(
            &store,
            &value_definitions,
            &non_escaping_frame_allocs,
            &frame_alloc_reads,
            tree,
        ) {
            dead_stores.insert(store.instruction);
            continue;
        }

        // remove stores clobbered along all paths
        if store_is_postdominated_by_clobber(
            &store,
            &def_accesses,
            memory_ssa,
            aa,
            postdom,
            function,
            tree,
            &value_definitions,
            value_types,
            target_layout,
        ) {
            dead_stores.insert(store.instruction);
        }
    }

    // exit early when nothing is removed
    if dead_stores.is_empty() {
        return false;
    }

    // remove dead stores
    for block_id in function.blocks().to_vec() {
        let mut instructions = tree.get(block_id).instructions.clone();
        instructions.retain(|id| !dead_stores.contains(id));
        function.replace_block_instructions(block_id, instructions, tree);
    }

    true
}

/// Store candidate for dead store elimination.
#[derive(Clone)]
struct StoreCandidate {
    /// Instruction that defines the store.
    instruction: mir::LocalNodeId<mir::Instruction>,
    /// Memory SSA access id for the store.
    access: MemoryAccessId,
    /// Block containing the instruction.
    block: mir::LocalNodeId<mir::Block>,
    /// Instruction index within the block.
    index: usize,
    /// Optional pointer for reference locations.
    pointer: Option<mir::Value>,
    /// Access location for the store.
    region: MemoryRegion,
    /// True when the store is volatile.
    is_volatile: bool,
    /// True when the store is a barrier.
    is_barrier: bool,
}

/// Memory SSA def access location in a block.
#[derive(Clone)]
struct DefAccessInfo {
    /// Memory SSA access id.
    access: MemoryAccessId,
    /// Block containing the def.
    block: mir::LocalNodeId<mir::Block>,
    /// Instruction index within the block.
    index: usize,
}

/// Collect store candidates with MemorySSA defs.
fn collect_store_candidates(
    function: &mir::Function,
    tree: &mir::Tree,
    memory_ssa: &MemorySSA,
) -> Vec<StoreCandidate> {
    // collect store instructions with MemorySSA defs
    let mut stores = Vec::new();

    // scan blocks for store instructions
    for &block_id in function.blocks() {
        // read the block
        let block = tree.get(block_id);

        // scan instructions in the block
        for (index, &instruction_id) in block.instructions.iter().enumerate() {
            // read the instruction
            let instruction = tree.get(instruction_id);

            // collect store-like instructions
            let is_mem_intrinsic = matches!(
                instruction,
                mir::Instruction::Intrinsic {
                    intrinsic: mir::Intrinsic::Memcpy
                        | mir::Intrinsic::Memmove
                        | mir::Intrinsic::Memset,
                    ..
                }
            );

            if !matches!(instruction, mir::Instruction::Store { .. }) && !is_mem_intrinsic {
                continue;
            }

            // read memory accesses for this instruction
            let Some(accesses) = memory_ssa.instruction_accesses(instruction_id) else {
                continue;
            };

            // record each MemorySSA def access
            for &access_id in accesses {
                let MemoryNode::Def(def_access) = memory_ssa.access(access_id) else {
                    continue;
                };

                // resolve reference locations when available
                let pointer = match &def_access.effect.region {
                    MemoryRegion::Reference { access, .. } => Some(access.reference),
                    _ => None,
                };

                // record the candidate store
                stores.push(StoreCandidate {
                    instruction: instruction_id,
                    access: access_id,
                    block: block_id,
                    index,
                    pointer,
                    region: def_access.effect.region.clone(),
                    is_volatile: def_access.effect.is_volatile,
                    is_barrier: def_access.effect.is_barrier,
                });
            }
        }
    }

    stores
}

/// Collect MemorySSA def accesses for the function.
fn collect_def_accesses(
    function: &mir::Function,
    tree: &mir::Tree,
    memory_ssa: &MemorySSA,
) -> Vec<DefAccessInfo> {
    // collect all MemorySSA def accesses
    let mut defs = Vec::new();

    // scan blocks for def accesses
    for &block_id in function.blocks() {
        // read the block
        let block = tree.get(block_id);

        // scan instructions in the block
        for (index, &instruction_id) in block.instructions.iter().enumerate() {
            // read the access list
            let Some(accesses) = memory_ssa.instruction_accesses(instruction_id) else {
                continue;
            };

            // record each def access
            for &access_id in accesses {
                let MemoryNode::Def(_def_access) = memory_ssa.access(access_id) else {
                    continue;
                };

                defs.push(DefAccessInfo {
                    access: access_id,
                    block: block_id,
                    index,
                });
            }
        }
    }

    defs
}

/// Collect def accesses that are needed by memory reads.
fn collect_live_defs(
    function: &mir::Function,
    tree: &mir::Tree,
    memory_ssa: &MemorySSA,
    aa: &AliasAnalysis,
) -> HashSet<MemoryAccessId> {
    // collect MemorySSA defs that feed reads
    let mut live_defs = HashSet::new();

    // scan blocks for read accesses
    for &block_id in function.blocks() {
        // read the block
        let block = tree.get(block_id);

        // scan instructions in the block
        for &instruction_id in &block.instructions {
            // read the access list
            let Some(accesses) = memory_ssa.instruction_accesses(instruction_id) else {
                continue;
            };

            // mark clobbering defs for reads
            for &access_id in accesses {
                match memory_ssa.access(access_id) {
                    MemoryNode::Use(_use_access) => {
                        // record the def that feeds this use
                        let clobber = memory_ssa.clobbering_use(access_id, aa);
                        record_live_clobber(clobber, memory_ssa, &mut live_defs);
                    }
                    MemoryNode::Def(def_access) => {
                        // skip defs that do not read memory
                        if !def_access.effect.reads {
                            continue;
                        }

                        // record the def that feeds the read portion
                        let clobber =
                            memory_ssa.clobbering_read(access_id, &def_access.effect.region, aa);
                        record_live_clobber(clobber, memory_ssa, &mut live_defs);
                    }
                    _ => {}
                }
            }
        }
    }

    live_defs
}

/// Record live memory defs reachable from a clobber access.
fn record_live_clobber(
    access_id: MemoryAccessId,
    memory_ssa: &MemorySSA,
    live_defs: &mut HashSet<MemoryAccessId>,
) {
    // seed the traversal state
    let mut worklist = vec![access_id];
    let mut visited = HashSet::new();

    // walk the access chain
    while let Some(current) = worklist.pop() {
        if !visited.insert(current) {
            continue;
        }

        // record defs and expand through phis and uses
        match memory_ssa.access(current) {
            MemoryNode::Def(_) => {
                live_defs.insert(current);
            }
            MemoryNode::Phi(phi) => {
                for (_, incoming) in &phi.incoming {
                    worklist.push(*incoming);
                }
            }
            MemoryNode::Use(use_access) => {
                if let Some(defining) = use_access.defining_access {
                    worklist.push(defining);
                }
            }
            MemoryNode::LiveOnEntry => {}
        }
    }
}

/// Collect stack allocation bases that are read by any memory access.
fn collect_frame_alloc_reads(
    function: &mir::Function,
    memory_ssa: &MemorySSA,
    definitions: &ValueDefinitions,
    frame_allocs: &HashSet<mir::Value>,
    tree: &mir::Tree,
) -> HashSet<mir::Value> {
    // collect stack bases with reads
    let mut reads = HashSet::new();

    // scan blocks for read accesses
    for &block_id in function.blocks() {
        // read the block
        let block = tree.get(block_id);

        // scan instructions for memory uses
        for &instruction_id in &block.instructions {
            // read the access list
            let Some(accesses) = memory_ssa.instruction_accesses(instruction_id) else {
                continue;
            };

            // track pointer reads that touch stack allocations
            for &access_id in accesses {
                let MemoryNode::Use(use_access) = memory_ssa.access(access_id) else {
                    continue;
                };
                let MemoryRegion::Reference { access, .. } = &use_access.effect.region else {
                    continue;
                };

                // resolve stack bases for the pointer
                let mut visited = HashSet::new();
                definitions.collect_frame_alloc_bases(
                    access.reference,
                    tree,
                    frame_allocs,
                    &mut visited,
                    &mut reads,
                );
            }
        }
    }

    reads
}

/// Return true when a store targets a non escaping stack allocation.
fn store_is_non_escaping_stack(
    store: &StoreCandidate,
    definitions: &ValueDefinitions,
    non_escaping_frame_allocs: &HashSet<mir::Value>,
    frame_alloc_reads: &HashSet<mir::Value>,
    tree: &mir::Tree,
) -> bool {
    // only reference locations can be stack allocations
    let MemoryRegion::Reference { .. } = store.region else {
        return false;
    };

    // resolve the stack base for this store pointer
    let Some(pointer) = store.pointer else {
        return false;
    };
    let Some(base) = definitions.frame_alloc_base(pointer, tree) else {
        return false;
    };

    // skip when the stack location is read
    if frame_alloc_reads.contains(&base) {
        return false;
    }

    // report whether the base is non escaping
    non_escaping_frame_allocs.contains(&base)
}

/// Return true when the store is postdominated by a clobbering access.
fn store_is_postdominated_by_clobber(
    store: &StoreCandidate,
    def_accesses: &[DefAccessInfo],
    memory_ssa: &MemorySSA,
    aa: &AliasAnalysis,
    postdom: &PostDominatorTree,
    function: &mir::Function,
    tree: &mir::Tree,
    definitions: &ValueDefinitions,
    value_types: &ValueTypes,
    target_layout: TargetLayout,
) -> bool {
    let mut regions = MemoryRegionBuilder::new(
        definitions,
        tree,
        &function.parameters,
        value_types,
        target_layout,
    );

    // search for clobbering defs that postdominate the store
    for def in def_accesses {
        // skip self and earlier defs in the block
        if def.access == store.access {
            continue;
        }
        if def.block == store.block && def.index <= store.index {
            continue;
        }

        // skip defs that do not postdominate
        if !postdom.postdominates(def.block, store.block) {
            continue;
        }
        let MemoryNode::Def(def_access) = memory_ssa.access(def.access) else {
            continue;
        };

        // skip barrier defs since they do not overwrite memory
        if def_access.effect.is_barrier {
            continue;
        }

        // return once a clobbering def is found
        if let Some(overwrites) =
            def_fully_overwrites_store(&def_access.effect.region, &store.region, &mut regions)
        {
            if overwrites && memory_ssa.def_clobbers_access(def.access, store.access, aa) {
                return true;
            }

            continue;
        }

        if matches!(store.region, MemoryRegion::Local(_))
            && memory_ssa.def_clobbers_access(def.access, store.access, aa)
        {
            return true;
        }
    }

    false
}

/// Return true when a def fully overwrites the store location.
fn def_fully_overwrites_store(
    overwrite_region: &MemoryRegion,
    store_region: &MemoryRegion,
    regions: &mut MemoryRegionBuilder<'_>,
) -> Option<bool> {
    let MemoryRegion::Reference {
        access: overwrite_access,
        ..
    } = overwrite_region
    else {
        return None;
    };
    let MemoryRegion::Reference {
        access: store_access,
        ..
    } = store_region
    else {
        return None;
    };

    let overwrite_size = overwrite_access.size?;
    let store_size = store_access.size?;

    let overwrite_region = regions.region(overwrite_access.reference);
    let store_region = regions.region(store_access.reference);

    let MemoryRegion::Place(overwrite_place) = overwrite_region else {
        return None;
    };
    let MemoryRegion::Place(store_place) = store_region else {
        return None;
    };

    // require constant offsets and identical field paths
    if !place_is_constant(&overwrite_place) || !place_is_constant(&store_place) {
        return None;
    }
    if overwrite_place.fields != store_place.fields {
        return None;
    }

    // disjoint identified storage cannot overwrite
    if overwrite_place.root != store_place.root {
        return Some(false);
    }

    let overwrite_range = ByteRange::new(overwrite_place.const_offset, overwrite_size);
    let store_range = ByteRange::new(store_place.const_offset, store_size);
    let relation = overwrite_range.relation(store_range);

    match relation {
        RangeRelation::Equal | RangeRelation::Contains => Some(true),
        RangeRelation::Disjoint | RangeRelation::ContainedBy | RangeRelation::Overlaps => {
            Some(false)
        }
    }
}

/// Return true when the memory place has only constant offsets.
fn place_is_constant(place: &MemoryPlace) -> bool {
    place.indexed_offsets.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Return the pointer operand for a store instruction.
    fn store_pointer(tree: &mir::Tree, store_id: mir::LocalNodeId<mir::Instruction>) -> mir::Value {
        let mir::Instruction::Store { pointer, .. } = tree.get(store_id) else {
            panic!("expected store instruction");
        };

        *pointer
    }

    /// Attach store access entries for a store instruction.
    fn tag_store_access(
        test: &mut TestProgram,
        store_id: mir::LocalNodeId<mir::Instruction>,
        size: u64,
        is_volatile: bool,
        ordering: Option<mir::MemoryOrdering>,
    ) {
        let pointer = store_pointer(&test.optimized.tree, store_id);

        test.insert_pointer_access_with_options(
            store_id,
            mir::MemoryOperation::Write,
            pointer,
            Some(size),
            is_volatile,
            ordering,
        );
    }

    /// Store overwritten before being read is eliminated.
    ///
    /// When a store to a location is followed by another store to the same
    /// location without an intervening load, the first store is dead.
    #[test]
    fn test_remove_overwritten_store() {
        let input = r#"
function test(): int32 {
entry:
    v0: ref<int32, raw, mutable, space(frame)> = frame.alloc.zeroed int32
    v1: int32 = 1
    store v0, v1
    v2: int32 = 2
    store v0, v2
    v3: int32 = load v0
    return v3
}
"#;
        let expected = r#"
function test(): int32 {
entry:
    v0: ref<int32, raw, mutable, space(frame)> = frame.alloc.zeroed int32
    v1: int32 = 1
    v2: int32 = 2
    store v0, v2
    v3: int32 = load v0
    return v3
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateDeadStores);
        test.assert_output(expected);
    }

    /// Store followed by load is preserved.
    ///
    /// Both stores are read before being overwritten, so both are preserved.
    #[test]
    fn test_preserve_read_store() {
        let input = r#"
function test(): int32 {
entry:
    v0: ref<int32, raw, mutable, space(frame)> = frame.alloc.zeroed int32
    v1: int32 = 1
    store v0, v1
    v2: int32 = load v0
    v3: int32 = 2
    store v0, v3
    v4: int32 = load v0
    return v4
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateDeadStores);
        test.assert_unchanged(input);
    }

    /// Store to allocation that is never read is eliminated.
    ///
    /// The stack allocation is never loaded from, so the store is dead.
    #[test]
    fn test_remove_store_to_unused_alloc() {
        let input = r#"
function test(): int32 {
entry:
    v0: ref<int32, raw, mutable, space(frame)> = frame.alloc.zeroed int32
    v1: int32 = 42
    store v0, v1
    v2: int32 = 0
    return v2
}
"#;
        let expected = r#"
function test(): int32 {
entry:
    v0: ref<int32, raw, mutable, space(frame)> = frame.alloc.zeroed int32
    v1: int32 = 42
    v2: int32 = 0
    return v2
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateDeadStores);
        test.assert_output(expected);
    }

    /// Volatile store is never removed even when overwritten.
    #[test]
    fn test_preserve_volatile_store() {
        let input = r#"
function test(): int32 {
entry:
    v0: ref<int32, raw, mutable, space(frame)> = frame.alloc.zeroed int32
    v1: int32 = 1
    store v0, v1
    v2: int32 = 2
    store v0, v2
    v3: int32 = load v0
    return v3
}
"#;

        let mut test = TestProgram::new(input);
        let function_id = test.entry_function_id();
        let store_id = test
            .store_instructions_in_entry(function_id)
            .into_iter()
            .next()
            .expect("missing store instruction");

        tag_store_access(&mut test, store_id, 4, true, None);

        test.run_pass(&EliminateDeadStores);
        test.assert_unchanged(input);
    }

    /// Store to escaping allocation is preserved.
    ///
    /// When the allocation escapes (is passed to an external function),
    /// the store cannot be eliminated because the external function may read it.
    #[test]
    fn test_preserve_escaping_store() {
        let input = r#"
external function imported(ref<int32, raw, mutable>): void

function test(): void {
entry:
    v0: ref<int32, raw, mutable, space(frame)> = frame.alloc.zeroed int32
    v1: int32 = 42
    store v0, v1
    call imported(v0)
    return
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateDeadStores);
        test.assert_unchanged(input);
    }

    /// Store before a nonescaping no memory call is removed.
    #[test]
    fn test_remove_store_before_nonescaping_no_memory_call() {
        let input = r#"
external function imported(ref<int32, raw, mutable>): void

function test(): int32 {
entry:
    v0: ref<int32, raw, mutable, space(frame)> = frame.alloc.zeroed int32
    v1: int32 = 42
    store v0, v1
    call imported(v0)
    v2: int32 = 0
    return v2
}
"#;
        let expected = r#"
external function imported(ref<int32, raw, mutable>): void

function test(): int32 {
entry:
    v0: ref<int32, raw, mutable, space(frame)> = frame.alloc.zeroed int32
    v1: int32 = 42
    call imported(v0)
    v2: int32 = 0
    return v2
}
"#;

        let mut test = TestProgram::new(input);

        let function_id = test.entry_function_id();
        let (call_inst, _callee) = test.first_call_in_entry(function_id);

        let arg0 = mir::CallArgumentEffect {
            access: mir::ArgumentAccess::None,
            escape: mir::ArgumentEscape::None,
        };

        let call = mir::CallSite::Instruction(call_inst);
        let tables = test.optimized.effects.call_mut(call);
        tables.memory = mir::MemoryEffect::none();
        tables.arguments = vec![arg0];

        test.run_pass(&EliminateDeadStores);
        test.assert_output(expected);
    }

    /// Multiple consecutive overwrites - all but last are eliminated.
    ///
    /// When a location is stored to multiple times before being read,
    /// only the final store is preserved.
    #[test]
    fn test_multiple_overwrites() {
        let input = r#"
function test(): int32 {
entry:
    v0: ref<int32, raw, mutable, space(frame)> = frame.alloc.zeroed int32
    v1: int32 = 1
    store v0, v1
    v2: int32 = 2
    store v0, v2
    v3: int32 = 3
    store v0, v3
    v4: int32 = load v0
    return v4
}
"#;
        let expected = r#"
function test(): int32 {
entry:
    v0: ref<int32, raw, mutable, space(frame)> = frame.alloc.zeroed int32
    v1: int32 = 1
    v2: int32 = 2
    v3: int32 = 3
    store v0, v3
    v4: int32 = load v0
    return v4
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateDeadStores);
        test.assert_output(expected);
    }

    /// No changes when no stores.
    ///
    /// Function without stores is unchanged.
    #[test]
    fn test_no_changes() {
        let input = r#"
function test(): int32 {
entry:
    v0: int32 = 42
    return v0
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateDeadStores);
        test.assert_unchanged(input);
    }

    /// Stores to different locations are independent.
    ///
    /// Stores to different memory locations don't interfere with each other.
    #[test]
    fn test_different_locations() {
        let input = r#"
function test(): int32 {
entry:
    v0: ref<int32, raw, mutable, space(frame)> = frame.alloc.zeroed int32
    v1: ref<int32, raw, mutable, space(frame)> = frame.alloc.zeroed int32
    v2: int32 = 1
    store v0, v2
    v3: int32 = 2
    store v1, v3
    v4: int32 = load v0
    v5: int32 = load v1
    v6: int32 = int.add v4, v5
    return v6
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateDeadStores);
        test.assert_unchanged(input);
    }

    /// Store before call that may read is preserved.
    ///
    /// Calls conservatively clobber memory state, so stores before calls must be preserved if the
    /// location may be read by the callee.
    #[test]
    fn test_preserve_store_before_call() {
        let input = r#"
external function readValue(ref<int32, raw, mutable>): int32

function test(): int32 {
entry:
    v0: ref<int32, raw, mutable, space(frame)> = frame.alloc.zeroed int32
    v1: int32 = 42
    store v0, v1
    v2: int32 = call readValue(v0)
    return v2
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateDeadStores);
        test.assert_unchanged(input);
    }

    /// Store after call but overwritten before read.
    ///
    /// Even after a call, dead store elimination applies to stores
    /// that are overwritten before being read.
    #[test]
    fn test_remove_after_call_overwritten() {
        let input = r#"
external function sideEffect(): void

function test(): int32 {
entry:
    v0: ref<int32, raw, mutable, space(frame)> = frame.alloc.zeroed int32
    call sideEffect()
    v1: int32 = 1
    store v0, v1
    v2: int32 = 2
    store v0, v2
    v3: int32 = load v0
    return v3
}
"#;
        let expected = r#"
external function sideEffect(): void

function test(): int32 {
entry:
    v0: ref<int32, raw, mutable, space(frame)> = frame.alloc.zeroed int32
    call sideEffect()
    v1: int32 = 1
    v2: int32 = 2
    store v0, v2
    v3: int32 = load v0
    return v3
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateDeadStores);
        test.assert_output(expected);
    }

    /// Store to returned allocation is preserved.
    ///
    /// If the allocation is returned, the store is visible to the caller.
    #[test]
    fn test_preserve_store_to_returned() {
        let input = r#"
function test(): ref<int32, raw, mutable> {
entry:
    v0: ref<int32, raw, mutable, space(frame)> = frame.alloc.zeroed int32
    v1: int32 = 42
    store v0, v1
    return v0
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateDeadStores);
        test.assert_unchanged(input);
    }

    /// Store to allocation used in returned aggregate is preserved.
    #[test]
    fn test_preserve_store_returned_aggregate() {
        let input = r#"
function test(): (ref<int32, raw, mutable>, int32) {
entry:
    v0: ref<int32, raw, mutable, space(frame)> = frame.alloc.zeroed int32
    v1: int32 = 42
    store v0, v1
    v2: int32 = 0
    v3: (ref<int32, raw, mutable>, int32) = aggregate (v0, v2)
    return v3
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateDeadStores);
        test.assert_unchanged(input);
    }

    /// Stores to disjoint fields are not treated as clobbers.
    #[test]
    fn test_preserve_disjoint_field_stores() {
        let input = r#"
type Pair {
    int32;
    int32;
}

function test(v0: ref<Pair, raw, mutable>): void {
entry(v0: ref<Pair, raw, mutable>):
    v1: ref<int32, borrowed, mutable> = field.address v0, 0
    v2: ref<int32, borrowed, mutable> = field.address v0, 1
    v3: int32 = 1
    v4: int32 = 2
    store v1, v3
    store v2, v4
    return
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateDeadStores);
        test.assert_unchanged(input);
    }

    /// Partial overwrite does not kill earlier bytes.
    #[test]
    fn test_preserve_partial_overwrite() {
        let input = r#"
function test(): ref<int64, raw, mutable> {
entry:
    v0: ref<int64, raw, mutable, space(frame)> = frame.alloc.zeroed int64
    v1: int64 = 0
    store v0, v1
    v2: int32 = 1
    store v0, v2
    return v0
}
"#;

        let mut test = TestProgram::new(input);
        let function_id = test.entry_function_id();
        let store_ids = test.store_instructions_in_entry(function_id);
        let [first_store, second_store] = store_ids.as_slice() else {
            panic!("expected two store instructions");
        };

        tag_store_access(&mut test, *first_store, 8, false, None);
        tag_store_access(&mut test, *second_store, 4, false, None);

        test.run_pass(&EliminateDeadStores);
        test.assert_unchanged(input);
    }

    /// Dead memset to non escaping stack memory is removed.
    #[test]
    fn test_remove_dead_memset() {
        let input = r#"
function test(): void {
entry:
    v0: ref<int32, raw, mutable, space(frame)> = frame.alloc.zeroed int32
    v1: int8 = 0
    v2: int64 = 4
    intrinsic.memory.raw.setBytes(v0, v1, v2)
    return
}
"#;
        let expected = r#"
function test(): void {
entry:
    v0: ref<int32, raw, mutable, space(frame)> = frame.alloc.zeroed int32
    v1: int8 = 0
    v2: int64 = 4
    return
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateDeadStores);
        test.assert_output(expected);
    }

    /// Memset to a live location is preserved.
    #[test]
    fn test_preserve_memset_with_read() {
        let input = r#"
function test(): int32 {
entry:
    v0: ref<int32, raw, mutable, space(frame)> = frame.alloc.zeroed int32
    v1: int8 = 0
    v2: int64 = 4
    intrinsic.memory.raw.setBytes(v0, v1, v2)
    v3: int32 = load v0
    return v3
}
"#;
        let expected = input;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateDeadStores);
        test.assert_output(expected);
    }

    /// Dead memcpy to non escaping stack memory is removed.
    #[test]
    fn test_remove_dead_memcpy() {
        // input test
        let input = r#"
function test(): void {
entry:
    v0: ref<int32, raw, mutable, space(frame)> = frame.alloc.zeroed int32
    v1: ref<int32, raw, mutable, space(frame)> = frame.alloc.zeroed int32
    v2: int64 = 4
    intrinsic.memory.raw.copyBytes(v0, v1, v2)
    return
}
"#;
        let expected = r#"
function test(): void {
entry:
    v0: ref<int32, raw, mutable, space(frame)> = frame.alloc.zeroed int32
    v1: ref<int32, raw, mutable, space(frame)> = frame.alloc.zeroed int32
    v2: int64 = 4
    return
}
"#;

        // run dse
        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateDeadStores);
        test.assert_output(expected);
    }

    /// Memcpy to a live location is preserved.
    #[test]
    fn test_preserve_memcpy_with_read() {
        // input test
        let input = r#"
function test(): int32 {
entry:
    v0: ref<int32, raw, mutable, space(frame)> = frame.alloc.zeroed int32
    v1: ref<int32, raw, mutable, space(frame)> = frame.alloc.zeroed int32
    v2: int64 = 4
    intrinsic.memory.raw.copyBytes(v0, v1, v2)
    v3: int32 = load v0
    return v3
}
"#;
        let expected = input;

        // run dse
        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateDeadStores);
        test.assert_output(expected);
    }

    /// Dead memmove to non escaping stack memory is removed.
    #[test]
    fn test_remove_dead_memmove() {
        // input test
        let input = r#"
function test(): void {
entry:
    v0: ref<int32, raw, mutable, space(frame)> = frame.alloc.zeroed int32
    v1: ref<int32, raw, mutable, space(frame)> = frame.alloc.zeroed int32
    v2: int64 = 4
    intrinsic.memory.raw.moveBytes(v0, v1, v2)
    return
}
"#;
        let expected = r#"
function test(): void {
entry:
    v0: ref<int32, raw, mutable, space(frame)> = frame.alloc.zeroed int32
    v1: ref<int32, raw, mutable, space(frame)> = frame.alloc.zeroed int32
    v2: int64 = 4
    return
}
"#;

        // run dse
        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateDeadStores);
        test.assert_output(expected);
    }

    /// Memmove to a live location is preserved.
    #[test]
    fn test_preserve_memmove_with_read() {
        // input test
        let input = r#"
function test(): int32 {
entry:
    v0: ref<int32, raw, mutable, space(frame)> = frame.alloc.zeroed int32
    v1: ref<int32, raw, mutable, space(frame)> = frame.alloc.zeroed int32
    v2: int64 = 4
    intrinsic.memory.raw.moveBytes(v0, v1, v2)
    v3: int32 = load v0
    return v3
}
"#;
        let expected = input;

        // run dse
        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateDeadStores);
        test.assert_output(expected);
    }

    /// Stores with unknown sizes are not treated as full overwrites.
    #[test]
    fn test_preserve_unknown_size_overwrite() {
        // input test
        let input = r#"
type Point {
    int32;
    int32;
}

function test(v0: ref<Point, raw, mutable>): void {
entry(v0: ref<Point, raw, mutable>):
    v1: int32 = 1
    v2: int32 = 2
    v3: Point = aggregate (v1, v2)
    store v0, v3
    v4: int32 = 3
    v5: int32 = 4
    v6: Point = aggregate (v4, v5)
    store v0, v6
    return
}
"#;
        let expected = input;

        // attach unknown size tables to both stores
        let mut test = TestProgram::new(input);
        let function_id = test.entry_function_id();
        let store_ids = test.store_instructions_in_entry(function_id);
        let [first_store, second_store] = store_ids.as_slice() else {
            panic!("expected two store instructions");
        };

        test.insert_pointer_access(
            *first_store,
            mir::MemoryOperation::Write,
            mir::Value::new(0),
            None,
        );
        test.insert_pointer_access(
            *second_store,
            mir::MemoryOperation::Write,
            mir::Value::new(0),
            None,
        );

        // run dse
        test.run_pass(&EliminateDeadStores);
        test.assert_output(expected);
    }

    /// Cross-block dead store: store overwritten in successor block.
    ///
    /// When a store is followed by an unconditional jump to a block that
    /// overwrites the same region before any read, the first store is dead.
    #[test]
    fn test_cross_block_overwritten() {
        let input = r#"
function test(): int32 {
entry:
    v0: ref<int32, raw, mutable, space(frame)> = frame.alloc.zeroed int32
    v1: int32 = 1
    store v0, v1
    jump b1

b1:
    v2: int32 = 2
    store v0, v2
    v3: int32 = load v0
    return v3
}
"#;
        let expected = r#"
function test(): int32 {
entry:
    v0: ref<int32, raw, mutable, space(frame)> = frame.alloc.zeroed int32
    v1: int32 = 1
    jump b1

b1:
    v2: int32 = 2
    store v0, v2
    v3: int32 = load v0
    return v3
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateDeadStores);
        test.assert_output(expected);
    }

    /// Store preserved when read in one branch.
    ///
    /// If a store might be read on some control flow path, it must be preserved.
    #[test]
    fn test_preserve_cross_block_read_on_path() {
        let input = r#"
function test(v0: boolean): int32 {
entry(v0: boolean):
    v1: ref<int32, raw, mutable, space(frame)> = frame.alloc.zeroed int32
    v2: int32 = 1
    store v1, v2
    branch v0, b1, b2

b1:
    v3: int32 = load v1
    return v3

b2:
    v4: int32 = 2
    store v1, v4
    v5: int32 = load v1
    return v5
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateDeadStores);
        test.assert_unchanged(input);
    }

    /// Switch terminator: allocation passed as default argument escapes.
    ///
    /// When an allocation is passed as an argument in a switch default,
    /// it escapes and stores to it must be preserved.
    #[test]
    fn test_preserve_switch_escape() {
        let input = r#"
function test(v0: int32): void {
entry(v0: int32):
    v1: ref<int32, raw, mutable, space(frame)> = frame.alloc.zeroed int32
    v2: int32 = 42
    store v1, v2
    switch v0, b1(v1), 0 => b2

b1(v3: ref<int32, raw, mutable>):
    return

b2:
    return
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateDeadStores);
        test.assert_unchanged(input);
    }
}
