use std::collections::{HashMap, HashSet};

use crate::optimize::declare_pass;
use destack_mir as mir;

use crate::optimize::{FunctionPass, MirOptimized, PipelineContext};
use destack_mir::{
    AliasAnalysis, ConstantPropagation, DominatorTree, MemoryAccessEffect, MemoryAccessId,
    MemoryNode, MemoryRegion, MemorySSA, Mutation, PureExpression, TargetLayout, ValueTypes,
    apply_substitutions_in_function, instruction_has_side_effects, resolve_substitution_chains,
};

declare_pass! {
    /// Redundant expression elimination.
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
    /// function before(v0: int32, v1: int32, v2: boolean): int32 {
    /// b0(v0: int32, v1: int32, v2: boolean):
    ///     v3 = int.add v0, v1
    ///     branch v2 => b1 | b2
    /// b1:
    ///     v4 = int.add v0, v1
    ///     return v4
    /// b2:
    ///     v5 = int.add v0, v1
    ///     return v5
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function after(v0: int32, v1: int32, v2: boolean): int32 {
    /// b0(v0: int32, v1: int32, v2: boolean):
    ///     v3 = int.add v0, v1
    ///     branch v2 => b1 | b2
    /// b1:
    ///     return v3
    /// b2:
    ///     return v3
    /// }
    /// ```
    #[pass(id = "eliminate-redundant-expressions")]
    pub EliminateRedundantExpressions,
    "Eliminate redundant expressions across blocks"
}

impl FunctionPass for EliminateRedundantExpressions {
    fn run(
        &self,
        function: &mut mir::Function,
        optimized: &mut MirOptimized,
        ctx: &PipelineContext<'_>,
        analyses: &mut mir::FunctionAnalyses,
    ) -> Mutation {
        let tree = &mut optimized.tree;
        let memory = &mut optimized.memory;
        let effects = &optimized.effects;

        // skip empty functions
        let entry = match function.entry() {
            Some(entry) => entry,
            None => return Mutation::NONE,
        };

        // get dominator tree children map for analysis
        let domtree = analyses.dominators(function, tree);
        let alias = analyses.alias(function, tree);
        let memory_ssa = analyses.memory_ssa(function, tree, memory, effects);
        let constants = analyses.constants(function, tree);
        let dom_children = build_dominator_children(function, domtree.as_ref());
        let value_types = analyses.value_types(function, tree);

        // run redundant-expression elimination
        let changed = run_eliminate_redundant_expressions(
            entry,
            function,
            tree,
            memory,
            &dom_children,
            &alias,
            memory_ssa.as_ref(),
            constants.as_ref(),
            &value_types,
            ctx.target_layout(),
        );

        // report what this pass changed
        if changed {
            Mutation::VALUE
        } else {
            Mutation::NONE
        }
    }

    fn name(&self) -> &'static str {
        "EliminateRedundantExpressions"
    }

    fn id(&self) -> &'static str {
        "eliminate-redundant-expressions"
    }
}

/// Core redundant-expression elimination logic. Returns true if changes were made.
fn run_eliminate_redundant_expressions(
    entry: mir::LocalNodeId<mir::Block>,
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    memory: &mut mir::MemoryTable,
    dom_children: &HashMap<mir::LocalNodeId<mir::Block>, Vec<mir::LocalNodeId<mir::Block>>>,
    alias: &AliasAnalysis,
    memory_ssa: &MemorySSA,
    constants: &ConstantPropagation,
    value_types: &ValueTypes,
    target_layout: TargetLayout,
) -> bool {
    // run redundant-expression elimination using dominator tree traversal
    let (substitutions, to_remove) = find_redundant_expressions(
        entry,
        tree,
        dom_children,
        alias,
        memory_ssa,
        constants,
        value_types,
        target_layout,
    );

    // nothing to do if no redundancies found
    if to_remove.is_empty() {
        return false;
    }

    // apply substitutions and remove redundant instructions
    apply_substitutions_in_function(function, tree, memory, &substitutions, Some(&to_remove));

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
    for &block_id in function.blocks() {
        children.insert(block_id, Vec::new());
    }

    // build parent to children mapping from idom relationships
    for &block_id in function.blocks() {
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
    scopes: Vec<HashMap<PureExpression, mir::Value>>,
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
    /// Clobbering access id for the memory state.
    clobber: MemoryAccessId,
    /// Memory location accessed by the load.
    region: MemoryRegion,
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
    fn get(&self, key: &PureExpression) -> Option<mir::Value> {
        // search from innermost to outermost scope
        for scope in self.scopes.iter().rev() {
            if let Some(&value) = scope.get(key) {
                return Some(value);
            }
        }
        None
    }

    /// Insert an expression into the current (innermost) scope.
    fn insert(&mut self, key: PureExpression, value: mir::Value) {
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
        clobber: MemoryAccessId,
        use_effect: &MemoryAccessEffect,
        alias: &AliasAnalysis,
    ) -> Option<mir::Value> {
        let region = &use_effect.region;

        // skip imprecise regions
        if matches!(region, MemoryRegion::Any { .. }) {
            return None;
        }

        // scan memory scopes from innermost to outermost
        for scope in self.memory_scopes.iter().rev() {
            // scan entries from newest to oldest
            for entry in scope.iter().rev() {
                // skip entries with a different clobber
                if entry.clobber != clobber {
                    continue;
                }

                if !entry.region.spaces().may_alias(use_effect.region.spaces()) {
                    continue;
                }

                // compare matching locations
                match (&entry.region, region) {
                    (MemoryRegion::Local(a), MemoryRegion::Local(b)) => {
                        if a == b {
                            return Some(entry.value);
                        }
                    }
                    (
                        MemoryRegion::Address { location: a, .. },
                        MemoryRegion::Address { location: b, .. },
                    ) => {
                        if a.address == b.address {
                            if a.is_compatible_with(b) {
                                return Some(entry.value);
                            }

                            return None;
                        }

                        // consult alias analysis for derived pointers
                        let result = alias.alias(a, b);
                        if result.is_no_alias() {
                            continue;
                        }
                        if result.is_must_alias() {
                            if a.is_compatible_with(b) {
                                return Some(entry.value);
                            }

                            return None;
                        }

                        return None;
                    }
                    _ => {}
                }
            }
        }

        None
    }

    /// Record a forwarded load value.
    fn insert_memory(&mut self, entry: MemoryEntry) {
        // insert into the current memory scope
        if let Some(scope) = self.memory_scopes.last_mut() {
            scope.push(entry);
        }
    }
}

/// Find redundant expressions by walking the dominator tree.
///
/// Returns a tuple of (substitutions, instructions_to_remove).
fn find_redundant_expressions(
    entry: mir::LocalNodeId<mir::Block>,
    tree: &mir::Tree,
    dom_children: &HashMap<mir::LocalNodeId<mir::Block>, Vec<mir::LocalNodeId<mir::Block>>>,
    alias: &AliasAnalysis,
    memory_ssa: &MemorySSA,
    constants: &ConstantPropagation,
    value_types: &ValueTypes,
    target_layout: TargetLayout,
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
                    memory_ssa,
                    constants,
                    value_types,
                    target_layout,
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
    tree: &mir::Tree,
    alias: &AliasAnalysis,
    memory_ssa: &MemorySSA,
    _constants: &ConstantPropagation,
    value_types: &ValueTypes,
    _target_layout: TargetLayout,
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
        if let mir::Instruction::Aggregate {
            destination,
            values,
        } = instruction
        {
            // record aggregate operands for forwarding
            let args = tree.get_values(*values);
            value_table.insert_aggregate(*destination, args.to_vec());
        }

        // check for aggregate field or element extraction simplification
        let aggregate_simplification = match instruction {
            mir::Instruction::FieldGet {
                destination,
                aggregate,
                field: index,
                ..
            }
            | mir::Instruction::ElementGet {
                destination,
                aggregate,
                index,
                ..
            } => {
                let aggregate = *aggregate;
                let destination = *destination;
                let agg = substitutions.get(&aggregate).copied().unwrap_or(aggregate);

                if let Some(operands) = value_table.get_aggregate(&agg)
                    && let Some(&operand) = operands.get(*index as usize)
                {
                    Some((destination, operand, instruction_id))
                } else {
                    None
                }
            }
            _ => None,
        };

        // apply aggregate forwarding when available
        if let Some((dest, replacement, inst_id)) = aggregate_simplification {
            if value_types.can_substitute(dest, replacement) {
                substitutions.insert(dest, replacement);
                to_remove.insert(inst_id);
            }
            continue;
        }

        // forward local reads
        if let mir::Instruction::LocalGet { destination, local } = instruction {
            let destination = *destination;
            let local = *local;

            if let Some(existing) = value_table.get_local(local)
                && value_types.can_substitute(destination, existing)
            {
                substitutions.insert(destination, existing);
                to_remove.insert(instruction_id);
            } else {
                value_table.insert_local(local, destination);
            }
            continue;
        }

        // update local state on writes
        if let mir::Instruction::LocalSet { local, value } = instruction {
            let local = *local;
            let value = *value;

            value_table.insert_local(local, value);
        }

        // forward redundant loads
        if let mir::Instruction::Load { destination, .. } = instruction {
            let destination = *destination;

            // resolve the memory ssa use access
            let Some(use_access_id) = memory_ssa.first_use_access(instruction_id) else {
                continue;
            };

            // read the use access data
            let MemoryNode::Use(use_access) = memory_ssa.access(use_access_id) else {
                continue;
            };

            // skip volatile or barrier reads
            if use_access.effect.is_volatile || use_access.effect.is_barrier {
                continue;
            }

            // skip imprecise regions
            if matches!(use_access.effect.region, MemoryRegion::Any { .. }) {
                continue;
            }

            // compute the clobbering access for the load
            let clobber = memory_ssa.clobbering_use(use_access_id, alias);
            if matches!(memory_ssa.access(clobber), MemoryNode::Phi(_)) {
                continue;
            }

            // forward from an existing load when possible
            if let Some(existing) = value_table.get_memory(clobber, &use_access.effect, alias)
                && value_types.can_substitute(destination, existing)
            {
                substitutions.insert(destination, existing);
                to_remove.insert(instruction_id);
            } else {
                value_table.insert_memory(MemoryEntry {
                    clobber,
                    region: use_access.effect.region.clone(),
                    value: destination,
                });
            }
            continue;
        }

        // skip instructions with side effects
        if instruction_has_side_effects(instruction) {
            continue;
        }

        // try to get an expression key
        let Some(key) = PureExpression::from_instruction(instruction) else {
            continue;
        };

        // get the destination value
        let Some(destination) = instruction.destination() else {
            continue;
        };

        // apply existing substitutions to the key
        let key = key.substitute(substitutions);

        // check if we've seen this expression in any dominating scope
        if let Some(existing_value) = value_table.get(&key) {
            // found a match: mark for substitution and removal
            if value_types.can_substitute(destination, existing_value) {
                substitutions.insert(destination, existing_value);
                to_remove.insert(instruction_id);
            }
        }
        // otherwise record as a new expression
        else {
            value_table.insert(key, destination);
        }
    }
}

/// Return true when two values can be safely substituted.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Expression in entry block is available in dominated blocks.
    #[test]
    fn test_eliminate_cross_block() {
        let input = r#"
function test(v0: int32, v1: int32, v2: boolean): int32 {
entry(v0: int32, v1: int32, v2: boolean):
    v3: int32 = int.add v0, v1
    branch v2 => b1 | b2

b1:
    v4: int32 = int.add v0, v1
    return v4

b2:
    v5: int32 = int.add v0, v1
    return v5
}
"#;
        let expected = r#"
function test(v0: int32, v1: int32, v2: boolean): int32 {
entry(v0: int32, v1: int32, v2: boolean):
    v3: int32 = int.add v0, v1
    branch v2 => b1 | b2

b1:
    return v3

b2:
    return v3
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateRedundantExpressions);
        test.assert_output(expected);
    }

    /// Expression in block1 is not available in block2 (not dominated).
    #[test]
    fn test_skip_non_dominating_blocks() {
        let input = r#"
function test(v0: int32, v1: int32, v2: boolean): int32 {
entry(v0: int32, v1: int32, v2: boolean):
    branch v2 => b1 | b2

b1:
    v3: int32 = int.add v0, v1
    jump b3(v3)

b2:
    v4: int32 = int.add v0, v1
    jump b3(v4)

b3(v5: int32):
    return v5
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateRedundantExpressions);
        test.assert_unchanged(input);
    }

    /// Expression from entry is available through multiple levels of domination.
    #[test]
    fn test_eliminate_through_dominator_chain() {
        let input = r#"
function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    jump b1

b1:
    v3: int32 = int.mul v2, v2
    jump b2

b2:
    v4: int32 = int.add v0, v1
    v5: int32 = int.add v3, v4
    return v5
}
"#;
        let expected = r#"
function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    jump b1

b1:
    v3: int32 = int.mul v2, v2
    jump b2

b2:
    v5: int32 = int.add v3, v2
    return v5
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateRedundantExpressions);
        test.assert_output(expected);
    }

    /// Commutative operands (v0 + v1 and v1 + v0) are recognized as equivalent.
    #[test]
    fn test_eliminate_commutative() {
        let input = r#"
function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    jump b1

b1:
    v3: int32 = int.add v1, v0
    v4: int32 = int.add v2, v3
    return v4
}
"#;
        let expected = r#"
function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    jump b1

b1:
    v4: int32 = int.add v2, v2
    return v4
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateRedundantExpressions);
        test.assert_output(expected);
    }

    /// Substitutions are applied transitively through multiple redundancies.
    #[test]
    fn test_apply_transitive_substitutions() {
        let input = r#"
function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    jump b1

b1:
    v3: int32 = int.add v0, v1
    v4: int32 = int.mul v3, v3
    jump b2

b2:
    v5: int32 = int.add v0, v1
    v6: int32 = int.mul v5, v5
    v7: int32 = int.add v4, v6
    return v7
}
"#;
        let expected = r#"
function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    jump b1

b1:
    v4: int32 = int.mul v2, v2
    jump b2

b2:
    v7: int32 = int.add v4, v4
    return v7
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateRedundantExpressions);
        test.assert_output(expected);
    }

    /// redundant-expression elimination also handles local redundancies within a single block.
    #[test]
    fn test_eliminate_local_redundancies() {
        let input = r#"
function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    v3: int32 = int.add v0, v1
    v4: int32 = int.add v2, v3
    return v4
}
"#;
        let expected = r#"
function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    v4: int32 = int.add v2, v2
    return v4
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateRedundantExpressions);
        test.assert_output(expected);
    }

    /// Deeply nested dominator tree is handled correctly.
    #[test]
    fn test_eliminate_through_deep_chain() {
        let input = r#"
function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    jump b1

b1:
    jump b2

b2:
    jump b3

b3:
    v3: int32 = int.add v0, v1
    return v3
}
"#;
        let expected = r#"
function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    jump b1

b1:
    jump b2

b2:
    jump b3

b3:
    return v2
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateRedundantExpressions);
        test.assert_output(expected);
    }

    /// Diamond CFG with expressions in both branches.
    #[test]
    fn test_eliminate_in_diamond_cfg() {
        let input = r#"
function test(v0: int32, v1: int32, v2: boolean): int32 {
entry(v0: int32, v1: int32, v2: boolean):
    v3: int32 = int.add v0, v1
    branch v2 => b1 | b2

b1:
    v4: int32 = int.add v0, v1
    jump b3(v4)

b2:
    v5: int32 = int.add v0, v1
    jump b3(v5)

b3(v6: int32):
    return v6
}
"#;
        let expected = r#"
function test(v0: int32, v1: int32, v2: boolean): int32 {
entry(v0: int32, v1: int32, v2: boolean):
    v3: int32 = int.add v0, v1
    branch v2 => b1 | b2

b1:
    jump b3(v3)

b2:
    jump b3(v3)

b3(v6: int32):
    return v6
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateRedundantExpressions);
        test.assert_output(expected);
    }

    /// Unique expressions are preserved unchanged.
    #[test]
    fn test_preserve_unique_expressions() {
        let input = r#"
function test(v0: int32, v1: int32, v2: boolean): int32 {
entry(v0: int32, v1: int32, v2: boolean):
    v3: int32 = int.add v0, v1
    branch v2 => b1 | b2

b1:
    v4: int32 = int.sub v0, v1
    return v4

b2:
    v5: int32 = int.mul v0, v1
    return v5
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateRedundantExpressions);
        test.assert_unchanged(input);
    }

    /// Unary operations are eliminated across blocks.
    #[test]
    fn test_eliminate_unary_cross_block() {
        let input = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = int.negate v0
    jump b1

b1:
    v2: int32 = int.negate v0
    v3: int32 = int.add v1, v2
    return v3
}
"#;
        let expected = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = int.negate v0
    jump b1

b1:
    v3: int32 = int.add v1, v1
    return v3
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateRedundantExpressions);
        test.assert_output(expected);
    }

    /// Field access is eliminated across blocks.
    #[test]
    fn test_eliminate_field_get_cross_block() {
        let input = r#"
function test(v0: (int32, int32)): int32 {
entry(v0: (int32, int32)):
    v1: int32 = field.get v0, 0
    jump b1

b1:
    v2: int32 = field.get v0, 0
    v3: int32 = int.add v1, v2
    return v3
}
"#;
        let expected = r#"
function test(v0: (int32, int32)): int32 {
entry(v0: (int32, int32)):
    v1: int32 = field.get v0, 0
    jump b1

b1:
    v3: int32 = int.add v1, v1
    return v3
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateRedundantExpressions);
        test.assert_output(expected);
    }

    /// Multiple independent expressions are all handled.
    #[test]
    fn test_eliminate_multiple_expressions() {
        let input = r#"
function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    v3: int32 = int.mul v0, v1
    jump b1

b1:
    v4: int32 = int.add v0, v1
    v5: int32 = int.mul v0, v1
    v6: int32 = int.add v4, v5
    return v6
}
"#;
        let expected = r#"
function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    v3: int32 = int.mul v0, v1
    jump b1

b1:
    v6: int32 = int.add v2, v3
    return v6
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateRedundantExpressions);
        test.assert_output(expected);
    }

    /// Tuple field extraction is forwarded across blocks.
    #[test]
    fn test_aggregate_tuple_cross_block() {
        let input = r#"
function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: (int32, int32) = aggregate (v0, v1)
    jump b1

b1:
    v3: int32 = field.get v2, 0
    v4: int32 = field.get v2, 1
    v5: int32 = int.add v3, v4
    return v5
}
"#;
        let expected = r#"
function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: (int32, int32) = aggregate (v0, v1)
    jump b1

b1:
    v5: int32 = int.add v0, v1
    return v5
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateRedundantExpressions);
        test.assert_output(expected);
    }

    /// Struct field extraction is forwarded across blocks.
    #[test]
    fn test_aggregate_struct_cross_block() {
        let input = r#"
type Point {
    int32;
    int32;
}

function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: Point = aggregate (v0, v1)
    jump b1

b1:
    v3: int32 = field.get v2, 0
    v4: int32 = field.get v2, 1
    v5: int32 = int.add v3, v4
    return v5
}
"#;
        let expected = r#"
type Point {
    int32;
    int32;
}

function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: Point = aggregate (v0, v1)
    jump b1

b1:
    v5: int32 = int.add v0, v1
    return v5
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateRedundantExpressions);
        test.assert_output(expected);
    }

    /// Aggregate forwarding through deep dominator chain.
    #[test]
    fn test_aggregate_through_deep_chain() {
        let input = r#"
function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: (int32, int32) = aggregate (v0, v1)
    jump b1

b1:
    jump b2

b2:
    jump b3

b3:
    v3: int32 = field.get v2, 1
    return v3
}
"#;
        let expected = r#"
function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: (int32, int32) = aggregate (v0, v1)
    jump b1

b1:
    jump b2

b2:
    jump b3

b3:
    return v1
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateRedundantExpressions);
        test.assert_output(expected);
    }

    /// Aggregate in non-dominating block is not forwarded.
    #[test]
    fn test_aggregate_skip_non_dominating() {
        let input = r#"
function test(v0: int32, v1: int32, v2: boolean): int32 {
entry(v0: int32, v1: int32, v2: boolean):
    branch v2 => b1 | b2

b1:
    v3: (int32, int32) = aggregate (v0, v1)
    jump b3(v3)

b2:
    v4: (int32, int32) = aggregate (v1, v0)
    v5: int32 = field.get v4, 0
    jump b3(v4)

b3(v6: (int32, int32)):
    v7: int32 = field.get v6, 0
    return v7
}
"#;
        let expected = r#"
function test(v0: int32, v1: int32, v2: boolean): int32 {
entry(v0: int32, v1: int32, v2: boolean):
    branch v2 => b1 | b2

b1:
    v3: (int32, int32) = aggregate (v0, v1)
    jump b3(v3)

b2:
    v4: (int32, int32) = aggregate (v1, v0)
    jump b3(v4)

b3(v6: (int32, int32)):
    v7: int32 = field.get v6, 0
    return v7
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateRedundantExpressions);
        test.assert_output(expected);
    }

    /// Diamond CFG with aggregate extraction in both branches.
    #[test]
    fn test_aggregate_diamond_cfg() {
        let input = r#"
function test(v0: int32, v1: int32, v2: boolean): int32 {
entry(v0: int32, v1: int32, v2: boolean):
    v3: (int32, int32) = aggregate (v0, v1)
    branch v2 => b1 | b2

b1:
    v4: int32 = field.get v3, 0
    jump b3(v4)

b2:
    v5: int32 = field.get v3, 1
    jump b3(v5)

b3(v6: int32):
    return v6
}
"#;
        let expected = r#"
function test(v0: int32, v1: int32, v2: boolean): int32 {
entry(v0: int32, v1: int32, v2: boolean):
    v3: (int32, int32) = aggregate (v0, v1)
    branch v2 => b1 | b2

b1:
    jump b3(v0)

b2:
    jump b3(v1)

b3(v6: int32):
    return v6
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateRedundantExpressions);
        test.assert_output(expected);
    }

    /// Aggregate forwarding combined with regular redundant-expression elimination.
    #[test]
    fn test_aggregate_combined_with_eliminate_redundant_expressions() {
        let input = r#"
function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    v3: (int32, int32) = aggregate (v2, v1)
    jump b1

b1:
    v4: int32 = int.add v0, v1
    v5: int32 = field.get v3, 0
    v6: int32 = int.add v4, v5
    return v6
}
"#;
        let expected = r#"
function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    v3: (int32, int32) = aggregate (v2, v1)
    jump b1

b1:
    v6: int32 = int.add v2, v2
    return v6
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateRedundantExpressions);
        test.assert_output(expected);
    }

    /// Loads are value numbered across dominated blocks when not clobbered.
    #[test]
    fn test_eliminate_loads_across_blocks() {
        let input = r#"
function test(): int32 {
    local l0: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    v1: int32 = load v0
    jump b1

b1:
    v2: int32 = load v0
    v3: int32 = int.add v1, v2
    return v3
}
"#;
        let expected = r#"
function test(): int32 {
    local l0: int32

entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    v1: int32 = load v0
    jump b1

b1:
    v3: int32 = int.add v1, v1
    return v3
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateRedundantExpressions);
        test.assert_output(expected);
    }

    /// Loads are not value numbered across intervening stores.
    #[test]
    fn test_preserve_loads_after_store() {
        let input = r#"
function test(): int32 {
    local l0: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    v1: int32 = load v0
    v2: int32 = 1
    store v0, v2
    jump b1

b1:
    v3: int32 = load v0
    return v3
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateRedundantExpressions);
        test.assert_output(input);
    }

    /// Size mismatches prevent load forwarding.
    #[test]
    fn test_no_forward_load_size_mismatch() {
        let input = r#"
function test(v0: ref<int32, borrowed, mutable>): int32 {
entry(v0: ref<int32, borrowed, mutable>):
    v1: int32 = load v0
    v2: int32 = load v0
    v3: int32 = int.add v1, v2
    return v3
}
"#;
        let expected = input;

        let mut test = TestProgram::new(input);

        // locate the load instructions
        let function_id = test.first_function_id();
        let instructions = test.entry_instructions(function_id);
        let load_first = instructions[0];
        let load_second = instructions[1];

        // attach mismatched sizes to block forwarding
        test.insert_pointer_access(
            load_first,
            mir::MemoryOperation::Read,
            mir::Value::new(0),
            Some(8),
        );
        test.insert_pointer_access(
            load_second,
            mir::MemoryOperation::Read,
            mir::Value::new(0),
            Some(4),
        );

        test.run_pass(&EliminateRedundantExpressions);
        test.assert_output(expected);
    }

    /// No memory calls do not block load value numbering.
    #[test]
    fn test_forward_loads_across_no_memory_call() {
        let input = r#"
external function imported(ref<int32, borrowed, mutable>): void

function test(): int32 {
    local l0: int32
entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    v1: int32 = load v0
    call imported(v0): (ref<int32, borrowed, mutable>) => void
    v2: int32 = load v0
    v3: int32 = int.add v1, v2
    return v3
}
"#;
        let expected = r#"
external function imported(ref<int32, borrowed, mutable>): void

function test(): int32 {
    local l0: int32

entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    v1: int32 = load v0
    call imported(v0): (ref<int32, borrowed, mutable>) => void
    v3: int32 = int.add v1, v1
    return v3
}
"#;

        let mut test = TestProgram::new(input);

        let function_id = test.entry_function_id();
        let (call_inst, _callee) = test.first_call_in_entry(function_id);

        let callsite = mir::CallSite::Instruction(call_inst);
        test.optimized.effects.call_mut(callsite).memory = mir::MemoryEffect::none();

        test.run_pass(&EliminateRedundantExpressions);
        test.assert_output(expected);
    }
}
