use std::ops::Range;
use std::sync::Arc;

use destack_core::{FxIndexMap, FxIndexSet};

use crate::{
    AliasTable, Analysis, Block, ControlTable, DominatorTable, Function, Instruction, LayoutError,
    LocalNodeId, MemoryAccessEffect, MemoryAccessOrder, MemoryEffectTable, MemoryRegion, Mutation,
    NodeTable, Point, Tree,
};

/// Maximum number of memory states examined by one clobber query.
const CLOBBER_LIMIT: usize = 100;

/// Memory definitions, uses, and merges for one function.
#[derive(Debug)]
pub struct MemorySsaTable {
    /// Memory nodes indexed by access identity.
    accesses: Vec<MemoryNode>,
    /// Memory merges indexed by block.
    phis: NodeTable<Block, Option<MemoryAccessId>>,
    /// Memory operations indexed by instruction.
    instructions: NodeTable<Instruction, Option<MemoryAccessId>>,
    /// Memory operations indexed by block terminator.
    terminators: NodeTable<Block, Option<MemoryAccessId>>,
    /// Contiguous operation nodes for each block.
    blocks: NodeTable<Block, Range<usize>>,
    /// Individual regions read or written by each operation.
    effects: Arc<MemoryEffectTable>,
}

/// Identity of one node in a function's memory SSA.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MemoryAccessId(u32);

impl MemoryAccessId {
    /// Return the dense node index.
    fn index(self) -> usize {
        self.0 as usize
    }
}

/// One function memory state or its use.
#[derive(Debug, Clone)]
pub enum MemoryNode {
    /// Memory entering the function.
    LiveOnEntry,
    /// Memory states merged at a block entry.
    Phi(MemoryPhi),
    /// An operation that writes memory or orders other accesses.
    Def(MemorySsaAccess),
    /// An operation that only reads memory.
    Use(MemorySsaAccess),
}

/// One instruction or terminator linked to its incoming memory state.
#[derive(Debug, Clone)]
pub struct MemorySsaAccess {
    /// The instruction or terminator.
    pub source: Point,
    /// The memory definition immediately before this operation.
    pub defining_access: MemoryAccessId,
}

/// Memory states entering one control-flow merge.
#[derive(Debug, Clone)]
pub struct MemoryPhi {
    /// The block where memory states merge.
    pub block: LocalNodeId<Block>,
    /// The nearest memory state that strictly dominates this merge.
    pub dominator: MemoryAccessId,
    /// Incoming memory by predecessor, with none denoting function entry.
    pub incoming: Vec<(Option<LocalNodeId<Block>>, MemoryAccessId)>,
}

impl MemorySsaTable {
    /// Analyse memory operations using iterated dominance frontiers and SSA renaming.
    pub fn analyse(
        function: &Function,
        control: &ControlTable,
        dominator: &DominatorTable,
        effects: Arc<MemoryEffectTable>,
        tree: &Tree,
    ) -> Self {
        // index all operations, including those in unreachable blocks
        let instructions = function
            .blocks()
            .iter()
            .flat_map(|block| tree.get(*block).instructions.iter().copied())
            .collect::<Vec<_>>();
        let mut result = Self {
            accesses: vec![MemoryNode::LiveOnEntry],
            phis: NodeTable::from_nodes(function.blocks(), || None),
            instructions: NodeTable::from_nodes(&instructions, || None),
            terminators: NodeTable::from_nodes(function.blocks(), || None),
            blocks: NodeTable::from_nodes(function.blocks(), || 0..0),
            effects,
        };
        let Some(entry) = function.entry() else {
            return result;
        };

        // record one memory access for each reachable operation
        let mut definitions = FxIndexSet::default();
        for block in control.reverse_postorder() {
            let start = result.accesses.len();
            for &instruction in &tree.get(block).instructions {
                let source = Point::Instruction(instruction);
                *result.instructions.get_mut(instruction) = result.append(source);
            }
            let source = Point::Terminator(block);
            *result.terminators.get_mut(block) = result.append(source);
            let end = result.accesses.len();
            *result.blocks.get_mut(block) = start..end;

            // seed merge placement with blocks that define memory
            if result.accesses[start..end]
                .iter()
                .any(|node| matches!(node, MemoryNode::Def(_)))
            {
                definitions.insert(block);
            }
        }

        // close definition blocks over their iterated dominance frontier
        let frontiers = dominator.frontiers();
        let mut pending = definitions.iter().copied().collect::<Vec<_>>();
        while let Some(block) = pending.pop() {
            for &merge in &frontiers[&block] {
                if result.phis.get(merge).is_some() {
                    continue;
                }

                // include incoming function memory when a backedge targets entry
                let incoming = if merge == entry {
                    vec![(None, result.live_on_entry())]
                } else {
                    Vec::new()
                };
                let id = MemoryAccessId(result.accesses.len() as u32);
                result.accesses.push(MemoryNode::Phi(MemoryPhi {
                    block: merge,
                    dominator: result.live_on_entry(),
                    incoming,
                }));
                *result.phis.get_mut(merge) = Some(id);

                // propagate newly introduced definitions through subsequent joins
                if !definitions.contains(&merge) {
                    pending.push(merge);
                }
            }
        }

        // rename in dominator order without recursive calls or per-block clones
        let mut pending = vec![(entry, result.live_on_entry())];
        while let Some((block, mut current)) = pending.pop() {
            if let Some(id) = *result.phis.get(block) {
                let MemoryNode::Phi(phi) = &mut result.accesses[id.index()] else {
                    unreachable!("memory phi index refers to a non-phi node");
                };
                phi.dominator = current;
                current = id;
            }
            for index in result.blocks.get(block).clone() {
                let id = MemoryAccessId(index as u32);
                match &mut result.accesses[index] {
                    MemoryNode::Use(access) => access.defining_access = current,
                    MemoryNode::Def(access) => {
                        access.defining_access = current;
                        current = id;
                    }
                    _ => unreachable!("operation range contains a non-operation memory node"),
                }
            }

            // connect each successor's phi to this block's outgoing memory
            for successor in control.successors(block) {
                if let Some(phi) = *result.phis.get(successor) {
                    let MemoryNode::Phi(phi) = &mut result.accesses[phi.index()] else {
                        unreachable!("memory phi index refers to a non-phi node");
                    };
                    if !phi
                        .incoming
                        .iter()
                        .any(|(predecessor, _)| *predecessor == Some(block))
                    {
                        phi.incoming.push((Some(block), current));
                    }
                }
            }

            // give each dominator child the memory state at its immediate dominator's exit
            let mut child = dominator.child(block);
            while let Some(block) = child {
                pending.push((block, current));
                child = dominator.sibling(block);
            }
        }

        result
    }

    /// Return the memory state entering the function.
    pub fn live_on_entry(&self) -> MemoryAccessId {
        MemoryAccessId(0)
    }

    /// Return one memory node.
    pub fn access(&self, id: MemoryAccessId) -> &MemoryNode {
        &self.accesses[id.index()]
    }

    /// Return the memory access associated with an instruction.
    pub fn instruction_access(
        &self,
        instruction: LocalNodeId<Instruction>,
    ) -> Option<MemoryAccessId> {
        *self.instructions.get(instruction)
    }

    /// Return the memory access associated with a terminator.
    pub fn terminator_access(&self, block: LocalNodeId<Block>) -> Option<MemoryAccessId> {
        *self.terminators.get(block)
    }

    /// Return the memory phi associated with a block.
    pub fn block_phi(&self, block: LocalNodeId<Block>) -> Option<MemoryAccessId> {
        *self.phis.get(block)
    }

    /// Return the memory state immediately before an operation.
    pub fn defining_access(&self, id: MemoryAccessId) -> Option<MemoryAccessId> {
        match self.access(id) {
            MemoryNode::Def(access) | MemoryNode::Use(access) => Some(access.defining_access),
            MemoryNode::LiveOnEntry | MemoryNode::Phi(_) => None,
        }
    }

    /// Iterate every region read or written by one source operation.
    pub fn effects(&self, source: Point) -> impl Iterator<Item = &MemoryAccessEffect> {
        let (instruction, terminator) = match source {
            Point::Instruction(instruction) => (Some(instruction), None),
            Point::Terminator(block) => (None, Some(block)),
        };

        instruction
            .into_iter()
            .flat_map(|id| self.effects.instruction_effects(id))
            .chain(
                terminator
                    .into_iter()
                    .flat_map(|id| self.effects.terminator_effects(id)),
            )
    }

    /// Create reusable storage for clobber queries over this immutable analysis.
    pub fn cursor<'a>(&'a self, alias: &'a AliasTable) -> MemorySsaCursor<'a> {
        MemorySsaCursor {
            table: self,
            alias,
            cache: FxIndexMap::default(),
            pending: Vec::new(),
            visited: FxIndexSet::default(),
        }
    }

    /// Append one node if an operation reads, writes, or orders memory.
    fn append(&mut self, source: Point) -> Option<MemoryAccessId> {
        // classify the operation across all of its accessed regions
        let mut reads = false;
        let mut defines = false;
        for effect in self.effects(source) {
            reads |= effect.reads;
            defines |= effect.writes
                || effect.is_barrier
                || !matches!(effect.order, MemoryAccessOrder::Plain);
        }
        if !reads && !defines {
            return None;
        }

        // initialize the operation with incoming function memory before renaming
        let access = MemorySsaAccess {
            source,
            defining_access: self.live_on_entry(),
        };
        let id = MemoryAccessId(self.accesses.len() as u32);
        let node = if defines {
            MemoryNode::Def(access)
        } else {
            MemoryNode::Use(access)
        };
        self.accesses.push(node);

        Some(id)
    }
}

impl Analysis for MemorySsaTable {
    const INVALIDATED_BY: Mutation =
        MemoryEffectTable::INVALIDATED_BY.union(DominatorTable::INVALIDATED_BY);
}

/// Cached clobber searches over an immutable memory SSA and alias table.
#[derive(Debug)]
pub struct MemorySsaCursor<'a> {
    /// The function's memory SSA.
    table: &'a MemorySsaTable,
    /// Physical alias relationships for this function.
    alias: &'a AliasTable,
    /// Completed searches indexed by starting state and accessed region.
    cache: FxIndexMap<(MemoryAccessId, MemoryRegion), MemoryAccessId>,
    /// Memory states awaiting inspection.
    pending: Vec<MemoryAccessId>,
    /// Memory states already inspected by the current query.
    visited: FxIndexSet<MemoryAccessId>,
}

impl MemorySsaCursor<'_> {
    /// Find a clobber at or before the starting memory state for one accessed region.
    pub fn clobber(
        &mut self,
        start: MemoryAccessId,
        region: &MemoryRegion,
    ) -> Result<MemoryAccessId, LayoutError> {
        // reuse completed searches for the same starting state and location
        let key = (start, region.clone());
        if let Some(&result) = self.cache.get(&key) {
            return Ok(result);
        }

        // walk through unrelated definitions and merges to the nearest clobber
        let mut current = start;
        let mut remaining = CLOBBER_LIMIT;
        while remaining > 0 {
            remaining -= 1;
            current = match self.table.access(current) {
                MemoryNode::LiveOnEntry => break,
                MemoryNode::Use(access) => access.defining_access,
                MemoryNode::Def(access) => {
                    if self.is_clobber(access, region)? {
                        break;
                    }

                    access.defining_access
                }
                MemoryNode::Phi(phi) => {
                    if !self.can_skip_phi(phi, region, &mut remaining)? {
                        break;
                    }

                    phi.dominator
                }
            };
        }

        // retain the last dominating state when the query exhausts its budget
        self.cache.insert(key, current);

        Ok(current)
    }

    /// Check whether every incoming path preserves a region from the dominating state.
    fn can_skip_phi(
        &mut self,
        phi: &MemoryPhi,
        region: &MemoryRegion,
        remaining: &mut usize,
    ) -> Result<bool, LayoutError> {
        // preserve merges whose address can change between incoming paths
        if !self.alias.is_invariant(region, phi.block) {
            return Ok(false);
        }

        // stop each incoming path at the common dominating memory state
        self.pending.clear();
        self.visited.clear();
        self.pending.extend(phi.incoming.iter().map(|(_, id)| *id));
        while let Some(current) = self.pending.pop() {
            if current == phi.dominator || !self.visited.insert(current) {
                continue;
            }
            if *remaining == 0 {
                return Ok(false);
            }
            *remaining -= 1;

            // reject paths that modify the region or change its address
            match self.table.access(current) {
                MemoryNode::LiveOnEntry => return Ok(false),
                MemoryNode::Phi(phi) => {
                    if !self.alias.is_invariant(region, phi.block) {
                        return Ok(false);
                    }
                    self.pending.extend(phi.incoming.iter().map(|(_, id)| *id));
                }
                MemoryNode::Use(access) => self.pending.push(access.defining_access),
                MemoryNode::Def(access) => {
                    if self.is_clobber(access, region)? {
                        return Ok(false);
                    }
                    self.pending.push(access.defining_access);
                }
            }
        }

        Ok(true)
    }

    /// Check whether an operation modifies or orders the queried region.
    fn is_clobber(
        &self,
        access: &MemorySsaAccess,
        region: &MemoryRegion,
    ) -> Result<bool, LayoutError> {
        // stop when any region written by this operation overlaps the query
        for effect in self.table.effects(access.source) {
            if effect.clobbers_region(region, self.alias)? {
                return Ok(true);
            }
        }

        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyses::tests::TestModule;
    use crate::{MemoryLocation, StorageSet, Value};

    /// Link a load through disjoint stores to the preceding write of its local.
    #[test]
    fn test_find_local_clobber() {
        let mut program = TestModule::new(
            r#"
function test(): int32 {
    local l0: int32
    local l1: int32

entry:
    v0: int32 = 7
    store l0, v0
    store l1, v0
    v1: ref<int32, borrowed, 'frame, mutable, frame> = address l0
    v2: int32 = load.copy (*v1)
    return v2
}
"#,
        );
        let function = program.entry_function_id();
        let mut analyses = program.function_analyses();
        let memory = analyses.ssa(
            program.entry_function_id(),
            &mut program.tree,
            &program.effects,
        );
        let alias = analyses
            .alias(
                program.entry_function_id(),
                program.layouts.clone(),
                &mut program.tree,
            )
            .unwrap();
        let instructions = &program
            .tree
            .get(program.tree.get(function).block(0))
            .instructions;
        let first = memory.instruction_access(instructions[1]).unwrap();
        let second = memory.instruction_access(instructions[2]).unwrap();
        let load = memory.instruction_access(instructions[4]).unwrap();
        let region = MemoryRegion::Local(program.tree.get(function).local(0));

        assert_eq!(memory.defining_access(first), Some(memory.live_on_entry()));
        assert_eq!(memory.defining_access(second), Some(first));
        assert_eq!(memory.defining_access(load), Some(second));
        assert_eq!(
            memory.cursor(&alias).clobber(second, &region).unwrap(),
            first
        );
    }

    /// Merge both branch stores before a load at their join.
    #[test]
    fn test_merge_branch_stores() {
        let mut program = TestModule::new(
            r#"
function test<'a>(v0: ref<int32, borrowed, 'a, mutable, local>, v1: boolean): int32 {
entry(v0: ref<int32, borrowed, 'a, mutable, local>, v1: boolean):
    branch v1 => left | right

left:
    v2: int32 = 1
    store (*v0), v2
    jump join

right:
    v3: int32 = 2
    store (*v0), v3
    jump join

join:
    v4: int32 = load.copy (*v0)
    return v4
}
"#,
        );
        let function = program.entry_function_id();
        let mut analyses = program.function_analyses();
        let memory = analyses.ssa(
            program.entry_function_id(),
            &mut program.tree,
            &program.effects,
        );
        let alias = analyses
            .alias(
                program.entry_function_id(),
                program.layouts.clone(),
                &mut program.tree,
            )
            .unwrap();
        let left = program.tree.get(function).block(1);
        let right = program.tree.get(function).block(2);
        let join = program.tree.get(function).block(3);
        let first = memory
            .instruction_access(program.tree.get(left).instructions[1])
            .unwrap();
        let second = memory
            .instruction_access(program.tree.get(right).instructions[1])
            .unwrap();
        let load_instruction = program.tree.get(join).instructions[0];
        let load = memory.instruction_access(load_instruction).unwrap();
        let phi = memory.block_phi(join).unwrap();
        let MemoryNode::Phi(merge) = memory.access(phi) else {
            panic!("expected memory phi")
        };
        let incoming = merge.incoming.iter().copied().collect::<FxIndexMap<_, _>>();
        let region = MemoryRegion::Address {
            location: MemoryLocation::with_size(Value(0), 4),
            spaces: StorageSet::LOCAL,
        };

        assert_eq!(
            incoming,
            FxIndexMap::from_iter([(Some(left), first), (Some(right), second)])
        );
        assert_eq!(memory.defining_access(load), Some(phi));
        assert_eq!(memory.cursor(&alias).clobber(phi, &region).unwrap(), phi);
    }

    /// Recover the earlier clobber merge through branches that write another local.
    #[test]
    fn test_skip_disjoint_branch_stores() {
        let mut program = TestModule::new(
            r#"
function test(v0: boolean): int32 {
    local l0: int32
    local l1: int32

entry(v0: boolean):
    v1: int32 = 1
    v2: int32 = 2
    branch v0 => left | right

left:
    store l0, v1
    jump join

right:
    store l0, v2
    jump join

join:
    v3: int32 = load.copy l0
    branch v0 => next_left | next_right

next_left:
    store l1, v1
    jump exit

next_right:
    store l1, v2
    jump exit

exit:
    v4: int32 = load.copy l0
    return v4
}
"#,
        );
        let function = program.entry_function_id();
        let mut analyses = program.function_analyses();
        let memory = analyses.ssa(
            program.entry_function_id(),
            &mut program.tree,
            &program.effects,
        );
        let alias = analyses
            .alias(
                program.entry_function_id(),
                program.layouts.clone(),
                &mut program.tree,
            )
            .unwrap();
        let join = memory
            .block_phi(program.tree.get(function).block(3))
            .unwrap();
        let exit = memory
            .block_phi(program.tree.get(function).block(6))
            .unwrap();
        let region = MemoryRegion::Local(program.tree.get(function).local(0));
        let mut cursor = memory.cursor(&alias);

        assert_eq!(cursor.clobber(join, &region).unwrap(), join);
        assert_eq!(cursor.clobber(exit, &region).unwrap(), join);
    }

    /// Skip a loop's disjoint store while retaining its memory merge.
    #[test]
    fn test_find_clobber_through_loop() {
        let mut program = TestModule::new(
            r#"
function test(v0: boolean): int32 {
    local l0: int32
    local l1: int32

entry(v0: boolean):
    v1: int32 = 7
    store l0, v1
    jump loop

loop:
    v2: int32 = load.copy l0
    store l1, v2
    branch v0 => loop | exit

exit:
    return v2
}
"#,
        );
        let function = program.entry_function_id();
        let mut analyses = program.function_analyses();
        let memory = analyses.ssa(
            program.entry_function_id(),
            &mut program.tree,
            &program.effects,
        );
        let alias = analyses
            .alias(
                program.entry_function_id(),
                program.layouts.clone(),
                &mut program.tree,
            )
            .unwrap();
        let entry = program.tree.get(function).block(0);
        let header = program.tree.get(function).block(1);
        let initial = memory
            .instruction_access(program.tree.get(entry).instructions[1])
            .unwrap();
        let load_instruction = program.tree.get(header).instructions[0];
        let load = memory.instruction_access(load_instruction).unwrap();
        let store = memory
            .instruction_access(program.tree.get(header).instructions[1])
            .unwrap();
        let phi = memory.block_phi(header).unwrap();
        let MemoryNode::Phi(merge) = memory.access(phi) else {
            panic!("expected memory phi")
        };
        let incoming = merge.incoming.iter().copied().collect::<FxIndexMap<_, _>>();
        let region = MemoryRegion::Local(program.tree.get(function).local(0));

        assert_eq!(
            incoming,
            FxIndexMap::from_iter([(Some(entry), initial), (Some(header), store)])
        );
        assert_eq!(memory.defining_access(load), Some(phi));
        assert_eq!(
            memory.cursor(&alias).clobber(phi, &region).unwrap(),
            initial
        );
    }

    /// Include incoming function memory when control returns to the entry block.
    #[test]
    fn test_merge_entry_backedge() {
        let mut program = TestModule::new(
            r#"
function test<'a>(v0: ref<int32, borrowed, 'a, mutable, local>, v1: boolean): int32 {
entry(v0: ref<int32, borrowed, 'a, mutable, local>, v1: boolean):
    v2: int32 = load.copy (*v0)
    store (*v0), v2
    branch v1 => entry(v0, v1) | exit

exit:
    return v2
}
"#,
        );
        let function = program.entry_function_id();
        let mut analyses = program.function_analyses();
        let memory = analyses.ssa(
            program.entry_function_id(),
            &mut program.tree,
            &program.effects,
        );
        let alias = analyses
            .alias(
                program.entry_function_id(),
                program.layouts.clone(),
                &mut program.tree,
            )
            .unwrap();
        let entry = program.tree.get(function).block(0);
        let instructions = &program.tree.get(entry).instructions;
        let load = memory.instruction_access(instructions[0]).unwrap();
        let store = memory.instruction_access(instructions[1]).unwrap();
        let phi = memory.block_phi(entry).unwrap();
        let MemoryNode::Phi(merge) = memory.access(phi) else {
            panic!("expected memory phi")
        };
        let region = MemoryRegion::Address {
            location: MemoryLocation::with_size(Value(0), 4),
            spaces: StorageSet::LOCAL,
        };

        assert_eq!(
            merge.incoming,
            [(None, memory.live_on_entry()), (Some(entry), store)]
        );
        assert_eq!(memory.defining_access(load), Some(phi));
        assert_eq!(memory.cursor(&alias).clobber(phi, &region).unwrap(), phi);
    }

    /// Keep both copy regions on one definition and both comparison regions on one use.
    #[test]
    fn test_link_copy_definition_to_comparison_use() {
        let mut program = TestModule::new(
            r#"
function test<'a>(v0: ref<int32, borrowed, 'a, mutable, local>, v1: ref<int32, borrowed, 'a, mutable, local>): int32 {
entry(v0: ref<int32, borrowed, 'a, mutable, local>, v1: ref<int32, borrowed, 'a, mutable, local>):
    v2: usize = 4
    intrinsic.memory.raw.copyBytes(v0, v1, v2)
    v3: int32 = intrinsic.memory.raw.compareBytes(v0, v1, v2)
    return v3
}
"#,
        );
        let function = program.entry_function_id();
        let mut analyses = program.function_analyses();
        let memory = analyses.ssa(
            program.entry_function_id(),
            &mut program.tree,
            &program.effects,
        );
        let instructions = &program
            .tree
            .get(program.tree.get(function).block(0))
            .instructions;
        let copy = memory.instruction_access(instructions[1]).unwrap();
        let compare = memory.instruction_access(instructions[2]).unwrap();
        let MemoryNode::Def(copy_node) = memory.access(copy) else {
            panic!("expected copy definition")
        };
        let MemoryNode::Use(compare_node) = memory.access(compare) else {
            panic!("expected comparison use")
        };

        assert_eq!(copy_node.defining_access, memory.live_on_entry());
        assert_eq!(compare_node.defining_access, copy);
    }

    /// Stop clobber searches at fences and acquire loads across disjoint storage.
    #[test]
    fn test_stop_clobber_search_at_fences_and_acquire_loads() {
        let mut program = TestModule::new(
            r#"
function test<'a>(v0: ref<int32, borrowed, 'a, mutable, local>, v1: ref<int32, borrowed, 'a, readonly, shared>): int32 {
entry(v0: ref<int32, borrowed, 'a, mutable, local>, v1: ref<int32, borrowed, 'a, readonly, shared>):
    atomic.fence sequentiallyConsistent, scope(device), storage(shared)
    v2: int32 = atomic.load (*v1), acquire, scope(device)
    v3: int32 = load.copy (*v0)
    return v3
}
"#,
        );
        let function = program.entry_function_id();
        let mut analyses = program.function_analyses();
        let memory = analyses.ssa(
            program.entry_function_id(),
            &mut program.tree,
            &program.effects,
        );
        let alias = analyses
            .alias(
                program.entry_function_id(),
                program.layouts.clone(),
                &mut program.tree,
            )
            .unwrap();
        let instructions = &program
            .tree
            .get(program.tree.get(function).block(0))
            .instructions;
        let fence = memory.instruction_access(instructions[0]).unwrap();
        let acquire = memory.instruction_access(instructions[1]).unwrap();
        let load = memory.instruction_access(instructions[2]).unwrap();
        let region = MemoryRegion::Address {
            location: MemoryLocation::with_size(Value::new(0), 4),
            spaces: StorageSet::LOCAL,
        };

        assert_eq!(memory.defining_access(acquire), Some(fence));
        assert_eq!(memory.defining_access(load), Some(acquire));
        assert_eq!(
            memory.cursor(&alias).clobber(acquire, &region).unwrap(),
            acquire
        );
        assert_eq!(
            memory.cursor(&alias).clobber(fence, &region).unwrap(),
            fence
        );
    }

    /// Preserve loop memory when an indexed query changes between iterations.
    #[test]
    fn test_preserve_loop_address_changes() {
        let mut program = TestModule::new(
            r#"
function test<'a>(v0: slice<int32, borrowed, 'a, mutable, local>, v1: boolean): int32 {
entry(v0: slice<int32, borrowed, 'a, mutable, local>, v1: boolean):
    v2: usize = 0
    v3: usize = 1
    v4: usize = 8
    v5: int32 = 7
    v6: slice<int32, borrowed, 'a, mutable, local> = address (*v0)[v3; v4]
    jump loop(v2)

loop(v7: usize):
    v8: ref<int32, borrowed, 'a, mutable, local> = address (*v0)[v7]
    v9: ref<int32, borrowed, 'a, mutable, local> = address (*v6)[v7]
    store (*v8), v5
    v10: int32 = load.copy (*v9)
    v11: usize = add v7, v3
    branch v1 => loop(v11) | exit

exit:
    return v10
}
"#,
        );
        let function = program.entry_function_id();
        let mut analyses = program.function_analyses();
        let memory = analyses.ssa(
            program.entry_function_id(),
            &mut program.tree,
            &program.effects,
        );
        let alias = analyses
            .alias(
                program.entry_function_id(),
                program.layouts.clone(),
                &mut program.tree,
            )
            .unwrap();
        let block = program.tree.get(function).block(1);
        let load = program.tree.get(block).instructions[3];
        let access = memory.instruction_access(load).unwrap();
        let start = memory.defining_access(access).unwrap();
        let region = MemoryRegion::Address {
            location: MemoryLocation::with_size(Value(9), 4),
            spaces: StorageSet::LOCAL,
        };
        let clobber = memory.cursor(&alias).clobber(start, &region).unwrap();

        assert_eq!(Some(clobber), memory.block_phi(block));
    }

    /// Link both invoke continuations to the call's memory definition.
    #[test]
    fn test_link_invoke_memory() {
        let mut program = TestModule::new(
            r#"
external function change(): void

function test(v0: ptr<int32, readonly>): int32 {
entry(v0: ptr<int32, readonly>):
    invoke change(): () => void => normal | unwind

normal:
    v1: int32 = load.copy (*v0)
    return v1

unwind:
    v2: int32 = load.copy (*v0)
    return v2

unused:
    v3: int32 = load.copy (*v0)
    return v3
}
"#,
        );
        let function = program.entry_function_id();
        let mut analyses = program.function_analyses();
        let memory = analyses.ssa(
            program.entry_function_id(),
            &mut program.tree,
            &program.effects,
        );
        let invoke = memory
            .terminator_access(program.tree.get(function).block(0))
            .unwrap();
        let actual = (1..4)
            .map(|index| {
                let instruction = program
                    .tree
                    .get(program.tree.get(function).block(index))
                    .instructions[0];
                memory
                    .instruction_access(instruction)
                    .and_then(|access| memory.defining_access(access))
            })
            .collect::<Vec<_>>();

        assert_eq!(actual, [Some(invoke), Some(invoke), None]);
    }
}
