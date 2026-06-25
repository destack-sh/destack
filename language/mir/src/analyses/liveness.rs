use std::collections::HashSet;

use super::{Analysis, AnalysisId, FunctionAnalyses, FunctionAnalysis};
use crate::{Block, Function, Instruction, Local, LocalNodeId, NodeTable, Tree, Value};

/// Per-block local use and definition sets for liveness.
#[derive(Debug, Default)]
struct BlockLiveness {
    /// Values used before local definition in the block.
    value_use: HashSet<Value>,
    /// Values defined in the block.
    value_def: HashSet<Value>,
    /// Locals used before local definition in the block.
    local_use: HashSet<LocalNodeId<Local>>,
    /// Locals defined in the block.
    local_def: HashSet<LocalNodeId<Local>>,
}

/// Liveness analysis for one MIR function.
#[derive(Debug, Clone, Default)]
pub struct FunctionLiveness {
    /// Values live at entry indexed by block id.
    value_live_in: NodeTable<Block, HashSet<Value>>,
    /// Values live at exit indexed by block id.
    value_live_out: NodeTable<Block, HashSet<Value>>,
    /// Locals live at entry indexed by block id.
    local_live_in: NodeTable<Block, HashSet<LocalNodeId<Local>>>,
    /// Locals live at exit indexed by block id.
    local_live_out: NodeTable<Block, HashSet<LocalNodeId<Local>>>,
}

impl FunctionLiveness {
    /// Build liveness for one MIR function.
    pub fn build(function: &Function, tree: &Tree) -> Self {
        // collect local block liveness
        let blocks = Self::collect_blocks(function, tree);

        // initial state
        let mut liveness = Self::initialize(function);

        // fixed point
        Self::propagate_to_fixed_point(&mut liveness, function, tree, &blocks);

        liveness
    }

    /// Collect local use and def sets for each block.
    fn collect_blocks(function: &Function, tree: &Tree) -> NodeTable<Block, BlockLiveness> {
        let mut blocks = NodeTable::from_nodes(&function.blocks, BlockLiveness::default);

        // scan each block independently
        for &block_id in &function.blocks {
            let block = tree.get(block_id);
            let terminator = tree.get(block.terminator);
            let mut seen_value_defs = HashSet::new();
            let mut seen_local_defs = HashSet::new();
            let mut block_liveness = BlockLiveness::default();

            // block parameters
            for parameter in &block.parameters {
                seen_value_defs.insert(parameter.value);
                block_liveness.value_def.insert(parameter.value);
            }

            // instructions
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
                );
                Self::record_instruction_defs(
                    &mut block_liveness,
                    &mut seen_value_defs,
                    &mut seen_local_defs,
                    instruction,
                );
            }

            // terminator uses
            for used in terminator.uses(tree) {
                if !seen_value_defs.contains(&used) {
                    block_liveness.value_use.insert(used);
                }
            }

            *blocks.get_mut(block_id) = block_liveness;
        }

        blocks
    }

    /// Record instruction value uses in one block liveness set.
    fn record_instruction_value_uses(
        liveness: &mut BlockLiveness,
        seen_value_defs: &HashSet<Value>,
        instruction: &Instruction,
        tree: &Tree,
    ) {
        for used in instruction.uses() {
            if !seen_value_defs.contains(&used) {
                liveness.value_use.insert(used);
            }
        }

        if let Some(arguments) = instruction.argument_slice() {
            for &argument in tree.get_values(arguments) {
                if !seen_value_defs.contains(&argument) {
                    liveness.value_use.insert(argument);
                }
            }
        }
    }

    /// Record instruction local uses in one block liveness set.
    fn record_instruction_local_uses(
        liveness: &mut BlockLiveness,
        seen_local_defs: &HashSet<LocalNodeId<Local>>,
        instruction: &Instruction,
    ) {
        match instruction {
            Instruction::LocalGet { local, .. } | Instruction::LocalAddr { local, .. } => {
                if !seen_local_defs.contains(local) {
                    liveness.local_use.insert(*local);
                }
            }
            _ => {}
        }
    }

    /// Record instruction defs in one block liveness set.
    fn record_instruction_defs(
        liveness: &mut BlockLiveness,
        seen_value_defs: &mut HashSet<Value>,
        seen_local_defs: &mut HashSet<LocalNodeId<Local>>,
        instruction: &Instruction,
    ) {
        if let Some(destination) = instruction.destination() {
            seen_value_defs.insert(destination);
            liveness.value_def.insert(destination);
        }

        if let Instruction::LocalSet { local, .. } = instruction {
            seen_local_defs.insert(*local);
            liveness.local_def.insert(*local);
        }
    }

    /// Initialize empty liveness state for all blocks.
    fn initialize(function: &Function) -> Self {
        Self {
            value_live_in: NodeTable::from_nodes(&function.blocks, HashSet::new),
            value_live_out: NodeTable::from_nodes(&function.blocks, HashSet::new),
            local_live_in: NodeTable::from_nodes(&function.blocks, HashSet::new),
            local_live_out: NodeTable::from_nodes(&function.blocks, HashSet::new),
        }
    }

    /// Propagate liveness until the block states stabilize.
    fn propagate_to_fixed_point(
        liveness: &mut Self,
        function: &Function,
        tree: &Tree,
        blocks: &NodeTable<Block, BlockLiveness>,
    ) {
        let mut changed = true;

        while changed {
            changed = false;

            for &block_id in function.blocks.iter().rev() {
                if Self::propagate_block(liveness, block_id, tree, blocks) {
                    changed = true;
                }
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

        // successor live-out
        let mut next_value_live_out = HashSet::new();
        let mut next_local_live_out = HashSet::new();

        for successor in terminator.successors(tree) {
            let successor_value_live_in = liveness.value_live_in.get(successor);
            next_value_live_out.extend(successor_value_live_in.iter().copied());

            let successor_local_live_in = liveness.local_live_in.get(successor);
            next_local_live_out.extend(successor_local_live_in.iter().copied());
        }

        // block live-in
        let mut next_value_live_in: HashSet<Value> = next_value_live_out
            .difference(&block_liveness.value_def)
            .copied()
            .collect();
        next_value_live_in.extend(block_liveness.value_use.iter().copied());

        let mut next_local_live_in: HashSet<LocalNodeId<Local>> = next_local_live_out
            .difference(&block_liveness.local_def)
            .copied()
            .collect();
        next_local_live_in.extend(block_liveness.local_use.iter().copied());

        let mut changed = false;

        // value live-in
        if next_value_live_in != *liveness.value_live_in.get(block_id) {
            *liveness.value_live_in.get_mut(block_id) = next_value_live_in;
            changed = true;
        }

        // value live-out
        if next_value_live_out != *liveness.value_live_out.get(block_id) {
            *liveness.value_live_out.get_mut(block_id) = next_value_live_out;
            changed = true;
        }

        // local live-in
        if next_local_live_in != *liveness.local_live_in.get(block_id) {
            *liveness.local_live_in.get_mut(block_id) = next_local_live_in;
            changed = true;
        }

        // local live-out
        if next_local_live_out != *liveness.local_live_out.get(block_id) {
            *liveness.local_live_out.get_mut(block_id) = next_local_live_out;
            changed = true;
        }

        changed
    }

    /// Return the values live at block entry.
    pub fn value_live_in(&self, block: LocalNodeId<Block>) -> &HashSet<Value> {
        self.value_live_in.get(block)
    }

    /// Return the values live at block exit.
    pub fn value_live_out(&self, block: LocalNodeId<Block>) -> &HashSet<Value> {
        self.value_live_out.get(block)
    }

    /// Return the locals live at block entry.
    pub fn local_live_in(&self, block: LocalNodeId<Block>) -> &HashSet<LocalNodeId<Local>> {
        self.local_live_in.get(block)
    }

    /// Return the locals live at block exit.
    pub fn local_live_out(&self, block: LocalNodeId<Block>) -> &HashSet<LocalNodeId<Local>> {
        self.local_live_out.get(block)
    }

    /// Return whether one value is live at block entry.
    pub fn is_value_live_in(&self, block: LocalNodeId<Block>, value: Value) -> bool {
        self.value_live_in(block).contains(&value)
    }

    /// Return whether one value is live at block exit.
    pub fn is_value_live_out(&self, block: LocalNodeId<Block>, value: Value) -> bool {
        self.value_live_out(block).contains(&value)
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

        // later instructions
        for &instruction_id in block.instructions.iter().skip(instruction_index + 1) {
            let instruction = tree.get(instruction_id);

            if instruction.uses().iter().copied().any(|used| used == value) {
                return true;
            }

            if let Some(arguments) = instruction.argument_slice()
                && tree
                    .get_values(arguments)
                    .iter()
                    .copied()
                    .any(|argument| argument == value)
            {
                return true;
            }
        }

        // terminator
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
    ) -> HashSet<Value> {
        let block = tree.get(block_id);
        let terminator = tree.get(block.terminator);

        // block entry
        if instruction_offset == 0 {
            return self.value_live_in(block_id).clone();
        }

        let mut live = self.value_live_out(block_id).clone();

        // later instructions
        for instruction_id in block.instructions.iter().skip(instruction_offset) {
            let instruction = tree.get(*instruction_id);

            if let Some(destination) = instruction.destination() {
                live.remove(&destination);
            }

            for used in instruction.uses() {
                live.insert(used);
            }

            if let Some(arguments) = instruction.argument_slice() {
                for &argument in tree.get_values(arguments) {
                    live.insert(argument);
                }
            }
        }

        // terminator
        for used in terminator.uses(tree) {
            live.insert(used);
        }

        live
    }

    /// Return all values live somewhere in the function.
    pub fn all_live_values(&self) -> HashSet<Value> {
        let mut values = HashSet::new();

        for live in self.value_live_in.values() {
            values.extend(live.iter().copied());
        }

        for live in self.value_live_out.values() {
            values.extend(live.iter().copied());
        }

        values
    }
}

impl Analysis for FunctionLiveness {
    const ID: AnalysisId = AnalysisId("liveness");
}

impl FunctionAnalysis for FunctionLiveness {
    fn compute(function: &Function, tree: &Tree, _analyses: &FunctionAnalyses) -> Self {
        Self::build(function, tree)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyses::tests::parse_test_function;

    #[test]
    fn test_build_liveness_for_simple_block() {
        let (tree, function_id) = parse_test_function(
            r#"
function test(): int32 {
entry:
    v0: int32 = 1
    v1: int32 = 2
    v2: int32 = int.add v0, v1
    return v2
}
"#,
        );

        let function = tree.get(function_id);
        let liveness = FunctionLiveness::build(function, &tree);

        let entry = function.entry.expect("missing entry");

        assert!(!liveness.is_value_live_in(entry, Value::new(0)));
        assert!(!liveness.is_value_live_in(entry, Value::new(1)));
        assert!(!liveness.is_value_live_in(entry, Value::new(2)));
        assert!(liveness.value_live_out(entry).is_empty());
        assert!(liveness.is_value_live_after_instruction(entry, 0, Value::new(0), &tree));
        assert!(liveness.is_value_live_after_instruction(entry, 2, Value::new(2), &tree));
    }

    #[test]
    fn test_build_liveness_across_blocks() {
        let (tree, function_id) = parse_test_function(
            r#"
function test(v0: boolean): int32 {
entry(v0: boolean):
    v1: int32 = 42
    branch v0, b1, b2

b1:
    return v1

b2:
    v2: int32 = 0
    return v2
}
"#,
        );

        let function = tree.get(function_id);
        let liveness = FunctionLiveness::build(function, &tree);

        let block1 = function.blocks[1];
        let block2 = function.blocks[2];

        assert!(liveness.is_value_live_in(block1, Value::new(1)));
        assert!(!liveness.is_value_live_in(block2, Value::new(1)));
    }

    #[test]
    fn test_build_liveness_for_loop() {
        let (tree, function_id) = parse_test_function(
            r#"
function test(v0: boolean): int32 {
entry(v0: boolean):
    v1: int32 = 1
    jump b1(v1)

b1(v2: int32):
    v3: int32 = 2
    v4: int32 = int.add v2, v3
    branch v0, b1(v4), b2

b2:
    return v4
}
"#,
        );

        let function = tree.get(function_id);
        let liveness = FunctionLiveness::build(function, &tree);

        let block1 = function.blocks[1];
        let block2 = function.blocks[2];

        assert!(!liveness.is_value_live_in(block1, Value::new(2)));
        assert!(liveness.is_value_live_in(block1, Value::new(0)));
        assert!(liveness.is_value_live_out(block1, Value::new(4)));
        assert!(liveness.is_value_live_in(block2, Value::new(4)));
    }

    #[test]
    fn test_ignore_dead_values() {
        let (tree, function_id) = parse_test_function(
            r#"
function test(): int32 {
entry:
    v0: int32 = 1
    v1: int32 = 2
    return v1
}
"#,
        );

        let function = tree.get(function_id);
        let liveness = FunctionLiveness::build(function, &tree);

        let entry = function.entry.expect("missing entry");

        assert!(!liveness.is_value_live_after_instruction(entry, 0, Value::new(0), &tree));
    }
}
