use destack_core::{FxIndexMap, FxIndexSet};

use crate::optimize::declare_pass;
use destack_mir as mir;

use crate::optimize::{FunctionPass, MirOptimized, PipelineContext};
use destack_mir::{
    AliasTable, ControlTable, DefinitionTable, DominatorTable, EdgeSplitPolicy, MemoryAccessEffect,
    MemoryAccessId, MemoryNode, MemoryRegion, MemoryTable, Mutation, ValueEquivalence,
    ensure_edge_block, instruction_is_read_only_access, instruction_is_speculatable,
    resolve_edge_value, value_available_in_block,
};

declare_pass! {
    /// Eliminate partially redundant stores by sinking them to predecessor edges.
    ///
    /// ```mir
    /// function before(v0: boolean, v1: int32): void {
    ///     local l0: int32
    /// b0(v0: boolean, v1: int32):
    ///     v2: ref<int32, borrowed, mutable, frame> = local.address l0
    ///     branch v0 => b1 | b2
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
    ///     local l0: int32
    /// b0(v0: boolean, v1: int32):
    ///     v2: ref<int32, borrowed, mutable, frame> = local.address l0
    ///     branch v0 => b1 | b2
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
    #[pass(id = "eliminate-partial-redundant-stores")]
    pub EliminatePartialRedundantStores,
    "Eliminate partially redundant stores"
}

impl FunctionPass for EliminatePartialRedundantStores {
    /// Run store PRE on the function.
    fn run(
        &self,
        function: &mut mir::Function,
        optimized: &mut MirOptimized,
        ctx: &PipelineContext<'_>,
        analyses: &mut mir::FunctionCache,
    ) -> Mutation {
        let tree = &mut optimized.tree;
        let accesses = &mut optimized.accesses;
        let effects = &optimized.effects;

        // skip imported functions
        if function.entry().is_none() {
            return Mutation::NONE;
        }

        // run store PRE
        let changed = run_eliminate_partial_redundant_stores(
            function, tree, accesses, effects, ctx, analyses,
        );

        // report what this pass changed
        if changed {
            Mutation::CONTROL | Mutation::VALUE
        } else {
            Mutation::NONE
        }
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
fn run_eliminate_partial_redundant_stores(
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    accesses: &mut mir::AccessTable,
    effects: &mir::EffectTable,
    _ctx: &PipelineContext<'_>,
    analyses: &mut mir::FunctionCache,
) -> bool {
    // gather analyses
    let cfg = analyses.control(function, tree).clone();
    let domtree = analyses.dominator(function, tree).clone();
    let memory = analyses.memory(function, tree, accesses, effects);
    let alias = analyses.alias(function, tree).clone();
    let constants = analyses.constant(function, tree);

    // snapshot value definitions
    let definitions = DefinitionTable::build(function, tree);

    // track modifications
    let mut edge_blocks: FxIndexMap<
        (mir::LocalNodeId<mir::Block>, mir::LocalNodeId<mir::Block>),
        mir::LocalNodeId<mir::Block>,
    > = FxIndexMap::default();
    let mut to_remove = FxIndexSet::default();
    let mut changed = false;

    // scan each block for eligible stores
    let block_ids = function.blocks().to_vec();
    for block_id in block_ids {
        // collect block parameters for edge resolution
        let block = tree.get(block_id).clone();
        let param_indices = block
            .parameters
            .iter()
            .enumerate()
            .map(|(index, param)| (param.value, index))
            .collect::<FxIndexMap<_, _>>();

        // scan block instructions for store candidates
        for &instruction_id in &block.instructions {
            // select store instructions
            let (kind, pointer, local, value) = match tree.get(instruction_id) {
                mir::Instruction::Store { pointer, value } => {
                    let pointer = *pointer;
                    let value = *value;

                    (StoreKind::Store, Some(pointer), None, value)
                }
                mir::Instruction::LocalSet { local, value } => {
                    let value = *value;

                    (StoreKind::LocalSet, None, Some(*local), value)
                }
                _ => continue,
            };

            // validate store eligibility
            let Some(candidate) = store_access(
                block_id,
                instruction_id,
                pointer,
                local,
                value,
                kind,
                function,
                tree,
                accesses,
                memory.as_ref(),
            ) else {
                continue;
            };

            // build edge insertions for each predecessor
            let edge_plan = {
                let mut equivalence =
                    ValueEquivalence::new(function, tree, &definitions, constants.as_ref());

                collect_edge_insertions(
                    &candidate,
                    &cfg,
                    &domtree,
                    tree,
                    &definitions,
                    &param_indices,
                    accesses,
                    memory.as_ref(),
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
                let store_id =
                    insert_store_for_plan(function, tree, insertion_block, plan, candidate.kind);
                clone_store_metadata(accesses, candidate.instruction, store_id, plan.pointer);
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
    for block_id in function.blocks().to_vec() {
        let mut instructions = tree.get(block_id).instructions.clone();
        instructions.retain(|id| !to_remove.contains(id));
        function.replace_block_instructions(block_id, instructions, tree);
    }

    // drop memory tables for removed stores
    for instruction_id in &to_remove {
        accesses.remove(*instruction_id);
    }

    true
}

/// Return MemoryTable data when a store is eligible for PRE.
fn store_access(
    block_id: mir::LocalNodeId<mir::Block>,
    instruction_id: mir::LocalNodeId<mir::Instruction>,
    pointer: Option<mir::Value>,
    local: Option<mir::LocalNodeId<mir::Local>>,
    value: mir::Value,
    kind: StoreKind,
    function: &mir::Function,
    tree: &mir::Tree,
    accesses: &mir::AccessTable,
    memory: &MemoryTable,
) -> Option<StoreCandidate> {
    // resolve the memory ssa def access
    let access_id = memory.instruction_access(instruction_id)?;
    let MemoryNode::Def(def_access) = memory.access(access_id) else {
        return None;
    };

    // require a write effect
    if !def_access.effect.writes {
        return None;
    }

    // skip ordered stores
    if accesses.is_ordered(instruction_id, tree) {
        return None;
    }

    // require a trackable effect
    if !def_access.effect.is_trackable() {
        return None;
    }

    // ensure the effect location matches the store kind
    match kind {
        StoreKind::Store => {
            if !matches!(def_access.effect.region, MemoryRegion::Address { .. }) {
                return None;
            }
        }
        StoreKind::LocalSet => {
            if !matches!(def_access.effect.region, MemoryRegion::Local(_)) {
                return None;
            }
        }
    }

    // require a reference for reference stores
    if matches!(kind, StoreKind::Store) && pointer.is_none() {
        return None;
    }

    // require a local for local stores
    if matches!(kind, StoreKind::LocalSet) && local.is_none() {
        return None;
    }

    // require a memory phi at the block entry
    let phi_access = memory.block_phi(block_id)?;
    if memory.defining_access(access_id) != Some(phi_access) {
        return None;
    }

    // ensure the store can move to the block entry
    let block = tree.get(block_id);
    if !store_can_move_to_entry(instruction_id, block, function, tree, memory) {
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
    function: &mir::Function,
    tree: &mir::Tree,
    memory: &MemoryTable,
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
        if !instruction_is_speculatable(instruction, function, tree) {
            return false;
        }

        // reject memory reads before the store
        if instruction_is_read_only_access(instruction_id, memory) {
            return false;
        }
    }

    true
}

/// Collect edge insertions for each predecessor of the store block.
fn collect_edge_insertions(
    store: &StoreCandidate,
    cfg: &ControlTable,
    domtree: &DominatorTable,
    tree: &mir::Tree,
    definitions: &DefinitionTable,
    param_indices: &FxIndexMap<mir::Value, usize>,
    accesses: &mir::AccessTable,
    memory: &MemoryTable,
    alias: &AliasTable,
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
    let MemoryNode::Phi(phi) = memory.access(memory.block_phi(store.block)?) else {
        return None;
    };
    let incoming_by_predecessor: FxIndexMap<_, _> = phi
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

        // ensure the reference value is available on this edge
        if let Some(ptr) = pointer
            && !value_available_in_block(ptr, predecessor, definitions, domtree)
        {
            return None;
        }

        // ensure the stored value is available on this edge
        if !value_available_in_block(value, predecessor, definitions, domtree) {
            return None;
        }

        // read the incoming memory access for this predecessor
        let incoming_access = incoming_by_predecessor.get(&predecessor).copied()?;

        // detect equivalent stores on this edge
        let already_stored = incoming_def_matches(
            store,
            incoming_access,
            pointer,
            value,
            accesses,
            memory,
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
fn incoming_def_matches(
    store: &StoreCandidate,
    incoming_access: MemoryAccessId,
    pointer: Option<mir::Value>,
    value: mir::Value,
    accesses: &mir::AccessTable,
    memory: &MemoryTable,
    alias: &AliasTable,
    tree: &mir::Tree,
    equivalence: &mut ValueEquivalence<'_>,
) -> bool {
    // require a memory def on the edge
    let MemoryNode::Def(def_access) = memory.access(incoming_access) else {
        return false;
    };

    // skip untrackable defs
    if !def_access.effect.is_trackable() {
        return false;
    }

    // require a write effect
    if !def_access.effect.writes {
        return false;
    }

    // require the same region tables
    if !store.effect.matches_region(alias, &def_access.effect) {
        return false;
    }

    // require an instruction backed definition
    let Some(def_instruction) = def_access.instruction() else {
        return false;
    };

    // require the instruction to match the store kind
    if accesses.is_ordered(def_instruction, tree) {
        return false;
    }
    let (def_kind, def_pointer, def_local, def_value) = match tree.get(def_instruction) {
        mir::Instruction::Store { pointer, value } => {
            (StoreKind::Store, Some(*pointer), None, *value)
        }
        mir::Instruction::LocalSet { local, value } => {
            let local = *local;
            let value = *value;

            (StoreKind::LocalSet, None, Some(local), value)
        }
        _ => return false,
    };

    // require matching store kind and local target
    if store.kind != def_kind || store.local != def_local {
        return false;
    }

    // ensure reference values match when applicable
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
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    block_id: mir::LocalNodeId<mir::Block>,
    plan: &EdgeStorePlan,
    kind: StoreKind,
) -> mir::LocalNodeId<mir::Instruction> {
    // build the new store instruction
    let instruction = match kind {
        StoreKind::Store => mir::Instruction::Store {
            pointer: plan.pointer.expect("store pointer required"),
            value: plan.value,
        },
        StoreKind::LocalSet => mir::Instruction::LocalSet {
            local: plan.local.expect("local target required"),
            value: plan.value,
        },
    };

    // insert the instruction in the edge block
    let instruction_id = tree.insert(instruction);
    let mut instructions = tree.get(block_id).instructions.clone();
    instructions.push(instruction_id);
    function.replace_block_instructions(block_id, instructions, tree);
    instruction_id
}

/// Clone store tables to a new instruction.
fn clone_store_metadata(
    accesses: &mut mir::AccessTable,
    source: mir::LocalNodeId<mir::Instruction>,
    destination: mir::LocalNodeId<mir::Instruction>,
    pointer: Option<mir::Value>,
) {
    // skip when there is no tables to clone
    let Some(entries) = accesses.get(source) else {
        return;
    };

    // update reference targets for cloned tables
    let mut cloned = Vec::with_capacity(entries.len());
    for access in entries {
        let mut updated = access.clone();
        if let (Some(pointer), mir::MemoryTarget::Address(_)) = (pointer, updated.target) {
            updated.target = mir::MemoryTarget::Address(pointer);
        }
        cloned.push(updated);
    }

    accesses.insert(destination, cloned);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Store PRE inserts edge stores for a join.
    #[test]
    fn test_eliminate_partial_redundant_stores_inserts_edge_store() {
        let input = r#"
function test(v0: boolean, v1: int32): void {
    local l0: int32
entry(v0: boolean, v1: int32):
    v2: ref<int32, borrowed, mutable, frame> = local.address l0
    branch v0 => b1 | b2

b1:
    store v2, v1
    jump b3

b2:
    jump b3

b3:
    store v2, v1
    return
}
"#;

        let expected = r#"
function test(v0: boolean, v1: int32): void {
    local l0: int32
entry(v0: boolean, v1: int32):
    v2: ref<int32, borrowed, mutable, frame> = local.address l0
    branch v0 => b1 | b2

b1:
    store v2, v1
    jump b3

b2:
    store v2, v1
    jump b3

b3:
    return
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminatePartialRedundantStores);
        test.assert_output(expected);
    }

    /// Stores are not moved when no predecessor already stores.
    #[test]
    fn test_eliminate_partial_redundant_stores_requires_existing_store() {
        let input = r#"
function test(v0: boolean, v1: int32): void {
    local l0: int32
entry(v0: boolean, v1: int32):
    v2: ref<int32, borrowed, mutable, frame> = local.address l0
    branch v0 => b1 | b2

b1:
    jump b3

b2:
    jump b3

b3:
    store v2, v1
    return
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminatePartialRedundantStores);
        test.assert_output(input);
    }

    /// Stores are not moved when values are defined in the join block.
    #[test]
    fn test_eliminate_partial_redundant_stores_skips_unavailable_values() {
        let input = r#"
function test(v0: boolean): void {
    local l0: int32
entry(v0: boolean):
    branch v0 => b1 | b2

b1:
    jump b3

b2:
    jump b3

b3:
    v1: ref<int32, borrowed, mutable, frame> = local.address l0
    v2: int32 = 1
    store v1, v2
    return
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminatePartialRedundantStores);
        test.assert_output(input);
    }

    /// Stores are not moved when earlier instructions are not speculatable.
    #[test]
    fn test_eliminate_partial_redundant_stores_skips_non_speculatable_prefix() {
        let input = r#"
function test(v0: boolean, v1: int32): void {
    local l0: int32
entry(v0: boolean, v1: int32):
    v2: ref<int32, borrowed, mutable, frame> = local.address l0
    branch v0 => b1 | b2

b1:
    jump b3

b2:
    jump b3

b3:
    v3: int32 = load v2
    store v2, v1
    return
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminatePartialRedundantStores);
        test.assert_output(input);
    }
}
