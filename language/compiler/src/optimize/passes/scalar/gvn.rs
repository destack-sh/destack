use std::collections::{HashMap, HashSet};

use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::optimize::analyses::{
    AliasAnalysis, ConstantPropagation, DominatorTree, MemoryAccess, MemoryAccessEffect,
    MemoryAccessId, MemoryAccessLocation, MemorySSA,
};
use crate::optimize::common::{
    ValueTypeMap, address_spaces_may_alias, alias_scopes_may_alias, can_substitute_value,
    memory_locations_compatible, space_sets_may_alias, type_alias_tags_may_alias,
};
use crate::optimize::{
    AnalysisPreservation, ExpressionKey, FunctionPass, PipelineContext, TypeContext,
    apply_substitutions_in_function, expression_key_from_instruction, expression_key_substitute,
    instruction_has_side_effects, resolve_substitution_chains,
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
    /// function before(v0: int32, v1: int32, v2: boolean): int32 {
    /// b0(v0: int32, v1: int32, v2: boolean):
    ///     v3 = int.add v0, v1
    ///     branch v2, b1, b2
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
    ///     branch v2, b1, b2
    /// b1:
    ///     return v3
    /// b2:
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
        ctx: &PipelineContext<'_>,
    ) -> AnalysisPreservation {
        // skip empty functions
        let entry = match function.entry {
            Some(entry) => entry,
            None => return AnalysisPreservation::all(),
        };

        // get dominator tree children map for analysis
        let analyses = ctx.function_analyses(function, tree);
        let domtree = analyses.get::<DominatorTree>();
        let alias = analyses.get::<AliasAnalysis>();
        let memory_ssa = analyses.get::<MemorySSA>();
        let constants = analyses.get::<ConstantPropagation>();
        let dom_children = build_dominator_children(function, domtree.as_ref());
        let value_types = ValueTypeMap::new(function, tree);

        // run GVN
        let changed = run_gvn(
            entry,
            function,
            tree,
            &dom_children,
            &alias,
            memory_ssa.as_ref(),
            constants.as_ref(),
            &value_types,
            ctx.type_context(),
        );

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
#[allow(clippy::too_many_arguments)]
fn run_gvn(
    entry: mir::LocalNodeId<mir::Block>,
    function: &mir::Function,
    tree: &mut mir::NodeTree,
    dom_children: &HashMap<mir::LocalNodeId<mir::Block>, Vec<mir::LocalNodeId<mir::Block>>>,
    alias: &AliasAnalysis,
    memory_ssa: &MemorySSA,
    constants: &ConstantPropagation,
    value_types: &ValueTypeMap,
    type_context: TypeContext,
) -> bool {
    // run GVN using dominator tree traversal
    let (substitutions, to_remove) = find_redundant_expressions(
        entry,
        tree,
        dom_children,
        alias,
        memory_ssa,
        constants,
        value_types,
        type_context,
    );

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

    // build parent to children mapping from idom relationships
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
    /// Clobbering access id for the memory state.
    clobber: MemoryAccessId,
    /// Memory location accessed by the load.
    location: MemoryAccessLocation,
    /// Value produced by the load.
    value: mir::Value,
    /// The memory space set for the access.
    space_set: mir::MemorySpaceSet,
    /// The address spaces for the access.
    address_spaces: Option<mir::AddressSpaceSet>,
    /// Alias scopes applied to the access.
    alias_scopes: Vec<mir::MemoryAliasScopeId>,
    /// No alias scopes applied to the access.
    noalias_scopes: Vec<mir::MemoryAliasScopeId>,
    /// Optional type-alias tag for the access.
    type_alias_tag: Option<mir::TypeAliasTagId>,
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
        clobber: MemoryAccessId,
        use_effect: &MemoryAccessEffect,
        alias: &AliasAnalysis,
        tree: &mir::NodeTree,
    ) -> Option<mir::Value> {
        let location = &use_effect.location;

        // skip unknown locations
        if matches!(location, MemoryAccessLocation::Unknown) {
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

                // disambiguate using alias scopes and tbaa tags
                if !alias_scopes_may_alias(
                    &entry.alias_scopes,
                    &entry.noalias_scopes,
                    &use_effect.alias_scopes,
                    &use_effect.noalias_scopes,
                ) {
                    continue;
                }

                if !space_sets_may_alias(entry.space_set, use_effect.space_set) {
                    continue;
                }

                if !address_spaces_may_alias(&entry.address_spaces, &use_effect.address_spaces) {
                    continue;
                }

                if !type_alias_tags_may_alias(
                    &tree.metadata.memory.type_alias,
                    entry.type_alias_tag,
                    use_effect.type_alias_tag,
                ) {
                    continue;
                }

                // compare matching locations
                match (&entry.location, location) {
                    (MemoryAccessLocation::Local(a), MemoryAccessLocation::Local(b)) => {
                        if a == b {
                            return Some(entry.value);
                        }
                    }
                    (MemoryAccessLocation::Pointer(a), MemoryAccessLocation::Pointer(b)) => {
                        if a.ptr == b.ptr {
                            if memory_locations_compatible(a, b) {
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
                            if memory_locations_compatible(a, b) {
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
#[allow(clippy::too_many_arguments)]
fn find_redundant_expressions(
    entry: mir::LocalNodeId<mir::Block>,
    tree: &mir::NodeTree,
    dom_children: &HashMap<mir::LocalNodeId<mir::Block>, Vec<mir::LocalNodeId<mir::Block>>>,
    alias: &AliasAnalysis,
    memory_ssa: &MemorySSA,
    constants: &ConstantPropagation,
    value_types: &ValueTypeMap,
    type_context: TypeContext,
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
                    type_context,
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
#[allow(clippy::too_many_arguments)]
fn process_block(
    block_id: mir::LocalNodeId<mir::Block>,
    tree: &mir::NodeTree,
    alias: &AliasAnalysis,
    memory_ssa: &MemorySSA,
    constants: &ConstantPropagation,
    value_types: &ValueTypeMap,
    _type_context: TypeContext,
    value_table: &mut ScopedValueTable,
    substitutions: &mut HashMap<mir::Value, mir::Value>,
    to_remove: &mut HashSet<mir::LocalNodeId<mir::Instruction>>,
) {
    // load the block for inspection
    let block = tree.get(block_id);

    // capture constant propagation facts for this block
    let block_constants = constants.exit(block_id);

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
                // record struct operands for forwarding
                let Some(destination) = destination.value() else {
                    continue;
                };

                let args = tree.get_arguments(*fields);
                value_table.insert_aggregate(
                    destination,
                    args.iter().filter_map(|value| value.value()).collect(),
                );
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
                // record tuple or array operands for forwarding
                let Some(destination) = destination.value() else {
                    continue;
                };

                let args = tree.get_arguments(*elements);
                value_table.insert_aggregate(
                    destination,
                    args.iter().filter_map(|value| value.value()).collect(),
                );
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
                if let (Some(destination), Some(aggregate)) =
                    (destination.value(), aggregate.value())
                {
                    let agg = substitutions.get(&aggregate).copied().unwrap_or(aggregate);

                    if let Some(operands) = value_table.get_aggregate(&agg)
                        && let Some(&operand) = operands.get(*index as usize)
                    {
                        Some((destination, operand, instruction_id))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            mir::Instruction::ElementGet {
                destination,
                array,
                index,
                ..
            } => {
                if let (Some(destination), Some(array), Some(index)) =
                    (destination.value(), array.value(), index.value())
                {
                    let agg = substitutions.get(&array).copied().unwrap_or(array);
                    let mut result = None;

                    if let Some(index_constant) = block_constants.get(&index)
                        && let Some(index_value) = constant_index_to_usize(index_constant)
                        && let Some(operands) = value_table.get_aggregate(&agg)
                        && let Some(&operand) = operands.get(index_value)
                    {
                        result = Some((destination, operand, instruction_id));
                    }

                    result
                } else {
                    None
                }
            }
            _ => None,
        };

        // apply aggregate forwarding when available
        if let Some((dest, replacement, inst_id)) = aggregate_simplification {
            if can_substitute_value(dest, replacement, value_types, tree) {
                substitutions.insert(dest, replacement);
                to_remove.insert(inst_id);
            }
            continue;
        }

        // forward local reads
        if let mir::Instruction::LocalGet { destination, local } = instruction {
            let Some(destination) = destination.value() else {
                continue;
            };
            let Some(local) = local.local() else {
                continue;
            };

            if let Some(existing) = value_table.get_local(local)
                && can_substitute_value(destination, existing, value_types, tree)
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
            let Some(local) = local.local() else {
                continue;
            };
            let Some(value) = value.value() else {
                continue;
            };

            value_table.insert_local(local, value);
        }

        // forward redundant loads
        if let mir::Instruction::Load { destination, .. } = instruction {
            let Some(destination) = destination.value() else {
                continue;
            };

            // resolve the memory ssa use access
            let Some(use_access_id) = memory_ssa.first_use_access(instruction_id) else {
                continue;
            };

            // read the use access data
            let MemoryAccess::Use(use_access) = memory_ssa.access(use_access_id) else {
                continue;
            };

            // skip volatile or barrier reads
            if use_access.effect.is_volatile || use_access.effect.is_barrier {
                continue;
            }

            // skip unknown locations
            if matches!(use_access.effect.location, MemoryAccessLocation::Unknown) {
                continue;
            }

            // compute the clobbering access for the load
            let clobber = memory_ssa.clobbering_access_for_use(use_access_id, alias, tree);
            if matches!(memory_ssa.access(clobber), MemoryAccess::Phi(_)) {
                continue;
            }

            // forward from an existing load when possible
            if let Some(existing) = value_table.get_memory(clobber, &use_access.effect, alias, tree)
                && can_substitute_value(destination, existing, value_types, tree)
            {
                substitutions.insert(destination, existing);
                to_remove.insert(instruction_id);
            } else {
                value_table.insert_memory(MemoryEntry {
                    clobber,
                    location: use_access.effect.location.clone(),
                    value: destination,
                    space_set: use_access.effect.space_set,
                    address_spaces: use_access.effect.address_spaces.clone(),
                    alias_scopes: use_access.effect.alias_scopes.clone(),
                    noalias_scopes: use_access.effect.noalias_scopes.clone(),
                    type_alias_tag: use_access.effect.type_alias_tag,
                });
            }
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
        let Some(destination) = instruction
            .destination()
            .and_then(|destination| destination.value())
        else {
            continue;
        };

        // apply existing substitutions to the key
        let key = expression_key_substitute(key, substitutions);

        // check if we've seen this expression in any dominating scope
        if let Some(existing_value) = value_table.get(&key) {
            // found a match: mark for substitution and removal
            if can_substitute_value(destination, existing_value, value_types, tree) {
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

/// Convert an integer constant into an array index when possible.
fn constant_index_to_usize(constant: &mir::Constant) -> Option<usize> {
    // map integer constants to indices
    match constant {
        mir::Constant::Int { value, .. } if *value >= 0 => usize::try_from(*value).ok(),
        mir::Constant::UInt { value, .. } => usize::try_from(*value).ok(),
        _ => None,
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
b0(v0: int32, v1: int32, v2: boolean):
    v3: int32 = int.add v0, v1
    branch v2, b1, b2
b1:
    v4: int32 = int.add v0, v1
    return v4
b2:
    v5: int32 = int.add v0, v1
    return v5
}"#;
        let expected = r#"
function test(v0: int32, v1: int32, v2: boolean): int32 {
b0(v0: int32, v1: int32, v2: boolean):
    v3: int32 = int.add v0, v1
    branch v2, b1, b2
b1:
    return v3
b2:
    return v3
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&GlobalValueNumbering);
        test.assert_output(expected);
    }

    /// Expression in block1 is not available in block2 (not dominated).
    #[test]
    fn test_skip_non_dominating_blocks() {
        let input = r#"
function test(v0: int32, v1: int32, v2: boolean): int32 {
b0(v0: int32, v1: int32, v2: boolean):
    branch v2, b1, b2
b1:
    v3: int32 = int.add v0, v1
    jump b3(v3)
b2:
    v4: int32 = int.add v0, v1
    jump b3(v4)
b3(v5: int32):
    return v5
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&GlobalValueNumbering);
        test.assert_unchanged(input);
    }

    /// Expression from entry is available through multiple levels of domination.
    #[test]
    fn test_eliminate_through_dominator_chain() {
        let input = r#"
function test(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    jump b1
b1:
    v3: int32 = int.mul v2, v2
    jump b2
b2:
    v4: int32 = int.add v0, v1
    v5: int32 = int.add v3, v4
    return v5
}"#;
        let expected = r#"
function test(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    jump b1
b1:
    v3: int32 = int.mul v2, v2
    jump b2
b2:
    v4: int32 = int.add v3, v2
    return v4
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&GlobalValueNumbering);
        test.assert_output(expected);
    }

    /// Commutative operands (v0 + v1 and v1 + v0) are recognized as equivalent.
    #[test]
    fn test_eliminate_commutative() {
        let input = r#"
function test(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    jump b1
b1:
    v3: int32 = int.add v1, v0
    v4: int32 = int.add v2, v3
    return v4
}"#;
        let expected = r#"
function test(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    jump b1
b1:
    v3: int32 = int.add v2, v2
    return v3
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&GlobalValueNumbering);
        test.assert_output(expected);
    }

    /// Substitutions are applied transitively through multiple redundancies.
    #[test]
    fn test_apply_transitive_substitutions() {
        let input = r#"
function test(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
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
}"#;
        let expected = r#"
function test(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    jump b1
b1:
    v3: int32 = int.mul v2, v2
    jump b2
b2:
    v4: int32 = int.add v3, v3
    return v4
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&GlobalValueNumbering);
        test.assert_output(expected);
    }

    /// GVN also handles local redundancies within a single block.
    #[test]
    fn test_eliminate_local_redundancies() {
        let input = r#"
function test(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    v3: int32 = int.add v0, v1
    v4: int32 = int.add v2, v3
    return v4
}"#;
        let expected = r#"
function test(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    v3: int32 = int.add v2, v2
    return v3
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&GlobalValueNumbering);
        test.assert_output(expected);
    }

    /// Deeply nested dominator tree is handled correctly.
    #[test]
    fn test_eliminate_through_deep_chain() {
        let input = r#"
function test(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    jump b1
b1:
    jump b2
b2:
    jump b3
b3:
    v3: int32 = int.add v0, v1
    return v3
}"#;
        let expected = r#"
function test(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    jump b1
b1:
    jump b2
b2:
    jump b3
b3:
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&GlobalValueNumbering);
        test.assert_output(expected);
    }

    /// Diamond CFG with expressions in both branches.
    #[test]
    fn test_eliminate_in_diamond_cfg() {
        let input = r#"
function test(v0: int32, v1: int32, v2: boolean): int32 {
b0(v0: int32, v1: int32, v2: boolean):
    v3: int32 = int.add v0, v1
    branch v2, b1, b2
b1:
    v4: int32 = int.add v0, v1
    jump b3(v4)
b2:
    v5: int32 = int.add v0, v1
    jump b3(v5)
b3(v6: int32):
    return v6
}"#;
        let expected = r#"
function test(v0: int32, v1: int32, v2: boolean): int32 {
b0(v0: int32, v1: int32, v2: boolean):
    v3: int32 = int.add v0, v1
    branch v2, b1, b2
b1:
    jump b3(v3)
b2:
    jump b3(v3)
b3(v4: int32):
    return v4
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&GlobalValueNumbering);
        test.assert_output(expected);
    }

    /// Unique expressions are preserved unchanged.
    #[test]
    fn test_preserve_unique_expressions() {
        let input = r#"
function test(v0: int32, v1: int32, v2: boolean): int32 {
b0(v0: int32, v1: int32, v2: boolean):
    v3: int32 = int.add v0, v1
    branch v2, b1, b2
b1:
    v4: int32 = int.sub v0, v1
    return v4
b2:
    v5: int32 = int.mul v0, v1
    return v5
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&GlobalValueNumbering);
        test.assert_unchanged(input);
    }

    /// Unary operations are properly GVN'd across blocks.
    #[test]
    fn test_eliminate_unary_cross_block() {
        let input = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = int.negate v0
    jump b1
b1:
    v2: int32 = int.negate v0
    v3: int32 = int.add v1, v2
    return v3
}"#;
        let expected = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = int.negate v0
    jump b1
b1:
    v2: int32 = int.add v1, v1
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&GlobalValueNumbering);
        test.assert_output(expected);
    }

    /// Field access is properly GVN'd across blocks.
    #[test]
    fn test_eliminate_field_get_cross_block() {
        let input = r#"
function test(v0: (int32, int32)): int32 {
b0(v0: (int32, int32)):
    v1: int32 = field.get v0, 0
    jump b1
b1:
    v2: int32 = field.get v0, 0
    v3: int32 = int.add v1, v2
    return v3
}"#;
        let expected = r#"
function test(v0: (int32, int32)): int32 {
b0(v0: (int32, int32)):
    v1: int32 = field.get v0, 0
    jump b1
b1:
    v2: int32 = int.add v1, v1
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&GlobalValueNumbering);
        test.assert_output(expected);
    }

    /// Multiple independent expressions are all handled.
    #[test]
    fn test_eliminate_multiple_expressions() {
        let input = r#"
function test(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    v3: int32 = int.mul v0, v1
    jump b1
b1:
    v4: int32 = int.add v0, v1
    v5: int32 = int.mul v0, v1
    v6: int32 = int.add v4, v5
    return v6
}"#;
        let expected = r#"
function test(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    v3: int32 = int.mul v0, v1
    jump b1
b1:
    v4: int32 = int.add v2, v3
    return v4
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&GlobalValueNumbering);
        test.assert_output(expected);
    }

    /// Tuple field extraction is forwarded across blocks.
    #[test]
    fn test_aggregate_tuple_cross_block() {
        let input = r#"
function test(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: (int32, int32) = tuple (int32, int32) (v0, v1)
    jump b1
b1:
    v3: int32 = field.get v2, 0
    v4: int32 = field.get v2, 1
    v5: int32 = int.add v3, v4
    return v5
}"#;
        let expected = r#"
function test(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: (int32, int32) = tuple (int32, int32) (v0, v1)
    jump b1
b1:
    v3: int32 = int.add v0, v1
    return v3
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&GlobalValueNumbering);
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
b0(v0: int32, v1: int32):
    v2: Point = struct Point (v0, v1)
    jump b1
b1:
    v3: int32 = field.get v2, 0
    v4: int32 = field.get v2, 1
    v5: int32 = int.add v3, v4
    return v5
}"#;
        let expected = r#"
type Point {
    int32;
    int32;
}
function test(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: Point = struct Point (v0, v1)
    jump b1
b1:
    v3: int32 = int.add v0, v1
    return v3
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&GlobalValueNumbering);
        test.assert_output(expected);
    }

    /// Aggregate forwarding through deep dominator chain.
    #[test]
    fn test_aggregate_through_deep_chain() {
        let input = r#"
function test(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: (int32, int32) = tuple (int32, int32) (v0, v1)
    jump b1
b1:
    jump b2
b2:
    jump b3
b3:
    v3: int32 = field.get v2, 1
    return v3
}"#;
        let expected = r#"
function test(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: (int32, int32) = tuple (int32, int32) (v0, v1)
    jump b1
b1:
    jump b2
b2:
    jump b3
b3:
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&GlobalValueNumbering);
        test.assert_output(expected);
    }

    /// Aggregate in non-dominating block is not forwarded.
    #[test]
    fn test_aggregate_skip_non_dominating() {
        let input = r#"
function test(v0: int32, v1: int32, v2: boolean): int32 {
b0(v0: int32, v1: int32, v2: boolean):
    branch v2, b1, b2
b1:
    v3: (int32, int32) = tuple (int32, int32) (v0, v1)
    jump b3(v3)
b2:
    v4: (int32, int32) = tuple (int32, int32) (v1, v0)
    v5: int32 = field.get v4, 0
    jump b3(v4)
b3(v6: (int32, int32)):
    v7: int32 = field.get v6, 0
    return v7
}"#;
        let expected = r#"
function test(v0: int32, v1: int32, v2: boolean): int32 {
b0(v0: int32, v1: int32, v2: boolean):
    branch v2, b1, b2
b1:
    v3: (int32, int32) = tuple (int32, int32) (v0, v1)
    jump b3(v3)
b2:
    v4: (int32, int32) = tuple (int32, int32) (v1, v0)
    jump b3(v4)
b3(v5: (int32, int32)):
    v6: int32 = field.get v5, 0
    return v6
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&GlobalValueNumbering);
        test.assert_output(expected);
    }

    /// Diamond CFG with aggregate extraction in both branches.
    #[test]
    fn test_aggregate_diamond_cfg() {
        let input = r#"
function test(v0: int32, v1: int32, v2: boolean): int32 {
b0(v0: int32, v1: int32, v2: boolean):
    v3: (int32, int32) = tuple (int32, int32) (v0, v1)
    branch v2, b1, b2
b1:
    v4: int32 = field.get v3, 0
    jump b3(v4)
b2:
    v5: int32 = field.get v3, 1
    jump b3(v5)
b3(v6: int32):
    return v6
}"#;
        let expected = r#"
function test(v0: int32, v1: int32, v2: boolean): int32 {
b0(v0: int32, v1: int32, v2: boolean):
    v3: (int32, int32) = tuple (int32, int32) (v0, v1)
    branch v2, b1, b2
b1:
    jump b3(v0)
b2:
    jump b3(v1)
b3(v4: int32):
    return v4
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&GlobalValueNumbering);
        test.assert_output(expected);
    }

    /// Aggregate forwarding combined with regular GVN.
    #[test]
    fn test_aggregate_combined_with_gvn() {
        let input = r#"
function test(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    v3: (int32, int32) = tuple (int32, int32) (v2, v1)
    jump b1
b1:
    v4: int32 = int.add v0, v1
    v5: int32 = field.get v3, 0
    v6: int32 = int.add v4, v5
    return v6
}"#;
        let expected = r#"
function test(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    v3: (int32, int32) = tuple (int32, int32) (v2, v1)
    jump b1
b1:
    v4: int32 = int.add v2, v2
    return v4
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&GlobalValueNumbering);
        test.assert_output(expected);
    }

    /// Loads are value numbered across dominated blocks when not clobbered.
    #[test]
    fn test_eliminate_loads_across_blocks() {
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = load v0
    jump b1
b1:
    v2: int32 = load v0
    v3: int32 = int.add v1, v2
    return v3
}"#;
        let expected = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = load v0
    jump b1
b1:
    v2: int32 = int.add v1, v1
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&GlobalValueNumbering);
        test.assert_output(expected);
    }

    /// Loads are not value numbered across intervening stores.
    #[test]
    fn test_preserve_loads_after_store() {
        let input = r#"
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = load v0
    v2: int32 = 1int32
    store v0, v2
    jump b1
b1:
    v3: int32 = load v0
    return v3
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&GlobalValueNumbering);
        test.assert_output(input);
    }

    /// Scoped noalias metadata keeps unrelated stores from blocking load GVN.
    #[test]
    fn test_forward_loads_across_noalias_scope() {
        let input = r#"
function test(v0: ref<int32, raw>, v1: ref<int32, raw>): int32 {
b0(v0: ref<int32, raw>, v1: ref<int32, raw>):
    v2: int32 = load v0
    v3: int32 = 2int32
    store v1, v3
    v4: int32 = load v0
    v5: int32 = int.add v2, v4
    return v5
}"#;
        let expected = r#"
function test(v0: ref<int32, raw>, v1: ref<int32, raw>): int32 {
b0(v0: ref<int32, raw>, v1: ref<int32, raw>):
    v2: int32 = load v0
    v3: int32 = 2int32
    store v1, v3
    v4: int32 = int.add v2, v2
    return v4
}"#;

        let mut test = TestProgram::new(input);

        // create alias scope metadata for the disjoint store
        let scope = test.create_alias_scope();

        // locate the relevant instructions
        let function_id = test.first_function_id();
        let instructions = test.entry_instructions(function_id);
        let store_v1 = instructions[2];
        let load_v0 = instructions[3];

        // attach scoped metadata to disambiguate the store and load
        test.insert_pointer_access(
            store_v1,
            mir::MemoryAccessKind::Write,
            mir::Value::new(1),
            Some(4),
            vec![scope],
            Vec::new(),
            None,
        );

        test.insert_pointer_access(
            load_v0,
            mir::MemoryAccessKind::Read,
            mir::Value::new(0),
            Some(4),
            Vec::new(),
            vec![scope],
            None,
        );

        test.run_pass(&GlobalValueNumbering);
        test.assert_output(expected);
    }

    /// Disjoint TBAA offsets allow loads to forward across unrelated stores.
    #[test]
    fn test_forward_loads_across_tbaa_disjoint_offsets() {
        let input = r#"
function test(v0: ref<int32, raw>, v1: ref<int32, raw>): int32 {
b0(v0: ref<int32, raw>, v1: ref<int32, raw>):
    v2: int32 = load v0
    v3: int32 = 2int32
    store v1, v3
    v4: int32 = load v0
    v5: int32 = int.add v2, v4
    return v5
}"#;
        let expected = r#"
function test(v0: ref<int32, raw>, v1: ref<int32, raw>): int32 {
b0(v0: ref<int32, raw>, v1: ref<int32, raw>):
    v2: int32 = load v0
    v3: int32 = 2int32
    store v1, v3
    v4: int32 = int.add v2, v2
    return v4
}"#;

        let mut test = TestProgram::new(input);

        // create tbaa tags with disjoint offsets
        let root = test.create_type_alias_node(None, false);
        let access = test.create_type_alias_node(Some(root), false);
        let tag_a = test.create_type_alias_tag(root, access, 0, 4, false);
        let tag_b = test.create_type_alias_tag(root, access, 8, 4, false);

        // locate the relevant instructions
        let function_id = test.first_function_id();
        let instructions = test.entry_instructions(function_id);
        let load_v0 = instructions[0];
        let store_v1 = instructions[2];
        let load_v0_again = instructions[3];

        // attach disjoint tbaa tags to the loads and store
        test.insert_pointer_access(
            load_v0,
            mir::MemoryAccessKind::Read,
            mir::Value::new(0),
            Some(4),
            Vec::new(),
            Vec::new(),
            Some(tag_a),
        );
        test.insert_pointer_access(
            store_v1,
            mir::MemoryAccessKind::Write,
            mir::Value::new(1),
            Some(4),
            Vec::new(),
            Vec::new(),
            Some(tag_b),
        );
        test.insert_pointer_access(
            load_v0_again,
            mir::MemoryAccessKind::Read,
            mir::Value::new(0),
            Some(4),
            Vec::new(),
            Vec::new(),
            Some(tag_a),
        );

        test.run_pass(&GlobalValueNumbering);
        test.assert_output(expected);
    }

    /// Size mismatches prevent load forwarding.
    #[test]
    fn test_no_forward_load_size_mismatch() {
        let input = r#"
function test(v0: ref<int32, raw>): int32 {
b0(v0: ref<int32, raw>):
    v1: int32 = load v0
    v2: int32 = load v0
    v3: int32 = int.add v1, v2
    return v3
}"#;
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
            mir::MemoryAccessKind::Read,
            mir::Value::new(0),
            Some(8),
            Vec::new(),
            Vec::new(),
            None,
        );
        test.insert_pointer_access(
            load_second,
            mir::MemoryAccessKind::Read,
            mir::Value::new(0),
            Some(4),
            Vec::new(),
            Vec::new(),
            None,
        );

        test.run_pass(&GlobalValueNumbering);
        test.assert_output(expected);
    }

    /// Readnone calls do not block load value numbering.
    #[test]
    fn test_forward_loads_across_readnone_call() {
        let input = r#"
extern function external(ref<int32, raw>): void
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = load v0
    call external(v0): (ref<int32, raw>) -> void
    v2: int32 = load v0
    v3: int32 = int.add v1, v2
    return v3
}"#;
        let expected = r#"
extern function external(ref<int32, raw>): void
function test(): int32 {
b0:
    v0: ref<int32, raw, space(stack)> = stack.alloc int32
    v1: int32 = load v0
    call external(v0): (ref<int32, raw>) -> void
    v2: int32 = int.add v1, v1
    return v2
}"#;

        let mut test = TestProgram::new(input);

        let function_id = test.entry_function_id();
        let (call_inst, _callee) = test.first_call_in_entry(function_id);

        let instruction = test.tree.get_mut(call_inst);
        let Some(memory_effect) = instruction.call_memory_effect_mut() else {
            panic!("expected call instruction");
        };
        *memory_effect = Some(mir::MemoryEffect::none());

        test.run_pass(&GlobalValueNumbering);
        test.assert_output(expected);
    }
}
