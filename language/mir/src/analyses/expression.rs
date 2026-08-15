use std::collections::HashSet;
use std::sync::OnceLock;

use crate as mir;

use crate::{Analysis, ControlTable, FunctionCache, Mutation, NodeTable, PureExpression};

use super::{DataflowResult, Lattice};

/// Available pure expressions for one function.
#[derive(Debug)]
pub struct ExpressionTable {
    /// Available expressions at entry indexed by block id.
    block_entry: NodeTable<mir::Block, Option<ExpressionState>>,
    /// Available expressions at exit indexed by block id.
    block_exit: NodeTable<mir::Block, Option<ExpressionState>>,
}

/// Set of expressions available at a test point.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ExpressionState {
    /// Pure expressions available on all paths.
    expressions: HashSet<PureExpression>,
}

impl ExpressionState {
    /// Create an empty available expression set.
    pub fn new() -> Self {
        // create empty expression storage
        let expressions = HashSet::new();

        Self { expressions }
    }

    /// Insert an available pure expression.
    pub fn insert(&mut self, key: PureExpression) {
        // record the available expression
        self.expressions.insert(key);
    }

    /// Check whether a pure expression is available.
    pub fn contains(&self, key: &PureExpression) -> bool {
        self.expressions.contains(key)
    }

    /// Iterate over available pure expressions.
    pub fn iter(&self) -> impl Iterator<Item = &PureExpression> {
        self.expressions.iter()
    }

    /// Check if the set is empty.
    pub fn is_empty(&self) -> bool {
        self.expressions.is_empty()
    }
}

impl Lattice for ExpressionState {
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

impl ExpressionTable {
    /// Build available expressions for a function.
    pub fn build(function: &mir::Function, tree: &mir::Tree, cfg: &ControlTable) -> Self {
        let entry_state = ExpressionState::new();
        let result = DataflowResult::forward(function, tree, cfg, entry_state, Self::transfer);
        let (block_entry, block_exit) = result.into_parts();

        Self {
            block_entry,
            block_exit,
        }
    }

    /// Get available expressions at block entry.
    pub fn entry(&self, block: mir::LocalNodeId<mir::Block>) -> &ExpressionState {
        let expressions = self.block_entry.get(block);
        match expressions {
            Some(expressions) => expressions,
            None => ExpressionState::empty(),
        }
    }

    /// Get available expressions at block exit.
    pub fn exit(&self, block: mir::LocalNodeId<mir::Block>) -> &ExpressionState {
        let expressions = self.block_exit.get(block);
        match expressions {
            Some(expressions) => expressions,
            None => ExpressionState::empty(),
        }
    }

    /// Compute available expressions before an instruction index.
    pub fn expressions_before_instruction(
        &self,
        block: mir::LocalNodeId<mir::Block>,
        instruction_index: usize,
        tree: &mir::Tree,
    ) -> ExpressionState {
        // read the block and entry state
        let block_data = tree.get(block);
        let mut state = self.entry(block).clone();

        // extend the set with expressions in the block prefix
        for &instruction_id in block_data.instructions.iter().take(instruction_index) {
            let instruction = tree.get(instruction_id);
            if let Some(key) = PureExpression::from_instruction(instruction) {
                state.insert(key);
            }
        }

        state
    }

    /// Return available expressions after one instruction index.
    pub fn expressions_after_instruction(
        &self,
        block: mir::LocalNodeId<mir::Block>,
        instruction_index: usize,
        tree: &mir::Tree,
    ) -> ExpressionState {
        // compute availability before the instruction
        let mut state = self.expressions_before_instruction(block, instruction_index, tree);

        // extend with the expression at the index
        if let Some(&instruction_id) = tree.get(block).instructions.get(instruction_index) {
            let instruction = tree.get(instruction_id);

            if let Some(key) = PureExpression::from_instruction(instruction) {
                state.insert(key);
            }
        }

        state
    }
}

impl Analysis for ExpressionTable {
    const INVALIDATED_BY: Mutation = Mutation::CONTROL.union(Mutation::VALUE);
}

impl ExpressionTable {
    /// Compute available expressions for one function.
    pub(crate) fn compute(
        function: &mir::Function,
        tree: &mir::Tree,
        analyses: &mut FunctionCache,
    ) -> Self {
        let control = analyses.control(function, tree);

        Self::build(function, tree, &control)
    }

    /// Apply available expression transfer over a block.
    fn transfer(
        block: mir::LocalNodeId<mir::Block>,
        entry_state: ExpressionState,
        tree: &mir::Tree,
    ) -> ExpressionState {
        let block_data = tree.get(block);
        let mut state = entry_state;

        // extend the available set with block expressions
        for &instruction_id in &block_data.instructions {
            let instruction = tree.get(instruction_id);
            if let Some(key) = PureExpression::from_instruction(instruction) {
                state.insert(key);
            }
        }

        state
    }
}

impl ExpressionState {
    /// Return a shared empty expression set.
    fn empty() -> &'static Self {
        // initialize the shared empty set
        static EMPTY: OnceLock<ExpressionState> = OnceLock::new();

        EMPTY.get_or_init(Self::new)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyses::tests::TestProgram;

    /// Return the first expression key in a block.
    fn first_expression_key(
        block: mir::LocalNodeId<mir::Block>,
        tree: &mir::Tree,
    ) -> PureExpression {
        // read the block
        let block_data = tree.get(block);

        // scan instructions for the first expression
        for &instruction_id in &block_data.instructions {
            let instruction = tree.get(instruction_id);

            if let Some(key) = PureExpression::from_instruction(instruction) {
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
entry(v0: int32, v1: int32):
    v2: int32 = add v0, v1
    jump b1

b1:
    v3: int32 = add v0, v1
    return v3
}
"#,
        );

        // fetch the function and analysis
        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let cfg = ControlTable::build(function, &test.tree);
        let available = ExpressionTable::build(function, &test.tree, &cfg);

        // capture the expression key and successor block
        let block0 = function.block(0);
        let block1 = function.block(1);
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
entry(v0: boolean, v1: int32, v2: int32):
    branch v0 => b1 | b2

b1:
    v3: int32 = add v1, v2
    jump b3

b2:
    jump b3

b3:
    v4: int32 = add v1, v2
    return v4
}
"#,
        );

        // fetch the function and analysis
        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let cfg = ControlTable::build(function, &test.tree);
        let available = ExpressionTable::build(function, &test.tree, &cfg);

        // capture the expression key and merge block
        let block1 = function.block(1);
        let merge_block = function.block(3);
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
entry(v0: boolean, v1: int32, v2: int32):
    branch v0 => b1 | b2

b1:
    v3: int32 = add v1, v2
    jump b3

b2:
    v4: int32 = add v1, v2
    jump b3

b3:
    v5: int32 = add v1, v2
    return v5
}
"#,
        );

        // fetch the function and analysis
        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let cfg = ControlTable::build(function, &test.tree);
        let available = ExpressionTable::build(function, &test.tree, &cfg);

        // capture the expression key and merge block
        let block1 = function.block(1);
        let merge_block = function.block(3);
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
    local l0: int32

entry(v0: int32):
    local.set l0, v0
    v1: int32 = local.get l0
    return v1
}
"#,
        );

        // fetch the function and analysis
        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let cfg = ControlTable::build(function, &test.tree);
        let available = ExpressionTable::build(function, &test.tree, &cfg);

        // confirm the exit set is empty
        let entry_block = function.entry().expect("missing entry block");
        assert!(available.exit(entry_block).is_empty());
    }

    /// Commutative expressions are treated as the same key across branches.
    #[test]
    fn test_available_expressions_commutative_merge() {
        let test = TestProgram::new(
            r#"
function test(v0: boolean, v1: int32, v2: int32): int32 {
entry(v0: boolean, v1: int32, v2: int32):
    branch v0 => b1 | b2

b1:
    v3: int32 = add v1, v2
    jump b3

b2:
    v4: int32 = add v2, v1
    jump b3

b3:
    v5: int32 = add v1, v2
    return v5
}
"#,
        );

        // fetch the function and analysis
        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let cfg = ControlTable::build(function, &test.tree);
        let available = ExpressionTable::build(function, &test.tree, &cfg);

        // capture the expression key and merge block
        let block1 = function.block(1);
        let merge_block = function.block(3);
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
entry(v0: int32, v1: int32, v2: int32):
    v3: int32 = add v0, v1
    v4: int32 = add v3, v2
    return v4
}
"#,
        );

        // fetch the function and analysis
        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let cfg = ControlTable::build(function, &test.tree);
        let available = ExpressionTable::build(function, &test.tree, &cfg);

        // capture the expression keys
        let entry_block = function.entry().expect("missing entry block");
        let block_data = test.tree.get(entry_block);
        let first_key = PureExpression::from_instruction(test.tree.get(block_data.instructions[0]))
            .expect("missing first expression");
        let second_key =
            PureExpression::from_instruction(test.tree.get(block_data.instructions[1]))
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
entry(v0: int32, v1: int32):
    v2: int32 = add v0, v1
    jump b1

b1:
    v3: int32 = add v0, v1
    return v3

b2:
    v4: int32 = add v0, v1
    return v4
}
"#,
        );

        // fetch the function and analysis
        let function_id = test.first_function_id();
        let function = test.tree.get(function_id);
        let cfg = ControlTable::build(function, &test.tree);
        let available = ExpressionTable::build(function, &test.tree, &cfg);

        // confirm unreachable block has empty entry
        let unreachable_block = function.block(2);
        assert!(available.entry(unreachable_block).is_empty());
    }
}
