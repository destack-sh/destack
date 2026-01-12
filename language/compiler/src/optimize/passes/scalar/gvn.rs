use std::collections::{HashMap, HashSet};

use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::optimize::analyses::{AliasAnalysis, DominatorTree};
use crate::optimize::{
    AnalysisPreservation, ExpressionKey, FunctionAnalyses, FunctionPass, PipelineContext,
    apply_substitutions_in_function, expression_key_from_instruction, expression_key_substitute,
    instruction_has_side_effects, instruction_may_affect_memory, resolve_substitution_chains,
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

impl FunctionPass for GlobalValueNumbering {
    fn run(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::NodeTree,
        _ctx: &PipelineContext<'_>,
    ) -> AnalysisPreservation {
        // skip empty functions
        let entry = match function.entry {
            Some(entry) => entry,
            None => return AnalysisPreservation::all(),
        };

        // get dominator tree children map (borrow immutably for analysis)
        let analyses = FunctionAnalyses::new(function, tree);
        let domtree = analyses.get::<DominatorTree>();
        let alias = analyses.get::<AliasAnalysis>();
        let dom_children = build_dominator_children(function, domtree.as_ref());

        // run GVN
        let changed = run_gvn(entry, function, tree, &dom_children, &alias);

        // preserve analyses when nothing changed
        if changed {
            AnalysisPreservation::none()
        } else {
            AnalysisPreservation::all()
        }
    }

    fn name(&self) -> &'static str {
        "GlobalValueNumbering"
    }

    fn id(&self) -> &'static str {
        "gvn"
    }
}

/// Core GVN logic. Returns true if changes were made.
fn run_gvn(
    entry: mir::LocalNodeId<mir::Block>,
    function: &mir::Function,
    tree: &mut mir::NodeTree,
    dom_children: &HashMap<mir::LocalNodeId<mir::Block>, Vec<mir::LocalNodeId<mir::Block>>>,
    alias: &AliasAnalysis,
) -> bool {
    // run GVN using dominator tree traversal
    let (substitutions, to_remove) =
        find_redundant_expressions(entry, function, tree, dom_children, alias);

    // nothing to do if no redundancies found
    if to_remove.is_empty() {
        return false;
    }

    // apply substitutions and remove redundant instructions
    apply_substitutions_in_function(function, tree, &substitutions, Some(&to_remove));

    true
}

/// Build a map from each block to its immediate children in the dominator tree.
fn build_dominator_children(
    function: &mir::Function,
    domtree: &DominatorTree,
) -> HashMap<mir::LocalNodeId<mir::Block>, Vec<mir::LocalNodeId<mir::Block>>> {
    // allocate the child map
    let mut children: HashMap<_, Vec<_>> = HashMap::new();

    // initialize all blocks with empty children lists
    for &block_id in &function.blocks {
        children.insert(block_id, Vec::new());
    }

    // build parent -> children mapping from idom relationships
    for &block_id in &function.blocks {
        // skip the root without a dominator
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
    /// Stack of scopes for load forwarding.
    memory_scopes: Vec<Vec<MemoryEntry>>,
    /// Stack of scopes for local forwarding.
    local_scopes: Vec<HashMap<mir::LocalNodeId<mir::Local>, mir::Value>>,
}

/// Memory entry tracked for load forwarding.
#[derive(Clone)]
struct MemoryEntry {
    /// Memory location accessed by the load.
    location: crate::optimize::common::MemoryLocation,
    /// Value produced by the load.
    value: mir::Value,
}

impl ScopedValueTable {
    /// Create a new table with a single root scope.
    fn new() -> Self {
        // seed each scope stack with a root entry
        Self {
            scopes: vec![HashMap::new()],
            aggregate_scopes: vec![HashMap::new()],
            memory_scopes: vec![Vec::new()],
            local_scopes: vec![HashMap::new()],
        }
    }

    /// Push a new scope (entering a dominated subtree).
    fn push_scope(&mut self) {
        // push a new scope for each tracked category
        self.scopes.push(HashMap::new());
        self.aggregate_scopes.push(HashMap::new());
        self.memory_scopes.push(Vec::new());
        self.local_scopes.push(HashMap::new());
    }

    /// Pop the current scope (leaving a dominated subtree).
    fn pop_scope(&mut self) {
        // keep at least the root scope
        if self.scopes.len() > 1 {
            self.scopes.pop();
            self.aggregate_scopes.pop();
            self.memory_scopes.pop();
            self.local_scopes.pop();
        }
    }

    /// Look up an expression in all scopes (from innermost to outermost).
    fn get(&self, key: &ExpressionKey) -> Option<mir::Value> {
        // search from innermost to outermost scope
        for scope in self.scopes.iter().rev() {
            if let Some(&value) = scope.get(key) {
                return Some(value);
            }
        }
        None
    }

    /// Insert an expression into the current (innermost) scope.
    fn insert(&mut self, key: ExpressionKey, value: mir::Value) {
        // insert into the current scope when available
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(key, value);
        }
    }

    /// Look up aggregate operands in all scopes.
    fn get_aggregate(&self, value: &mir::Value) -> Option<&Vec<mir::Value>> {
        // search aggregate scopes from innermost to outermost
        for scope in self.aggregate_scopes.iter().rev() {
            if let Some(operands) = scope.get(value) {
                return Some(operands);
            }
        }
        None
    }

    /// Record aggregate construction operands.
    fn insert_aggregate(&mut self, value: mir::Value, operands: Vec<mir::Value>) {
        // insert into the current aggregate scope
        if let Some(scope) = self.aggregate_scopes.last_mut() {
            scope.insert(value, operands);
        }
    }

    /// Look up a forwarded local value.
    fn get_local(&self, local: mir::LocalNodeId<mir::Local>) -> Option<mir::Value> {
        // search local scopes from innermost to outermost
        for scope in self.local_scopes.iter().rev() {
            if let Some(value) = scope.get(&local) {
                return Some(*value);
            }
        }
        None
    }

    /// Record a forwarded local value.
    fn insert_local(&mut self, local: mir::LocalNodeId<mir::Local>, value: mir::Value) {
        // insert into the current local scope
        if let Some(scope) = self.local_scopes.last_mut() {
            scope.insert(local, value);
        }
    }

    /// Look up a forwarded load value.
    fn get_memory(
        &self,
        location: &crate::optimize::common::MemoryLocation,
        alias: &AliasAnalysis,
    ) -> Option<mir::Value> {
        // scan memory scopes from innermost to outermost
        for scope in self.memory_scopes.iter().rev() {
            // scan entries from newest to oldest
            for entry in scope.iter().rev() {
                // treat identical pointers as a match
                if entry.location.ptr == location.ptr {
                    return Some(entry.value);
                }

                // consult alias analysis for the memory location
                let result = alias.alias(&entry.location, location);
                if result.is_no_alias() {
                    continue;
                }
                if result.is_must_alias() {
                    return Some(entry.value);
                }
                return None;
            }
        }
        None
    }

    /// Record a forwarded load value.
    fn insert_memory(
        &mut self,
        location: crate::optimize::common::MemoryLocation,
        value: mir::Value,
    ) {
        // insert into the current memory scope
        if let Some(scope) = self.memory_scopes.last_mut() {
            scope.push(MemoryEntry { location, value });
        }
    }

    /// Remove memory entries clobbered by an instruction.
    fn clobber_memory(
        &mut self,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
        alias: &AliasAnalysis,
    ) {
        // drop entries that may be clobbered by the instruction
        for scope in &mut self.memory_scopes {
            scope.retain(|entry| !alias.may_clobber(instruction_id, &entry.location));
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
    alias: &AliasAnalysis,
) -> (
    HashMap<mir::Value, mir::Value>,
    HashSet<mir::LocalNodeId<mir::Instruction>>,
) {
    // initialize substitution state
    let mut substitutions: HashMap<mir::Value, mir::Value> = HashMap::new();
    let mut to_remove: HashSet<mir::LocalNodeId<mir::Instruction>> = HashSet::new();
    let mut value_table = ScopedValueTable::new();

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
                // push a new scope for this block's expressions
                // children see this scope, siblings do not
                value_table.push_scope();

                // process instructions in this block
                process_block(
                    block_id,
                    tree,
                    alias,
                    &mut value_table,
                    &mut substitutions,
                    &mut to_remove,
                );

                // schedule leave so it runs after children
                stack.push(Action::Leave);

                // schedule children in reverse so first child runs first
                let children = dom_children.get(&block_id).cloned().unwrap_or_default();
                for child in children.into_iter().rev() {
                    stack.push(Action::Enter(child));
                }
            }
            Action::Leave => {
                // discard scopes for the dominated subtree
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
    alias: &AliasAnalysis,
    value_table: &mut ScopedValueTable,
    substitutions: &mut HashMap<mir::Value, mir::Value>,
    to_remove: &mut HashSet<mir::LocalNodeId<mir::Instruction>>,
) {
    // load the block for inspection
    let block = tree.get(block_id);

    // scan the block instructions
    for &instruction_id in &block.instructions {
        // load the instruction for analysis
        let instruction = tree.get(instruction_id);

        // track aggregate construction operands for cross block forwarding
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

        // check for aggregate field or element extraction simplification
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
                // NOTE #Incomplete: GVN needs constant propagation for ElementGet
                None
            }
            _ => None,
        };

        // apply aggregate forwarding when available
        if let Some((dest, replacement, inst_id)) = aggregate_simplification {
            substitutions.insert(dest, replacement);
            to_remove.insert(inst_id);
            continue;
        }

        // forward local reads
        if let mir::Instruction::LocalGet { destination, local } = instruction {
            if let Some(existing) = value_table.get_local(*local) {
                substitutions.insert(*destination, existing);
                to_remove.insert(instruction_id);
            } else {
                value_table.insert_local(*local, *destination);
            }
            continue;
        }

        // update local state on writes
        if let mir::Instruction::LocalSet { local, value } = instruction {
            value_table.insert_local(*local, *value);
        }

        // forward redundant loads
        if let mir::Instruction::Load {
            destination,
            pointer,
            ..
        } = instruction
        {
            let location = crate::optimize::common::MemoryLocation::from_ptr(*pointer);
            if let Some(existing) = value_table.get_memory(&location, alias) {
                substitutions.insert(*destination, existing);
                to_remove.insert(instruction_id);
            } else {
                value_table.insert_memory(location, *destination);
            }
            continue;
        }

        // clear clobbered load entries
        if instruction_may_affect_memory(instruction) {
            value_table.clobber_memory(instruction_id, alias);
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

    /// Loads are value numbered across dominated blocks when not clobbered.
    #[test]
    fn test_eliminate_loads_across_blocks() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = load v0
    jump block1
block1:
    v2 = load v0
    v3 = iadd v1, v2
    return v3
}"#;
        let expected = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = load v0
    jump block1
block1:
    v3 = iadd v1, v1
    return v3
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&GlobalValueNumbering);
        program.assert_output(expected);
    }

    /// Loads are not value numbered across intervening stores.
    #[test]
    fn test_preserve_loads_after_store() {
        let input = r#"function @test() -> i32 {
block0:
    v0 = stack.alloc i32
    v1 = load v0
    v2 = iconst 1i32
    store v0, v2
    jump block1
block1:
    v3 = load v0
    return v3
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&GlobalValueNumbering);
        program.assert_output(input);
    }
}
