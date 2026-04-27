use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use destack_mir as mir;

use crate::optimize::common::{ExpressionKey, expression_key_from_instruction};
use crate::optimize::{Analysis, AnalysisId, ControlFlowGraph, FunctionAnalyses, FunctionAnalysis};

use super::{Lattice, forward_dataflow};

/// Set of expressions available at a test point.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AvailableExpressionSet {
    /// Expression keys available on all paths.
    expressions: HashSet<ExpressionKey>,
}

impl AvailableExpressionSet {
    /// Create an empty available expression set.
    pub fn new() -> Self {
        // create empty expression storage
        let expressions = HashSet::new();

        Self { expressions }
    }

    /// Insert an available expression key.
    pub fn insert(&mut self, key: ExpressionKey) {
        // record the available expression
        self.expressions.insert(key);
    }

    /// Check whether an expression key is available.
    pub fn contains(&self, key: &ExpressionKey) -> bool {
        self.expressions.contains(key)
    }

    /// Iterate over available expression keys.
    pub fn iter(&self) -> impl Iterator<Item = &ExpressionKey> {
        self.expressions.iter()
    }

    /// Check if the set is empty.
    pub fn is_empty(&self) -> bool {
        self.expressions.is_empty()
    }
}

impl Lattice for AvailableExpressionSet {
    /// Intersect expressions that are available on all paths.
    fn meet(&self, other: &Self) -> Self {
        let expressions = self
            .expressions
            .intersection(&other.expressions)
            .cloned()
            .collect();

        Self { expressions }
    }
}

/// Available expressions analysis for pure computations.
///
/// Only instructions that produce an ExpressionKey are tracked.
/// Memory loads and other side effects are intentionally excluded.
#[derive(Debug)]
pub struct AvailableExpressions {
    /// Available expressions at entry to each block.
    block_entry: HashMap<mir::LocalNodeId<mir::Block>, AvailableExpressionSet>,
    /// Available expressions at exit of each block.
    block_exit: HashMap<mir::LocalNodeId<mir::Block>, AvailableExpressionSet>,
}

impl AvailableExpressions {
    /// Build available expressions for a function.
    fn build(function: &mir::Function, tree: &mir::Tree, cfg: &ControlFlowGraph) -> Self {
        let entry_state = AvailableExpressionSet::new();
        let result = forward_dataflow(function, tree, cfg, entry_state, transfer_block);

        Self {
            block_entry: result.block_entry,
            block_exit: result.block_exit,
        }
    }

    /// Get available expressions at block entry.
    pub fn entry(&self, block: mir::LocalNodeId<mir::Block>) -> &AvailableExpressionSet {
        match self.block_entry.get(&block) {
            Some(expressions) => expressions,
            None => empty_expression_set(),
        }
    }

    /// Get available expressions at block exit.
    pub fn exit(&self, block: mir::LocalNodeId<mir::Block>) -> &AvailableExpressionSet {
        match self.block_exit.get(&block) {
            Some(expressions) => expressions,
            None => empty_expression_set(),
        }
    }

    /// Compute available expressions before an instruction index.
    pub fn expressions_before_instruction(
        &self,
        block: mir::LocalNodeId<mir::Block>,
        instruction_index: usize,
        tree: &mir::Tree,
    ) -> AvailableExpressionSet {
        // read the block data and entry state
        let block_data = tree.get(block);
        let mut state = self.entry(block).clone();

        // extend the set with expressions in the block prefix
        for &instruction_id in block_data.instructions.iter().take(instruction_index) {
            let instruction = tree.get(instruction_id);
            if let Some(key) = expression_key_from_instruction(instruction, tree) {
                state.insert(key);
            }
        }

        state
    }

    /// Compute available expressions after an instruction index.
    ///
    /// If the index is out of bounds, returns the same set as
    /// `expressions_before_instruction`.
    pub fn expressions_after_instruction(
        &self,
        block: mir::LocalNodeId<mir::Block>,
        instruction_index: usize,
        tree: &mir::Tree,
    ) -> AvailableExpressionSet {
        // compute availability before the instruction
        let mut state = self.expressions_before_instruction(block, instruction_index, tree);

        // extend with the expression at the index
        if let Some(&instruction_id) = tree.get(block).instructions.get(instruction_index) {
            let instruction = tree.get(instruction_id);

            if let Some(key) = expression_key_from_instruction(instruction, tree) {
                state.insert(key);
            }
        }

        state
    }
}

impl Analysis for AvailableExpressions {
    const ID: AnalysisId = AnalysisId("available-exprs");
    const DEPENDENCIES: &'static [AnalysisId] = &[ControlFlowGraph::ID];
}

impl FunctionAnalysis for AvailableExpressions {
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

/// Apply available expression transfer over a block.
fn transfer_block(
    block: mir::LocalNodeId<mir::Block>,
    entry_state: AvailableExpressionSet,
    tree: &mir::Tree,
) -> AvailableExpressionSet {
    let block_data = tree.get(block);
    let mut state = entry_state;

    // extend the available set with block expressions
    for &instruction_id in &block_data.instructions {
        let instruction = tree.get(instruction_id);
        if let Some(key) = expression_key_from_instruction(instruction, tree) {
            state.insert(key);
        }
    }

    state
}

/// Return an empty available expression set.
fn empty_expression_set() -> &'static AvailableExpressionSet {
    // cache an empty expression set
    static EMPTY: OnceLock<AvailableExpressionSet> = OnceLock::new();

    EMPTY.get_or_init(AvailableExpressionSet::new)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Return the first expression key in a block.
    fn first_expression_key(
        block: mir::LocalNodeId<mir::Block>,
        tree: &mir::Tree,
    ) -> ExpressionKey {
        // read the block data
        let block_data = tree.get(block);

        // scan instructions for the first expression
        for &instruction_id in &block_data.instructions {
            let instruction = tree.get(instruction_id);

            if let Some(key) = expression_key_from_instruction(instruction, tree) {
                return key;
            }
        }

        panic!("missing expression key")
    }

    /// Single predecessor makes expressions available to successors.
    #[test]
    fn test_available_expressions_linear_flow() {
        let test = TestProgram::new(
            r#"
function test(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    jump b1
b1:
    v3: int32 = int.add v0, v1
    return v3
}"#,
        );

        // fetch the function and analysis
        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let available = analyses.get::<AvailableExpressions>();

        // capture the expression key and successor block
        let block0 = function.blocks[0];
        let block1 = function.blocks[1];
        let expression_key = first_expression_key(block0, &test.tree);

        // confirm entry block starts empty
        assert!(available.entry(block0).is_empty());

        // confirm the expression is available at the successor entry
        assert!(available.entry(block1).contains(&expression_key));
    }

    /// Missing expression on one branch prevents availability at merge.
    #[test]
    fn test_available_expressions_branch_missing() {
        let test = TestProgram::new(
            r#"
function test(v0: boolean, v1: int32, v2: int32): int32 {
b0(v0: boolean, v1: int32, v2: int32):
    branch v0, b1, b2
b1:
    v3: int32 = int.add v1, v2
    jump b3
b2:
    jump b3
b3:
    v4: int32 = int.add v1, v2
    return v4
}"#,
        );

        // fetch the function and analysis
        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let available = analyses.get::<AvailableExpressions>();

        // capture the expression key and merge block
        let block1 = function.blocks[1];
        let merge_block = function.blocks[3];
        let expression_key = first_expression_key(block1, &test.tree);

        // confirm the expression is not available at the merge
        assert!(!available.entry(merge_block).contains(&expression_key));
    }

    /// Matching expressions on both branches are available at merge.
    #[test]
    fn test_available_expressions_branch_merge() {
        let test = TestProgram::new(
            r#"
function test(v0: boolean, v1: int32, v2: int32): int32 {
b0(v0: boolean, v1: int32, v2: int32):
    branch v0, b1, b2
b1:
    v3: int32 = int.add v1, v2
    jump b3
b2:
    v4: int32 = int.add v1, v2
    jump b3
b3:
    v5: int32 = int.add v1, v2
    return v5
}"#,
        );

        // fetch the function and analysis
        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let available = analyses.get::<AvailableExpressions>();

        // capture the expression key and merge block
        let block1 = function.blocks[1];
        let merge_block = function.blocks[3];
        let expression_key = first_expression_key(block1, &test.tree);

        // confirm the expression is available at the merge
        assert!(available.entry(merge_block).contains(&expression_key));
    }

    /// Non expression instructions do not populate the available set.
    #[test]
    fn test_available_expressions_non_expression() {
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
        let available = analyses.get::<AvailableExpressions>();

        // confirm the exit set is empty
        let entry_block = function.entry.expect("missing entry block");
        assert!(available.exit(entry_block).is_empty());
    }

    /// Commutative expressions are treated as the same key across branches.
    #[test]
    fn test_available_expressions_commutative_merge() {
        let test = TestProgram::new(
            r#"
function test(v0: boolean, v1: int32, v2: int32): int32 {
b0(v0: boolean, v1: int32, v2: int32):
    branch v0, b1, b2
b1:
    v3: int32 = int.add v1, v2
    jump b3
b2:
    v4: int32 = int.add v2, v1
    jump b3
b3:
    v5: int32 = int.add v1, v2
    return v5
}"#,
        );

        // fetch the function and analysis
        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let available = analyses.get::<AvailableExpressions>();

        // capture the expression key and merge block
        let block1 = function.blocks[1];
        let merge_block = function.blocks[3];
        let expression_key = first_expression_key(block1, &test.tree);

        // confirm the commutative expression is available at the merge
        assert!(available.entry(merge_block).contains(&expression_key));
    }

    /// Prefix queries expose only the expressions computed so far.
    #[test]
    fn test_available_expressions_prefix_query() {
        let test = TestProgram::new(
            r#"
function test(v0: int32, v1: int32, v2: int32): int32 {
b0(v0: int32, v1: int32, v2: int32):
    v3: int32 = int.add v0, v1
    v4: int32 = int.add v3, v2
    return v4
}"#,
        );

        // fetch the function and analysis
        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let available = analyses.get::<AvailableExpressions>();

        // capture the expression keys
        let entry_block = function.entry.expect("missing entry block");
        let block_data = test.tree.get(entry_block);
        let first_key =
            expression_key_from_instruction(test.tree.get(block_data.instructions[0]), &test.tree)
                .expect("missing first expression");
        let second_key =
            expression_key_from_instruction(test.tree.get(block_data.instructions[1]), &test.tree)
                .expect("missing second expression");

        // confirm no expressions are available before the first instruction
        let before_first = available.expressions_before_instruction(entry_block, 0, &test.tree);
        assert!(before_first.is_empty());

        // confirm only the first expression is available after the first instruction
        let after_first = available.expressions_after_instruction(entry_block, 0, &test.tree);
        assert!(after_first.contains(&first_key));
        assert!(!after_first.contains(&second_key));

        // confirm both expressions are available at block exit
        let exit = available.exit(entry_block);
        assert!(exit.contains(&first_key));
        assert!(exit.contains(&second_key));
    }

    /// Unreachable blocks report no available expressions.
    #[test]
    fn test_available_expressions_unreachable_block() {
        let test = TestProgram::new(
            r#"
function test(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    jump b1
b1:
    v3: int32 = int.add v0, v1
    return v3
b2:
    v4: int32 = int.add v0, v1
    return v4
}"#,
        );

        // fetch the function and analysis
        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let available = analyses.get::<AvailableExpressions>();

        // confirm unreachable block has empty entry
        let unreachable_block = function.blocks[2];
        assert!(available.entry(unreachable_block).is_empty());
    }
}
