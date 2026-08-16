use std::collections::HashSet;

use crate::optimize::declare_pass;
use destack_mir as mir;

use crate::optimize::{FunctionPass, MirOptimized, PipelineContext};
use destack_mir::{
    AliasTable, ByteRange, MemoryAccessId, MemoryNode, MemoryPlace, MemoryRegion, MemoryTable,
    Mutation, PostdominatorTable, RangeRelation,
};

declare_pass! {
    /// Dead Store Elimination.
    ///
    /// ```mir
    /// // before DSE
    /// function before(): int32 {
    ///     local l0: int32
    /// b0:
    ///     v0: ref<int32, borrowed, mutable, frame> = local.address l0
    ///     v1: int32 = 1
    ///     store v0, v1        // dead: overwritten below
    ///     v2: int32 = 2
    ///     store v0, v2
    ///     v3: int32 = load v0
    ///     return v3
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// // after DSE
    /// function after(): int32 {
    ///     local l0: int32
    /// b0:
    ///     v0: ref<int32, borrowed, mutable, frame> = local.address l0
    ///     v1: int32 = 1
    ///     // store removed
    ///     v2: int32 = 2
    ///     store v0, v2
    ///     v3: int32 = load v0
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
        _ctx: &PipelineContext<'_>,
        analyses: &mut mir::FunctionCache,
    ) -> Mutation {
        let tree = &mut optimized.tree;
        let accesses = &optimized.accesses;
        let effects = &optimized.effects;

        // skip empty functions
        let _entry = match function.entry() {
            Some(entry) => entry,
            None => return Mutation::NONE,
        };

        // get analyses
        let aa = analyses.alias(function, tree).clone();
        let memory = analyses.memory(function, tree, accesses, effects);
        let postdom = analyses.postdominator(function, tree);

        // run dead store elimination
        let changed = run_eliminate_dead_stores(function, tree, &aa, memory.as_ref(), &postdom);

        // report what this pass changed
        if changed {
            Mutation::VALUE
        } else {
            Mutation::NONE
        }
    }
}

/// Core DSE logic. Returns true if changes were made.
fn run_eliminate_dead_stores(
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    aa: &AliasTable,
    memory: &MemoryTable,
    postdom: &PostdominatorTable,
) -> bool {
    // collect store candidates
    let store_candidates = collect_store_candidates(function, tree, memory);

    // exit early when there are no stores
    if store_candidates.is_empty() {
        return false;
    }

    // collect memory definitions
    let def_accesses = collect_def_accesses(function, tree, memory);

    // collect live definitions from memory reads
    let live_defs = collect_live_defs(function, tree, memory, aa);

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

        // remove unread stores owned by this frame
        let resolved_region = aa.resolve(&store.region);
        if resolved_region.is_frame_storage() {
            dead_stores.insert(store.instruction);
            continue;
        }

        // remove stores clobbered along all paths
        if store_is_postdominated_by_clobber(&store, &def_accesses, memory, aa, postdom) {
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

/// Collect store candidates with MemoryTable defs.
fn collect_store_candidates(
    function: &mir::Function,
    tree: &mir::Tree,
    memory: &MemoryTable,
) -> Vec<StoreCandidate> {
    // collect store instructions with MemoryTable defs
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
            let Some(accesses) = memory.instruction_accesses(instruction_id) else {
                continue;
            };

            // record each MemoryTable def access
            for &access_id in accesses {
                let MemoryNode::Def(def_access) = memory.access(access_id) else {
                    continue;
                };

                // record the candidate store
                stores.push(StoreCandidate {
                    instruction: instruction_id,
                    access: access_id,
                    block: block_id,
                    index,
                    region: def_access.effect.region.clone(),
                    is_volatile: def_access.effect.is_volatile,
                    is_barrier: def_access.effect.is_barrier,
                });
            }
        }
    }

    stores
}

/// Collect MemoryTable def accesses for the function.
fn collect_def_accesses(
    function: &mir::Function,
    tree: &mir::Tree,
    memory: &MemoryTable,
) -> Vec<DefAccessInfo> {
    // collect all MemoryTable def accesses
    let mut defs = Vec::new();

    // scan blocks for def accesses
    for &block_id in function.blocks() {
        // read the block
        let block = tree.get(block_id);

        // scan instructions in the block
        for (index, &instruction_id) in block.instructions.iter().enumerate() {
            // read the access list
            let Some(accesses) = memory.instruction_accesses(instruction_id) else {
                continue;
            };

            // record each def access
            for &access_id in accesses {
                let MemoryNode::Def(_def_access) = memory.access(access_id) else {
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
    memory: &MemoryTable,
    aa: &AliasTable,
) -> HashSet<MemoryAccessId> {
    // collect MemoryTable defs that feed reads
    let mut live_defs = HashSet::new();

    // scan blocks for read accesses
    for &block_id in function.blocks() {
        // read the block
        let block = tree.get(block_id);

        // scan instructions in the block
        for &instruction_id in &block.instructions {
            // read the access list
            let Some(accesses) = memory.instruction_accesses(instruction_id) else {
                continue;
            };

            // mark clobbering defs for reads
            for &access_id in accesses {
                match memory.access(access_id) {
                    MemoryNode::Use(_use_access) => {
                        // record the def that feeds this use
                        let clobber = memory.clobbering_use(access_id, aa);
                        record_live_clobber(clobber, memory, &mut live_defs);
                    }
                    MemoryNode::Def(def_access) => {
                        // skip defs that do not read memory
                        if !def_access.effect.reads {
                            continue;
                        }

                        // record the def that feeds the read portion
                        let clobber =
                            memory.clobbering_read(access_id, &def_access.effect.region, aa);
                        record_live_clobber(clobber, memory, &mut live_defs);
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
    memory: &MemoryTable,
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
        match memory.access(current) {
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

/// Return true when the store is postdominated by a clobbering access.
fn store_is_postdominated_by_clobber(
    store: &StoreCandidate,
    def_accesses: &[DefAccessInfo],
    memory: &MemoryTable,
    aa: &AliasTable,
    postdom: &PostdominatorTable,
) -> bool {
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
        let MemoryNode::Def(def_access) = memory.access(def.access) else {
            continue;
        };

        // skip barrier defs since they do not overwrite memory
        if def_access.effect.is_barrier {
            continue;
        }

        // return once a clobbering def is found
        if let Some(overwrites) =
            def_fully_overwrites_store(&def_access.effect.region, &store.region, aa)
        {
            if overwrites && memory.def_clobbers_access(def.access, store.access, aa) {
                return true;
            }

            continue;
        }

        if matches!(store.region, MemoryRegion::Local(_))
            && memory.def_clobbers_access(def.access, store.access, aa)
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
    alias: &AliasTable,
) -> Option<bool> {
    let MemoryRegion::Address {
        location: overwrite_location,
        ..
    } = overwrite_region
    else {
        return None;
    };
    let MemoryRegion::Address {
        location: store_location,
        ..
    } = store_region
    else {
        return None;
    };

    let overwrite_size = overwrite_location.size?;
    let store_size = store_location.size?;

    let overwrite_region = alias.region(overwrite_location.address);
    let store_region = alias.region(store_location.address);

    let MemoryRegion::Place(overwrite_place) = overwrite_region else {
        return None;
    };
    let MemoryRegion::Place(store_place) = store_region else {
        return None;
    };

    // require constant offsets and identical field paths
    if !place_is_constant(overwrite_place) || !place_is_constant(store_place) {
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
    local l0: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
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
    local l0: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
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
    local l0: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
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
    local l0: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    v1: int32 = 42
    store v0, v1
    v2: int32 = 0
    return v2
}
"#;
        let expected = r#"
function test(): int32 {
    local l0: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
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
    local l0: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
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
external function imported(ref<int32, borrowed, mutable>): void

function test(): void {
    local l0: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    v1: int32 = 42
    store v0, v1
    call imported(v0): (ref<int32, borrowed, mutable>) => void
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
external function imported(ref<int32, borrowed, mutable>): void

function test(): int32 {
    local l0: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    v1: int32 = 42
    store v0, v1
    call imported(v0): (ref<int32, borrowed, mutable>) => void
    v2: int32 = 0
    return v2
}
"#;
        let expected = r#"
external function imported(ref<int32, borrowed, mutable>): void

function test(): int32 {
    local l0: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    v1: int32 = 42
    call imported(v0): (ref<int32, borrowed, mutable>) => void
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
        let tables = test.optimized.effects.upsert_call(call);
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
    local l0: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
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
    local l0: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
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
    local l0: int32
    local l1: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    v1: ref<int32, borrowed, mutable, frame> = local.address l1
    v2: int32 = 1
    store v0, v2
    v3: int32 = 2
    store v1, v3
    v4: int32 = load v0
    v5: int32 = load v1
    v6: int32 = add v4, v5
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
external function readValue(ref<int32, borrowed, mutable>): int32

function test(): int32 {
    local l0: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    v1: int32 = 42
    store v0, v1
    v2: int32 = call readValue(v0): (ref<int32, borrowed, mutable>) => int32
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
    local l0: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    call sideEffect(): () => void
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
    local l0: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    call sideEffect(): () => void
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

    /// Stores to disjoint fields are not treated as clobbers.
    #[test]
    fn test_preserve_disjoint_field_stores() {
        let input = r#"
type Pair {
    int32;
    int32;
}

function test(v0: ref<Pair, borrowed, mutable>): void {
entry(v0: ref<Pair, borrowed, mutable>):
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
function test(v0: ref<int64, borrowed, mutable>): void {
entry(v0: ref<int64, borrowed, mutable>):
    v1: int64 = 0
    store v0, v1
    v2: int32 = 1
    store v0, v2
    return
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
    local l0: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    v1: int8 = 0
    v2: int64 = 4
    intrinsic.memory.raw.setBytes(v0, v1, v2)
    return
}
"#;
        let expected = r#"
function test(): void {
    local l0: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
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
    local l0: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
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
    local l0: int32
    local l1: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    v1: ref<int32, borrowed, mutable, frame> = local.address l1
    v2: int64 = 4
    intrinsic.memory.raw.copyBytes(v0, v1, v2)
    return
}
"#;
        let expected = r#"
function test(): void {
    local l0: int32
    local l1: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    v1: ref<int32, borrowed, mutable, frame> = local.address l1
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
    local l0: int32
    local l1: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    v1: ref<int32, borrowed, mutable, frame> = local.address l1
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
    local l0: int32
    local l1: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    v1: ref<int32, borrowed, mutable, frame> = local.address l1
    v2: int64 = 4
    intrinsic.memory.raw.moveBytes(v0, v1, v2)
    return
}
"#;
        let expected = r#"
function test(): void {
    local l0: int32
    local l1: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    v1: ref<int32, borrowed, mutable, frame> = local.address l1
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
    local l0: int32
    local l1: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    v1: ref<int32, borrowed, mutable, frame> = local.address l1
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

function test(v0: ref<Point, borrowed, mutable>): void {
entry(v0: ref<Point, borrowed, mutable>):
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
    local l0: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
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
    local l0: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
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
    local l0: int32
entry(v0: boolean):
    v1: ref<int32, borrowed, mutable, frame> = local.address l0
    v2: int32 = 1
    store v1, v2
    branch v0 => b1 | b2

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
}
