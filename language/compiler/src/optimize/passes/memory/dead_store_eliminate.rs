use std::collections::{HashMap, HashSet};

use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::optimize::analyses::{
    AliasAnalysis, MemoryAccess, MemoryAccessId, MemoryAccessLocation, MemorySSA,
    OwnershipAnalysis, PostDominatorTree,
};
use crate::optimize::common::{
    DecomposedPointer, PointerDecomposer, RangeRelation, build_value_definition_map,
    range_relation, stack_alloc_base,
};
use crate::optimize::{AnalysisPreservation, FunctionAnalyses, FunctionPass, PipelineContext};

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
        // skip empty functions
        let _entry = match function.entry {
            Some(entry) => entry,
            None => return AnalysisPreservation::all(),
        };

        // get analyses
        let (aa, memory_ssa, ownership, postdom) = {
            let analyses = FunctionAnalyses::new(function, tree);
            (
                analyses.get::<AliasAnalysis>().clone(),
                analyses.get::<MemorySSA>(),
                analyses.get::<OwnershipAnalysis>(),
                analyses.get::<PostDominatorTree>(),
            )
        };

        // run dead store elimination
        let changed = run_dead_store_eliminate(
            function,
            tree,
            &aa,
            memory_ssa.as_ref(),
            ownership.as_ref(),
            postdom.as_ref(),
        );

        // preserve analyses when unchanged
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
    memory_ssa: &MemorySSA,
    ownership: &OwnershipAnalysis,
    postdom: &PostDominatorTree,
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
    let non_escaping_stack_allocs = collect_non_escaping_stack_allocs(function, tree, &definitions);

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
        if store_is_non_escaping_stack(&store, &definitions, &non_escaping_stack_allocs, tree) {
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
            ownership.value_types(),
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

#[derive(Clone)]
struct StoreCandidate {
    instruction: mir::LocalNodeId<mir::Instruction>,
    access: MemoryAccessId,
    block: mir::LocalNodeId<mir::Block>,
    index: usize,
    pointer: Option<mir::Value>,
    location: MemoryAccessLocation,
    is_volatile: bool,
    is_barrier: bool,
}

#[derive(Clone)]
struct DefAccessInfo {
    access: MemoryAccessId,
    block: mir::LocalNodeId<mir::Block>,
    index: usize,
}

fn collect_store_candidates(
    function: &mir::Function,
    tree: &mir::NodeTree,
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

            let Some(accesses) = memory_ssa.accesses_for_instruction(instruction_id) else {
                continue;
            };

            for &access_id in accesses {
                let MemoryAccess::Def(def_access) = memory_ssa.access(access_id) else {
                    continue;
                };

                let pointer = match &def_access.effect.location {
                    MemoryAccessLocation::Pointer(location) => Some(location.ptr),
                    _ => None,
                };

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

fn collect_def_accesses(
    function: &mir::Function,
    tree: &mir::NodeTree,
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

fn collect_live_defs(
    function: &mir::Function,
    tree: &mir::NodeTree,
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
                        live_defs.insert(clobber);
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
                        live_defs.insert(clobber);
                    }
                    _ => {}
                }
            }
        }
    }

    live_defs
}

fn collect_non_escaping_stack_allocs(
    function: &mir::Function,
    tree: &mir::NodeTree,
    definitions: &HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
) -> HashSet<mir::Value> {
    // collect stack allocation bases
    let mut stack_allocs = HashSet::new();

    // scan blocks for stack allocations
    for &block_id in &function.blocks {
        // read the block
        let block = tree.get(block_id);

        // scan instructions in the block
        for &instruction_id in &block.instructions {
            // read the instruction
            let instruction = tree.get(instruction_id);
            if let mir::Instruction::StackAlloc { destination, .. } = instruction {
                stack_allocs.insert(*destination);
            }
        }
    }

    // collect escaping stack allocations
    let mut escaping = HashSet::new();

    // scan blocks for escaping uses
    for &block_id in &function.blocks {
        // read the block
        let block = tree.get(block_id);

        // scan instructions in the block
        for &instruction_id in &block.instructions {
            // read the instruction
            let instruction = tree.get(instruction_id);
            match instruction {
                mir::Instruction::Call { .. } | mir::Instruction::CallIndirect { .. } => {
                    // capture call metadata for escape checks
                    let call_metadata = tree.call_table.call_metadata(instruction_id);

                    // mark stack pointers passed to calls as escaping
                    if let Some(arg_slice) = instruction.argument_slice() {
                        let arguments = tree.get_arguments(arg_slice);

                        for (index, &arg) in arguments.iter().enumerate() {
                            if call_argument_escapes(call_metadata, index) {
                                record_stack_escape(
                                    arg,
                                    definitions,
                                    tree,
                                    &stack_allocs,
                                    &mut escaping,
                                );
                            }
                        }
                    }
                }
                mir::Instruction::Store { value, .. } => {
                    // mark stored stack pointers as escaping
                    record_stack_escape(*value, definitions, tree, &stack_allocs, &mut escaping);
                }
                _ => {}
            }
        }

        // scan terminators for escaping values
        match &block.terminator {
            mir::Terminator::Return { value: Some(value) } => {
                record_stack_escape(*value, definitions, tree, &stack_allocs, &mut escaping);
            }
            mir::Terminator::Jump { arguments, .. } => {
                for &arg in arguments {
                    record_stack_escape(arg, definitions, tree, &stack_allocs, &mut escaping);
                }
            }
            mir::Terminator::Branch {
                then_arguments,
                else_arguments,
                ..
            } => {
                for &arg in then_arguments.iter().chain(else_arguments.iter()) {
                    record_stack_escape(arg, definitions, tree, &stack_allocs, &mut escaping);
                }
            }
            mir::Terminator::Check {
                success, failure, ..
            } => {
                for &arg in success.arguments.iter().chain(failure.arguments.iter()) {
                    record_stack_escape(arg, definitions, tree, &stack_allocs, &mut escaping);
                }
            }
            mir::Terminator::Switch {
                cases,
                default_arguments,
                ..
            } => {
                for &arg in default_arguments {
                    record_stack_escape(arg, definitions, tree, &stack_allocs, &mut escaping);
                }
                for case in cases {
                    for &arg in &case.arguments {
                        record_stack_escape(arg, definitions, tree, &stack_allocs, &mut escaping);
                    }
                }
            }
            mir::Terminator::Yield {
                value,
                resume_arguments,
                ..
            } => {
                record_stack_escape(*value, definitions, tree, &stack_allocs, &mut escaping);
                for &arg in resume_arguments {
                    record_stack_escape(arg, definitions, tree, &stack_allocs, &mut escaping);
                }
            }
            mir::Terminator::TailCall { arguments, .. } => {
                for &arg in arguments {
                    record_stack_escape(arg, definitions, tree, &stack_allocs, &mut escaping);
                }
            }
            mir::Terminator::TailCallIndirect { callee, arguments } => {
                record_stack_escape(*callee, definitions, tree, &stack_allocs, &mut escaping);
                for &arg in arguments {
                    record_stack_escape(arg, definitions, tree, &stack_allocs, &mut escaping);
                }
            }
            mir::Terminator::Return { value: None } | mir::Terminator::Unreachable => {}
        }
    }

    // filter non escaping stack allocations
    stack_allocs
        .into_iter()
        .filter(|alloc| !escaping.contains(alloc))
        .collect()
}

/// Mark stack allocations that may escape through a value.
fn record_stack_escape(
    value: mir::Value,
    definitions: &HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
    tree: &mir::NodeTree,
    stack_allocs: &HashSet<mir::Value>,
    escaping: &mut HashSet<mir::Value>,
) {
    // visit values recursively to detect aggregate escapes
    let mut visited = HashSet::new();
    record_stack_escape_value(
        value,
        definitions,
        tree,
        stack_allocs,
        escaping,
        &mut visited,
    );
}

/// Walk a value to find stack allocations that escape.
fn record_stack_escape_value(
    value: mir::Value,
    definitions: &HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
    tree: &mir::NodeTree,
    stack_allocs: &HashSet<mir::Value>,
    escaping: &mut HashSet<mir::Value>,
    visited: &mut HashSet<mir::Value>,
) {
    // stop on cycles
    if !visited.insert(value) {
        return;
    }

    // resolve the stack base and mark it as escaping
    if let Some(base) = stack_alloc_base(value, definitions, tree) {
        if stack_allocs.contains(&base) {
            escaping.insert(base);
        }
        return;
    }

    // inspect aggregate construction for nested pointers
    let Some(instruction_id) = definitions.get(&value) else {
        return;
    };
    let instruction = tree.get(*instruction_id);
    match instruction {
        mir::Instruction::Struct { fields, .. } => {
            let args = tree.get_arguments(*fields);
            for &arg in args {
                record_stack_escape_value(arg, definitions, tree, stack_allocs, escaping, visited);
            }
        }
        mir::Instruction::Tuple { elements, .. } | mir::Instruction::Array { elements, .. } => {
            let args = tree.get_arguments(*elements);
            for &arg in args {
                record_stack_escape_value(arg, definitions, tree, stack_allocs, escaping, visited);
            }
        }
        _ => {}
    }
}

/// Report whether a call argument may escape.
fn call_argument_escapes(call_metadata: Option<&mir::CallMetadata>, index: usize) -> bool {
    // default to escaping when metadata is missing
    let Some(metadata) = call_metadata else {
        return true;
    };

    // default to escaping when argument metadata is missing
    let Some(arg_metadata) = metadata.argument_metadata.get(index) else {
        return true;
    };

    // treat no capture arguments as non escaping
    !matches!(arg_metadata.attributes.capture, mir::CaptureKind::NoCapture)
}

/// Return true when a store targets a non escaping stack allocation.
fn store_is_non_escaping_stack(
    store: &StoreCandidate,
    definitions: &HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
    non_escaping_stack_allocs: &HashSet<mir::Value>,
    tree: &mir::NodeTree,
) -> bool {
    // only pointer locations can be stack allocations
    let MemoryAccessLocation::Pointer(_) = store.location else {
        return false;
    };

    // resolve the stack base for this store pointer
    let Some(pointer) = store.pointer else {
        return false;
    };
    let Some(base) = stack_alloc_base(pointer, definitions, tree) else {
        return false;
    };

    // report whether the base is non escaping
    non_escaping_stack_allocs.contains(&base)
}

/// Return true when a later clobbering def postdominates the store.
// allow extra context parameters for clarity
#[allow(clippy::too_many_arguments)]
fn store_is_postdominated_by_clobber(
    store: &StoreCandidate,
    def_accesses: &[DefAccessInfo],
    memory_ssa: &MemorySSA,
    aa: &AliasAnalysis,
    postdom: &PostDominatorTree,
    function: &mir::Function,
    tree: &mir::NodeTree,
    definitions: &HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
    constants: &HashMap<mir::Value, i64>,
    value_types: &HashMap<mir::Value, mir::LocalNodeId<mir::Type>>,
) -> bool {
    let mut decomposer = PointerDecomposer::new(
        constants,
        definitions,
        tree,
        &function.parameters,
        false,
        Some(value_types),
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
            if overwrites && memory_ssa.def_clobbers_access(def.access, store.access, aa, tree) {
                return true;
            }

            continue;
        }

        if matches!(store.location, MemoryAccessLocation::Local(_))
            && memory_ssa.def_clobbers_access(def.access, store.access, aa, tree)
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
    tree: &mir::NodeTree,
) -> HashMap<mir::Value, i64> {
    let mut constants = HashMap::new();

    for &block_id in &function.blocks {
        let block = tree.get(block_id);
        for &instruction_id in &block.instructions {
            let instruction = tree.get(instruction_id);
            if let mir::Instruction::Const { destination, value } = instruction {
                match value {
                    mir::Constant::Int { value, .. } => {
                        constants.insert(*destination, *value);
                    }
                    mir::Constant::UInt { value, .. } => {
                        if let Ok(value) = i64::try_from(*value) {
                            constants.insert(*destination, value);
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

    /// Return store instructions from the entry block.
    fn store_instructions_in_entry(
        function_id: mir::LocalNodeId<mir::Function>,
        tree: &mir::NodeTree,
    ) -> Vec<mir::LocalNodeId<mir::Instruction>> {
        let function = tree.get(function_id);
        let entry = function.entry.expect("missing entry block");
        let block = tree.get(entry);

        block
            .instructions
            .iter()
            .copied()
            .filter(|instruction_id| {
                matches!(tree.get(*instruction_id), mir::Instruction::Store { .. })
            })
            .collect()
    }

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

    /// Volatile store is never removed even when overwritten.
    #[test]
    fn test_preserve_volatile_store() {
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

        let mut program = TestProgram::new(input);
        let function_id = program.entry_function_id();
        let store_id = store_instructions_in_entry(function_id, &program.tree)
            .into_iter()
            .next()
            .expect("missing store instruction");

        let pointer = match program.tree.get(store_id) {
            mir::Instruction::Store { pointer, .. } => *pointer,
            _ => panic!("expected store instruction"),
        };

        let access = mir::MemoryAccessMetadata {
            kind: mir::MemoryAccessKind::Write,
            target: mir::MemoryAccessTarget::Pointer(pointer),
            size: Some(4),
            alignment: None,
            is_volatile: true,
            is_invariant: false,
            is_non_temporal: false,
            ordering: None,
            address_space: None,
            alias_scopes: Vec::new(),
            noalias_scopes: Vec::new(),
            tbaa_tag: None,
        };
        program
            .tree
            .memory_table
            .insert_memory_accesses(store_id, vec![access]);

        program.run_pass(&DeadStoreEliminate);
        program.assert_unchanged(input);
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

    /// Store before a nocapture readnone call is removed.
    #[test]
    fn test_remove_store_before_nocapture_readnone_call() {
        let input = r#"extern function @external(ref<raw i32>) -> void
function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = iconst 42i32
    store v0, v1
    call @external(v0)
    v2 = iconst 0i32
    return v2
}"#;
        let expected = r#"extern function @external(ref<raw i32>) -> void
function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = iconst 42i32
    call @external(v0)
    v2 = iconst 0i32
    return v2
}"#;

        let mut program = TestProgram::new(input);

        let function_id = program.entry_function_id();
        let (call_inst, callee) = program.first_call_in_entry(function_id);
        let signature = program.call_signature_for_callee(callee);

        let mut arg0 = mir::CallArgumentMetadata::default();
        arg0.attributes.capture = mir::CaptureKind::NoCapture;
        arg0.access = mir::ArgumentAccess::None;

        let metadata = mir::CallMetadata::direct(callee, signature)
            .with_memory_effects(mir::MemoryEffect::none())
            .with_argument_metadata(vec![arg0]);

        program
            .tree
            .call_table
            .call_metadata_by_instruction_id
            .insert(call_inst, metadata);

        program.run_pass(&DeadStoreEliminate);
        program.assert_output(expected);
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

    /// Store to allocation used in returned aggregate is preserved.
    #[test]
    fn test_preserve_store_returned_aggregate() {
        let input = r#"function @test() -> (ref<raw i32>, i32) {
block0:
    v0 = stack.alloc i32
    v1 = iconst 42i32
    store v0, v1
    v2 = iconst 0i32
    v3 = tuple (ref<raw i32>, i32) (v0, v2)
    return v3
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&DeadStoreEliminate);
        program.assert_unchanged(input);
    }

    /// Stores to disjoint fields are not treated as clobbers.
    #[test]
    fn test_preserve_disjoint_field_stores() {
        let input = r#"type @Pair = { i32, i32 }
function @test(v0: ref<raw @Pair>) -> void {
block0(v0: ref<raw @Pair>):
    v1 = field.addr v0, 0
    v2 = field.addr v0, 1
    v3 = iconst 1i32
    v4 = iconst 2i32
    store v1, v3
    store v2, v4
    return
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&DeadStoreEliminate);
        program.assert_unchanged(input);
    }

    /// Partial overwrite does not kill earlier bytes.
    #[test]
    fn test_preserve_partial_overwrite() {
        let input = r#"function @test() -> ref<raw i64> {
block0:
    v0 = stack.alloc i64
    v1 = iconst 0i64
    store v0, v1
    v2 = iconst 1i32
    store v0, v2
    return v0
}"#;

        let mut program = TestProgram::new(input);
        let function_id = program.entry_function_id();
        let store_ids = store_instructions_in_entry(function_id, &program.tree);
        let [first_store, second_store] = store_ids.as_slice() else {
            panic!("expected two store instructions");
        };

        let first_pointer = match program.tree.get(*first_store) {
            mir::Instruction::Store { pointer, .. } => *pointer,
            _ => panic!("expected store instruction"),
        };
        let second_pointer = match program.tree.get(*second_store) {
            mir::Instruction::Store { pointer, .. } => *pointer,
            _ => panic!("expected store instruction"),
        };

        let first_access = mir::MemoryAccessMetadata {
            kind: mir::MemoryAccessKind::Write,
            target: mir::MemoryAccessTarget::Pointer(first_pointer),
            size: Some(8),
            alignment: None,
            is_volatile: false,
            is_invariant: false,
            is_non_temporal: false,
            ordering: None,
            address_space: None,
            alias_scopes: Vec::new(),
            noalias_scopes: Vec::new(),
            tbaa_tag: None,
        };
        let second_access = mir::MemoryAccessMetadata {
            kind: mir::MemoryAccessKind::Write,
            target: mir::MemoryAccessTarget::Pointer(second_pointer),
            size: Some(4),
            alignment: None,
            is_volatile: false,
            is_invariant: false,
            is_non_temporal: false,
            ordering: None,
            address_space: None,
            alias_scopes: Vec::new(),
            noalias_scopes: Vec::new(),
            tbaa_tag: None,
        };

        program
            .tree
            .memory_table
            .insert_memory_accesses(*first_store, vec![first_access]);
        program
            .tree
            .memory_table
            .insert_memory_accesses(*second_store, vec![second_access]);

        program.run_pass(&DeadStoreEliminate);
        program.assert_unchanged(input);
    }

    /// Dead memset to non escaping stack memory is removed.
    #[test]
    fn test_remove_dead_memset() {
        let input = r#"function @test() -> void {
block0:
    v0 = stack.alloc i32
    v1 = iconst 0i8
    v2 = iconst 4i64
    intrinsic.memset(v0, v1, v2)
    return
}"#;
        let expected = r#"function @test() -> void {
block0:
    v0 = stack.alloc i32
    v1 = iconst 0i8
    v2 = iconst 4i64
    return
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&DeadStoreEliminate);
        program.assert_output(expected);
    }

    /// Memset to a live location is preserved.
    #[test]
    fn test_preserve_memset_with_read() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = iconst 0i8
    v2 = iconst 4i64
    intrinsic.memset(v0, v1, v2)
    v3 = load v0
    return v3
}"#;
        let expected = input;

        let mut program = TestProgram::new(input);
        program.run_pass(&DeadStoreEliminate);
        program.assert_output(expected);
    }

    /// Dead memcpy to non escaping stack memory is removed.
    #[test]
    fn test_remove_dead_memcpy() {
        // input program
        let input = r#"function @test() -> void {
block0:
    v0 = stack.alloc i32
    v1 = stack.alloc i32
    v2 = iconst 4i64
    intrinsic.memcpy(v0, v1, v2)
    return
}"#;
        let expected = r#"function @test() -> void {
block0:
    v0 = stack.alloc i32
    v1 = stack.alloc i32
    v2 = iconst 4i64
    return
}"#;

        // run dse
        let mut program = TestProgram::new(input);
        program.run_pass(&DeadStoreEliminate);
        program.assert_output(expected);
    }

    /// Memcpy to a live location is preserved.
    #[test]
    fn test_preserve_memcpy_with_read() {
        // input program
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = stack.alloc i32
    v2 = iconst 4i64
    intrinsic.memcpy(v0, v1, v2)
    v3 = load v0
    return v3
}"#;
        let expected = input;

        // run dse
        let mut program = TestProgram::new(input);
        program.run_pass(&DeadStoreEliminate);
        program.assert_output(expected);
    }

    /// Dead memmove to non escaping stack memory is removed.
    #[test]
    fn test_remove_dead_memmove() {
        // input program
        let input = r#"function @test() -> void {
block0:
    v0 = stack.alloc i32
    v1 = stack.alloc i32
    v2 = iconst 4i64
    intrinsic.memmove(v0, v1, v2)
    return
}"#;
        let expected = r#"function @test() -> void {
block0:
    v0 = stack.alloc i32
    v1 = stack.alloc i32
    v2 = iconst 4i64
    return
}"#;

        // run dse
        let mut program = TestProgram::new(input);
        program.run_pass(&DeadStoreEliminate);
        program.assert_output(expected);
    }

    /// Memmove to a live location is preserved.
    #[test]
    fn test_preserve_memmove_with_read() {
        // input program
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = stack.alloc i32
    v2 = iconst 4i64
    intrinsic.memmove(v0, v1, v2)
    v3 = load v0
    return v3
}"#;
        let expected = input;

        // run dse
        let mut program = TestProgram::new(input);
        program.run_pass(&DeadStoreEliminate);
        program.assert_output(expected);
    }

    /// Stores with unknown sizes are not treated as full overwrites.
    #[test]
    fn test_preserve_unknown_size_overwrite() {
        // input program
        let input = r#"type @Point = { i32, i32 }
function @test(v0: ref<raw mut @Point>) -> void {
block0(v0: ref<raw mut @Point>):
    v1 = iconst 1i32
    v2 = iconst 2i32
    v3 = struct @Point (v1, v2)
    store v0, v3
    v4 = iconst 3i32
    v5 = iconst 4i32
    v6 = struct @Point (v4, v5)
    store v0, v6
    return
}"#;
        let expected = input;

        // attach unknown size metadata to both stores
        let mut program = TestProgram::new(input);
        let function_id = program.entry_function_id();
        let store_ids = store_instructions_in_entry(function_id, &program.tree);
        let [first_store, second_store] = store_ids.as_slice() else {
            panic!("expected two store instructions");
        };

        program.insert_pointer_access(
            *first_store,
            mir::MemoryAccessKind::Write,
            mir::Value::new(0),
            None,
            Vec::new(),
            Vec::new(),
            None,
        );
        program.insert_pointer_access(
            *second_store,
            mir::MemoryAccessKind::Write,
            mir::Value::new(0),
            None,
            Vec::new(),
            Vec::new(),
            None,
        );

        // run dse
        program.run_pass(&DeadStoreEliminate);
        program.assert_output(expected);
    }

    /// Stores in disjoint alias scopes are not treated as clobbers.
    #[test]
    fn test_preserve_store_with_alias_scope_disjoint() {
        // input program
        let input = r#"function @test(v0: ref<raw mut i32>, v1: ref<raw mut i32>) -> void {
block0(v0: ref<raw mut i32>, v1: ref<raw mut i32>):
    v2 = iconst 1i32
    store v0, v2
    v3 = iconst 2i32
    store v1, v3
    return
}"#;
        let expected = input;

        // create a scope for disambiguation
        let mut program = TestProgram::new(input);
        let scope = {
            let scopes = &mut program.tree.memory_table.alias_scopes;
            let domain = scopes.create_domain(None);
            scopes.create_scope(domain, None)
        };

        // attach disjoint scope metadata to the stores
        let function_id = program.entry_function_id();
        let store_ids = store_instructions_in_entry(function_id, &program.tree);
        let [first_store, second_store] = store_ids.as_slice() else {
            panic!("expected two store instructions");
        };

        program.insert_pointer_access(
            *first_store,
            mir::MemoryAccessKind::Write,
            mir::Value::new(0),
            Some(4),
            vec![scope],
            Vec::new(),
            None,
        );
        program.insert_pointer_access(
            *second_store,
            mir::MemoryAccessKind::Write,
            mir::Value::new(1),
            Some(4),
            Vec::new(),
            vec![scope],
            None,
        );

        // run dse
        program.run_pass(&DeadStoreEliminate);
        program.assert_output(expected);
    }

    /// Stores with disjoint tbaa offsets are not treated as clobbers.
    #[test]
    fn test_preserve_store_with_tbaa_disjoint_offsets() {
        // input program
        let input = r#"function @test(v0: ref<raw mut i32>, v1: ref<raw mut i32>) -> void {
block0(v0: ref<raw mut i32>, v1: ref<raw mut i32>):
    v2 = iconst 1i32
    store v0, v2
    v3 = iconst 2i32
    store v1, v3
    return
}"#;
        let expected = input;

        // create disjoint tbaa tags
        let mut program = TestProgram::new(input);
        let (tag_a, tag_b) = {
            let tbaa = &mut program.tree.memory_table.tbaa;
            let root = tbaa.create_node(None, None, false);
            let access = tbaa.create_node(None, Some(root), false);
            let tag_a = tbaa.create_tag(root, access, 0, 4, false);
            let tag_b = tbaa.create_tag(root, access, 8, 4, false);
            (tag_a, tag_b)
        };

        // attach disjoint tbaa metadata to the stores
        let function_id = program.entry_function_id();
        let store_ids = store_instructions_in_entry(function_id, &program.tree);
        let [first_store, second_store] = store_ids.as_slice() else {
            panic!("expected two store instructions");
        };

        program.insert_pointer_access(
            *first_store,
            mir::MemoryAccessKind::Write,
            mir::Value::new(0),
            Some(4),
            Vec::new(),
            Vec::new(),
            Some(tag_a),
        );
        program.insert_pointer_access(
            *second_store,
            mir::MemoryAccessKind::Write,
            mir::Value::new(1),
            Some(4),
            Vec::new(),
            Vec::new(),
            Some(tag_b),
        );

        // run dse
        program.run_pass(&DeadStoreEliminate);
        program.assert_output(expected);
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
