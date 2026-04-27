use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use destack_mir as mir;
use mir::{Instruction, Local};

use crate::optimize::{Analysis, AnalysisId, ControlFlowGraph, FunctionAnalyses, FunctionAnalysis};

use super::{Lattice, forward_dataflow};

/// Definition site for a local variable.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LocalDefinition {
    /// Definition that originates at function entry (no local.set on the path).
    Entry,
    /// Definition produced by a specific local.set instruction.
    Instruction(mir::LocalNodeId<Instruction>),
}

/// Reaching definitions for locals at a test point.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReachingDefinitionMap {
    /// Definitions available for each local.
    definitions: HashMap<mir::LocalNodeId<Local>, HashSet<LocalDefinition>>,
}

impl ReachingDefinitionMap {
    /// Create an empty definition map.
    pub fn new() -> Self {
        // create empty definition storage
        let definitions = HashMap::new();

        Self { definitions }
    }

    /// Seed entry definitions for all locals in a function.
    pub fn with_entry_definitions(function: &mir::Function) -> Self {
        // create definition storage
        let mut definitions = HashMap::new();

        // assign entry definitions for each local
        for &local in &function.locals {
            let mut local_definitions = HashSet::new();
            local_definitions.insert(LocalDefinition::Entry);

            definitions.insert(local, local_definitions);
        }

        Self { definitions }
    }

    /// Get the reaching definitions for a local.
    pub fn definitions_for(&self, local: mir::LocalNodeId<Local>) -> &HashSet<LocalDefinition> {
        // read the definitions for the local

        match self.definitions.get(&local) {
            Some(definitions) => definitions,
            None => empty_definition_set(),
        }
    }

    /// Overwrite the reaching definitions for a local.
    pub fn set_definition(&mut self, local: mir::LocalNodeId<Local>, definition: LocalDefinition) {
        // build the single definition set
        let mut definitions = HashSet::new();
        definitions.insert(definition);

        // store the definitions for the local
        self.definitions.insert(local, definitions);
    }

    /// Iterate over locals and their reaching definitions.
    pub fn iter(
        &self,
    ) -> impl Iterator<Item = (mir::LocalNodeId<Local>, &HashSet<LocalDefinition>)> + '_ {
        self.definitions.iter().map(|(local, defs)| (*local, defs))
    }
}

impl Lattice for ReachingDefinitionMap {
    /// Union reaching definitions from incoming paths.
    fn meet(&self, other: &Self) -> Self {
        // clone the current definitions
        let mut definitions = self.definitions.clone();

        // merge reaching definitions for each local
        for (local, other_defs) in &other.definitions {
            definitions
                .entry(*local)
                .and_modify(|current| {
                    current.extend(other_defs.iter().cloned());
                })
                .or_insert_with(|| other_defs.clone());
        }

        Self { definitions }
    }
}

/// Reaching definitions analysis for local variables.
#[derive(Debug)]
pub struct ReachingDefinitions {
    /// Reaching definitions at entry to each block.
    block_entry: HashMap<mir::LocalNodeId<mir::Block>, ReachingDefinitionMap>,
    /// Reaching definitions at exit of each block.
    block_exit: HashMap<mir::LocalNodeId<mir::Block>, ReachingDefinitionMap>,
}

impl ReachingDefinitions {
    /// Build reaching definitions for a function.
    fn build(function: &mir::Function, tree: &mir::Tree, cfg: &ControlFlowGraph) -> Self {
        // seed entry state
        let entry_state = ReachingDefinitionMap::with_entry_definitions(function);

        // run forward dataflow
        let result = forward_dataflow(function, tree, cfg, entry_state, transfer_block);

        Self {
            block_entry: result.block_entry,
            block_exit: result.block_exit,
        }
    }

    /// Get reaching definitions at block entry.
    pub fn entry(&self, block: mir::LocalNodeId<mir::Block>) -> &ReachingDefinitionMap {
        // read the entry map

        match self.block_entry.get(&block) {
            Some(definitions) => definitions,
            None => empty_definition_map(),
        }
    }

    /// Get reaching definitions at block exit.
    pub fn exit(&self, block: mir::LocalNodeId<mir::Block>) -> &ReachingDefinitionMap {
        // read the exit map

        match self.block_exit.get(&block) {
            Some(definitions) => definitions,
            None => empty_definition_map(),
        }
    }

    /// Get the reaching definitions for a local at block entry.
    pub fn definitions_at_entry(
        &self,
        block: mir::LocalNodeId<mir::Block>,
        local: mir::LocalNodeId<Local>,
    ) -> &HashSet<LocalDefinition> {
        self.entry(block).definitions_for(local)
    }

    /// Get the reaching definitions for a local at block exit.
    pub fn definitions_at_exit(
        &self,
        block: mir::LocalNodeId<mir::Block>,
        local: mir::LocalNodeId<Local>,
    ) -> &HashSet<LocalDefinition> {
        self.exit(block).definitions_for(local)
    }

    /// Compute reaching definitions before an instruction index.
    pub fn definitions_before_instruction(
        &self,
        block: mir::LocalNodeId<mir::Block>,
        instruction_index: usize,
        tree: &mir::Tree,
    ) -> ReachingDefinitionMap {
        // read the block data and entry state
        let block_data = tree.get(block);
        let mut state = self.entry(block).clone();

        // update definitions for local.set instructions before the index
        for &instruction_id in block_data.instructions.iter().take(instruction_index) {
            let instruction = tree.get(instruction_id);

            if let Instruction::LocalSet { local, .. } = instruction
                && let Some(local) = local.local()
            {
                state.set_definition(local, LocalDefinition::Instruction(instruction_id));
            }
        }

        state
    }
}

impl Analysis for ReachingDefinitions {
    const ID: AnalysisId = AnalysisId("reaching-defs");
    const DEPENDENCIES: &'static [AnalysisId] = &[ControlFlowGraph::ID];
}

impl FunctionAnalysis for ReachingDefinitions {
    fn compute(
        function: &mir::Function,
        tree: &mir::Tree,
        analyses: &FunctionAnalyses<'_>,
    ) -> Self {
        // read the control flow graph
        let cfg = analyses.get::<ControlFlowGraph>();

        Self::build(function, tree, &cfg)
    }
}

/// Apply a reaching definitions transfer over a block.
fn transfer_block(
    block: mir::LocalNodeId<mir::Block>,
    entry_state: ReachingDefinitionMap,
    tree: &mir::Tree,
) -> ReachingDefinitionMap {
    // read block data and entry state
    let block_data = tree.get(block);
    let mut state = entry_state;

    // update reaching definitions for each local.set
    for &instruction_id in &block_data.instructions {
        let instruction = tree.get(instruction_id);
        if let Instruction::LocalSet { local, .. } = instruction
            && let Some(local) = local.local()
        {
            state.set_definition(local, LocalDefinition::Instruction(instruction_id));
        }
    }

    state
}

/// Return an empty reaching definition map.
fn empty_definition_map() -> &'static ReachingDefinitionMap {
    // cache an empty definition map
    static EMPTY: OnceLock<ReachingDefinitionMap> = OnceLock::new();

    EMPTY.get_or_init(ReachingDefinitionMap::new)
}

/// Return an empty definition set.
fn empty_definition_set() -> &'static HashSet<LocalDefinition> {
    // cache an empty definition set
    static EMPTY: OnceLock<HashSet<LocalDefinition>> = OnceLock::new();

    EMPTY.get_or_init(HashSet::new)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Return the first local set instruction in a block.
    fn first_local_set_instruction(
        block: mir::LocalNodeId<mir::Block>,
        tree: &mir::Tree,
    ) -> mir::LocalNodeId<Instruction> {
        // read the block data
        let block_data = tree.get(block);

        // scan instructions for a local.set
        for &instruction_id in &block_data.instructions {
            let instruction = tree.get(instruction_id);
            if matches!(instruction, Instruction::LocalSet { .. }) {
                return instruction_id;
            }
        }

        panic!("missing local.set instruction")
    }

    /// Local set overwrites entry definition within a block.
    #[test]
    fn test_reaching_definitions_single_block() {
        let test = TestProgram::new(
            r#"
function test(v0: int32): int32 {
    local local0: int32, owned
b0(v0: int32):
    local.set local0, v0
    v1: int32 = local.get local0
    return v1
}"#,
        );

        // fetch the function and analysis
        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let reaching = analyses.get::<ReachingDefinitions>();

        // capture the local and entry block
        let local_id = function.locals[0];
        let entry_block = function.entry.expect("missing entry block");

        // confirm entry state contains the entry definition
        let mut expected_entry = HashSet::new();
        expected_entry.insert(LocalDefinition::Entry);

        let entry_defs = reaching.definitions_at_entry(entry_block, local_id);
        assert_eq!(entry_defs, &expected_entry);

        // confirm the local.set definition reaches the local.get
        let before_get = reaching.definitions_before_instruction(entry_block, 1, &test.tree);
        let local_set = test.tree.get(entry_block).instructions[0];

        let mut expected_before = HashSet::new();
        expected_before.insert(LocalDefinition::Instruction(local_set));

        let before_defs = before_get.definitions_for(local_id);
        assert_eq!(before_defs, &expected_before);

        // confirm exit state reflects the latest definition
        let exit_defs = reaching.definitions_at_exit(entry_block, local_id);
        assert_eq!(exit_defs, &expected_before);
    }

    /// Entry definitions appear before the first instruction.
    #[test]
    fn test_reaching_definitions_before_first_instruction() {
        let test = TestProgram::new(
            r#"
function test(v0: int32): int32 {
    local local0: int32, owned
b0(v0: int32):
    local.set local0, v0
    v1: int32 = local.get local0
    return v1
}"#,
        );

        // fetch the function and analysis
        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let reaching = analyses.get::<ReachingDefinitions>();

        // capture the local and entry block
        let local_id = function.locals[0];
        let entry_block = function.entry.expect("missing entry block");

        // confirm entry definitions before the first instruction
        let mut expected_entry = HashSet::new();
        expected_entry.insert(LocalDefinition::Entry);

        let before_state = reaching.definitions_before_instruction(entry_block, 0, &test.tree);
        let before_defs = before_state.definitions_for(local_id);
        assert_eq!(before_defs, &expected_entry);
    }

    /// Later local.set overwrites earlier definitions within a block.
    #[test]
    fn test_reaching_definitions_overwrite_in_block() {
        let test = TestProgram::new(
            r#"
function test(v0: int32, v1: int32): int32 {
    local local0: int32, owned
b0(v0: int32, v1: int32):
    local.set local0, v0
    local.set local0, v1
    v2: int32 = local.get local0
    return v2
}"#,
        );

        // fetch the function and analysis
        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let reaching = analyses.get::<ReachingDefinitions>();

        // capture the local and entry block
        let local_id = function.locals[0];
        let entry_block = function.entry.expect("missing entry block");
        let block = test.tree.get(entry_block);
        let first_set = block.instructions[0];
        let second_set = block.instructions[1];

        // confirm the first local.set reaches the second instruction
        let mut expected_first = HashSet::new();
        expected_first.insert(LocalDefinition::Instruction(first_set));

        let before_state = reaching.definitions_before_instruction(entry_block, 1, &test.tree);
        let before_second = before_state.definitions_for(local_id);
        assert_eq!(before_second, &expected_first);

        // confirm the second local.set overwrites the first definition
        let mut expected_second = HashSet::new();
        expected_second.insert(LocalDefinition::Instruction(second_set));

        let after_state = reaching.definitions_before_instruction(entry_block, 2, &test.tree);
        let after_second = after_state.definitions_for(local_id);
        assert_eq!(after_second, &expected_second);

        // confirm exit state reflects the last definition
        let exit_defs = reaching.definitions_at_exit(entry_block, local_id);
        assert_eq!(exit_defs, &expected_second);
    }

    /// Merge keeps entry definition when a path misses a local set.
    #[test]
    fn test_reaching_definitions_branch_with_entry() {
        let test = TestProgram::new(
            r#"
function test(v0: boolean): int32 {
    local local0: int32, owned
b0(v0: boolean):
    branch v0, b1, b2
b1:
    v1: int32 = 1int32
    local.set local0, v1
    jump b3
b2:
    jump b3
b3:
    v2: int32 = local.get local0
    return v2
}"#,
        );

        // fetch the function and analysis
        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let reaching = analyses.get::<ReachingDefinitions>();

        // collect ids for the join block and local
        let local_id = function.locals[0];
        let join_block = function.blocks[3];

        let local_set = first_local_set_instruction(function.blocks[1], &test.tree);

        // expect entry and the branch definition at the join
        let mut expected = HashSet::new();
        expected.insert(LocalDefinition::Entry);
        expected.insert(LocalDefinition::Instruction(local_set));

        let join_defs = reaching.definitions_at_entry(join_block, local_id);
        assert_eq!(join_defs, &expected);
    }

    /// Local reachability is tracked independently per local.
    #[test]
    fn test_reaching_definitions_multiple_locals() {
        let test = TestProgram::new(
            r#"
function test(v0: boolean, v1: int32, v2: int32): int32 {
    local local0: int32, owned
    local local1: int32, owned
b0(v0: boolean, v1: int32, v2: int32):
    branch v0, b1, b2
b1:
    local.set local0, v1
    jump b3
b2:
    local.set local1, v2
    jump b3
b3:
    v3: int32 = local.get local0
    v4: int32 = local.get local1
    v5: int32 = int.add v3, v4
    return v5
}"#,
        );

        // fetch the function and analysis
        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let reaching = analyses.get::<ReachingDefinitions>();

        // collect ids for the join block and locals
        let local0 = function.locals[0];
        let local1 = function.locals[1];
        let join_block = function.blocks[3];

        let set_local0 = first_local_set_instruction(function.blocks[1], &test.tree);
        let set_local1 = first_local_set_instruction(function.blocks[2], &test.tree);

        // expect entry and local0 definition at the join
        let mut expected_local0 = HashSet::new();
        expected_local0.insert(LocalDefinition::Entry);
        expected_local0.insert(LocalDefinition::Instruction(set_local0));

        let local0_defs = reaching.definitions_at_entry(join_block, local0);
        assert_eq!(local0_defs, &expected_local0);

        // expect entry and local1 definition at the join
        let mut expected_local1 = HashSet::new();
        expected_local1.insert(LocalDefinition::Entry);
        expected_local1.insert(LocalDefinition::Instruction(set_local1));

        let local1_defs = reaching.definitions_at_entry(join_block, local1);
        assert_eq!(local1_defs, &expected_local1);
    }

    /// Merge joins definitions from both branches without entry.
    #[test]
    fn test_reaching_definitions_branch_merge() {
        let test = TestProgram::new(
            r#"
function test(v0: boolean): int32 {
    local local0: int32, owned
b0(v0: boolean):
    branch v0, b1, b2
b1:
    v1: int32 = 1int32
    local.set local0, v1
    jump b3
b2:
    v2: int32 = 2int32
    local.set local0, v2
    jump b3
b3:
    v3: int32 = local.get local0
    return v3
}"#,
        );

        // fetch the function and analysis
        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let reaching = analyses.get::<ReachingDefinitions>();

        // collect ids for the join block and local
        let local_id = function.locals[0];
        let join_block = function.blocks[3];

        let local_set_then = first_local_set_instruction(function.blocks[1], &test.tree);
        let local_set_else = first_local_set_instruction(function.blocks[2], &test.tree);

        // expect both branch definitions at the join
        let mut expected = HashSet::new();
        expected.insert(LocalDefinition::Instruction(local_set_then));
        expected.insert(LocalDefinition::Instruction(local_set_else));

        let join_defs = reaching.definitions_at_entry(join_block, local_id);
        assert_eq!(join_defs, &expected);
    }

    /// Loop headers merge entry and backedge definitions.
    #[test]
    fn test_reaching_definitions_loop_backedge() {
        let test = TestProgram::new(
            r#"
function test(v0: boolean): int32 {
    local local0: int32, owned
b0(v0: boolean):
    jump b1
b1:
    v1: int32 = local.get local0
    v2: int32 = 1int32
    v3: int32 = int.add v1, v2
    local.set local0, v3
    branch v0, b1, b2
b2:
    v4: int32 = local.get local0
    return v4
}"#,
        );

        // fetch the function and analysis
        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let reaching = analyses.get::<ReachingDefinitions>();

        // collect ids for the loop header and local
        let local_id = function.locals[0];
        let loop_header = function.blocks[1];

        let local_set = first_local_set_instruction(loop_header, &test.tree);

        // expect entry and backedge definitions at the loop header
        let mut expected = HashSet::new();
        expected.insert(LocalDefinition::Entry);
        expected.insert(LocalDefinition::Instruction(local_set));

        let header_defs = reaching.definitions_at_entry(loop_header, local_id);
        assert_eq!(header_defs, &expected);
    }

    /// Unreachable blocks have no reaching definitions.
    #[test]
    fn test_reaching_definitions_unreachable_block() {
        let test = TestProgram::new(
            r#"
function test(v0: int32): int32 {
    local local0: int32, owned
b0(v0: int32):
    local.set local0, v0
    jump b1
b1:
    v1: int32 = local.get local0
    return v1
b2:
    v2: int32 = local.get local0
    return v2
}"#,
        );

        // fetch the function and analysis
        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let reaching = analyses.get::<ReachingDefinitions>();

        // capture the unreachable block
        let local_id = function.locals[0];
        let unreachable_block = function.blocks[2];

        // confirm the unreachable block has no reaching definitions
        let entry_defs = reaching.definitions_at_entry(unreachable_block, local_id);
        assert!(entry_defs.is_empty());
    }
}
