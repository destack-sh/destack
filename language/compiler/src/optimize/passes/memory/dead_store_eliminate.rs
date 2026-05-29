use std::collections::{HashMap, HashSet};

use crate::declare_mir_pass;
use destack_mir as mir;

use crate::common::mir::analysis::{
    AliasAnalysis, MemoryAccess, MemoryAccessId, MemoryAccessLocation, MemorySSA, PostDominatorTree,
};
use crate::common::mir::{
    DecomposedPointer, PointerDecomposer, RangeRelation, ValueTypeMap, build_value_definition_map,
    collect_block_param_defs, collect_frame_alloc_bases_for_value, collect_local_defs,
    collect_non_escaping_frame_allocs, frame_alloc_base, range_relation,
};
use crate::optimize::{AnalysisPreservation, FunctionPass, PipelineContext, TypeContext};

declare_mir_pass! {
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
    #[pass(id = "dse", requires(call_effects, memory_access_metadata))]
    pub DeadStoreEliminate,
    "Remove dead stores"
}

impl FunctionPass for DeadStoreEliminate {
    /// Run dead store elimination on the function.
    fn run(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::Tree,
        ctx: &PipelineContext<'_>,
    ) -> AnalysisPreservation {
        // skip empty functions
        let _entry = match function.entry {
            Some(entry) => entry,
            None => return AnalysisPreservation::all(),
        };

        // get analyses
        let (aa, memory_ssa, postdom) = {
            let analyses = ctx.function_analyses(function, tree);
            (
                analyses.get::<AliasAnalysis>().clone(),
                analyses.get::<MemorySSA>(),
                analyses.get::<PostDominatorTree>(),
            )
        };

        let value_types = ValueTypeMap::new(function, tree);

        // run dead store elimination
        let changed = run_dead_store_eliminate(
            function,
            tree,
            &aa,
            memory_ssa.as_ref(),
            &value_types,
            postdom.as_ref(),
            ctx.type_context(),
        );

        // preserve analyses when unchanged
        if changed {
            AnalysisPreservation::none()
        } else {
            AnalysisPreservation::all()
        }
    }

    /// Return the pass name.
    fn name(&self) -> &'static str {
        "DeadStoreEliminate"
    }

    /// Return the pass id.
    fn id(&self) -> &'static str {
        "dse"
    }
}

/// Core DSE logic. Returns true if changes were made.
fn run_dead_store_eliminate(
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    aa: &AliasAnalysis,
    memory_ssa: &MemorySSA,
    value_types: &ValueTypeMap,
    postdom: &PostDominatorTree,
    type_context: TypeContext,
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
    let definitions = build_value_definition_map(function, tree);
    let constants = build_integer_constant_map(function, tree);
    let non_escaping_frame_allocs = collect_non_escaping_frame_allocs(function, tree, &definitions);
    let local_defs = collect_local_defs(function, tree);
    let param_defs = collect_block_param_defs(function, tree);
    let frame_alloc_reads = collect_frame_alloc_reads(
        function,
        memory_ssa,
        &definitions,
        &local_defs,
        &param_defs,
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

        // skip unknown locations
        if matches!(store.location, MemoryAccessLocation::Unknown) {
            continue;
        }

        // remove stores to non escaping stack memory
        if store_is_non_escaping_stack(
            &store,
            &definitions,
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
            &definitions,
            &constants,
            value_types,
            type_context,
        ) {
            dead_stores.insert(store.instruction);
        }
    }

    // exit early when nothing is removed
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
    /// Optional pointer for pointer locations.
    pointer: Option<mir::Value>,
    /// Access location for the store.
    location: MemoryAccessLocation,
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
    for &block_id in &function.blocks {
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
            let Some(accesses) = memory_ssa.accesses_for_instruction(instruction_id) else {
                continue;
            };

            // record each MemorySSA def access
            for &access_id in accesses {
                let MemoryAccess::Def(def_access) = memory_ssa.access(access_id) else {
                    continue;
                };

                // resolve pointer locations when available
                let pointer = match &def_access.effect.location {
                    MemoryAccessLocation::Pointer(location) => Some(location.ptr),
                    _ => None,
                };

                // record the candidate store
                stores.push(StoreCandidate {
                    instruction: instruction_id,
                    access: access_id,
                    block: block_id,
                    index,
                    pointer,
                    location: def_access.effect.location.clone(),
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
    for &block_id in &function.blocks {
        // read the block
        let block = tree.get(block_id);

        // scan instructions in the block
        for (index, &instruction_id) in block.instructions.iter().enumerate() {
            // read the access list
            let Some(accesses) = memory_ssa.accesses_for_instruction(instruction_id) else {
                continue;
            };

            // record each def access
            for &access_id in accesses {
                let MemoryAccess::Def(_def_access) = memory_ssa.access(access_id) else {
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
    for &block_id in &function.blocks {
        // read the block
        let block = tree.get(block_id);

        // scan instructions in the block
        for &instruction_id in &block.instructions {
            // read the access list
            let Some(accesses) = memory_ssa.accesses_for_instruction(instruction_id) else {
                continue;
            };

            // mark clobbering defs for reads
            for &access_id in accesses {
                match memory_ssa.access(access_id) {
                    MemoryAccess::Use(_use_access) => {
                        // record the def that feeds this use
                        let clobber = memory_ssa.clobbering_access_for_use(access_id, aa, tree);
                        record_live_clobber(clobber, memory_ssa, &mut live_defs);
                    }
                    MemoryAccess::Def(def_access) => {
                        // skip defs that do not read memory
                        if !def_access.effect.reads {
                            continue;
                        }

                        // record the def that feeds the read portion
                        let clobber = memory_ssa.clobbering_access_for_read(
                            access_id,
                            &def_access.effect.location,
                            aa,
                            tree,
                        );
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
            MemoryAccess::Def(_) => {
                live_defs.insert(current);
            }
            MemoryAccess::Phi(phi) => {
                for (_, incoming) in &phi.incoming {
                    worklist.push(*incoming);
                }
            }
            MemoryAccess::Use(use_access) => {
                if let Some(defining) = use_access.defining_access {
                    worklist.push(defining);
                }
            }
            MemoryAccess::LiveOnEntry => {}
        }
    }
}

/// Collect stack allocation bases that are read by any memory access.
fn collect_frame_alloc_reads(
    function: &mir::Function,
    memory_ssa: &MemorySSA,
    definitions: &HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
    local_defs: &HashMap<mir::LocalNodeId<mir::Local>, Vec<mir::Value>>,
    param_defs: &HashMap<mir::Value, Vec<mir::Value>>,
    frame_allocs: &HashSet<mir::Value>,
    tree: &mir::Tree,
) -> HashSet<mir::Value> {
    // collect stack bases with reads
    let mut reads = HashSet::new();

    // scan blocks for read accesses
    for &block_id in &function.blocks {
        // read the block
        let block = tree.get(block_id);

        // scan instructions for memory uses
        for &instruction_id in &block.instructions {
            // read the access list
            let Some(accesses) = memory_ssa.accesses_for_instruction(instruction_id) else {
                continue;
            };

            // track pointer reads that touch stack allocations
            for &access_id in accesses {
                let MemoryAccess::Use(use_access) = memory_ssa.access(access_id) else {
                    continue;
                };
                let MemoryAccessLocation::Pointer(location) = &use_access.effect.location else {
                    continue;
                };

                // resolve stack bases for the pointer
                let mut visited = HashSet::new();
                collect_frame_alloc_bases_for_value(
                    location.ptr,
                    definitions,
                    local_defs,
                    param_defs,
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
    definitions: &HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
    non_escaping_frame_allocs: &HashSet<mir::Value>,
    frame_alloc_reads: &HashSet<mir::Value>,
    tree: &mir::Tree,
) -> bool {
    // only pointer locations can be stack allocations
    let MemoryAccessLocation::Pointer(_) = store.location else {
        return false;
    };

    // resolve the stack base for this store pointer
    let Some(pointer) = store.pointer else {
        return false;
    };
    let Some(base) = frame_alloc_base(pointer, definitions, tree) else {
        return false;
    };

    // skip when the stack location is read
    if frame_alloc_reads.contains(&base) {
        return false;
    }

    // report whether the base is non escaping
    non_escaping_frame_allocs.contains(&base)
}

/// Return true when a later clobbering def postdominates the store.
// allow extra context parameters for clarity
#[allow(clippy::too_many_arguments)]
/// Return true when the store is postdominated by a clobbering access.
fn store_is_postdominated_by_clobber(
    store: &StoreCandidate,
    def_accesses: &[DefAccessInfo],
    memory_ssa: &MemorySSA,
    aa: &AliasAnalysis,
    postdom: &PostDominatorTree,
    function: &mir::Function,
    tree: &mir::Tree,
    definitions: &HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
    constants: &HashMap<mir::Value, i64>,
    value_types: &ValueTypeMap,
    type_context: TypeContext,
) -> bool {
    let mut decomposer = PointerDecomposer::new(
        constants,
        definitions,
        tree,
        &function.parameters,
        false,
        value_types,
        type_context,
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
        let MemoryAccess::Def(def_access) = memory_ssa.access(def.access) else {
            continue;
        };

        // skip barrier defs since they do not overwrite memory
        if def_access.effect.is_barrier {
            continue;
        }

        // return once a clobbering def is found
        if let Some(overwrites) = def_fully_overwrites_store(
            &def_access.effect.location,
            &store.location,
            &mut decomposer,
        ) {
            if overwrites && memory_ssa.def_clobbers_access(def.access, store.access, aa) {
                return true;
            }

            continue;
        }

        if matches!(store.location, MemoryAccessLocation::Local(_))
            && memory_ssa.def_clobbers_access(def.access, store.access, aa)
        {
            return true;
        }
    }

    false
}

/// Return true when a def fully overwrites the store location.
fn def_fully_overwrites_store(
    def_location: &MemoryAccessLocation,
    store_location: &MemoryAccessLocation,
    decomposer: &mut PointerDecomposer<'_>,
) -> Option<bool> {
    let MemoryAccessLocation::Pointer(def_loc) = def_location else {
        return None;
    };
    let MemoryAccessLocation::Pointer(store_loc) = store_location else {
        return None;
    };

    let def_size = def_loc.size?;
    let store_size = store_loc.size?;

    let def_decomp = decomposer.decompose(def_loc.ptr);
    let store_decomp = decomposer.decompose(store_loc.ptr);

    // require identified bases for overwrite reasoning
    if !def_decomp.base.is_identified() || !store_decomp.base.is_identified() {
        return None;
    }

    if !decomposition_is_constant(&def_decomp) || !decomposition_is_constant(&store_decomp) {
        return None;
    }

    if def_decomp.field_path != store_decomp.field_path {
        return None;
    }

    if def_decomp.base != store_decomp.base {
        if def_decomp.base.is_identified() && store_decomp.base.is_identified() {
            return Some(false);
        }

        return None;
    }

    let relation = range_relation(
        def_decomp.const_offset,
        def_size,
        store_decomp.const_offset,
        store_size,
    );

    match relation {
        RangeRelation::Equal | RangeRelation::Contains => Some(true),
        RangeRelation::Disjoint | RangeRelation::ContainedBy | RangeRelation::Overlaps => {
            Some(false)
        }
    }
}

/// Return true when the decomposition has only constant offsets.
fn decomposition_is_constant(pointer: &DecomposedPointer) -> bool {
    pointer.var_offsets.is_empty()
}

/// Build a map from values to constant integer values.
fn build_integer_constant_map(
    function: &mir::Function,
    tree: &mir::Tree,
) -> HashMap<mir::Value, i64> {
    let mut constants = HashMap::new();

    for &block_id in &function.blocks {
        let block = tree.get(block_id);
        for &instruction_id in &block.instructions {
            let instruction = tree.get(instruction_id);
            if let mir::Instruction::Const { destination, value } = instruction {
                let Some(destination) = destination.value() else {
                    continue;
                };
                match value {
                    mir::Constant::Int { value, .. } => {
                        if let Ok(value) = i64::try_from(*value) {
                            constants.insert(destination, value);
                        }
                    }
                    mir::Constant::UInt { value, .. } => {
                        if let Ok(value) = i64::try_from(*value) {
                            constants.insert(destination, value);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    constants
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

        pointer
            .value()
            .expect("store instruction should reference a concrete pointer value")
    }

    /// Attach store access metadata for a store instruction.
    fn tag_store_access(
        test: &mut TestProgram,
        store_id: mir::LocalNodeId<mir::Instruction>,
        size: u64,
        is_volatile: bool,
        ordering: Option<mir::MemoryOrdering>,
    ) {
        let pointer = store_pointer(&test.tree, store_id);

        test.insert_pointer_access_with_options(
            store_id,
            mir::MemoryAccessKind::Write,
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
b0:
    v0: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    v1: int32 = 1int32
    store v0, v1
    v2: int32 = 2int32
    store v0, v2
    v3: int32 = load v0
    return v3
}"#;
        let expected = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    v1: int32 = 1int32
    v2: int32 = 2int32
    store v0, v2
    v3: int32 = load v0
    return v3
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&DeadStoreEliminate);
        test.assert_output(expected);
    }

    /// Store followed by load is preserved.
    ///
    /// Both stores are read before being overwritten, so both are preserved.
    #[test]
    fn test_preserve_read_store() {
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    v1: int32 = 1int32
    store v0, v1
    v2: int32 = load v0
    v3: int32 = 2int32
    store v0, v3
    v4: int32 = load v0
    return v4
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&DeadStoreEliminate);
        test.assert_unchanged(input);
    }

    /// Store to allocation that is never read is eliminated.
    ///
    /// The stack allocation is never loaded from, so the store is dead.
    #[test]
    fn test_remove_store_to_unused_alloc() {
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    v1: int32 = 42int32
    store v0, v1
    v2: int32 = 0int32
    return v2
}"#;
        let expected = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    v1: int32 = 42int32
    v2: int32 = 0int32
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&DeadStoreEliminate);
        test.assert_output(expected);
    }

    /// Volatile store is never removed even when overwritten.
    #[test]
    fn test_preserve_volatile_store() {
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    v1: int32 = 1int32
    store v0, v1
    v2: int32 = 2int32
    store v0, v2
    v3: int32 = load v0
    return v3
}"#;

        let mut test = TestProgram::new(input);
        let function_id = test.entry_function_id();
        let store_id = test
            .store_instructions_in_entry(function_id)
            .into_iter()
            .next()
            .expect("missing store instruction");

        tag_store_access(&mut test, store_id, 4, true, None);

        test.run_pass(&DeadStoreEliminate);
        test.assert_unchanged(input);
    }

    /// Store to escaping allocation is preserved.
    ///
    /// When the allocation escapes (is passed to an external function),
    /// the store cannot be eliminated because the external function may read it.
    #[test]
    fn test_preserve_escaping_store() {
        let input = r#"
external function external(ref<int32, raw>): void
function test(): void {
b0:
    v0: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    v1: int32 = 42int32
    store v0, v1
    call external(v0): (ref<int32, raw>) -> void
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&DeadStoreEliminate);
        test.assert_unchanged(input);
    }

    /// Store before a nonescaping no memory call is removed.
    #[test]
    fn test_remove_store_before_nonescaping_no_memory_call() {
        let input = r#"
external function external(ref<int32, raw>): void
function test(): int32 {
b0:
    v0: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    v1: int32 = 42int32
    store v0, v1
    call external(v0): (ref<int32, raw>) -> void
    v2: int32 = 0int32
    return v2
}"#;
        let expected = r#"
external function external(ref<int32, raw>): void
function test(): int32 {
b0:
    v0: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    v1: int32 = 42int32
    call external(v0): (ref<int32, raw>) -> void
    v2: int32 = 0int32
    return v2
}"#;

        let mut test = TestProgram::new(input);

        let function_id = test.entry_function_id();
        let (call_inst, _callee) = test.first_call_in_entry(function_id);

        let arg0 = mir::CallArgumentEffect {
            access: mir::ArgumentAccess::None,
            escape: mir::ArgumentEscape::None,
            ..Default::default()
        };

        let call = mir::CallSite::Instruction(call_inst);
        let metadata = test.tree.metadata.functions.call_mut(call);
        metadata.memory = mir::MemoryEffect::none();
        metadata.arguments = vec![arg0];

        test.run_pass(&DeadStoreEliminate);
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
b0:
    v0: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    v1: int32 = 1int32
    store v0, v1
    v2: int32 = 2int32
    store v0, v2
    v3: int32 = 3int32
    store v0, v3
    v4: int32 = load v0
    return v4
}"#;
        let expected = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    v1: int32 = 1int32
    v2: int32 = 2int32
    v3: int32 = 3int32
    store v0, v3
    v4: int32 = load v0
    return v4
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&DeadStoreEliminate);
        test.assert_output(expected);
    }

    /// No changes when no stores.
    ///
    /// Function without stores is unchanged.
    #[test]
    fn test_no_changes() {
        let input = r#"
function test(): int32 {
b0:
    v0: int32 = 42int32
    return v0
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&DeadStoreEliminate);
        test.assert_unchanged(input);
    }

    /// Stores to different locations are independent.
    ///
    /// Stores to different memory locations don't interfere with each other.
    #[test]
    fn test_different_locations() {
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    v1: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    v2: int32 = 1int32
    store v0, v2
    v3: int32 = 2int32
    store v1, v3
    v4: int32 = load v0
    v5: int32 = load v1
    v6: int32 = int.add v4, v5
    return v6
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&DeadStoreEliminate);
        test.assert_unchanged(input);
    }

    /// Store before call that may read is preserved.
    ///
    /// Calls conservatively clobber memory state (for now?), so stores before calls
    /// must be preserved if the location may be read by the callee.
    #[test]
    fn test_preserve_store_before_call() {
        let input = r#"
external function readValue(ref<int32, raw>): int32
function test(): int32 {
b0:
    v0: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    v1: int32 = 42int32
    store v0, v1
    v2: int32 = call readValue(v0): (ref<int32, raw>) -> int32
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&DeadStoreEliminate);
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
b0:
    v0: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    call sideEffect(): () -> void
    v1: int32 = 1int32
    store v0, v1
    v2: int32 = 2int32
    store v0, v2
    v3: int32 = load v0
    return v3
}"#;
        let expected = r#"
external function sideEffect(): void
function test(): int32 {
b0:
    v0: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    call sideEffect(): () -> void
    v1: int32 = 1int32
    v2: int32 = 2int32
    store v0, v2
    v3: int32 = load v0
    return v3
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&DeadStoreEliminate);
        test.assert_output(expected);
    }

    /// Store to returned allocation is preserved.
    ///
    /// If the allocation is returned, the store is visible to the caller.
    #[test]
    fn test_preserve_store_to_returned() {
        let input = r#"
function test(): ref<int32, raw> {
b0:
    v0: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    v1: int32 = 42int32
    store v0, v1
    return v0
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&DeadStoreEliminate);
        test.assert_unchanged(input);
    }

    /// Store to allocation used in returned aggregate is preserved.
    #[test]
    fn test_preserve_store_returned_aggregate() {
        let input = r#"
function test(): (ref<int32, raw>, int32) {
b0:
    v0: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    v1: int32 = 42int32
    store v0, v1
    v2: int32 = 0int32
    v3: (ref<int32, raw>, int32) = tuple (ref<int32, raw>, int32) (v0, v2)
    return v3
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&DeadStoreEliminate);
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
function test(v0: ref<Pair, raw>): void {
b0(v0: ref<Pair, raw>):
    v1: ref<int32, borrowed> = field.address v0, 0
    v2: ref<int32, borrowed> = field.address v0, 1
    v3: int32 = 1int32
    v4: int32 = 2int32
    store v1, v3
    store v2, v4
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&DeadStoreEliminate);
        test.assert_unchanged(input);
    }

    /// Partial overwrite does not kill earlier bytes.
    #[test]
    fn test_preserve_partial_overwrite() {
        let input = r#"
function test(): ref<int64, raw> {
b0:
    v0: ref<int64, raw, space(frame)> = frame.alloc.zeroed int64
    v1: int64 = 0int64
    store v0, v1
    v2: int32 = 1int32
    store v0, v2
    return v0
}"#;

        let mut test = TestProgram::new(input);
        let function_id = test.entry_function_id();
        let store_ids = test.store_instructions_in_entry(function_id);
        let [first_store, second_store] = store_ids.as_slice() else {
            panic!("expected two store instructions");
        };

        tag_store_access(&mut test, *first_store, 8, false, None);
        tag_store_access(&mut test, *second_store, 4, false, None);

        test.run_pass(&DeadStoreEliminate);
        test.assert_unchanged(input);
    }

    /// Dead memset to non escaping stack memory is removed.
    #[test]
    fn test_remove_dead_memset() {
        let input = r#"
function test(): void {
b0:
    v0: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    v1: int8 = 0int8
    v2: int64 = 4int64
    intrinsic.memory.raw.setBytes(v0, v1, v2)
    return
}"#;
        let expected = r#"
function test(): void {
b0:
    v0: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    v1: int8 = 0int8
    v2: int64 = 4int64
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&DeadStoreEliminate);
        test.assert_output(expected);
    }

    /// Memset to a live location is preserved.
    #[test]
    fn test_preserve_memset_with_read() {
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    v1: int8 = 0int8
    v2: int64 = 4int64
    intrinsic.memory.raw.setBytes(v0, v1, v2)
    v3: int32 = load v0
    return v3
}"#;
        let expected = input;

        let mut test = TestProgram::new(input);
        test.run_pass(&DeadStoreEliminate);
        test.assert_output(expected);
    }

    /// Dead memcpy to non escaping stack memory is removed.
    #[test]
    fn test_remove_dead_memcpy() {
        // input test
        let input = r#"
function test(): void {
b0:
    v0: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    v1: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    v2: int64 = 4int64
    intrinsic.memory.raw.copyBytes(v0, v1, v2)
    return
}"#;
        let expected = r#"
function test(): void {
b0:
    v0: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    v1: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    v2: int64 = 4int64
    return
}"#;

        // run dse
        let mut test = TestProgram::new(input);
        test.run_pass(&DeadStoreEliminate);
        test.assert_output(expected);
    }

    /// Memcpy to a live location is preserved.
    #[test]
    fn test_preserve_memcpy_with_read() {
        // input test
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    v1: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    v2: int64 = 4int64
    intrinsic.memory.raw.copyBytes(v0, v1, v2)
    v3: int32 = load v0
    return v3
}"#;
        let expected = input;

        // run dse
        let mut test = TestProgram::new(input);
        test.run_pass(&DeadStoreEliminate);
        test.assert_output(expected);
    }

    /// Dead memmove to non escaping stack memory is removed.
    #[test]
    fn test_remove_dead_memmove() {
        // input test
        let input = r#"
function test(): void {
b0:
    v0: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    v1: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    v2: int64 = 4int64
    intrinsic.memory.raw.moveBytes(v0, v1, v2)
    return
}"#;
        let expected = r#"
function test(): void {
b0:
    v0: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    v1: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    v2: int64 = 4int64
    return
}"#;

        // run dse
        let mut test = TestProgram::new(input);
        test.run_pass(&DeadStoreEliminate);
        test.assert_output(expected);
    }

    /// Memmove to a live location is preserved.
    #[test]
    fn test_preserve_memmove_with_read() {
        // input test
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    v1: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    v2: int64 = 4int64
    intrinsic.memory.raw.moveBytes(v0, v1, v2)
    v3: int32 = load v0
    return v3
}"#;
        let expected = input;

        // run dse
        let mut test = TestProgram::new(input);
        test.run_pass(&DeadStoreEliminate);
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
function test(v0: ref<Point, raw>): void {
b0(v0: ref<Point, raw>):
    v1: int32 = 1int32
    v2: int32 = 2int32
    v3: Point = struct Point (v1, v2)
    store v0, v3
    v4: int32 = 3int32
    v5: int32 = 4int32
    v6: Point = struct Point (v4, v5)
    store v0, v6
    return
}"#;
        let expected = input;

        // attach unknown size metadata to both stores
        let mut test = TestProgram::new(input);
        let function_id = test.entry_function_id();
        let store_ids = test.store_instructions_in_entry(function_id);
        let [first_store, second_store] = store_ids.as_slice() else {
            panic!("expected two store instructions");
        };

        test.insert_pointer_access(
            *first_store,
            mir::MemoryAccessKind::Write,
            mir::Value::new(0),
            None,
        );
        test.insert_pointer_access(
            *second_store,
            mir::MemoryAccessKind::Write,
            mir::Value::new(0),
            None,
        );

        // run dse
        test.run_pass(&DeadStoreEliminate);
        test.assert_output(expected);
    }

    /// Cross-block dead store: store overwritten in successor block.
    ///
    /// When a store is followed by an unconditional jump to a block that
    /// overwrites the same location before any read, the first store is dead.
    #[test]
    fn test_cross_block_overwritten() {
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    v1: int32 = 1int32
    store v0, v1
    jump b1
b1:
    v2: int32 = 2int32
    store v0, v2
    v3: int32 = load v0
    return v3
}"#;
        let expected = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    v1: int32 = 1int32
    jump b1
b1:
    v2: int32 = 2int32
    store v0, v2
    v3: int32 = load v0
    return v3
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&DeadStoreEliminate);
        test.assert_output(expected);
    }

    /// Store preserved when read in one branch.
    ///
    /// If a store might be read on some control flow path, it must be preserved.
    #[test]
    fn test_preserve_cross_block_read_on_path() {
        let input = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    v1: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    v2: int32 = 1int32
    store v1, v2
    branch v0, b1, b2
b1:
    v3: int32 = load v1
    return v3
b2:
    v4: int32 = 2int32
    store v1, v4
    v5: int32 = load v1
    return v5
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&DeadStoreEliminate);
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
b0(v0: int32):
    v1: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    v2: int32 = 42int32
    store v1, v2
    switch v0, b1(v1), 0 => b2
b1(v3: ref<int32, raw>):
    return
b2:
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&DeadStoreEliminate);
        test.assert_unchanged(input);
    }
}
