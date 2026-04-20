use std::collections::{HashMap, HashSet};

use crate::{
    Block, BlockReference, Function, Instruction, Local, LocalNodeId, LocalReference, NodeTree,
    Value, ValueReference,
};

/// Per-block use and def facts for liveness.
#[derive(Debug, Default)]
struct BlockLivenessFacts {
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
    /// Values live at entry to each block.
    value_live_in: HashMap<LocalNodeId<Block>, HashSet<Value>>,
    /// Values live at exit of each block.
    value_live_out: HashMap<LocalNodeId<Block>, HashSet<Value>>,
    /// Locals live at entry to each block.
    local_live_in: HashMap<LocalNodeId<Block>, HashSet<LocalNodeId<Local>>>,
    /// Locals live at exit of each block.
    local_live_out: HashMap<LocalNodeId<Block>, HashSet<LocalNodeId<Local>>>,
}

impl FunctionLiveness {
    /// Build liveness for one MIR function.
    pub fn build(function: &Function, tree: &NodeTree) -> Self {
        // block facts
        let facts = Self::collect_block_facts(function, tree);

        // initial state
        let mut liveness = Self::initialize(function);

        // fixed point
        Self::propagate_to_fixed_point(&mut liveness, function, tree, &facts);

        liveness
    }

    /// Collect local use and def facts for each block.
    fn collect_block_facts(
        function: &Function,
        tree: &NodeTree,
    ) -> HashMap<LocalNodeId<Block>, BlockLivenessFacts> {
        let mut facts = HashMap::new();

        // per block facts
        for &block_id in &function.blocks {
            let block = tree.get(block_id);
            let terminator = tree.get(block.terminator);
            let mut seen_value_defs = HashSet::new();
            let mut seen_local_defs = HashSet::new();
            let mut block_facts = BlockLivenessFacts::default();

            // block parameters
            for parameter in &block.parameters {
                let Some(parameter_value) = concrete_value(parameter.value) else {
                    continue;
                };

                seen_value_defs.insert(parameter_value);
                block_facts.value_def.insert(parameter_value);
            }

            // instructions
            for &instruction_id in &block.instructions {
                let instruction = tree.get(instruction_id);
                Self::record_instruction_value_uses(
                    &mut block_facts,
                    &seen_value_defs,
                    instruction,
                    tree,
                );
                Self::record_instruction_local_uses(
                    &mut block_facts,
                    &seen_local_defs,
                    instruction,
                );
                Self::record_instruction_defs(
                    &mut block_facts,
                    &mut seen_value_defs,
                    &mut seen_local_defs,
                    instruction,
                );
            }

            // terminator uses
            for used in terminator.uses() {
                let Some(used) = concrete_value(used) else {
                    continue;
                };

                if !seen_value_defs.contains(&used) {
                    block_facts.value_use.insert(used);
                }
            }

            facts.insert(block_id, block_facts);
        }

        facts
    }

    /// Record instruction value uses in one block fact set.
    fn record_instruction_value_uses(
        facts: &mut BlockLivenessFacts,
        seen_value_defs: &HashSet<Value>,
        instruction: &Instruction,
        tree: &NodeTree,
    ) {
        for used in instruction.uses() {
            let Some(used) = concrete_value(used) else {
                continue;
            };

            if !seen_value_defs.contains(&used) {
                facts.value_use.insert(used);
            }
        }

        if let Some(arguments) = instruction.argument_slice() {
            for &argument in tree.get_arguments(arguments) {
                let Some(argument) = concrete_value(argument) else {
                    continue;
                };

                if !seen_value_defs.contains(&argument) {
                    facts.value_use.insert(argument);
                }
            }
        }
    }

    /// Record instruction local uses in one block fact set.
    fn record_instruction_local_uses(
        facts: &mut BlockLivenessFacts,
        seen_local_defs: &HashSet<LocalNodeId<Local>>,
        instruction: &Instruction,
    ) {
        match instruction {
            Instruction::LocalGet { local, .. } | Instruction::LocalAddr { local, .. } => {
                let Some(local) = concrete_local(*local) else {
                    return;
                };

                if !seen_local_defs.contains(&local) {
                    facts.local_use.insert(local);
                }
            }
            _ => {}
        }
    }

    /// Record instruction defs in one block fact set.
    fn record_instruction_defs(
        facts: &mut BlockLivenessFacts,
        seen_value_defs: &mut HashSet<Value>,
        seen_local_defs: &mut HashSet<LocalNodeId<Local>>,
        instruction: &Instruction,
    ) {
        if let Some(destination) = instruction.destination() {
            let Some(destination) = concrete_value(destination) else {
                return;
            };

            seen_value_defs.insert(destination);
            facts.value_def.insert(destination);
        }

        if let Instruction::LocalSet { local, .. } = instruction {
            let Some(local) = concrete_local(*local) else {
                return;
            };

            seen_local_defs.insert(local);
            facts.local_def.insert(local);
        }
    }

    /// Initialize empty liveness state for all blocks.
    fn initialize(function: &Function) -> Self {
        let mut liveness = Self::default();

        // empty block state
        for &block_id in &function.blocks {
            liveness.value_live_in.insert(block_id, HashSet::new());
            liveness.value_live_out.insert(block_id, HashSet::new());
            liveness.local_live_in.insert(block_id, HashSet::new());
            liveness.local_live_out.insert(block_id, HashSet::new());
        }

        liveness
    }

    /// Propagate liveness until the block states stabilize.
    fn propagate_to_fixed_point(
        liveness: &mut Self,
        function: &Function,
        tree: &NodeTree,
        facts: &HashMap<LocalNodeId<Block>, BlockLivenessFacts>,
    ) {
        let mut changed = true;

        while changed {
            changed = false;

            for &block_id in function.blocks.iter().rev() {
                if Self::propagate_block(liveness, block_id, tree, facts) {
                    changed = true;
                }
            }
        }
    }

    /// Propagate one block of liveness information.
    fn propagate_block(
        liveness: &mut Self,
        block_id: LocalNodeId<Block>,
        tree: &NodeTree,
        facts: &HashMap<LocalNodeId<Block>, BlockLivenessFacts>,
    ) -> bool {
        let block = tree.get(block_id);
        let terminator = tree.get(block.terminator);
        let facts = facts
            .get(&block_id)
            .unwrap_or_else(|| panic!("missing liveness facts for block: {block_id:?}"));

        // successor live-out
        let mut next_value_live_out = HashSet::new();
        let mut next_local_live_out = HashSet::new();

        for successor in terminator.successors() {
            let BlockReference::Block(successor) = successor else {
                continue;
            };

            if let Some(successor_live_in) = liveness.value_live_in.get(&successor) {
                next_value_live_out.extend(successor_live_in.iter().copied());
            }

            if let Some(successor_live_in) = liveness.local_live_in.get(&successor) {
                next_local_live_out.extend(successor_live_in.iter().copied());
            }
        }

        // block live-in
        let mut next_value_live_in: HashSet<Value> = next_value_live_out
            .difference(&facts.value_def)
            .copied()
            .collect();
        next_value_live_in.extend(facts.value_use.iter().copied());

        let mut next_local_live_in: HashSet<LocalNodeId<Local>> = next_local_live_out
            .difference(&facts.local_def)
            .copied()
            .collect();
        next_local_live_in.extend(facts.local_use.iter().copied());

        let mut changed = false;

        // value live-in
        if next_value_live_in
            != *liveness
                .value_live_in
                .get(&block_id)
                .unwrap_or_else(|| panic!("missing value live-in for block: {block_id:?}"))
        {
            liveness.value_live_in.insert(block_id, next_value_live_in);
            changed = true;
        }

        // value live-out
        if next_value_live_out
            != *liveness
                .value_live_out
                .get(&block_id)
                .unwrap_or_else(|| panic!("missing value live-out for block: {block_id:?}"))
        {
            liveness
                .value_live_out
                .insert(block_id, next_value_live_out);
            changed = true;
        }

        // local live-in
        if next_local_live_in
            != *liveness
                .local_live_in
                .get(&block_id)
                .unwrap_or_else(|| panic!("missing local live-in for block: {block_id:?}"))
        {
            liveness.local_live_in.insert(block_id, next_local_live_in);
            changed = true;
        }

        // local live-out
        if next_local_live_out
            != *liveness
                .local_live_out
                .get(&block_id)
                .unwrap_or_else(|| panic!("missing local live-out for block: {block_id:?}"))
        {
            liveness
                .local_live_out
                .insert(block_id, next_local_live_out);
            changed = true;
        }

        changed
    }

    /// Return the values live at block entry.
    pub fn value_live_in(&self, block: LocalNodeId<Block>) -> &HashSet<Value> {
        self.value_live_in.get(&block).unwrap_or_else(|| {
            static EMPTY: std::sync::OnceLock<HashSet<Value>> = std::sync::OnceLock::new();
            EMPTY.get_or_init(HashSet::new)
        })
    }

    /// Return the values live at block exit.
    pub fn value_live_out(&self, block: LocalNodeId<Block>) -> &HashSet<Value> {
        self.value_live_out.get(&block).unwrap_or_else(|| {
            static EMPTY: std::sync::OnceLock<HashSet<Value>> = std::sync::OnceLock::new();
            EMPTY.get_or_init(HashSet::new)
        })
    }

    /// Return the locals live at block entry.
    pub fn local_live_in(&self, block: LocalNodeId<Block>) -> &HashSet<LocalNodeId<Local>> {
        self.local_live_in.get(&block).unwrap_or_else(|| {
            static EMPTY: std::sync::OnceLock<HashSet<LocalNodeId<Local>>> =
                std::sync::OnceLock::new();
            EMPTY.get_or_init(HashSet::new)
        })
    }

    /// Return the locals live at block exit.
    pub fn local_live_out(&self, block: LocalNodeId<Block>) -> &HashSet<LocalNodeId<Local>> {
        self.local_live_out.get(&block).unwrap_or_else(|| {
            static EMPTY: std::sync::OnceLock<HashSet<LocalNodeId<Local>>> =
                std::sync::OnceLock::new();
            EMPTY.get_or_init(HashSet::new)
        })
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
        tree: &NodeTree,
    ) -> bool {
        let block = tree.get(block_id);
        let terminator = tree.get(block.terminator);

        // later instructions
        for &instruction_id in block.instructions.iter().skip(instruction_index + 1) {
            let instruction = tree.get(instruction_id);

            if instruction
                .uses()
                .iter()
                .copied()
                .filter_map(concrete_value)
                .any(|used| used == value)
            {
                return true;
            }

            if let Some(arguments) = instruction.argument_slice()
                && tree
                    .get_arguments(arguments)
                    .iter()
                    .copied()
                    .filter_map(concrete_value)
                    .any(|argument| argument == value)
            {
                return true;
            }
        }

        // terminator
        if terminator
            .uses()
            .iter()
            .copied()
            .filter_map(concrete_value)
            .any(|used| used == value)
        {
            return true;
        }

        self.is_value_live_out(block_id, value)
    }

    /// Return the values live before one instruction offset in one block.
    pub fn value_live_before_instruction(
        &self,
        tree: &NodeTree,
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
                if let Some(destination) = concrete_value(destination) {
                    live.remove(&destination);
                }
            }

            for used in instruction.uses() {
                if let Some(used) = concrete_value(used) {
                    live.insert(used);
                }
            }

            if let Some(arguments) = instruction.argument_slice() {
                for &argument in tree.get_arguments(arguments) {
                    if let Some(argument) = concrete_value(argument) {
                        live.insert(argument);
                    }
                }
            }
        }

        // terminator
        for used in terminator.uses() {
            if let Some(used) = concrete_value(used) {
                live.insert(used);
            }
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

fn concrete_value(value: ValueReference) -> Option<Value> {
    match value {
        ValueReference::Value(value) => Some(value),
        ValueReference::Missing | ValueReference::Error => None,
    }
}

fn concrete_local(local: LocalReference) -> Option<LocalNodeId<Local>> {
    match local {
        LocalReference::Local(local) => Some(local),
        LocalReference::Missing | LocalReference::Error => None,
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
b0:
    v0: int32 = 1int32
    v1: int32 = 2int32
    v2: int32 = int.add v0, v1
    return v2
}"#,
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
b0(v0: boolean):
    v1: int32 = 42int32
    branch v0, b1, b2
b1:
    return v1
b2:
    v2: int32 = 0int32
    return v2
}"#,
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
b0(v0: boolean):
    v1: int32 = 1int32
    jump b1(v1)
b1(v2: int32):
    v3: int32 = 2int32
    v4: int32 = int.add v2, v3
    branch v0, b1(v4), b2
b2:
    return v4
}"#,
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
b0:
    v0: int32 = 1int32
    v1: int32 = 2int32
    return v1
}"#,
        );

        let function = tree.get(function_id);
        let liveness = FunctionLiveness::build(function, &tree);

        let entry = function.entry.expect("missing entry");

        assert!(!liveness.is_value_live_after_instruction(entry, 0, Value::new(0), &tree));
    }
}
