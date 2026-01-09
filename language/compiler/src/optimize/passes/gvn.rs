use std::collections::{HashMap, HashSet};

use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::AnalysisKind;
use crate::optimize::analyses::DominatorTree;
use crate::optimize::{
    AnalysisPreservation, ExpressionKey, FunctionPass, OptimizationContext, Pass, PassMetadata,
    expression_key_from_instruction, expression_key_substitute, instruction_has_side_effects,
    instruction_substitute_uses, resolve_substitution_chains, terminator_substitute_uses,
};

declare_pass! {
    /// Global Value Numbering.
    ///
    /// Eliminates redundant computations across basic blocks by walking the dominator
    /// tree and propagating available expressions to dominated blocks. This is more
    /// powerful than local CSE because it can eliminate an expression in a block if
    /// the same expression was computed in a dominating block.
    ///
    /// Also performs cross-block aggregate forwarding: if a tuple/struct is constructed
    /// in a dominating block, field extractions in dominated blocks are replaced with
    /// the original operands.
    ///
    /// ```mir
    /// function @before(v0: i32, v1: i32, v2: bool) -> i32 {
    /// block0(v0: i32, v1: i32, v2: bool):
    ///     v3 = iadd v0, v1
    ///     branch v2, block1, block2
    /// block1:
    ///     v4 = iadd v0, v1
    ///     return v4
    /// block2:
    ///     v5 = iadd v0, v1
    ///     return v5
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function @after(v0: i32, v1: i32, v2: bool) -> i32 {
    /// block0(v0: i32, v1: i32, v2: bool):
    ///     v3 = iadd v0, v1
    ///     branch v2, block1, block2
    /// block1:
    ///     return v3
    /// block2:
    ///     return v3
    /// }
    /// ```
    #[pass(id = "gvn")]
    pub GlobalValueNumbering,
    "Eliminate redundant expressions across blocks"
}

impl Pass for GlobalValueNumbering {
    fn metadata(&self) -> &'static PassMetadata {
        GlobalValueNumbering::metadata()
    }
}

impl FunctionPass for GlobalValueNumbering {
    fn run_on_function(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::NodeTree,
        context: &OptimizationContext<'_>,
    ) -> AnalysisPreservation {
        let entry = match function.entry {
            Some(entry) => entry,
            None => return AnalysisPreservation::all(),
        };

        // get dominator tree for this function
        let domtree = context
            .analyses
            .get::<DominatorTree>(function, tree, context);

        // build dominator tree children map for traversal
        let dom_children = build_dominator_children(function, &domtree);

        // run GVN using dominator tree traversal
        let (substitutions, to_remove) =
            find_redundant_expressions(entry, function, tree, &dom_children);

        // nothing to do if no redundancies found
        if to_remove.is_empty() {
            return AnalysisPreservation::all();
        }

        // apply substitutions and remove redundant instructions
        apply_substitutions(function, tree, &substitutions, &to_remove);

        AnalysisPreservation::Some(vec![AnalysisKind::ControlFlowGraph])
    }
}

/// Build a map from each block to its immediate children in the dominator tree.
fn build_dominator_children(
    function: &mir::Function,
    domtree: &DominatorTree,
) -> HashMap<mir::LocalNodeId<mir::Block>, Vec<mir::LocalNodeId<mir::Block>>> {
    let mut children: HashMap<_, Vec<_>> = HashMap::new();

    // initialize all blocks with empty children lists
    for &block_id in &function.blocks {
        children.insert(block_id, Vec::new());
    }

    // build parent -> children mapping from idom relationships
    for &block_id in &function.blocks {
        if let Some(idom) = domtree.immediate_dominator(block_id) {
            children.get_mut(&idom).unwrap().push(block_id);
        }
    }

    children
}

/// Scoped hash table for value numbering across the dominator tree.
///
/// Supports pushing and popping scopes as we enter and leave dominated blocks.
/// Lookups search from innermost to outermost scope.
struct ScopedValueTable {
    /// Stack of scopes, each mapping expression keys to values.
    scopes: Vec<HashMap<ExpressionKey, mir::Value>>,
    /// Stack of scopes for aggregate operands (value -> operand list).
    aggregate_scopes: Vec<HashMap<mir::Value, Vec<mir::Value>>>,
}

impl ScopedValueTable {
    /// Create a new table with a single root scope.
    fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
            aggregate_scopes: vec![HashMap::new()],
        }
    }

    /// Push a new scope (entering a dominated subtree).
    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
        self.aggregate_scopes.push(HashMap::new());
    }

    /// Pop the current scope (leaving a dominated subtree).
    fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
            self.aggregate_scopes.pop();
        }
    }

    /// Look up an expression in all scopes (from innermost to outermost).
    fn get(&self, key: &ExpressionKey) -> Option<mir::Value> {
        for scope in self.scopes.iter().rev() {
            if let Some(&value) = scope.get(key) {
                return Some(value);
            }
        }
        None
    }

    /// Insert an expression into the current (innermost) scope.
    fn insert(&mut self, key: ExpressionKey, value: mir::Value) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(key, value);
        }
    }

    /// Look up aggregate operands in all scopes.
    fn get_aggregate(&self, value: &mir::Value) -> Option<&Vec<mir::Value>> {
        for scope in self.aggregate_scopes.iter().rev() {
            if let Some(operands) = scope.get(value) {
                return Some(operands);
            }
        }
        None
    }

    /// Record aggregate construction operands.
    fn insert_aggregate(&mut self, value: mir::Value, operands: Vec<mir::Value>) {
        if let Some(scope) = self.aggregate_scopes.last_mut() {
            scope.insert(value, operands);
        }
    }
}

/// Find redundant expressions by walking the dominator tree.
///
/// Returns a tuple of (substitutions, instructions_to_remove).
fn find_redundant_expressions(
    entry: mir::LocalNodeId<mir::Block>,
    _function: &mir::Function,
    tree: &mir::NodeTree,
    dom_children: &HashMap<mir::LocalNodeId<mir::Block>, Vec<mir::LocalNodeId<mir::Block>>>,
) -> (
    HashMap<mir::Value, mir::Value>,
    HashSet<mir::LocalNodeId<mir::Instruction>>,
) {
    let mut substitutions: HashMap<mir::Value, mir::Value> = HashMap::new();
    let mut to_remove: HashSet<mir::LocalNodeId<mir::Instruction>> = HashSet::new();
    let mut value_table = ScopedValueTable::new();

    // work stack for dominator tree traversal
    enum Action {
        Enter(mir::LocalNodeId<mir::Block>),
        Leave,
    }

    let mut stack = vec![Action::Enter(entry)];
    while let Some(action) = stack.pop() {
        match action {
            Action::Enter(block_id) => {
                // push a new scope for THIS block's expressions
                // (children will see this scope, siblings will not)
                value_table.push_scope();

                // process instructions in this block
                process_block(
                    block_id,
                    tree,
                    &mut value_table,
                    &mut substitutions,
                    &mut to_remove,
                );

                // schedule Leave FIRST (will be processed AFTER all children)
                stack.push(Action::Leave);

                // schedule children in reverse so first child is processed first
                let children = dom_children.get(&block_id).cloned().unwrap_or_default();
                for child in children.into_iter().rev() {
                    stack.push(Action::Enter(child));
                }
            }
            Action::Leave => {
                value_table.pop_scope();
            }
        }
    }

    // resolve transitive substitution chains
    let substitutions = resolve_substitution_chains(substitutions);

    (substitutions, to_remove)
}

/// Process a single block, recording expressions and finding redundancies.
fn process_block(
    block_id: mir::LocalNodeId<mir::Block>,
    tree: &mir::NodeTree,
    value_table: &mut ScopedValueTable,
    substitutions: &mut HashMap<mir::Value, mir::Value>,
    to_remove: &mut HashSet<mir::LocalNodeId<mir::Instruction>>,
) {
    let block = tree.get(block_id);

    for &instruction_id in &block.instructions {
        let instruction = tree.get(instruction_id);

        // track aggregate construction operands for cross-block forwarding
        match instruction {
            mir::Instruction::Struct {
                destination,
                fields,
                ..
            } => {
                let args = tree.get_arguments(*fields);
                value_table.insert_aggregate(*destination, args.to_vec());
            }
            mir::Instruction::Tuple {
                destination,
                elements,
                ..
            }
            | mir::Instruction::Array {
                destination,
                elements,
                ..
            } => {
                let args = tree.get_arguments(*elements);
                value_table.insert_aggregate(*destination, args.to_vec());
            }
            _ => {}
        }

        // check for aggregate field/element extraction simplification
        let aggregate_simplification = match instruction {
            mir::Instruction::FieldGet {
                destination,
                aggregate,
                index,
                ..
            } => {
                // resolve through any existing substitutions
                let agg = substitutions.get(aggregate).copied().unwrap_or(*aggregate);
                if let Some(operands) = value_table.get_aggregate(&agg) {
                    if let Some(&operand) = operands.get(*index as usize) {
                        Some((*destination, operand, instruction_id))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            mir::Instruction::ElementGet { .. } => {
                // #Incomplete: GVN needs constant propagation for ElementGet (?)
                None
            }
            _ => None,
        };

        if let Some((dest, replacement, inst_id)) = aggregate_simplification {
            substitutions.insert(dest, replacement);
            to_remove.insert(inst_id);
            continue;
        }

        // skip instructions with side effects
        if instruction_has_side_effects(instruction) {
            continue;
        }

        // try to get an expression key
        let Some(key) = expression_key_from_instruction(instruction, tree) else {
            continue;
        };

        // get the destination value
        let Some(destination) = instruction.destination() else {
            continue;
        };

        // apply existing substitutions to the key
        let key = expression_key_substitute(key, substitutions);

        // check if we've seen this expression in any dominating scope
        if let Some(existing_value) = value_table.get(&key) {
            // found a match: mark for substitution and removal
            substitutions.insert(destination, existing_value);
            to_remove.insert(instruction_id);
        }
        // otherwise record as a new expression
        else {
            value_table.insert(key, destination);
        }
    }
}

/// Apply substitutions to all instructions and terminators, and remove redundant instructions.
fn apply_substitutions(
    function: &mir::Function,
    tree: &mut mir::NodeTree,
    substitutions: &HashMap<mir::Value, mir::Value>,
    to_remove: &HashSet<mir::LocalNodeId<mir::Instruction>>,
) {
    // apply substitutions to instructions
    for &block_id in &function.blocks {
        let instruction_ids: Vec<_> = tree.get(block_id).instructions.clone();

        for instruction_id in instruction_ids {
            if to_remove.contains(&instruction_id) {
                continue;
            }

            let instruction = tree.get(instruction_id);
            let new_instruction = instruction_substitute_uses(instruction, substitutions);
            if new_instruction != *instruction {
                tree.replace(instruction_id, new_instruction);
            }
        }
    }

    // apply substitutions to terminators and remove redundant instructions
    for &block_id in &function.blocks {
        let block = tree.get(block_id);
        let new_terminator = terminator_substitute_uses(&block.terminator, substitutions);
        let new_instructions: Vec<_> = block
            .instructions
            .iter()
            .copied()
            .filter(|id| !to_remove.contains(id))
            .collect();

        // update block if anything changed
        if new_terminator != block.terminator || new_instructions.len() != block.instructions.len()
        {
            let mut new_block = block.clone();
            new_block.terminator = new_terminator;
            new_block.instructions = new_instructions;
            tree.replace(block_id, new_block);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Expression in entry block is available in dominated blocks.
    #[test]
    fn test_eliminate_cross_block() {
        let input = r#"function @test(v0: i32, v1: i32, v2: bool) -> i32 {
block0(v0: i32, v1: i32, v2: bool):
    v3 = iadd v0, v1
    branch v2, block1, block2
block1:
    v4 = iadd v0, v1
    return v4
block2:
    v5 = iadd v0, v1
    return v5
}"#;
        let expected = r#"function @test(v0: i32, v1: i32, v2: bool) -> i32 {
block0(v0: i32, v1: i32, v2: bool):
    v3 = iadd v0, v1
    branch v2, block1, block2
block1:
    return v3
block2:
    return v3
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&GlobalValueNumbering);
        program.assert_output(expected);
    }

    /// Expression in block1 is not available in block2 (not dominated).
    #[test]
    fn test_skip_non_dominating_blocks() {
        let input = r#"function @test(v0: i32, v1: i32, v2: bool) -> i32 {
block0(v0: i32, v1: i32, v2: bool):
    branch v2, block1, block2
block1:
    v3 = iadd v0, v1
    jump block3(v3)
block2:
    v4 = iadd v0, v1
    jump block3(v4)
block3(v5: i32):
    return v5
}"#;
        // should be unchanged: block1 doesn't dominate block2
        let mut program = TestProgram::new(input);
        program.run_pass(&GlobalValueNumbering);
        program.assert_unchanged(input);
    }

    /// Expression from entry is available through multiple levels of domination.
    #[test]
    fn test_eliminate_through_dominator_chain() {
        let input = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = iadd v0, v1
    jump block1
block1:
    v3 = imul v2, v2
    jump block2
block2:
    v4 = iadd v0, v1
    v5 = iadd v3, v4
    return v5
}"#;
        let expected = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = iadd v0, v1
    jump block1
block1:
    v3 = imul v2, v2
    jump block2
block2:
    v5 = iadd v3, v2
    return v5
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&GlobalValueNumbering);
        program.assert_output(expected);
    }

    /// Commutative operands (v0 + v1 and v1 + v0) are recognized as equivalent.
    #[test]
    fn test_eliminate_commutative() {
        let input = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = iadd v0, v1
    jump block1
block1:
    v3 = iadd v1, v0
    v4 = iadd v2, v3
    return v4
}"#;
        let expected = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = iadd v0, v1
    jump block1
block1:
    v4 = iadd v2, v2
    return v4
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&GlobalValueNumbering);
        program.assert_output(expected);
    }

    /// Substitutions are applied transitively through multiple redundancies.
    #[test]
    fn test_apply_transitive_substitutions() {
        let input = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = iadd v0, v1
    jump block1
block1:
    v3 = iadd v0, v1
    v4 = imul v3, v3
    jump block2
block2:
    v5 = iadd v0, v1
    v6 = imul v5, v5
    v7 = iadd v4, v6
    return v7
}"#;
        // v3 -> v2, v5 -> v2
        // v4 = imul v2, v2
        // v6 = imul v2, v2 -> v4
        let expected = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = iadd v0, v1
    jump block1
block1:
    v4 = imul v2, v2
    jump block2
block2:
    v7 = iadd v4, v4
    return v7
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&GlobalValueNumbering);
        program.assert_output(expected);
    }

    /// GVN also handles local redundancies within a single block.
    #[test]
    fn test_eliminate_local_redundancies() {
        let input = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = iadd v0, v1
    v3 = iadd v0, v1
    v4 = iadd v2, v3
    return v4
}"#;
        let expected = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = iadd v0, v1
    v4 = iadd v2, v2
    return v4
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&GlobalValueNumbering);
        program.assert_output(expected);
    }

    /// Deeply nested dominator tree is handled correctly.
    #[test]
    fn test_eliminate_through_deep_chain() {
        let input = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = iadd v0, v1
    jump block1
block1:
    jump block2
block2:
    jump block3
block3:
    v3 = iadd v0, v1
    return v3
}"#;
        // v3 should be replaced with v2 through the dominator chain
        let expected = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = iadd v0, v1
    jump block1
block1:
    jump block2
block2:
    jump block3
block3:
    return v2
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&GlobalValueNumbering);
        program.assert_output(expected);
    }

    /// Diamond CFG with expressions in both branches.
    #[test]
    fn test_eliminate_in_diamond_cfg() {
        let input = r#"function @test(v0: i32, v1: i32, v2: bool) -> i32 {
block0(v0: i32, v1: i32, v2: bool):
    v3 = iadd v0, v1
    branch v2, block1, block2
block1:
    v4 = iadd v0, v1
    jump block3(v4)
block2:
    v5 = iadd v0, v1
    jump block3(v5)
block3(v6: i32):
    return v6
}"#;
        // v4 -> v3, v5 -> v3
        let expected = r#"function @test(v0: i32, v1: i32, v2: bool) -> i32 {
block0(v0: i32, v1: i32, v2: bool):
    v3 = iadd v0, v1
    branch v2, block1, block2
block1:
    jump block3(v3)
block2:
    jump block3(v3)
block3(v6: i32):
    return v6
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&GlobalValueNumbering);
        program.assert_output(expected);
    }

    /// Unique expressions are preserved unchanged.
    #[test]
    fn test_preserve_unique_expressions() {
        let input = r#"function @test(v0: i32, v1: i32, v2: bool) -> i32 {
block0(v0: i32, v1: i32, v2: bool):
    v3 = iadd v0, v1
    branch v2, block1, block2
block1:
    v4 = isub v0, v1
    return v4
block2:
    v5 = imul v0, v1
    return v5
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&GlobalValueNumbering);
        program.assert_unchanged(input);
    }

    /// Unary operations are properly GVN'd across blocks.
    #[test]
    fn test_eliminate_unary_cross_block() {
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = ineg v0
    jump block1
block1:
    v2 = ineg v0
    v3 = iadd v1, v2
    return v3
}"#;
        let expected = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = ineg v0
    jump block1
block1:
    v3 = iadd v1, v1
    return v3
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&GlobalValueNumbering);
        program.assert_output(expected);
    }

    /// Field access is properly GVN'd across blocks.
    #[test]
    fn test_eliminate_field_get_cross_block() {
        let input = r#"function @test(v0: (i32, i32)) -> i32 {
block0(v0: (i32, i32)):
    v1 = field.get v0, 0
    jump block1
block1:
    v2 = field.get v0, 0
    v3 = iadd v1, v2
    return v3
}"#;
        let expected = r#"function @test(v0: (i32, i32)) -> i32 {
block0(v0: (i32, i32)):
    v1 = field.get v0, 0
    jump block1
block1:
    v3 = iadd v1, v1
    return v3
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&GlobalValueNumbering);
        program.assert_output(expected);
    }

    /// Multiple independent expressions are all handled.
    #[test]
    fn test_eliminate_multiple_expressions() {
        let input = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = iadd v0, v1
    v3 = imul v0, v1
    jump block1
block1:
    v4 = iadd v0, v1
    v5 = imul v0, v1
    v6 = iadd v4, v5
    return v6
}"#;
        // both v4 and v5 are redundant
        let expected = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = iadd v0, v1
    v3 = imul v0, v1
    jump block1
block1:
    v6 = iadd v2, v3
    return v6
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&GlobalValueNumbering);
        program.assert_output(expected);
    }

    /// Tuple field extraction is forwarded across blocks.
    #[test]
    fn test_aggregate_tuple_cross_block() {
        let input = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = tuple (i32, i32) (v0, v1)
    jump block1
block1:
    v3 = field.get v2, 0
    v4 = field.get v2, 1
    v5 = iadd v3, v4
    return v5
}"#;
        // v3 -> v0, v4 -> v1
        let expected = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = tuple (i32, i32) (v0, v1)
    jump block1
block1:
    v5 = iadd v0, v1
    return v5
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&GlobalValueNumbering);
        program.assert_output(expected);
    }

    /// Struct field extraction is forwarded across blocks.
    #[test]
    fn test_aggregate_struct_cross_block() {
        let input = r#"type @Point = { i32, i32 }
function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = struct @Point (v0, v1)
    jump block1
block1:
    v3 = field.get v2, 0
    v4 = field.get v2, 1
    v5 = iadd v3, v4
    return v5
}"#;
        // v3 -> v0, v4 -> v1
        let expected = r#"type @Point = { i32, i32 }
function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = struct @Point (v0, v1)
    jump block1
block1:
    v5 = iadd v0, v1
    return v5
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&GlobalValueNumbering);
        program.assert_output(expected);
    }

    /// Aggregate forwarding through deep dominator chain.
    #[test]
    fn test_aggregate_through_deep_chain() {
        let input = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = tuple (i32, i32) (v0, v1)
    jump block1
block1:
    jump block2
block2:
    jump block3
block3:
    v3 = field.get v2, 1
    return v3
}"#;
        // v3 -> v1 through the dominator chain
        let expected = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = tuple (i32, i32) (v0, v1)
    jump block1
block1:
    jump block2
block2:
    jump block3
block3:
    return v1
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&GlobalValueNumbering);
        program.assert_output(expected);
    }

    /// Aggregate in non-dominating block is not forwarded.
    #[test]
    fn test_aggregate_skip_non_dominating() {
        let input = r#"function @test(v0: i32, v1: i32, v2: bool) -> i32 {
block0(v0: i32, v1: i32, v2: bool):
    branch v2, block1, block2
block1:
    v3 = tuple (i32, i32) (v0, v1)
    jump block3(v3)
block2:
    v4 = tuple (i32, i32) (v1, v0)
    v5 = field.get v4, 0
    jump block3(v4)
block3(v6: (i32, i32)):
    v7 = field.get v6, 0
    return v7
}"#;
        // block1 doesn't dominate block2, so v3's tuple construction
        // shouldn't affect the field.get in block2
        // v5 should still be simplified to v1 (local simplification via InstructionCombine,
        // but GVN should leave the non-dominated one alone)
        // v7 cannot be simplified because v6 is a block parameter
        let mut program = TestProgram::new(input);
        program.run_pass(&GlobalValueNumbering);
        // v5 should be simplified to v1 since v4 is constructed in same scope
        let expected = r#"function @test(v0: i32, v1: i32, v2: bool) -> i32 {
block0(v0: i32, v1: i32, v2: bool):
    branch v2, block1, block2
block1:
    v3 = tuple (i32, i32) (v0, v1)
    jump block3(v3)
block2:
    v4 = tuple (i32, i32) (v1, v0)
    jump block3(v4)
block3(v6: (i32, i32)):
    v7 = field.get v6, 0
    return v7
}"#;
        program.assert_output(expected);
    }

    /// Diamond CFG with aggregate extraction in both branches.
    #[test]
    fn test_aggregate_diamond_cfg() {
        let input = r#"function @test(v0: i32, v1: i32, v2: bool) -> i32 {
block0(v0: i32, v1: i32, v2: bool):
    v3 = tuple (i32, i32) (v0, v1)
    branch v2, block1, block2
block1:
    v4 = field.get v3, 0
    jump block3(v4)
block2:
    v5 = field.get v3, 1
    jump block3(v5)
block3(v6: i32):
    return v6
}"#;
        // v4 -> v0, v5 -> v1
        let expected = r#"function @test(v0: i32, v1: i32, v2: bool) -> i32 {
block0(v0: i32, v1: i32, v2: bool):
    v3 = tuple (i32, i32) (v0, v1)
    branch v2, block1, block2
block1:
    jump block3(v0)
block2:
    jump block3(v1)
block3(v6: i32):
    return v6
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&GlobalValueNumbering);
        program.assert_output(expected);
    }

    /// Aggregate forwarding combined with regular GVN.
    #[test]
    fn test_aggregate_combined_with_gvn() {
        let input = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = iadd v0, v1
    v3 = tuple (i32, i32) (v2, v1)
    jump block1
block1:
    v4 = iadd v0, v1
    v5 = field.get v3, 0
    v6 = iadd v4, v5
    return v6
}"#;
        // v4 -> v2 (regular GVN), v5 -> v2 (aggregate forwarding)
        let expected = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = iadd v0, v1
    v3 = tuple (i32, i32) (v2, v1)
    jump block1
block1:
    v6 = iadd v2, v2
    return v6
}"#;
        let mut program = TestProgram::new(input);
        program.run_pass(&GlobalValueNumbering);
        program.assert_output(expected);
    }
}
