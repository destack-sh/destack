use std::sync::Arc;

use destack_core::{BitSet, FxIndexMap};

use crate as mir;

use crate::{
    Analysis, ControlTable, ForwardTransfer, FunctionCache, Mutation, NodeTable, PureExpression,
};

use super::{Dataflow, Lattice};

/// Available pure expressions for one function.
#[derive(Debug)]
pub struct ExpressionTable {
    /// Available expressions at entry indexed by block id.
    block_entry: NodeTable<mir::Block, Option<ExpressionState>>,
    /// Available expressions at exit indexed by block id.
    block_exit: NodeTable<mir::Block, Option<ExpressionState>>,
}

/// Set of expressions available at a test point.
#[derive(Debug, Clone)]
pub struct ExpressionState {
    /// The expressions occurring in this function.
    universe: Arc<ExpressionUniverse>,
    /// Expressions available on all paths.
    available: BitSet,
}

/// Interned expressions for one function.
#[derive(Debug)]
struct ExpressionUniverse {
    /// Expressions indexed by dense expression id.
    expressions: Vec<PureExpression>,
    /// Dense ids indexed by expression.
    indices: FxIndexMap<PureExpression, usize>,
}

impl PartialEq for ExpressionState {
    fn eq(&self, other: &Self) -> bool {
        self.available == other.available
    }
}

impl Eq for ExpressionState {}

impl ExpressionState {
    /// Create an empty set for one expression universe.
    fn new(universe: Arc<ExpressionUniverse>) -> Self {
        let available = BitSet::new(universe.expressions.len());

        Self {
            universe,
            available,
        }
    }

    /// Return whether a pure expression is available.
    pub fn contains(&self, key: &PureExpression) -> bool {
        self.universe
            .indices
            .get(key)
            .is_some_and(|index| self.available.contains(*index))
    }

    /// Iterate over available pure expressions.
    pub fn iter(&self) -> impl Iterator<Item = &PureExpression> {
        self.available
            .iter()
            .map(|index| &self.universe.expressions[index])
    }

    /// Return whether the set is empty.
    pub fn is_empty(&self) -> bool {
        self.available.is_empty()
    }

    /// Mark one expression as available.
    fn insert(&mut self, key: &PureExpression) {
        let index = self.universe.indices[key];
        self.available.insert(index);
    }
}

impl Lattice for ExpressionState {
    /// Intersect expressions that are available on all paths.
    fn meet(&self, other: &Self) -> Self {
        let mut available = self.available.clone();
        available.intersect_with(&other.available);

        Self {
            universe: self.universe.clone(),
            available,
        }
    }
}

impl ExpressionTable {
    /// Build available expressions for a function.
    pub fn build(function: &mir::Function, tree: &mir::Tree, cfg: &ControlTable) -> Self {
        let universe = ExpressionUniverse::build(function, tree);
        let entry_state = ExpressionState::new(universe);
        let result =
            Dataflow::forward(function, tree, cfg, entry_state, |transfer, state, tree| {
                match transfer {
                    ForwardTransfer::Block(block) => Self::transfer(block, state, tree),
                    ForwardTransfer::Edge { .. } => state,
                }
            });
        let (block_entry, block_exit) = result.into_parts();

        Self {
            block_entry,
            block_exit,
        }
    }

    /// Return available expressions at block entry.
    pub fn entry(&self, block: mir::LocalNodeId<mir::Block>) -> Option<&ExpressionState> {
        self.block_entry.get(block).as_ref()
    }

    /// Return available expressions at block exit.
    pub fn exit(&self, block: mir::LocalNodeId<mir::Block>) -> Option<&ExpressionState> {
        self.block_exit.get(block).as_ref()
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
                state.insert(&key);
            }
        }

        state
    }
}

impl ExpressionUniverse {
    /// Intern every expression occurring in one function.
    fn build(function: &mir::Function, tree: &mir::Tree) -> Arc<Self> {
        let mut expressions = Vec::new();
        let mut indices = FxIndexMap::default();

        // assign one dense id to each distinct expression
        for &block_id in function.blocks() {
            for &instruction_id in &tree.get(block_id).instructions {
                let instruction = tree.get(instruction_id);
                let Some(expression) = PureExpression::from_instruction(instruction) else {
                    continue;
                };

                if !indices.contains_key(&expression) {
                    let index = expressions.len();
                    indices.insert(expression.clone(), index);
                    expressions.push(expression);
                }
            }
        }

        Arc::new(Self {
            expressions,
            indices,
        })
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
        assert!(
            available
                .entry(block0)
                .is_some_and(ExpressionState::is_empty)
        );

        // confirm the expression is available at the successor entry
        assert!(
            available
                .entry(block1)
                .is_some_and(|state| state.contains(&expression_key))
        );
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
        assert!(
            !available
                .entry(merge_block)
                .is_some_and(|state| state.contains(&expression_key))
        );
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
        assert!(
            available
                .entry(merge_block)
                .is_some_and(|state| state.contains(&expression_key))
        );
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
        assert!(
            available
                .exit(entry_block)
                .is_some_and(ExpressionState::is_empty)
        );
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
        assert!(
            available
                .entry(merge_block)
                .is_some_and(|state| state.contains(&expression_key))
        );
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
        assert!(available.entry(unreachable_block).is_none());
    }
}
