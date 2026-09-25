use std::cmp::Reverse;

use tspp_core::{BitSet, FxIndexSet};

use crate::{
    Analysis, Block, ControlTable, Function, Instruction, Local, LocalId, LocalNodeId, Mutation,
    NodeTable, PlaceOrigin, PlaceTable, Tree, Value,
};

/// Liveness for one MIR function.
#[derive(Debug, Clone, Default)]
pub struct LivenessTable {
    /// Values live at entry indexed by block id.
    value_live_in: NodeTable<Block, BitSet>,
    /// Values live at exit indexed by block id.
    value_live_out: NodeTable<Block, BitSet>,
    /// Locals live at entry indexed by block id.
    local_live_in: NodeTable<Block, BitSet>,
    /// Locals live at exit indexed by block id.
    local_live_out: NodeTable<Block, BitSet>,
    /// Function locals in compact liveness order.
    locals: Vec<LocalNodeId<Local>>,
}

/// Live values and locals while traversing one block.
#[derive(Debug)]
pub struct LivenessCursor<'a> {
    /// Values live at the current operation.
    values: BitSet,
    /// Locals live at the current operation.
    locals: BitSet,
    /// Function locals in compact liveness order.
    local_ids: &'a [LocalNodeId<Local>],
    /// Final uses sorted by SSA value.
    last_uses: Vec<LastUse>,
    /// Local liveness changes in instruction order.
    local_changes: Vec<LocalChange>,
    /// The next instruction offset.
    offset: u32,
    /// The number of instructions in the block.
    length: u32,
    /// The next local liveness change.
    local_change: usize,
}

/// Final use of one SSA value in a block.
#[derive(Debug, Clone, Copy)]
struct LastUse {
    /// The used SSA value.
    value: Value,
    /// The final block offset using the value.
    offset: u32,
}

/// One instruction's local liveness change.
#[derive(Debug, Clone, Copy)]
enum LocalChange {
    /// One defined local becomes live.
    Add {
        /// The defining instruction offset.
        offset: u32,
        /// The local liveness index.
        local: u32,
    },
    /// One local reaches its final read.
    Remove {
        /// The final read instruction offset.
        offset: u32,
        /// The local liveness index.
        local: u32,
    },
}

/// Per-block local use and definition sets for liveness.
#[derive(Debug)]
struct BlockLiveness {
    /// Values used before local definition in the block.
    value_use: BitSet,
    /// Values defined in the block.
    value_def: BitSet,
    /// Locals used before local definition in the block.
    local_use: BitSet,
    /// Locals defined in the block.
    local_def: BitSet,
}

impl LivenessTable {
    /// Build liveness for one MIR function.
    pub fn analyse(function: &Function, control: &ControlTable, tree: &Tree) -> Self {
        let mut locals = function.locals().to_vec();
        locals.sort_unstable_by_key(|local| local.get());

        // index locals in their stable order
        let mut local_indices = NodeTable::from_nodes(&locals, || 0);
        for (index, local) in locals.iter().copied().enumerate() {
            *local_indices.get_mut(local) = index;
        }

        // collect local block liveness
        let blocks = Self::collect_blocks(function, tree, &local_indices);

        // initialize empty block states
        let mut liveness = Self::initialize(function, locals);

        // propagate liveness to a fixed point
        Self::propagate_to_fixed_point(&mut liveness, control, tree, &blocks);

        liveness
    }

    /// Collect local use and def sets for each block.
    fn collect_blocks(
        function: &Function,
        tree: &Tree,
        local_indices: &NodeTable<Local, usize>,
    ) -> NodeTable<Block, BlockLiveness> {
        let value_count = function.value_capacity();
        let local_count = local_indices.values().len();
        let mut blocks = NodeTable::from_nodes(function.blocks(), || BlockLiveness {
            value_use: BitSet::new(value_count),
            value_def: BitSet::new(value_count),
            local_use: BitSet::new(local_count),
            local_def: BitSet::new(local_count),
        });

        // scan each block independently
        for &block_id in function.blocks() {
            let block = tree.get(block_id);
            let terminator = tree.get(block.terminator);
            let mut seen_value_defs = BitSet::new(value_count);
            let mut seen_local_defs = BitSet::new(local_count);
            let mut block_liveness = BlockLiveness {
                value_use: BitSet::new(value_count),
                value_def: BitSet::new(value_count),
                local_use: BitSet::new(local_count),
                local_def: BitSet::new(local_count),
            };

            // record block parameter definitions
            for parameter in &block.parameters {
                seen_value_defs.insert(parameter.value.0 as usize);
                block_liveness.value_def.insert(parameter.value.0 as usize);
            }

            // record instruction uses and definitions
            for &instruction_id in &block.instructions {
                let instruction = tree.get(instruction_id);
                Self::record_instruction_value_uses(
                    &mut block_liveness,
                    &seen_value_defs,
                    instruction,
                    tree,
                );
                Self::record_instruction_local_uses(
                    &mut block_liveness,
                    &seen_local_defs,
                    instruction,
                    local_indices,
                );
                Self::record_instruction_defs(
                    &mut block_liveness,
                    &mut seen_value_defs,
                    &mut seen_local_defs,
                    instruction,
                    local_indices,
                );
            }

            // record terminator uses
            for used in terminator.uses(tree) {
                if !seen_value_defs.contains(used.0 as usize) {
                    block_liveness.value_use.insert(used.0 as usize);
                }
            }

            *blocks.get_mut(block_id) = block_liveness;
        }

        blocks
    }

    /// Record instruction value uses in one block liveness set.
    fn record_instruction_value_uses(
        liveness: &mut BlockLiveness,
        seen_value_defs: &BitSet,
        instruction: &Instruction,
        tree: &Tree,
    ) {
        for used in instruction.reads(tree) {
            if !seen_value_defs.contains(used.0 as usize) {
                liveness.value_use.insert(used.0 as usize);
            }
        }
    }

    /// Record instruction local uses in one block liveness set.
    fn record_instruction_local_uses(
        liveness: &mut BlockLiveness,
        seen_local_defs: &BitSet,
        instruction: &Instruction,
        local_indices: &NodeTable<Local, usize>,
    ) {
        if let Some(local) = Self::used_local(instruction) {
            let index = *local_indices.get(local);
            if !seen_local_defs.contains(index) {
                liveness.local_use.insert(index);
            }
        }
    }

    /// Record instruction defs in one block liveness set.
    fn record_instruction_defs(
        liveness: &mut BlockLiveness,
        seen_value_defs: &mut BitSet,
        seen_local_defs: &mut BitSet,
        instruction: &Instruction,
        local_indices: &NodeTable<Local, usize>,
    ) {
        if let Some(destination) = instruction.destination() {
            seen_value_defs.insert(destination.0 as usize);
            liveness.value_def.insert(destination.0 as usize);
        }

        // record the local defined by a store
        if let Some(local) = Self::defined_local(instruction) {
            let index = *local_indices.get(local);
            seen_local_defs.insert(index);
            liveness.local_def.insert(index);
        }
    }

    /// Return the local completely replaced by an instruction.
    fn defined_local(instruction: &Instruction) -> Option<LocalId> {
        let (Instruction::Store { place, .. } | Instruction::AtomicStore { place, .. }) =
            instruction
        else {
            return None;
        };

        match place.origin {
            PlaceOrigin::Local(local) if place.path.is_root() => Some(local),
            _ => None,
        }
    }

    /// Return the local whose contents or address an instruction uses.
    fn used_local(instruction: &Instruction) -> Option<LocalId> {
        if Self::defined_local(instruction).is_some() {
            return None;
        }

        match instruction.place()?.origin {
            PlaceOrigin::Local(local) => Some(local),
            _ => None,
        }
    }

    /// Initialize empty liveness state for all blocks.
    fn initialize(function: &Function, locals: Vec<LocalNodeId<Local>>) -> Self {
        let value_count = function.value_capacity();
        let local_count = locals.len();

        Self {
            value_live_in: NodeTable::from_nodes(function.blocks(), || BitSet::new(value_count)),
            value_live_out: NodeTable::from_nodes(function.blocks(), || BitSet::new(value_count)),
            local_live_in: NodeTable::from_nodes(function.blocks(), || BitSet::new(local_count)),
            local_live_out: NodeTable::from_nodes(function.blocks(), || BitSet::new(local_count)),
            locals,
        }
    }

    /// Propagate changed liveness entries to their predecessors.
    fn propagate_to_fixed_point(
        liveness: &mut Self,
        control: &ControlTable,
        tree: &Tree,
        blocks: &NodeTable<Block, BlockLiveness>,
    ) {
        // visit successors before predecessors and reschedule only affected blocks
        let mut pending = control.reverse_postorder().collect::<FxIndexSet<_>>();
        while let Some(block) = pending.pop() {
            if Self::propagate_block(liveness, block, tree, blocks) {
                pending.extend(
                    control
                        .predecessors(block)
                        .filter(|block| control.is_reachable(*block)),
                );
            }
        }
    }

    /// Propagate one block of liveness information.
    fn propagate_block(
        liveness: &mut Self,
        block_id: LocalNodeId<Block>,
        tree: &Tree,
        blocks: &NodeTable<Block, BlockLiveness>,
    ) -> bool {
        let block = tree.get(block_id);
        let terminator = tree.get(block.terminator);
        let block_liveness = blocks.get(block_id);

        // merge successor entries into block exits
        let mut next_value_live_out = BitSet::new(block_liveness.value_use.len());
        let mut next_local_live_out = BitSet::new(block_liveness.local_use.len());

        // merge liveness from each successor
        for successor in terminator.successors(tree) {
            let successor_value_live_in = liveness.value_live_in.get(successor);
            next_value_live_out.union_with(successor_value_live_in);

            // merge live locals from the successor
            let successor_local_live_in = liveness.local_live_in.get(successor);
            next_local_live_out.union_with(successor_local_live_in);
        }

        // subtract definitions and add upward-exposed uses
        let mut next_value_live_in = next_value_live_out.clone();
        next_value_live_in.subtract(&block_liveness.value_def);
        next_value_live_in.union_with(&block_liveness.value_use);

        // remove local definitions and add incoming uses
        let mut next_local_live_in = next_local_live_out.clone();
        next_local_live_in.subtract(&block_liveness.local_def);
        next_local_live_in.union_with(&block_liveness.local_use);

        // record changed block states
        let mut changed = false;

        // update value entry state
        if next_value_live_in != *liveness.value_live_in.get(block_id) {
            *liveness.value_live_in.get_mut(block_id) = next_value_live_in;
            changed = true;
        }

        // update value exit state
        if next_value_live_out != *liveness.value_live_out.get(block_id) {
            *liveness.value_live_out.get_mut(block_id) = next_value_live_out;
            changed = true;
        }

        // update local entry state
        if next_local_live_in != *liveness.local_live_in.get(block_id) {
            *liveness.local_live_in.get_mut(block_id) = next_local_live_in;
            changed = true;
        }

        // update local exit state
        if next_local_live_out != *liveness.local_live_out.get(block_id) {
            *liveness.local_live_out.get_mut(block_id) = next_local_live_out;
            changed = true;
        }

        changed
    }

    /// Return the values live at block entry.
    pub fn value_live_in(&self, block: LocalNodeId<Block>) -> impl Iterator<Item = Value> + '_ {
        self.value_live_in
            .get(block)
            .iter()
            .map(|index| Value(index as u32))
    }

    /// Return the values live at block exit.
    pub fn value_live_out(&self, block: LocalNodeId<Block>) -> impl Iterator<Item = Value> + '_ {
        self.value_live_out
            .get(block)
            .iter()
            .map(|index| Value(index as u32))
    }

    /// Return the locals live at block entry.
    pub fn local_live_in(
        &self,
        block: LocalNodeId<Block>,
    ) -> impl Iterator<Item = LocalNodeId<Local>> + '_ {
        self.local_live_in
            .get(block)
            .iter()
            .map(|index| self.locals[index])
    }

    /// Return the locals live at block exit.
    pub fn local_live_out(
        &self,
        block: LocalNodeId<Block>,
    ) -> impl Iterator<Item = LocalNodeId<Local>> + '_ {
        self.local_live_out
            .get(block)
            .iter()
            .map(|index| self.locals[index])
    }

    /// Build linear liveness traversal for one block.
    pub fn cursor<'a>(&'a self, tree: &Tree, block_id: LocalNodeId<Block>) -> LivenessCursor<'a> {
        let block = tree.get(block_id);
        let terminator = tree.get(block.terminator);
        let mut values = self.value_live_out.get(block_id).clone();
        let mut locals = self.local_live_out.get(block_id).clone();
        let mut last_uses = Vec::new();
        let mut local_changes = Vec::new();
        let terminator_offset = block.instructions.len() as u32;

        // retain values consumed by the terminator
        for value in terminator.uses(tree) {
            values.insert(value.id() as usize);
            last_uses.push(LastUse {
                value,
                offset: terminator_offset,
            });
        }

        // retain values passed to successor blocks
        for value in self.value_live_out(block_id) {
            last_uses.push(LastUse {
                value,
                offset: terminator_offset + 1,
            });
        }

        // derive entry liveness and local transitions in one reverse scan
        for (offset, &instruction_id) in block.instructions.iter().enumerate().rev() {
            let instruction = tree.get(instruction_id);
            let offset = offset as u32;

            // remove the SSA definition from entry liveness
            if let Some(destination) = instruction.destination() {
                values.remove(destination.id() as usize);
            }

            // add operands used before this instruction
            for value in instruction.reads(tree) {
                values.insert(value.id() as usize);
                last_uses.push(LastUse { value, offset });
            }

            // transfer the one local read or definition
            if let Some(local) = Self::defined_local(instruction) {
                let index = self.local_index(local);
                if locals.contains(index) {
                    local_changes.push(LocalChange::Add {
                        offset,
                        local: index as u32,
                    });
                }
                locals.remove(index);
            } else if let Some(local) = Self::used_local(instruction) {
                let index = self.local_index(local);
                if !locals.contains(index) {
                    locals.insert(index);
                    local_changes.push(LocalChange::Remove {
                        offset,
                        local: index as u32,
                    });
                }
            }
        }

        // retain only each value's final use
        last_uses.sort_unstable_by_key(|last| (last.value, Reverse(last.offset)));
        last_uses.dedup_by_key(|last| last.value);
        local_changes.reverse();

        LivenessCursor {
            values,
            locals,
            local_ids: &self.locals,
            last_uses,
            local_changes,
            offset: 0,
            length: terminator_offset,
            local_change: 0,
        }
    }

    /// Return whether one value is live at block entry.
    pub fn is_value_live_in(&self, block: LocalNodeId<Block>, value: Value) -> bool {
        self.value_live_in.get(block).contains(value.0 as usize)
    }

    /// Return whether one available value is live after entering a block.
    pub fn is_value_live_after_entry(
        &self,
        block_id: LocalNodeId<Block>,
        value: Value,
        tree: &Tree,
    ) -> bool {
        if self.is_value_live_in(block_id, value) {
            return true;
        }

        // inspect uses in the selected block
        let block = tree.get(block_id);
        if !block
            .parameters
            .iter()
            .any(|parameter| parameter.value == value)
        {
            return false;
        }

        // include block parameters used by the physical block body
        let is_used = block
            .instructions
            .iter()
            .any(|instruction| tree.get(*instruction).reads(tree).contains(&value));
        let terminator = tree.get(block.terminator);

        is_used || terminator.uses(tree).contains(&value) || self.is_value_live_out(block_id, value)
    }

    /// Return whether one value is live at block exit.
    pub fn is_value_live_out(&self, block: LocalNodeId<Block>, value: Value) -> bool {
        self.value_live_out.get(block).contains(value.0 as usize)
    }

    /// Return whether one value is live after one instruction.
    pub fn is_value_live_after_instruction(
        &self,
        block_id: LocalNodeId<Block>,
        instruction_index: usize,
        value: Value,
        tree: &Tree,
    ) -> bool {
        let block = tree.get(block_id);
        let terminator = tree.get(block.terminator);

        // scan later instructions
        for &instruction_id in block.instructions.iter().skip(instruction_index + 1) {
            let instruction = tree.get(instruction_id);

            // retain an operand until this instruction executes
            if instruction.reads(tree).contains(&value) {
                return true;
            }
        }

        // scan the terminator
        if terminator
            .uses(tree)
            .iter()
            .copied()
            .any(|used| used == value)
        {
            return true;
        }

        self.is_value_live_out(block_id, value)
    }

    /// Return the values live before one instruction offset in one block.
    pub fn value_live_before_instruction(
        &self,
        tree: &Tree,
        block_id: LocalNodeId<Block>,
        instruction_offset: usize,
    ) -> FxIndexSet<Value> {
        let block = tree.get(block_id);
        let terminator = tree.get(block.terminator);
        let mut live = self.value_live_out(block_id).collect::<FxIndexSet<_>>();

        // retain values consumed by the terminator
        live.extend(terminator.uses(tree));

        // walk later definitions and uses backward
        for instruction_id in block.instructions.iter().skip(instruction_offset).rev() {
            let instruction = tree.get(*instruction_id);

            // remove the value defined by this instruction
            if let Some(destination) = instruction.destination() {
                live.shift_remove(&destination);
            }

            // retain the operands read by this instruction
            for used in instruction.reads(tree) {
                live.insert(used);
            }
        }

        live
    }

    /// Return the locals live before one instruction offset in one block.
    pub fn local_live_before_instruction(
        &self,
        tree: &Tree,
        block_id: LocalNodeId<Block>,
        instruction_offset: usize,
    ) -> FxIndexSet<LocalNodeId<Local>> {
        let block = tree.get(block_id);

        // return entry liveness directly
        if instruction_offset == 0 {
            return self.local_live_in(block_id).collect();
        }

        // start with locals live at the block exit
        let mut live = self.local_live_out(block_id).collect::<FxIndexSet<_>>();

        // walk later local reads and writes backward
        for instruction_id in block.instructions.iter().skip(instruction_offset).rev() {
            let instruction = tree.get(*instruction_id);
            if let Some(local) = Self::defined_local(instruction) {
                live.shift_remove(&local);
            } else if let Some(local) = Self::used_local(instruction) {
                live.insert(local);
            }
        }

        live
    }

    /// Return the values live before one block terminator.
    pub fn value_live_before_terminator(
        &self,
        tree: &Tree,
        block_id: LocalNodeId<Block>,
    ) -> FxIndexSet<Value> {
        let block = tree.get(block_id);
        let terminator = tree.get(block.terminator);
        let mut live = self.value_live_out(block_id).collect::<FxIndexSet<_>>();

        // retain values consumed by the terminator itself
        live.extend(terminator.uses(tree));

        live
    }

    /// Return the locals live before one block terminator.
    pub fn local_live_before_terminator(
        &self,
        block_id: LocalNodeId<Block>,
    ) -> FxIndexSet<LocalNodeId<Local>> {
        self.local_live_out(block_id).collect()
    }

    /// Return one local's compact liveness index.
    fn local_index(&self, local: LocalNodeId<Local>) -> usize {
        self.locals
            .binary_search_by_key(&local.get(), |candidate| candidate.get())
            .unwrap_or_else(|_| unreachable!("missing liveness local {local:?}"))
    }
}

impl LivenessCursor<'_> {
    /// Return values live at the current operation.
    pub fn values(&self) -> impl Iterator<Item = Value> + '_ {
        self.values.iter().map(|index| Value::new(index as u32))
    }

    /// Return whether one value is live at the current operation.
    pub fn contains_value(&self, value: Value) -> bool {
        self.values.contains(value.id() as usize)
    }

    /// Return whether one local is live at the current operation.
    pub fn contains_local(&self, local: LocalNodeId<Local>) -> bool {
        self.local_ids
            .binary_search_by_key(&local.get(), |candidate| candidate.get())
            .is_ok_and(|index| self.locals.contains(index))
    }

    /// Return one live representation for a canonical place origin.
    pub fn find_representation(
        &self,
        origin: PlaceOrigin,
        places: &PlaceTable,
    ) -> Option<PlaceOrigin> {
        match origin {
            PlaceOrigin::Local(local) => self.contains_local(local).then_some(origin),
            PlaceOrigin::Global(_) => Some(origin),
            PlaceOrigin::Value(_) => self
                .values()
                .filter(|value| places.get(*value).origin == origin)
                .min_by_key(|value| value.id())
                .map(PlaceOrigin::Value),
        }
    }

    /// Advance past one instruction.
    pub fn advance(&mut self, instruction: &Instruction, tree: &Tree) {
        if self.offset >= self.length {
            unreachable!("liveness advanced past block terminator");
        }

        // transfer SSA liveness
        if let Some(destination) = instruction.destination()
            && self.last_use(destination).is_some()
        {
            self.values.insert(destination.id() as usize);
        }
        for value in instruction.reads(tree) {
            if self.last_use(value) == Some(self.offset) {
                self.values.remove(value.id() as usize);
            }
        }

        // transfer local liveness
        if let Some(change) = self.local_changes.get(self.local_change)
            && change.offset() == self.offset
        {
            match change {
                LocalChange::Add { local, .. } => self.locals.insert(*local as usize),
                LocalChange::Remove { local, .. } => self.locals.remove(*local as usize),
            };
            self.local_change += 1;
        }

        self.offset += 1;
    }

    /// Return one value's final use offset.
    fn last_use(&self, value: Value) -> Option<u32> {
        let index = self
            .last_uses
            .binary_search_by_key(&value, |last| last.value)
            .ok()?;

        Some(self.last_uses[index].offset)
    }
}

impl LocalChange {
    /// Return the instruction offset applying this change.
    fn offset(self) -> u32 {
        match self {
            Self::Add { offset, .. } | Self::Remove { offset, .. } => offset,
        }
    }
}

impl Analysis for LivenessTable {
    const INVALIDATED_BY: Mutation = Mutation::CONTROL.union(Mutation::VALUE);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyses::tests::TestModule;

    /// Keep values and local storage live until their final read and omit unused definitions.
    #[test]
    fn test_expire_values_and_locals_after_last_use() {
        let program = TestModule::new(
            r#"
function test(v0: int32): int32 {
    local l0: int32

entry(v0: int32):
    store l0, v0
    v3: int32 = 99
    v1: int32 = load l0
    v2: int32 = add v0, v1
    return v2
}
"#,
        );
        let function = program.tree.get(program.entry_function_id());
        let control = ControlTable::analyse(function, &program.tree);
        let liveness = LivenessTable::analyse(function, &control, &program.tree);
        let entry = function.block(0);
        let local = function.local(0);
        let block = program.tree.get(entry);
        let mut cursor = liveness.cursor(&program.tree, entry);
        assert_eq!(cursor.values().collect::<Vec<_>>(), [Value(0)]);
        assert!(!cursor.contains_local(local));

        for (index, values, local_live) in [
            (0, vec![Value(0)], true),
            (1, vec![Value(0)], true),
            (2, vec![Value(0), Value(1)], false),
            (3, vec![Value(2)], false),
        ] {
            cursor.advance(program.tree.get(block.instructions[index]), &program.tree);
            assert_eq!(
                cursor.values().collect::<Vec<_>>(),
                values,
                "after instruction {index}"
            );
            assert_eq!(
                cursor.contains_local(local),
                local_live,
                "after instruction {index}"
            );
        }
    }

    /// Keep a dominating value live on the branch that reads it.
    #[test]
    fn test_keep_values_live_only_on_reading_branches() {
        let program = TestModule::new(
            r#"
function test(v0: boolean): int32 {
entry(v0: boolean):
    v1: int32 = 42
    branch v0 => b1 | b2

b1:
    return v1

b2:
    v2: int32 = 0
    return v2
}
"#,
        );
        let function = program.tree.get(program.entry_function_id());
        let control = ControlTable::analyse(function, &program.tree);
        let liveness = LivenessTable::analyse(function, &control, &program.tree);
        for (index, incoming, outgoing) in [
            (0, vec![], vec![1]),
            (1, vec![1], vec![]),
            (2, vec![], vec![]),
        ] {
            let block = function.block(index);
            let actual_in = liveness
                .value_live_in(block)
                .map(|value| value.id())
                .collect::<Vec<_>>();
            let actual_out = liveness
                .value_live_out(block)
                .map(|value| value.id())
                .collect::<Vec<_>>();

            assert_eq!(actual_in, incoming, "block {index} entry");
            assert_eq!(actual_out, outgoing, "block {index} exit");
        }
    }

    /// Keep external loop inputs live while defining block parameters at their entries.
    #[test]
    fn test_keep_loop_inputs_live_across_backedges() {
        let program = TestModule::new(
            r#"
function test(v0: boolean): int32 {
entry(v0: boolean):
    v1: int32 = 1
    jump b1(v1)

b1(v2: int32):
    v3: int32 = 2
    v4: int32 = add v2, v3
    branch v0 => b1(v4) | b2

b2:
    return v4
}
"#,
        );
        let function = program.tree.get(program.entry_function_id());
        let control = ControlTable::analyse(function, &program.tree);
        let liveness = LivenessTable::analyse(function, &control, &program.tree);
        for (index, incoming, outgoing) in [
            (0, vec![], vec![0]),
            (1, vec![0], vec![0, 4]),
            (2, vec![4], vec![]),
        ] {
            let block = function.block(index);
            let actual_in = liveness
                .value_live_in(block)
                .map(|value| value.id())
                .collect::<Vec<_>>();
            let actual_out = liveness
                .value_live_out(block)
                .map(|value| value.id())
                .collect::<Vec<_>>();

            assert_eq!(actual_in, incoming, "block {index} entry");
            assert_eq!(actual_out, outgoing, "block {index} exit");
        }
    }

    /// Define the invoke result on its normal edge and retain inputs read during unwind.
    #[test]
    fn test_define_invoke_results_on_normal_edges() {
        let program = TestModule::new(
            r#"
external function called(int32): int32

function test(v0: int32): int32 {
entry(v0: int32):
    invoke called(v0): (int32) => int32 => normal | unwind

normal(v1: int32):
    return v1

unwind:
    return v0

unused:
    return v0
}
"#,
        );
        let function = program.tree.get(program.entry_function_id());
        let control = ControlTable::analyse(function, &program.tree);
        let liveness = LivenessTable::analyse(function, &control, &program.tree);
        for (index, incoming, outgoing) in [
            (0, vec![], vec![0]),
            (1, vec![], vec![]),
            (2, vec![0], vec![]),
            (3, vec![], vec![]),
        ] {
            let block = function.block(index);
            let actual_in = liveness
                .value_live_in(block)
                .map(|value| value.id())
                .collect::<Vec<_>>();
            let actual_out = liveness
                .value_live_out(block)
                .map(|value| value.id())
                .collect::<Vec<_>>();

            assert_eq!(actual_in, incoming, "block {index} entry");
            assert_eq!(actual_out, outgoing, "block {index} exit");
        }
    }
}
