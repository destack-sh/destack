use std::collections::{HashMap, HashSet};

use crate::optimize::declare_pass;
use destack_mir as mir;

use crate::optimize::{FunctionPass, MirOptimized, PipelineContext};
use destack_mir::{
    AliasTable, DominatorTable, MemoryAccessEffect, MemoryAccessId, MemoryNode, MemoryRegion,
    MemoryTable, Mutation, TargetLayout, apply_substitutions_in_function,
    resolve_substitution_chains,
};

declare_pass! {
    /// Forward stored values to subsequent loads.
    ///
    /// ```mir
    /// function before(): int32 {
    ///     local l0: int32
    /// b0:
    ///     v0: ref<int32, borrowed, mutable, frame> = local.address l0
    ///     v1: int32 = 42
    ///     store v0, v1
    ///     v2: int32 = load v0       // forwarded from store
    ///     v3: int32 = load v0       // forwarded from store load to load
    ///     v4: int32 = add v2, v3
    ///     return v4
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function after(): int32 {
    ///     local l0: int32
    /// b0:
    ///     v0: ref<int32, borrowed, mutable, frame> = local.address l0
    ///     v1: int32 = 42
    ///     store v0, v1
    ///     v4: int32 = add v1, v1
    ///     return v4
    /// }
    /// ```
    #[pass(id = "forward-stored-values")]
    pub ForwardStoredValues,
    "Forward stored values to subsequent loads"
}

/// An available value tied to a MemoryTable clobber.
#[derive(Clone)]
struct MemoryEntry {
    /// The clobbering access id for the memory state.
    clobber: MemoryAccessId,
    /// The accessed memory location.
    region: MemoryRegion,
    /// The available value.
    value: mir::Value,
}

impl FunctionPass for ForwardStoredValues {
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

        // skip empty functions
        let entry = match function.entry() {
            Some(entry) => entry,
            None => return Mutation::NONE,
        };

        // get analyses
        let (aa, memory, dom_children) = {
            let domtree = analyses.dominator(function, tree);
            let aa = analyses.alias(function, tree).clone();
            let memory = analyses.memory(function, tree, accesses, effects);
            let dom_children = build_dominator_children(function, &domtree);
            (aa, memory, dom_children)
        };

        // run load store forwarding
        let changed = run_forward_stored_values(
            entry,
            function,
            tree,
            accesses,
            &aa,
            memory.as_ref(),
            &dom_children,
            ctx.target_layout(),
        );

        // report what this pass changed
        if changed {
            Mutation::VALUE
        } else {
            Mutation::NONE
        }
    }
}

/// Core load store forwarding logic. Returns true if changes were made.
fn run_forward_stored_values(
    entry: mir::LocalNodeId<mir::Block>,
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    accesses: &mut mir::AccessTable,
    aa: &AliasTable,
    memory: &MemoryTable,
    dom_children: &HashMap<mir::LocalNodeId<mir::Block>, Vec<mir::LocalNodeId<mir::Block>>>,
    target_layout: TargetLayout,
) -> bool {
    // run forwarding using dominator tree traversal
    let (substitutions, to_remove) = find_forwardable_loads(
        entry,
        function,
        tree,
        aa,
        memory,
        dom_children,
        target_layout,
    );

    // nothing to do if no forwarding found
    if substitutions.is_empty() {
        return false;
    }

    // resolve transitive substitution chains
    let substitutions = resolve_substitution_chains(substitutions);

    // apply substitutions and remove forwarded loads
    apply_substitutions_in_function(function, tree, accesses, &substitutions, Some(&to_remove));

    true
}

/// Build a map from each block to its children in the dominator tree.
fn build_dominator_children(
    function: &mir::Function,
    domtree: &DominatorTable,
) -> HashMap<mir::LocalNodeId<mir::Block>, Vec<mir::LocalNodeId<mir::Block>>> {
    // prepare the child mapping
    let mut children: HashMap<_, Vec<_>> = HashMap::new();

    // initialize all blocks with empty children lists
    for &block_id in function.blocks() {
        children.insert(block_id, Vec::new());
    }

    // build parent to children mapping from idom relationships
    for &block_id in function.blocks() {
        if let Some(idom) = domtree.immediate_dominator(block_id) {
            children.get_mut(&idom).unwrap().push(block_id);
        }
    }

    children
}

/// Scoped table of available memory values.
///
/// Tracks values by MemoryTable clobbering access.
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
        aa: &AliasTable,
    ) -> Option<mir::Value> {
        // skip untrackable effects
        if !use_effect.is_trackable() {
            return None;
        }

        let region = &use_effect.region;

        // search from innermost to outermost scope
        for scope in self.scopes.iter().rev() {
            // scan entries from newest to oldest
            for entry in scope.iter().rev() {
                // skip entries with a different clobber
                if entry.clobber != clobber {
                    continue;
                }

                if !entry.region.spaces().may_alias(use_effect.region.spaces()) {
                    continue;
                }

                // compare matching locations
                match (&entry.region, region) {
                    (MemoryRegion::Local(a), MemoryRegion::Local(b)) => {
                        if a == b {
                            return Some(entry.value);
                        }
                    }
                    (
                        MemoryRegion::Address { location: a, .. },
                        MemoryRegion::Address { location: b, .. },
                    ) => {
                        if a.address == b.address {
                            if a.is_compatible_with(b) {
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
                            if a.is_compatible_with(b) {
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
    function: &mir::Function,
    tree: &mir::Tree,
    aa: &AliasTable,
    memory: &MemoryTable,
    dom_children: &HashMap<mir::LocalNodeId<mir::Block>, Vec<mir::LocalNodeId<mir::Block>>>,
    target_layout: TargetLayout,
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
                    function,
                    tree,
                    aa,
                    memory,
                    target_layout,
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
fn process_block(
    block_id: mir::LocalNodeId<mir::Block>,
    function: &mir::Function,
    tree: &mir::Tree,
    aa: &AliasTable,
    memory: &MemoryTable,
    _target_layout: TargetLayout,
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
                let value = *value;

                // resolve the memory def access
                let Some(def_access_id) = def_access_id(memory, instruction_id) else {
                    continue;
                };

                // read the def access data
                let MemoryNode::Def(def_access) = memory.access(def_access_id) else {
                    continue;
                };

                // skip untrackable accesses
                if !def_access.effect.is_trackable() {
                    continue;
                }

                // record the available value
                available.insert(MemoryEntry {
                    clobber: def_access_id,
                    region: def_access.effect.region.clone(),
                    value,
                });
            }

            mir::Instruction::Load { destination, .. } => {
                let destination = *destination;

                // resolve the memory use access
                let Some(use_access_id) = use_access_id(memory, instruction_id) else {
                    continue;
                };

                // read the use access data
                let MemoryNode::Use(use_access) = memory.access(use_access_id) else {
                    continue;
                };

                // skip untrackable reads
                if !use_access.effect.is_trackable() {
                    continue;
                }

                // compute the clobbering access for this read
                let clobber = memory.clobbering_use(use_access_id, aa);
                let Some(clobber) = resolve_trivial_clobber(memory, clobber) else {
                    continue;
                };

                // forward from an existing value when possible
                if let Some(existing) = available.get(clobber, &use_access.effect, aa)
                    && function.can_substitute(destination, existing)
                {
                    substitutions.insert(destination, existing);
                    to_remove.insert(instruction_id);
                } else {
                    available.insert(MemoryEntry {
                        clobber,
                        region: use_access.effect.region.clone(),
                        value: destination,
                    });
                }
            }

            mir::Instruction::LocalGet { destination, .. } => {
                let destination = *destination;

                // resolve the memory use access
                let Some(use_access_id) = use_access_id(memory, instruction_id) else {
                    continue;
                };

                // read the use access data
                let MemoryNode::Use(use_access) = memory.access(use_access_id) else {
                    continue;
                };

                // skip untrackable reads
                if !use_access.effect.is_trackable() {
                    continue;
                }

                // compute the clobbering access for this read
                let clobber = memory.clobbering_use(use_access_id, aa);
                let Some(clobber) = resolve_trivial_clobber(memory, clobber) else {
                    continue;
                };

                // forward from an existing value when possible
                if let Some(existing) = available.get(clobber, &use_access.effect, aa)
                    && function.can_substitute(destination, existing)
                {
                    substitutions.insert(destination, existing);
                    to_remove.insert(instruction_id);
                } else {
                    available.insert(MemoryEntry {
                        clobber,
                        region: use_access.effect.region.clone(),
                        value: destination,
                    });
                }
            }

            mir::Instruction::Intrinsic { .. } => {}
            mir::Instruction::AtomicLoad { .. }
            | mir::Instruction::AtomicStore { .. }
            | mir::Instruction::AtomicCompareExchange { .. }
            | mir::Instruction::AtomicRmw { .. }
            | mir::Instruction::AtomicFence { .. }
            | mir::Instruction::BarrierWrite { .. } => {
                available.clear();
            }

            mir::Instruction::Pin { .. }
            | mir::Instruction::Unpin { .. }
            | mir::Instruction::Free { .. } => {
                // clear across storage release boundaries
                available.clear();
            }

            _ => {}
        }
    }
}

fn def_access_id(
    memory: &MemoryTable,
    instruction_id: mir::LocalNodeId<mir::Instruction>,
) -> Option<MemoryAccessId> {
    // find the first def access for the instruction
    let accesses = memory.instruction_accesses(instruction_id)?;
    for &access_id in accesses {
        if matches!(memory.access(access_id), MemoryNode::Def(_)) {
            return Some(access_id);
        }
    }

    None
}

fn use_access_id(
    memory: &MemoryTable,
    instruction_id: mir::LocalNodeId<mir::Instruction>,
) -> Option<MemoryAccessId> {
    // find the first use access for the instruction
    let accesses = memory.instruction_accesses(instruction_id)?;
    for &access_id in accesses {
        if matches!(memory.access(access_id), MemoryNode::Use(_)) {
            return Some(access_id);
        }
    }

    None
}

/// Resolve trivial memory phi nodes to a single clobbering access.
fn resolve_trivial_clobber(memory: &MemoryTable, access: MemoryAccessId) -> Option<MemoryAccessId> {
    let mut current = access;
    let mut visited = HashSet::new();

    loop {
        if !visited.insert(current) {
            return None;
        }

        let MemoryNode::Phi(phi) = memory.access(current) else {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Store then load from same reference forwards the stored value.
    #[test]
    fn test_forward_simple_store_load() {
        let input = r#"
function test(): int32 {
    local l0: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    v1: int32 = 42
    store v0, v1
    v2: int32 = load v0
    return v2
}
"#;
        let expected = r#"
function test(): int32 {
    local l0: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    v1: int32 = 42
    store v0, v1
    return v1
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&ForwardStoredValues);
        test.assert_output(expected);
    }

    /// Store to one pointer, load from different pointer: no forwarding.
    #[test]
    fn test_no_forward_different_pointers() {
        let input = r#"
function test(): int32 {
    local l0: int32
    local l1: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    v1: ref<int32, borrowed, mutable, frame> = local.address l1
    v2: int32 = 42
    store v0, v2
    v3: int32 = load v1
    return v3
}
"#;
        let expected = input;

        let mut test = TestProgram::new(input);
        test.run_pass(&ForwardStoredValues);
        test.assert_output(expected);
    }

    /// Second store kills first, load forwards from second store.
    #[test]
    fn test_kill_on_clobbering_store() {
        let input = r#"
function test(): int32 {
    local l0: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    v1: int32 = 42
    v2: int32 = 100
    store v0, v1
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
    v1: int32 = 42
    v2: int32 = 100
    store v0, v1
    store v0, v2
    return v2
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&ForwardStoredValues);
        test.assert_output(expected);
    }

    /// Multiple loads from same reference all forward to stored value.
    #[test]
    fn test_forward_multiple_loads() {
        let input = r#"
function test(): int32 {
    local l0: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    v1: int32 = 42
    store v0, v1
    v2: int32 = load v0
    v3: int32 = load v0
    v4: int32 = add v2, v3
    return v4
}
"#;
        let expected = r#"
function test(): int32 {
    local l0: int32

entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    v1: int32 = 42
    store v0, v1
    v4: int32 = add v1, v1
    return v4
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&ForwardStoredValues);
        test.assert_output(expected);
    }

    /// Trivial memory phis forward through a merge.
    #[test]
    fn test_forward_through_trivial_memory_phi() {
        let input = r#"
function test(): int32 {
    local l0: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    v1: int32 = 7
    v2: boolean = true
    store v0, v1
    branch v2 => b1 | b2

b1:
    jump b3

b2:
    jump b3

b3:
    v3: int32 = load v0
    return v3
}
"#;
        let expected = r#"
function test(): int32 {
    local l0: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    v1: int32 = 7
    v2: boolean = true
    store v0, v1
    branch v2 => b1 | b2

b1:
    jump b3

b2:
    jump b3

b3:
    return v1
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&ForwardStoredValues);
        test.assert_output(expected);
    }

    /// Trivial memory phis with multiple incoming edges are forwarded.
    #[test]
    fn test_forward_through_triple_memory_phi() {
        let input = r#"
function test(): int32 {
    local l0: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    v1: int32 = 7
    v2: uint32 = 0
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
}
"#;
        let expected = r#"
function test(): int32 {
    local l0: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    v1: int32 = 7
    v2: uint32 = 0
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
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&ForwardStoredValues);
        test.assert_output(expected);
    }

    /// Store to non aliasing pointer does not kill available store.
    #[test]
    fn test_forward_through_non_aliasing_store() {
        let input = r#"
function test(): int32 {
    local l0: int32
    local l1: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    v1: ref<int32, borrowed, mutable, frame> = local.address l1
    v2: int32 = 42
    v3: int32 = 100
    store v0, v2
    store v1, v3
    v4: int32 = load v0
    return v4
}
"#;
        let expected = r#"
function test(): int32 {
    local l0: int32
    local l1: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    v1: ref<int32, borrowed, mutable, frame> = local.address l1
    v2: int32 = 42
    v3: int32 = 100
    store v0, v2
    store v1, v3
    return v2
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&ForwardStoredValues);
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
    local l0: Point
entry:
    v0: ref<Point, borrowed, mutable, frame> = local.address l0
    v1: ref<int32, borrowed, mutable> = field.address v0, 0
    v2: int32 = 42
    store v1, v2
    v3: int32 = load v1
    return v3
}
"#;
        let expected = r#"
type Point {
    int32;
    int32;
}

function test(): int32 {
    local l0: Point
entry:
    v0: ref<Point, borrowed, mutable, frame> = local.address l0
    v1: ref<int32, borrowed, mutable> = field.address v0, 0
    v2: int32 = 42
    store v1, v2
    return v2
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&ForwardStoredValues);
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
    local l0: Point
entry:
    v0: ref<Point, borrowed, mutable, frame> = local.address l0
    v1: ref<int32, borrowed, mutable> = field.address v0, 0
    v2: ref<int32, borrowed, mutable> = field.address v0, 1
    v3: int32 = 10
    v4: int32 = 20
    store v1, v3
    store v2, v4
    v5: int32 = load v1
    v6: int32 = load v2
    v7: int32 = add v5, v6
    return v7
}
"#;
        let expected = r#"
type Point {
    int32;
    int32;
}

function test(): int32 {
    local l0: Point

entry:
    v0: ref<Point, borrowed, mutable, frame> = local.address l0
    v1: ref<int32, borrowed, mutable> = field.address v0, 0
    v2: ref<int32, borrowed, mutable> = field.address v0, 1
    v3: int32 = 10
    v4: int32 = 20
    store v1, v3
    store v2, v4
    v7: int32 = add v3, v4
    return v7
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&ForwardStoredValues);
        test.assert_output(expected);
    }

    /// Load followed by another load from same region uses first result.
    #[test]
    fn test_load_to_load_forwarding() {
        let input = r#"
function test(v0: ref<int32, borrowed, mutable>): int32 {
entry(v0: ref<int32, borrowed, mutable>):
    v1: int32 = load v0
    v2: int32 = load v0
    v3: int32 = add v1, v2
    return v3
}
"#;
        let expected = r#"
function test(v0: ref<int32, borrowed, mutable>): int32 {
entry(v0: ref<int32, borrowed, mutable>):
    v1: int32 = load v0
    v3: int32 = add v1, v1
    return v3
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&ForwardStoredValues);
        test.assert_output(expected);
    }

    /// Store between loads kills the first load's availability.
    #[test]
    fn test_load_load_killed_by_store() {
        let input = r#"
function test(v0: ref<int32, borrowed, mutable>): int32 {
entry(v0: ref<int32, borrowed, mutable>):
    v1: int32 = load v0
    v2: int32 = 99
    store v0, v2
    v3: int32 = load v0
    v4: int32 = add v1, v3
    return v4
}
"#;
        let expected = r#"
function test(v0: ref<int32, borrowed, mutable>): int32 {
entry(v0: ref<int32, borrowed, mutable>):
    v1: int32 = load v0
    v2: int32 = 99
    store v0, v2
    v4: int32 = add v1, v2
    return v4
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&ForwardStoredValues);
        test.assert_output(expected);
    }

    /// Store in entry block forwards to dominated block.
    #[test]
    fn test_cross_block_forward_simple() {
        let input = r#"
function test(): int32 {
    local l0: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    v1: int32 = 42
    store v0, v1
    jump b1

b1:
    v2: int32 = load v0
    return v2
}
"#;
        let expected = r#"
function test(): int32 {
    local l0: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    v1: int32 = 42
    store v0, v1
    jump b1

b1:
    return v1
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&ForwardStoredValues);
        test.assert_output(expected);
    }

    /// Store in entry forwards to both branches of a diamond.
    #[test]
    fn test_cross_block_forward_diamond() {
        let input = r#"
function test(v0: boolean): int32 {
    local l0: int32
entry(v0: boolean):
    v1: ref<int32, borrowed, mutable, frame> = local.address l0
    v2: int32 = 42
    store v1, v2
    branch v0 => b1 | b2

b1:
    v3: int32 = load v1
    jump b3(v3)

b2:
    v4: int32 = load v1
    jump b3(v4)

b3(v5: int32):
    return v5
}
"#;
        let expected = r#"
function test(v0: boolean): int32 {
    local l0: int32

entry(v0: boolean):
    v1: ref<int32, borrowed, mutable, frame> = local.address l0
    v2: int32 = 42
    store v1, v2
    branch v0 => b1 | b2

b1:
    jump b3(v2)

b2:
    jump b3(v2)

b3(v5: int32):
    return v5
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&ForwardStoredValues);
        test.assert_output(expected);
    }

    /// Store in one branch does not forward to sibling branch.
    #[test]
    fn test_no_forward_across_non_dominating_blocks() {
        let input = r#"
function test(v0: boolean, v1: ref<int32, borrowed, mutable>): int32 {
entry(v0: boolean, v1: ref<int32, borrowed, mutable>):
    branch v0 => b1 | b2

b1:
    v2: int32 = 42
    store v1, v2
    jump b3

b2:
    v3: int32 = load v1
    jump b3

b3:
    v4: int32 = 0
    return v4
}
"#;
        let expected = input;

        let mut test = TestProgram::new(input);
        test.run_pass(&ForwardStoredValues);
        test.assert_output(expected);
    }

    /// Deep dominator chain: store in entry reaches deeply nested block.
    #[test]
    fn test_cross_block_deep_chain() {
        let input = r#"
function test(): int32 {
    local l0: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    v1: int32 = 42
    store v0, v1
    jump b1

b1:
    jump b2

b2:
    jump b3

b3:
    v2: int32 = load v0
    return v2
}
"#;
        let expected = r#"
function test(): int32 {
    local l0: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    v1: int32 = 42
    store v0, v1
    jump b1

b1:
    jump b2

b2:
    jump b3

b3:
    return v1
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&ForwardStoredValues);
        test.assert_output(expected);
    }

    /// Load in entry forwards to dominated blocks.
    #[test]
    fn test_cross_block_load_to_load() {
        let input = r#"
function test(v0: ref<int32, borrowed, mutable>): int32 {
entry(v0: ref<int32, borrowed, mutable>):
    v1: int32 = load v0
    jump b1

b1:
    v2: int32 = load v0
    v3: int32 = add v1, v2
    return v3
}
"#;
        let expected = r#"
function test(v0: ref<int32, borrowed, mutable>): int32 {
entry(v0: ref<int32, borrowed, mutable>):
    v1: int32 = load v0
    jump b1

b1:
    v3: int32 = add v1, v1
    return v3
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&ForwardStoredValues);
        test.assert_output(expected);
    }

    /// Call with pointer argument may clobber: no forwarding.
    #[test]
    fn test_no_forward_after_call() {
        let input = r#"
external function imported(ref<int32, borrowed, mutable>): void

function test(): int32 {
    local l0: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    v1: int32 = 42
    store v0, v1
    call imported(v0): (ref<int32, borrowed, mutable>) => void
    v2: int32 = load v0
    return v2
}
"#;
        let expected = input;

        let mut test = TestProgram::new(input);
        test.run_pass(&ForwardStoredValues);
        test.assert_output(expected);
    }

    /// No memory calls do not block forwarding.
    #[test]
    fn test_forward_across_no_memory_call() {
        let input = r#"
external function imported(ref<int32, borrowed, mutable>): void

function test(): int32 {
    local l0: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    v1: int32 = 42
    store v0, v1
    call imported(v0): (ref<int32, borrowed, mutable>) => void
    v2: int32 = load v0
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
    store v0, v1
    call imported(v0): (ref<int32, borrowed, mutable>) => void
    return v1
}
"#;

        let mut test = TestProgram::new(input);

        let function_id = test.entry_function_id();
        let (call_inst, _callee) = test.first_call_in_entry(function_id);

        let callsite = mir::CallSite::Instruction(call_inst);
        test.optimized.effects.upsert_call(callsite).memory = mir::MemoryEffect::none();

        test.run_pass(&ForwardStoredValues);
        test.assert_output(expected);
    }

    /// Call in dominator block kills forwarding to dominated blocks.
    #[test]
    fn test_call_kills_cross_block() {
        let input = r#"
external function imported(ref<int32, borrowed, mutable>): void

function test(): int32 {
    local l0: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    v1: int32 = 42
    store v0, v1
    call imported(v0): (ref<int32, borrowed, mutable>) => void
    jump b1

b1:
    v2: int32 = load v0
    return v2
}
"#;
        let expected = input;

        let mut test = TestProgram::new(input);
        test.run_pass(&ForwardStoredValues);
        test.assert_output(expected);
    }

    /// Volatile load only blocks forwarding for the accessed location.
    #[test]
    fn test_volatile_load_is_barrier() {
        let input = r#"
function test(): int32 {
    local l0: int32
    local l1: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    v1: ref<int32, borrowed, mutable, frame> = local.address l1
    v2: int32 = 42
    store v0, v2
    v3: int32 = load v1
    v4: int32 = load v0
    v5: int32 = add v3, v4
    return v5
}
"#;
        let expected = r#"
function test(): int32 {
    local l0: int32
    local l1: int32

entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    v1: ref<int32, borrowed, mutable, frame> = local.address l1
    v2: int32 = 42
    store v0, v2
    v3: int32 = load v1
    v5: int32 = add v3, v2
    return v5
}
"#;

        let mut test = TestProgram::new(input);
        let function_id = test.first_function_id();
        let function = test.optimized.tree.get(function_id);
        let block = test.optimized.tree.get(function.block(0));
        let volatile_load = block.instructions[2];
        test.insert_pointer_access_with_options(
            volatile_load,
            mir::MemoryOperation::Read,
            mir::Value::new(1),
            Some(4),
            true,
            None,
        );
        test.run_pass(&ForwardStoredValues);
        test.assert_output(expected);
    }

    /// Volatile store only blocks forwarding for the accessed location.
    #[test]
    fn test_volatile_store_is_barrier() {
        let input = r#"
function test(): int32 {
    local l0: int32
    local l1: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    v1: ref<int32, borrowed, mutable, frame> = local.address l1
    v2: int32 = 42
    v3: int32 = 99
    store v0, v2
    store v1, v3
    v4: int32 = load v0
    return v4
}
"#;
        let expected = r#"
function test(): int32 {
    local l0: int32
    local l1: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    v1: ref<int32, borrowed, mutable, frame> = local.address l1
    v2: int32 = 42
    v3: int32 = 99
    store v0, v2
    store v1, v3
    return v2
}
"#;

        let mut test = TestProgram::new(input);
        let function_id = test.first_function_id();
        let function = test.optimized.tree.get(function_id);
        let block = test.optimized.tree.get(function.block(0));
        let volatile_store = block.instructions[3];
        test.insert_pointer_access_with_options(
            volatile_store,
            mir::MemoryOperation::Write,
            mir::Value::new(1),
            Some(4),
            true,
            None,
        );
        test.run_pass(&ForwardStoredValues);
        test.assert_output(expected);
    }

    /// Atomic load acts as memory barrier.
    #[test]
    fn test_atomic_load_is_barrier() {
        let input = r#"
function test(): int32 {
    local l0: int32
    local l1: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    v1: ref<int32, borrowed, mutable, frame> = local.address l1
    v2: int32 = 42
    store v0, v2
    v3: int32 = atomic.load v1, acquire, scope(device)
    v4: int32 = load v0
    v5: int32 = add v3, v4
    return v5
}
"#;
        let expected = input;

        let mut test = TestProgram::new(input);
        test.run_pass(&ForwardStoredValues);
        test.assert_output(expected);
    }

    /// Atomic store acts as memory barrier.
    #[test]
    fn test_atomic_store_is_barrier() {
        let input = r#"
function test(): int32 {
    local l0: int32
    local l1: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    v1: ref<int32, borrowed, mutable, frame> = local.address l1
    v2: int32 = 42
    v3: int32 = 99
    store v0, v2
    atomic.store v1, v3, release, scope(device)
    v4: int32 = load v0
    return v4
}
"#;
        let expected = input;

        let mut test = TestProgram::new(input);
        test.run_pass(&ForwardStoredValues);
        test.assert_output(expected);
    }

    /// Atomic fence acts as memory barrier.
    #[test]
    fn test_atomic_fence_is_barrier() {
        let input = r#"
function test(): int32 {
    local l0: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    v1: int32 = 42
    store v0, v1
    atomic.fence sequentiallyConsistent, scope(device), storage(device)
    v2: int32 = load v0
    return v2
}
"#;
        let expected = input;

        let mut test = TestProgram::new(input);
        test.run_pass(&ForwardStoredValues);
        test.assert_output(expected);
    }

    /// Size mismatches prevent forwarding from matching pointers.
    #[test]
    fn test_no_forward_size_mismatch() {
        let input = r#"
function test(): int32 {
    local l0: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    v1: int32 = 1
    store v0, v1
    v2: int32 = load v0
    return v2
}
"#;
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
            mir::MemoryOperation::Write,
            mir::Value::new(0),
            Some(8),
        );

        test.insert_pointer_access(
            load_v0,
            mir::MemoryOperation::Read,
            mir::Value::new(0),
            Some(4),
        );

        test.run_pass(&ForwardStoredValues);
        test.assert_output(expected);
    }

    /// Transitive substitutions are resolved correctly.
    #[test]
    fn test_transitive_substitution() {
        let input = r#"
function test(): int32 {
    local l0: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    v1: int32 = 42
    store v0, v1
    v2: int32 = load v0
    v3: int32 = load v0
    v4: int32 = add v2, v3
    return v4
}
"#;
        let expected = r#"
function test(): int32 {
    local l0: int32

entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    v1: int32 = 42
    store v0, v1
    v4: int32 = add v1, v1
    return v4
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&ForwardStoredValues);
        test.assert_output(expected);
    }

    /// Empty function (import) is handled.
    #[test]
    fn test_skip_import_function() {
        let input = r#"
external function imported(): void
"#;
        let expected = input;

        let mut test = TestProgram::new(input);
        test.run_pass(&ForwardStoredValues);
        test.assert_output(expected);
    }

    /// No changes returns Mutation::NONE.
    #[test]
    fn test_no_changes_preserves_all() {
        let input = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = add v0, v0
    return v1
}
"#;
        let expected = input;

        let mut test = TestProgram::new(input);
        test.run_pass(&ForwardStoredValues);
        test.assert_output(expected);
    }
}
