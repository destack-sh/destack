use std::collections::{HashMap, HashSet};

use crate::declare_mir_pass;
use destack_mir as mir;

use crate::common::mir::analysis::{
    AliasAnalysis, ConstantPropagation, ControlFlowGraph, DominatorTree, MemoryAccess,
    MemoryAccessEffect, MemoryAccessId, MemoryAccessLocation, MemorySSA,
};
use crate::common::mir::{
    EdgeSplitPolicy, ValueEquivalence, build_instruction_block_map, build_use_def_maps,
    build_value_definition_map, effect_is_trackable, effects_match_location, ensure_edge_block,
    instruction_has_atomic_ordering, instruction_is_read_only_access, instruction_is_speculatable,
    resolve_edge_value, value_available_in_block,
};
use crate::optimize::{AnalysisPreservation, FunctionPass, PipelineContext};

declare_mir_pass! {
    /// Eliminate partially redundant stores by sinking them to predecessor edges.
    ///
    /// When a join block stores a value that is already stored on some incoming paths,
    /// insert stores on the missing edges and remove the redundant join store.
    ///
    /// ```mir
    /// function before(v0: boolean, v1: int32): void {
    /// b0(v0: boolean, v1: int32):
    ///     v2 = frame.alloc.zeroed int32 -> ref<int32, raw, space(frame)>
    ///     branch v0, b1, b2
    /// b1:
    ///     store v2, v1
    ///     jump b3
    /// b2:
    ///     jump b3
    /// b3:
    ///     store v2, v1
    ///     return
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function after(v0: boolean, v1: int32): void {
    /// b0(v0: boolean, v1: int32):
    ///     v2 = frame.alloc.zeroed int32 -> ref<int32, raw, space(frame)>
    ///     branch v0, b1, b2
    /// b1:
    ///     store v2, v1
    ///     jump b3
    /// b2:
    ///     store v2, v1
    ///     jump b3
    /// b3:
    ///     return
    /// }
    /// ```
    #[pass(id = "store-pre", requires(call_effects, memory_access_metadata))]
    pub StorePre,
    "Eliminate partially redundant stores"
}

impl FunctionPass for StorePre {
    /// Run store PRE on the function.
    fn run(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::Tree,
        ctx: &PipelineContext<'_>,
    ) -> AnalysisPreservation {
        // skip imported functions
        if function.entry.is_none() {
            return AnalysisPreservation::all();
        }

        // run store PRE
        let changed = run_store_pre(function, tree, ctx);

        if changed {
            AnalysisPreservation::none()
        } else {
            AnalysisPreservation::all()
        }
    }

    /// Return the pass name.
    fn name(&self) -> &'static str {
        "StorePre"
    }

    /// Return the pass id.
    fn id(&self) -> &'static str {
        "store-pre"
    }
}

/// Kind of store candidate.
#[derive(Clone, Copy, PartialEq, Eq)]
enum StoreKind {
    /// Store to a pointer.
    Store,
    /// Store to a local slot.
    LocalSet,
}

/// Candidate store for PRE.
#[derive(Clone)]
struct StoreCandidate {
    /// Block containing the store.
    block: mir::LocalNodeId<mir::Block>,
    /// Store instruction id.
    instruction: mir::LocalNodeId<mir::Instruction>,
    /// Store pointer when applicable.
    pointer: Option<mir::Value>,
    /// Local target when applicable.
    local: Option<mir::LocalNodeId<mir::Local>>,
    /// Stored value.
    value: mir::Value,
    /// Memory effect for the store.
    effect: MemoryAccessEffect,
    /// Store kind.
    kind: StoreKind,
}

/// Store insertion plan for a predecessor edge.
#[derive(Clone, Copy)]
struct EdgeStorePlan {
    /// Predecessor block id.
    predecessor: mir::LocalNodeId<mir::Block>,
    /// Pointer value to store to on this edge.
    pointer: Option<mir::Value>,
    /// Local target when applicable.
    local: Option<mir::LocalNodeId<mir::Local>>,
    /// Value to store on this edge.
    value: mir::Value,
    /// True when the store is already available on this edge.
    already_stored: bool,
}

/// Run store PRE and return true when changes are made.
fn run_store_pre(
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    ctx: &PipelineContext<'_>,
) -> bool {
    // gather analyses
    let analyses = ctx.function_analyses(function, tree);
    let cfg = analyses.get::<ControlFlowGraph>().clone();
    let domtree = analyses.get::<DominatorTree>().clone();
    let memory_ssa = analyses.get::<MemorySSA>();
    let alias = analyses.get::<AliasAnalysis>().clone();
    let constants = analyses.get::<ConstantPropagation>();

    // build value definition info
    let use_def = build_use_def_maps(function, tree);
    let definitions = build_value_definition_map(function, tree);
    let instruction_blocks = build_instruction_block_map(function, tree);
    let function_params: HashSet<_> = function
        .parameters
        .iter()
        .filter_map(|param| param.value.value())
        .collect();

    // track modifications
    let mut edge_blocks: HashMap<
        (mir::LocalNodeId<mir::Block>, mir::LocalNodeId<mir::Block>),
        mir::LocalNodeId<mir::Block>,
    > = HashMap::new();
    let mut to_remove = HashSet::new();
    let mut changed = false;

    // scan each block for eligible stores
    let block_ids = function.blocks.clone();
    for block_id in block_ids {
        // collect block parameters for edge resolution
        let block = tree.get(block_id).clone();
        let param_indices = block
            .parameters
            .iter()
            .enumerate()
            .filter_map(|(index, param)| Some((param.value.value()?, index)))
            .collect::<HashMap<_, _>>();

        // scan block instructions for store candidates
        for &instruction_id in &block.instructions {
            // select store instructions
            let (kind, pointer, local, value) = match tree.get(instruction_id) {
                mir::Instruction::Store { pointer, value } => {
                    let Some(pointer) = pointer.value() else {
                        continue;
                    };
                    let Some(value) = value.value() else {
                        continue;
                    };

                    (StoreKind::Store, Some(pointer), None, value)
                }
                mir::Instruction::LocalSet { local, value } => {
                    let Some(local) = local.local() else {
                        continue;
                    };
                    let Some(value) = value.value() else {
                        continue;
                    };

                    (StoreKind::LocalSet, None, Some(local), value)
                }
                _ => continue,
            };

            // validate store eligibility
            let Some(candidate) = store_access_info(
                block_id,
                instruction_id,
                pointer,
                local,
                value,
                kind,
                tree,
                memory_ssa.as_ref(),
            ) else {
                continue;
            };

            // build edge insertions for each predecessor
            let edge_plan = {
                let mut equivalence = ValueEquivalence::new_with_constants(
                    tree,
                    &definitions,
                    constants.as_ref(),
                    &instruction_blocks,
                );

                collect_edge_insertions(
                    &candidate,
                    &cfg,
                    &domtree,
                    tree,
                    &use_def.def_block,
                    &function_params,
                    &param_indices,
                    memory_ssa.as_ref(),
                    &alias,
                    &mut equivalence,
                )
            };

            let Some(edge_plan) = edge_plan else {
                continue;
            };

            // require at least one redundant predecessor
            if !edge_plan.iter().any(|plan| plan.already_stored) {
                continue;
            }

            // insert stores on missing edges
            for plan in &edge_plan {
                if plan.already_stored {
                    continue;
                }

                // select or create the insertion block
                let insertion_block = ensure_edge_block(
                    plan.predecessor,
                    candidate.block,
                    function,
                    tree,
                    &cfg,
                    &mut edge_blocks,
                    EdgeSplitPolicy::PredecessorMultiSuccessor,
                    &mut changed,
                );

                // insert a new store at the edge block
                let store_id = insert_store_for_plan(tree, insertion_block, plan, candidate.kind);
                clone_store_metadata(tree, candidate.instruction, store_id, plan.pointer);
            }

            // record removal of the original store
            to_remove.insert(candidate.instruction);
        }
    }

    // return early when nothing changes
    if to_remove.is_empty() && !changed {
        return false;
    }

    // remove original stores
    for &block_id in &function.blocks {
        let block = tree.get_mut(block_id);
        block.instructions.retain(|id| !to_remove.contains(id));
    }

    // drop memory metadata for removed stores
    for instruction_id in &to_remove {
        tree.metadata.memory.remove_memory_accesses(*instruction_id);
    }

    true
}

/// Return MemorySSA data when a store is eligible for PRE.
#[allow(clippy::too_many_arguments)]
fn store_access_info(
    block_id: mir::LocalNodeId<mir::Block>,
    instruction_id: mir::LocalNodeId<mir::Instruction>,
    pointer: Option<mir::Value>,
    local: Option<mir::LocalNodeId<mir::Local>>,
    value: mir::Value,
    kind: StoreKind,
    tree: &mir::Tree,
    memory_ssa: &MemorySSA,
) -> Option<StoreCandidate> {
    // resolve the memory ssa def access
    let access_id = memory_ssa.access_for_instruction(instruction_id)?;
    let MemoryAccess::Def(def_access) = memory_ssa.access(access_id) else {
        return None;
    };

    // require a write effect
    if !def_access.effect.writes {
        return None;
    }

    // skip ordered stores
    if instruction_has_atomic_ordering(tree, instruction_id) {
        return None;
    }

    // require a trackable effect
    if !effect_is_trackable(&def_access.effect) {
        return None;
    }

    // ensure the effect location matches the store kind
    match kind {
        StoreKind::Store => {
            if !matches!(def_access.effect.location, MemoryAccessLocation::Pointer(_)) {
                return None;
            }
        }
        StoreKind::LocalSet => {
            if !matches!(def_access.effect.location, MemoryAccessLocation::Local(_)) {
                return None;
            }
        }
    }

    // require a pointer for pointer stores
    if matches!(kind, StoreKind::Store) && pointer.is_none() {
        return None;
    }

    // require a local for local stores
    if matches!(kind, StoreKind::LocalSet) && local.is_none() {
        return None;
    }

    // require a memory phi at the block entry
    let phi_access = memory_ssa.phi_for_block(block_id)?;
    if memory_ssa.defining_access(access_id) != Some(phi_access) {
        return None;
    }

    // ensure the store can move to the block entry
    let block = tree.get(block_id);
    if !store_can_move_to_entry(instruction_id, block, tree, memory_ssa) {
        return None;
    }

    Some(StoreCandidate {
        block: block_id,
        instruction: instruction_id,
        pointer,
        local,
        value,
        effect: def_access.effect.clone(),
        kind,
    })
}

/// Return true when a store can be moved to the block entry.
fn store_can_move_to_entry(
    store_id: mir::LocalNodeId<mir::Instruction>,
    block: &mir::Block,
    tree: &mir::Tree,
    memory_ssa: &MemorySSA,
) -> bool {
    // inspect instructions before the store
    for &instruction_id in &block.instructions {
        // stop once the store is reached
        if instruction_id == store_id {
            break;
        }

        // read the instruction data
        let instruction = tree.get(instruction_id);

        // require speculatable instructions before the store
        if !instruction_is_speculatable(instruction, tree) {
            return false;
        }

        // reject memory reads before the store
        if instruction_is_read_only_access(instruction_id, memory_ssa) {
            return false;
        }
    }

    true
}

/// Collect edge insertions for each predecessor of the store block.
#[allow(clippy::too_many_arguments)]
fn collect_edge_insertions(
    store: &StoreCandidate,
    cfg: &ControlFlowGraph,
    domtree: &DominatorTree,
    tree: &mir::Tree,
    def_blocks: &HashMap<mir::Value, mir::LocalNodeId<mir::Block>>,
    function_params: &HashSet<mir::Value>,
    param_indices: &HashMap<mir::Value, usize>,
    memory_ssa: &MemorySSA,
    alias: &AliasAnalysis,
    equivalence: &mut ValueEquivalence<'_>,
) -> Option<Vec<EdgeStorePlan>> {
    // collect predecessor edge insertions
    let mut insertions = Vec::new();
    let predecessors = cfg.predecessors(store.block);

    // require at least one predecessor
    if predecessors.is_empty() {
        return None;
    }

    // resolve incoming memory accesses for the store block
    let MemoryAccess::Phi(phi) = memory_ssa.access(memory_ssa.phi_for_block(store.block)?) else {
        return None;
    };
    let incoming_by_pred: HashMap<_, _> = phi
        .incoming
        .iter()
        .map(|(block, access)| (*block, *access))
        .collect();

    // scan predecessors for insertion opportunities
    for &predecessor in predecessors {
        // resolve the pointer and value for this edge
        let predecessor_block = tree.get(predecessor);
        let pointer = match store.pointer {
            Some(ptr) => Some(resolve_edge_value(
                ptr,
                store.block,
                predecessor_block,
                tree,
                param_indices,
            )?),
            None => None,
        };
        let value = resolve_edge_value(
            store.value,
            store.block,
            predecessor_block,
            tree,
            param_indices,
        )?;

        // ensure the pointer value is available on this edge
        if let Some(ptr) = pointer
            && !value_available_in_block(ptr, predecessor, def_blocks, function_params, domtree)
        {
            return None;
        }

        // ensure the stored value is available on this edge
        if !value_available_in_block(value, predecessor, def_blocks, function_params, domtree) {
            return None;
        }

        // read the incoming memory access for this predecessor
        let incoming_access = incoming_by_pred.get(&predecessor).copied()?;

        // detect equivalent stores on this edge
        let already_stored = incoming_def_matches(
            store,
            incoming_access,
            pointer,
            value,
            memory_ssa,
            alias,
            tree,
            equivalence,
        );

        // record the insertion decision
        insertions.push(EdgeStorePlan {
            predecessor,
            pointer,
            local: store.local,
            value,
            already_stored,
        });
    }

    Some(insertions)
}

/// Return true when an incoming def is an equivalent store.
#[allow(clippy::too_many_arguments)]
fn incoming_def_matches(
    store: &StoreCandidate,
    incoming_access: MemoryAccessId,
    pointer: Option<mir::Value>,
    value: mir::Value,
    memory_ssa: &MemorySSA,
    alias: &AliasAnalysis,
    tree: &mir::Tree,
    equivalence: &mut ValueEquivalence<'_>,
) -> bool {
    // require a memory def on the edge
    let MemoryAccess::Def(def_access) = memory_ssa.access(incoming_access) else {
        return false;
    };

    // skip untrackable defs
    if !effect_is_trackable(&def_access.effect) {
        return false;
    }

    // require a write effect
    if !def_access.effect.writes {
        return false;
    }

    // require the same location metadata
    if !effects_match_location(alias, &store.effect, &def_access.effect) {
        return false;
    }

    // require the instruction to match the store kind
    let def_instruction = def_access.instruction;
    if instruction_has_atomic_ordering(tree, def_instruction) {
        return false;
    }
    let (def_kind, def_pointer, def_local, def_value) = match tree.get(def_instruction) {
        mir::Instruction::Store { pointer, value } => {
            let Some(pointer) = pointer.value() else {
                return false;
            };
            let Some(value) = value.value() else {
                return false;
            };

            (StoreKind::Store, Some(pointer), None, value)
        }
        mir::Instruction::LocalSet { local, value } => {
            let Some(local) = local.local() else {
                return false;
            };
            let Some(value) = value.value() else {
                return false;
            };

            (StoreKind::LocalSet, None, Some(local), value)
        }
        _ => return false,
    };

    // require matching store kind and local target
    if store.kind != def_kind || store.local != def_local {
        return false;
    }

    // ensure pointer values match when applicable
    if let (Some(expected), Some(actual)) = (pointer, def_pointer) {
        if !equivalence.equivalent(expected, actual) {
            return false;
        }
    } else if pointer.is_some() || def_pointer.is_some() {
        return false;
    }

    // ensure stored values match
    equivalence.equivalent(value, def_value)
}

/// Insert a store instruction for the plan.
fn insert_store_for_plan(
    tree: &mut mir::Tree,
    block_id: mir::LocalNodeId<mir::Block>,
    plan: &EdgeStorePlan,
    kind: StoreKind,
) -> mir::LocalNodeId<mir::Instruction> {
    // build the new store instruction
    let instruction = match kind {
        StoreKind::Store => mir::Instruction::Store {
            pointer: plan.pointer.expect("store pointer required").into(),
            value: plan.value.into(),
        },
        StoreKind::LocalSet => mir::Instruction::LocalSet {
            local: plan.local.expect("local target required").into(),
            value: plan.value.into(),
        },
    };

    // insert the instruction in the edge block
    let instruction_id = tree.insert(instruction);
    let mut block = tree.get(block_id).clone();
    block.instructions.push(instruction_id);
    tree.replace(block_id, block);
    instruction_id
}

/// Clone store metadata to a new instruction.
fn clone_store_metadata(
    tree: &mut mir::Tree,
    source: mir::LocalNodeId<mir::Instruction>,
    destination: mir::LocalNodeId<mir::Instruction>,
    pointer: Option<mir::Value>,
) {
    // skip when there is no metadata to clone
    let Some(accesses) = tree.metadata.memory.memory_accesses(source) else {
        return;
    };

    // update pointer targets for cloned metadata
    let mut cloned = Vec::with_capacity(accesses.len());
    for access in accesses {
        let mut updated = access.clone();
        if let (Some(pointer), mir::MemoryAccessTarget::Pointer(_)) = (pointer, updated.target) {
            updated.target = mir::MemoryAccessTarget::Pointer(pointer);
        }
        cloned.push(updated);
    }

    tree.metadata
        .memory
        .insert_memory_accesses(destination, cloned);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Store PRE inserts edge stores for a join.
    #[test]
    fn test_store_pre_inserts_edge_store() {
        let input = r#"
function test(v0: boolean, v1: int32): void {
b0(v0: boolean, v1: int32):
    v2: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    branch v0, b1, b2
b1:
    store v2, v1
    jump b3
b2:
    jump b3
b3:
    store v2, v1
    return
}"#;

        let expected = r#"
function test(v0: boolean, v1: int32): void {
b0(v0: boolean, v1: int32):
    v2: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    branch v0, b1, b2
b1:
    store v2, v1
    jump b3
b2:
    store v2, v1
    jump b3
b3:
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&StorePre);
        test.assert_output(expected);
    }

    /// Stores are not moved when no predecessor already stores.
    #[test]
    fn test_store_pre_requires_existing_store() {
        let input = r#"
function test(v0: boolean, v1: int32): void {
b0(v0: boolean, v1: int32):
    v2: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    branch v0, b1, b2
b1:
    jump b3
b2:
    jump b3
b3:
    store v2, v1
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&StorePre);
        test.assert_output(input);
    }

    /// Stores are not moved when values are defined in the join block.
    #[test]
    fn test_store_pre_skips_unavailable_values() {
        let input = r#"
function test(v0: boolean): void {
b0(v0: boolean):
    branch v0, b1, b2
b1:
    jump b3
b2:
    jump b3
b3:
    v1: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    v2: int32 = 1int32
    store v1, v2
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&StorePre);
        test.assert_output(input);
    }

    /// Stores are not moved when earlier instructions are not speculatable.
    #[test]
    fn test_store_pre_skips_non_speculatable_prefix() {
        let input = r#"
function test(v0: boolean, v1: int32): void {
b0(v0: boolean, v1: int32):
    v2: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    branch v0, b1, b2
b1:
    jump b3
b2:
    jump b3
b3:
    v3: int32 = load v2
    store v2, v1
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&StorePre);
        test.assert_output(input);
    }
}
