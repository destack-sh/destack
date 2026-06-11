use std::collections::{HashMap, HashSet, VecDeque};

use crate::declare_mir_pass;
use destack_mir as mir;

use crate::common::mir::analysis::{AvailableExpressions, ControlFlowGraph, DominatorTree};
use crate::common::mir::{
    EdgeSplitPolicy, UseDefMaps, ValueTypeMap, append_successor_arguments, build_use_def_maps,
    collect_reachable_blocks, compute_dominance_frontiers, ensure_edge_block,
    instruction_has_side_effects, instruction_is_speculatable,
};
use crate::optimize::{
    AnalysisPreservation, ExpressionKey, FunctionPass, PipelineContext,
    apply_substitutions_in_function, expression_key_from_instruction, expression_key_substitute,
};

declare_mir_pass! {
    /// Eliminate partially redundant expressions by inserting computations.
    ///
    /// This pass computes SSA like phi values for pure expressions at join points.
    /// This pass inserts missing computations on incoming edges.
    /// This pass removes redundant recomputations dominated by the new values.
    ///
    /// ```mir
    /// function before(v0: int32, v1: int32, v2: boolean): int32 {
    /// b0(v0: int32, v1: int32, v2: boolean):
    ///     branch v2, b1, b2
    /// b1:
    ///     v3 = int.add v0, v1
    ///     jump b3
    /// b2:
    ///     jump b3
    /// b3:
    ///     v4 = int.add v0, v1
    ///     return v4
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function after(v0: int32, v1: int32, v2: boolean): int32 {
    /// b0(v0: int32, v1: int32, v2: boolean):
    ///     branch v2, b1, b2
    /// b1:
    ///     v3 = int.add v0, v1
    ///     jump b3(v3)
    /// b2:
    ///     v4 = int.add v0, v1
    ///     jump b3(v4)
    /// b3(v5: int32):
    ///     return v5
    /// }
    /// ```
    #[pass(id = "pre")]
    pub PartialRedundancyElim,
    "Eliminate partially redundant expressions"
}

impl FunctionPass for PartialRedundancyElim {
    /// Run partial redundancy elimination on a function.
    fn run(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::Tree,
        ctx: &PipelineContext<'_>,
    ) -> AnalysisPreservation {
        // skip imported functions
        let Some(entry) = function.entry else {
            return AnalysisPreservation::all();
        };

        // gather analyses
        let analyses = ctx.function_analyses(function, tree);
        let cfg = analyses.get::<ControlFlowGraph>().clone();
        let domtree = analyses.get::<DominatorTree>().clone();
        let available = analyses.get::<AvailableExpressions>().clone();
        let value_types = ValueTypeMap::new(function, tree);

        // run PRE
        let changed = run_pre(
            entry,
            function,
            tree,
            &cfg,
            &domtree,
            &available,
            &value_types,
        );

        if changed {
            AnalysisPreservation::none()
        } else {
            AnalysisPreservation::all()
        }
    }

    /// Return the display name for this pass.
    fn name(&self) -> &'static str {
        "PartialRedundancyElim"
    }

    /// Return the pipeline identifier for this pass.
    fn id(&self) -> &'static str {
        "pre"
    }
}

/// Records an expression occurrence in a block.
#[derive(Debug, Clone)]
struct ExpressionOccurrence {
    /// The block containing the expression.
    block: mir::LocalNodeId<mir::Block>,
    /// The instruction order index in the traversal.
    order: usize,
}

/// Describes a phi parameter inserted for an expression.
#[derive(Debug, Clone)]
struct PhiPlacement {
    /// The expression key this phi represents.
    key: ExpressionKey,
    /// The block parameter inserted for the expression.
    param: mir::Parameter,
    /// The ordering index used for insertion.
    order: usize,
}

/// Defines how to rebuild an expression instruction.
#[derive(Debug, Clone)]
enum ExpressionTemplate {
    /// Binary expression template.
    Binary { operator: mir::BinaryOperator },
    /// Unary expression template.
    Unary { operator: mir::UnaryOperator },
    /// Cast expression template.
    Cast {
        operator: mir::CastOperator,
        to_type: mir::TypeReference,
    },
    /// Select expression template.
    Select,
    /// Field access expression template.
    FieldGet { index: u32 },
    /// Element access expression template.
    ElementGet,
}

/// Run PRE on a single function and report whether it changed.
fn run_pre(
    entry: mir::LocalNodeId<mir::Block>,
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    cfg: &ControlFlowGraph,
    domtree: &DominatorTree,
    available: &AvailableExpressions,
    value_types: &ValueTypeMap,
) -> bool {
    // collect reachable blocks
    let reachable = collect_reachable_blocks(function, tree, entry);
    if reachable.is_empty() {
        return false;
    }

    // collect expression occurrences and templates
    let mut occurrences: HashMap<ExpressionKey, Vec<ExpressionOccurrence>> = HashMap::new();
    let mut templates: HashMap<ExpressionKey, ExpressionTemplate> = HashMap::new();
    let mut key_types: HashMap<ExpressionKey, mir::LocalNodeId<mir::Type>> = HashMap::new();
    let mut speculatable_keys: HashMap<ExpressionKey, bool> = HashMap::new();

    let mut order = 0usize;
    for &block_id in &function.blocks {
        // skip unreachable blocks when collecting occurrences
        if !reachable.contains(&block_id) {
            continue;
        }

        let block = tree.get(block_id);
        for &instruction_id in &block.instructions {
            let instruction = tree.get(instruction_id);
            if instruction_has_side_effects(instruction) {
                continue;
            }

            let Some(key) = expression_key_from_instruction(instruction, tree) else {
                continue;
            };
            let Some(destination) = instruction
                .destination()
                .and_then(|destination| destination.value())
            else {
                continue;
            };
            let value_type = value_types.require_value_type(destination);

            // track whether the expression can be speculated
            let is_speculatable = instruction_is_speculatable(instruction, tree);

            // record template for the expression
            templates
                .entry(key.clone())
                .or_insert_with(|| template_from_instruction(instruction));

            // record type for the expression
            key_types.entry(key.clone()).or_insert(value_type);
            // merge speculatability across occurrences
            speculatable_keys
                .entry(key.clone())
                .and_modify(|current| *current = *current && is_speculatable)
                .or_insert(is_speculatable);

            // record occurrence order and location
            occurrences
                .entry(key)
                .or_default()
                .push(ExpressionOccurrence {
                    block: block_id,
                    order,
                });
            order += 1;
        }
    }

    if occurrences.is_empty() {
        return false;
    }

    // build dominance frontiers for reachable blocks
    let frontiers = compute_dominance_frontiers(&reachable, cfg, domtree);

    // build maps for value availability checks
    let use_def = build_use_def_maps(function, tree);
    // plan phi placements per block
    let mut phi_map: HashMap<mir::LocalNodeId<mir::Block>, Vec<PhiPlacement>> = HashMap::new();
    let mut changed = false;

    // ensure value ids are fresh before inserting parameters
    function.recompute_next_value_id(tree);

    for (key, occs) in &occurrences {
        let Some(value_type) = key_types.get(key).copied() else {
            continue;
        };
        // skip non speculatable expressions for placement
        if !speculatable_keys.get(key).copied().unwrap_or(false) {
            continue;
        }

        // collect definition blocks for this expression
        let mut def_blocks: HashSet<mir::LocalNodeId<mir::Block>> = HashSet::new();
        for occ in occs {
            def_blocks.insert(occ.block);
        }

        // compute iterated dominance frontier for phi placements
        let mut worklist: VecDeque<mir::LocalNodeId<mir::Block>> =
            def_blocks.iter().copied().collect();
        let mut phi_blocks: HashSet<mir::LocalNodeId<mir::Block>> = HashSet::new();

        while let Some(block) = worklist.pop_front() {
            let Some(frontier) = frontiers.get(&block) else {
                continue;
            };

            for &df_block in frontier {
                if phi_blocks.insert(df_block) && !def_blocks.contains(&df_block) {
                    worklist.push_back(df_block);
                }
            }
        }

        // place phi parameters where the expression is useful and fillable
        for phi_block in phi_blocks {
            if !phi_is_useful(&phi_block, occs, domtree) {
                continue;
            }

            if !phi_is_fillable(&phi_block, key, function, cfg, domtree, available, &use_def) {
                continue;
            }

            // allocate a new parameter for the expression
            let param_value = function.next_typed_value(value_type);
            let param = mir::Parameter {
                value: param_value.into(),
                ty: value_type.into(),
            };
            let order = occs.iter().map(|occ| occ.order).min().unwrap_or(usize::MAX);
            phi_map.entry(phi_block).or_default().push(PhiPlacement {
                key: key.clone(),
                param,
                order,
            });
            changed = true;
        }
    }

    if phi_map.is_empty() {
        return false;
    }

    // keep phi parameters in deterministic order
    for placements in phi_map.values_mut() {
        placements.sort_by_key(|placement| placement.order);
    }

    // append the new parameters to each phi block
    for (&block_id, placements) in &phi_map {
        let mut block = tree.get(block_id).clone();
        for placement in placements {
            block.parameters.push(placement.param.clone());
        }
        tree.set(block_id, block);
    }

    // compute substitutions, exit values, and edge blocks
    let dom_children = build_dominator_children(function, domtree);
    let mut substitutions: HashMap<mir::Value, mir::Value> = HashMap::new();
    let mut to_remove: HashSet<mir::LocalNodeId<mir::Instruction>> = HashSet::new();
    let mut exit_values: HashMap<mir::LocalNodeId<mir::Block>, HashMap<ExpressionKey, mir::Value>> =
        HashMap::new();
    let mut edge_blocks: HashMap<
        (mir::LocalNodeId<mir::Block>, mir::LocalNodeId<mir::Block>),
        mir::LocalNodeId<mir::Block>,
    > = HashMap::new();

    let mut current: HashMap<ExpressionKey, Vec<mir::Value>> = HashMap::new();
    rename_block(
        entry,
        tree,
        &phi_map,
        &dom_children,
        &mut current,
        &mut substitutions,
        &mut to_remove,
        &mut exit_values,
    );

    // fill predecessor arguments for phi parameters
    let mut inserted = false;
    for (&block_id, placements) in &phi_map {
        let predecessors = cfg.predecessors(block_id);
        if predecessors.is_empty() {
            continue;
        }

        for &pred in predecessors {
            // split critical edges so insertions are placed on a dedicated edge
            let insertion_block = ensure_edge_block(
                pred,
                block_id,
                function,
                tree,
                cfg,
                &mut edge_blocks,
                EdgeSplitPolicy::CriticalOnly,
                &mut changed,
            );

            // collect arguments for each phi placement
            let mut args = Vec::new();
            for placement in placements {
                // resolve the value that should flow along this edge
                let value = if let Some(values) = exit_values.get(&pred)
                    && let Some(value) = values.get(&placement.key).copied()
                {
                    value
                } else {
                    // insert a missing computation along this edge when possible
                    let Some(value_type) = placement.param.ty.ty() else {
                        continue;
                    };

                    let inserted_value = insert_expression_in_block(
                        pred,
                        insertion_block,
                        placement.key.clone(),
                        value_type,
                        function,
                        tree,
                        templates.get(&placement.key),
                        &use_def,
                        domtree,
                    );

                    let Some(inserted_value) = inserted_value else {
                        continue;
                    };

                    exit_values
                        .entry(pred)
                        .or_default()
                        .insert(placement.key.clone(), inserted_value);
                    inserted = true;
                    inserted_value
                };
                args.push(value);
            }

            if !args.is_empty() {
                // append the arguments on the chosen edge
                append_successor_arguments(tree, insertion_block, block_id, &args);
                changed = true;
            }
        }
    }

    if inserted {
        // inserted instructions may enable more substitutions
        apply_substitutions_in_function(function, tree, &substitutions, Some(&to_remove));
        true
    } else if changed || !substitutions.is_empty() || !to_remove.is_empty() {
        apply_substitutions_in_function(function, tree, &substitutions, Some(&to_remove));
        true
    } else {
        false
    }
}

/// Build an expression template for a given instruction.
fn template_from_instruction(instruction: &mir::Instruction) -> ExpressionTemplate {
    match instruction {
        mir::Instruction::Binary { operator, .. } => ExpressionTemplate::Binary {
            operator: *operator,
        },
        mir::Instruction::Unary { operator, .. } => ExpressionTemplate::Unary {
            operator: *operator,
        },
        mir::Instruction::Cast {
            operator, to_type, ..
        } => ExpressionTemplate::Cast {
            operator: *operator,
            to_type: to_type.clone(),
        },
        mir::Instruction::Select { .. } => ExpressionTemplate::Select,
        mir::Instruction::FieldGet { index, .. } => ExpressionTemplate::FieldGet { index: *index },
        mir::Instruction::ElementGet { .. } => ExpressionTemplate::ElementGet,
        _ => panic!("unsupported expression template: {instruction:?}"),
    }
}

/// Return the operand values used by an expression key.
fn expression_operands(key: &ExpressionKey) -> Vec<mir::Value> {
    match key {
        ExpressionKey::Binary { left, right, .. } => vec![*left, *right],
        ExpressionKey::Unary { argument, .. } => vec![*argument],
        ExpressionKey::Cast { argument, .. } => vec![*argument],
        ExpressionKey::Select {
            condition,
            then_value,
            else_value,
        } => vec![*condition, *then_value, *else_value],
        ExpressionKey::FieldGet { aggregate, .. } => vec![*aggregate],
        ExpressionKey::ElementGet { array, .. } => vec![*array],
    }
}

/// Decide whether inserting a phi would remove redundancy.
fn phi_is_useful(
    block: &mir::LocalNodeId<mir::Block>,
    occurrences: &[ExpressionOccurrence],
    domtree: &DominatorTree,
) -> bool {
    // check whether this block dominates any occurrence
    occurrences
        .iter()
        .any(|occ| domtree.dominates(*block, occ.block))
}

/// Decide whether all phi operands can be filled on incoming edges.
fn phi_is_fillable(
    block: &mir::LocalNodeId<mir::Block>,
    key: &ExpressionKey,
    function: &mir::Function,
    cfg: &ControlFlowGraph,
    domtree: &DominatorTree,
    available: &AvailableExpressions,
    use_def: &UseDefMaps,
) -> bool {
    // every predecessor must either have the expression available or be able to compute it
    for &pred in cfg.predecessors(*block) {
        if available.exit(pred).contains(key) {
            continue;
        }

        if !operands_available_in_block(key, pred, function, domtree, use_def) {
            return false;
        }
    }

    true
}

/// Check that all operands are available at the end of a block.
fn operands_available_in_block(
    key: &ExpressionKey,
    block: mir::LocalNodeId<mir::Block>,
    function: &mir::Function,
    domtree: &DominatorTree,
    use_def: &UseDefMaps,
) -> bool {
    // collect operands for the expression
    let operands = expression_operands(key);

    for operand in operands {
        if value_available_in_block(operand, block, function, domtree, use_def) {
            continue;
        }

        return false;
    }

    true
}

/// Check that a value is available at the end of a block.
fn value_available_in_block(
    value: mir::Value,
    block: mir::LocalNodeId<mir::Block>,
    function: &mir::Function,
    domtree: &DominatorTree,
    use_def: &UseDefMaps,
) -> bool {
    // function parameters are always available
    if function
        .parameters
        .iter()
        .any(|param| param.value.value() == Some(value))
    {
        return true;
    }

    // block parameters and instruction destinations must dominate
    let Some(def_block) = use_def.def_block.get(&value).copied() else {
        return false;
    };

    if !domtree.dominates(def_block, block) {
        return false;
    }

    true
}

/// Build a list of dominator tree children for each block.
fn build_dominator_children(
    function: &mir::Function,
    domtree: &DominatorTree,
) -> HashMap<mir::LocalNodeId<mir::Block>, Vec<mir::LocalNodeId<mir::Block>>> {
    let mut children: HashMap<_, Vec<_>> = HashMap::new();

    // initialize child lists
    for &block_id in &function.blocks {
        children.insert(block_id, Vec::new());
    }

    // build parent to children mapping
    for &block_id in &function.blocks {
        if let Some(idom) = domtree.immediate_dominator(block_id) {
            children.get_mut(&idom).unwrap().push(block_id);
        }
    }

    children
}

/// Rename expression operands while traversing the dominator tree.
#[allow(clippy::too_many_arguments)]
fn rename_block(
    block_id: mir::LocalNodeId<mir::Block>,
    tree: &mir::Tree,
    phi_map: &HashMap<mir::LocalNodeId<mir::Block>, Vec<PhiPlacement>>,
    dom_children: &HashMap<mir::LocalNodeId<mir::Block>, Vec<mir::LocalNodeId<mir::Block>>>,
    current: &mut HashMap<ExpressionKey, Vec<mir::Value>>,
    substitutions: &mut HashMap<mir::Value, mir::Value>,
    to_remove: &mut HashSet<mir::LocalNodeId<mir::Instruction>>,
    exit_values: &mut HashMap<mir::LocalNodeId<mir::Block>, HashMap<ExpressionKey, mir::Value>>,
) {
    // track pushed keys to pop on exit
    let mut pushed_keys: Vec<ExpressionKey> = Vec::new();

    // register phi parameters as available values
    if let Some(placements) = phi_map.get(&block_id) {
        for placement in placements {
            current
                .entry(placement.key.clone())
                .or_default()
                .push(placement.param.value.value().unwrap());
            pushed_keys.push(placement.key.clone());
        }
    }

    // scan instructions for redundant expressions
    let block = tree.get(block_id);
    for &instruction_id in &block.instructions {
        let instruction = tree.get(instruction_id);
        if instruction_has_side_effects(instruction) {
            continue;
        }

        let Some(key) = expression_key_from_instruction(instruction, tree) else {
            continue;
        };
        let Some(destination) = instruction
            .destination()
            .and_then(|destination| destination.value())
        else {
            continue;
        };

        let key = expression_key_substitute(key, substitutions);
        if let Some(existing) = current.get(&key).and_then(|stack| stack.last()) {
            substitutions.insert(destination, *existing);
            to_remove.insert(instruction_id);
            continue;
        }

        current.entry(key.clone()).or_default().push(destination);
        pushed_keys.push(key);
    }

    // record exit values for this block
    let mut exit_map = HashMap::new();
    for (key, stack) in current.iter() {
        if let Some(value) = stack.last() {
            exit_map.insert(key.clone(), *value);
        }
    }
    exit_values.insert(block_id, exit_map);

    // recurse into dominated children
    if let Some(children) = dom_children.get(&block_id) {
        for &child in children {
            rename_block(
                child,
                tree,
                phi_map,
                dom_children,
                current,
                substitutions,
                to_remove,
                exit_values,
            );
        }
    }

    // pop keys added in this block
    for key in pushed_keys.into_iter().rev() {
        if let Some(stack) = current.get_mut(&key) {
            stack.pop();
            if stack.is_empty() {
                current.remove(&key);
            }
        }
    }
}

/// Insert an expression instruction into a block when operands are available.
#[allow(clippy::too_many_arguments)]
fn insert_expression_in_block(
    availability_block: mir::LocalNodeId<mir::Block>,
    insert_block: mir::LocalNodeId<mir::Block>,
    key: ExpressionKey,
    value_type: mir::LocalNodeId<mir::Type>,
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    template: Option<&ExpressionTemplate>,
    use_def: &UseDefMaps,
    domtree: &DominatorTree,
) -> Option<mir::Value> {
    // require a template describing how to rebuild the expression
    let template = template?;

    // require operands to be available at the insertion point
    if !operands_available_in_block(&key, availability_block, function, domtree, use_def) {
        return None;
    }

    // build the new instruction
    let destination = function.next_typed_value(value_type);
    let instruction = build_instruction_from_key(&key, template, destination);
    let instruction_id = tree.insert(instruction);

    // insert into the block before the terminator
    let mut block = tree.get(insert_block).clone();
    block.instructions.push(instruction_id);
    tree.set(insert_block, block);

    Some(destination)
}

/// Rebuild an instruction for the given expression key.
fn build_instruction_from_key(
    key: &ExpressionKey,
    template: &ExpressionTemplate,
    destination: mir::Value,
) -> mir::Instruction {
    // rebuild the instruction for the given expression key
    match (key, template) {
        (ExpressionKey::Binary { left, right, .. }, ExpressionTemplate::Binary { operator }) => {
            mir::Instruction::Binary {
                destination: destination.into(),
                operator: *operator,
                left: (*left).into(),
                right: (*right).into(),
            }
        }
        (ExpressionKey::Unary { argument, .. }, ExpressionTemplate::Unary { operator }) => {
            mir::Instruction::Unary {
                destination: destination.into(),
                operator: *operator,
                argument: (*argument).into(),
            }
        }
        (ExpressionKey::Cast { argument, .. }, ExpressionTemplate::Cast { operator, to_type }) => {
            mir::Instruction::Cast {
                destination: destination.into(),
                operator: *operator,
                argument: (*argument).into(),
                to_type: to_type.clone(),
            }
        }
        (
            ExpressionKey::Select {
                condition,
                then_value,
                else_value,
            },
            ExpressionTemplate::Select,
        ) => mir::Instruction::Select {
            destination: destination.into(),
            condition: (*condition).into(),
            then_value: (*then_value).into(),
            else_value: (*else_value).into(),
        },
        (ExpressionKey::FieldGet { aggregate, .. }, ExpressionTemplate::FieldGet { index }) => {
            mir::Instruction::FieldGet {
                destination: destination.into(),
                aggregate: (*aggregate).into(),
                index: *index,
            }
        }
        (ExpressionKey::ElementGet { array, index }, ExpressionTemplate::ElementGet) => {
            mir::Instruction::ElementGet {
                destination: destination.into(),
                array: (*array).into(),
                index: *index,
            }
        }
        _ => panic!("mismatched expression template"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Partial redundancy on a diamond inserts the missing computation.
    #[test]
    fn test_pre_diamond_inserts_expression() {
        let input = r#"
function test(v0: int32, v1: int32, v2: boolean): int32 {
b0(v0: int32, v1: int32, v2: boolean):
    branch v2, b1, b2
b1:
    v3: int32 = int.add v0, v1
    jump b3
b2:
    jump b3
b3:
    v4: int32 = int.add v0, v1
    return v4
}"#;

        let expected = r#"
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
        test.run_pass(&PartialRedundancyElim);
        test.assert_output(expected);
    }

    /// Critical edges receive a split block for inserted expressions.
    #[test]
    fn test_pre_splits_critical_edge_for_insertion() {
        let input = r#"
function test(v0: int32, v1: int32, v2: boolean): int32 {
b0(v0: int32, v1: int32, v2: boolean):
    branch v2, b1, b2
b1:
    v3: int32 = int.add v0, v1
    jump b3
b2:
    branch v2, b3, b4
b3:
    v4: int32 = int.add v0, v1
    return v4
b4:
    return v0
}"#;

        let expected = r#"
function test(v0: int32, v1: int32, v2: boolean): int32 {
b0(v0: int32, v1: int32, v2: boolean):
    branch v2, b1, b2
b1:
    v3: int32 = int.add v0, v1
    jump b4(v3)
b2:
    branch v2, b3, b5
b3:
    v4: int32 = int.add v0, v1
    jump b4(v4)
b4(v5: int32):
    return v5
b5:
    return v0
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&PartialRedundancyElim);
        test.assert_output(expected);
    }

    /// Switch default edges receive inserted expressions.
    #[test]
    fn test_pre_switch_default_inserts_expression() {
        let input = r#"
function test(v0: int32, v1: int32, v2: int32): int32 {
b0(v0: int32, v1: int32, v2: int32):
    switch v2, b2, 0 => b1
b1:
    v3: int32 = int.add v0, v1
    jump b2
b2:
    v4: int32 = int.add v0, v1
    return v4
}"#;

        let expected = r#"
function test(v0: int32, v1: int32, v2: int32): int32 {
b0(v0: int32, v1: int32, v2: int32):
    switch v2, b1, 0 => b2
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
        test.run_pass(&PartialRedundancyElim);
        test.assert_output(expected);
    }

    /// Switch cases keep their existing arguments when adding expressions.
    #[test]
    fn test_pre_switch_case_appends_expression() {
        let input = r#"
function test(v0: int32, v1: int32, v2: int32): int32 {
b0(v0: int32, v1: int32, v2: int32):
    v3: int32 = 7int32
    switch v2, b2, 0 => b1, 1 => b3(v3)
b1:
    v4: int32 = int.add v0, v1
    jump b3(v3)
b2:
    v5: int32 = 0int32
    return v5
b3(v6: int32):
    v7: int32 = int.add v0, v1
    return v7
}"#;

        let expected = r#"
function test(v0: int32, v1: int32, v2: int32): int32 {
b0(v0: int32, v1: int32, v2: int32):
    v3: int32 = 7int32
    switch v2, b3, 0 => b2, 1 => b1
b1:
    v4: int32 = int.add v0, v1
    jump b4(v3, v4)
b2:
    v5: int32 = int.add v0, v1
    jump b4(v3, v5)
b3:
    v6: int32 = 0int32
    return v6
b4(v7: int32, v8: int32):
    return v8
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&PartialRedundancyElim);
        test.assert_output(expected);
    }

    /// Check edges receive inserted expressions.
    #[test]
    fn test_pre_check_inserts_expression() {
        let input = r#"
function test(v0: uint32, v1: uint32, v2: boolean, v3: [uint8; 8]): uint32 {
b0(v0: uint32, v1: uint32, v2: boolean, v3: [uint8; 8]):
    branch v2, b1, b2
b1:
    v4: uint32 = int.add v0, v1
    v5: boolean = int.lt.u v0, v1
    check bounds.u v0, v1, v3 -> b3, b4
b2:
    jump b3
b3:
    v6: uint32 = int.add v0, v1
    return v6
b4:
    unreachable
}"#;

        let expected = r#"
function test(v0: uint32, v1: uint32, v2: boolean, v3: [uint8; 8]): uint32 {
b0(v0: uint32, v1: uint32, v2: boolean, v3: [uint8; 8]):
    branch v2, b1, b3
b1:
    v4: uint32 = int.add v0, v1
    v5: boolean = int.lt.u v0, v1
    check bounds.u v0, v1, v3 -> b2, b5
b2:
    jump b4(v4)
b3:
    v6: uint32 = int.add v0, v1
    jump b4(v6)
b4(v7: uint32):
    return v7
b5:
    unreachable
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&PartialRedundancyElim);
        test.assert_output(expected);
    }

    /// Check failure edges receive inserted expressions.
    #[test]
    fn test_pre_check_failure_inserts_expression() {
        let input = r#"
function test(v0: uint32, v1: uint32, v2: boolean, v3: [uint8; 8]): uint32 {
b0(v0: uint32, v1: uint32, v2: boolean, v3: [uint8; 8]):
    branch v2, b1, b2
b1:
    v4: uint32 = int.add v0, v1
    v5: boolean = int.lt.u v0, v1
    check bounds.u v0, v1, v3 -> b3, b4
b2:
    jump b4
b3:
    return v4
b4:
    v6: uint32 = int.add v0, v1
    return v6
}"#;

        let expected = r#"
function test(v0: uint32, v1: uint32, v2: boolean, v3: [uint8; 8]): uint32 {
b0(v0: uint32, v1: uint32, v2: boolean, v3: [uint8; 8]):
    branch v2, b1, b3
b1:
    v4: uint32 = int.add v0, v1
    v5: boolean = int.lt.u v0, v1
    check bounds.u v0, v1, v3 -> b4, b2
b2:
    jump b5(v4)
b3:
    v6: uint32 = int.add v0, v1
    jump b5(v6)
b4:
    return v4
b5(v7: uint32):
    return v7
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&PartialRedundancyElim);
        test.assert_output(expected);
    }

    /// Yield resume edges receive inserted expressions.
    #[test]
    fn test_pre_yield_resume_inserts_expression() {
        let input = r#"
function test(v0: int32, v1: int32, v2: boolean): int32 {
b0(v0: int32, v1: int32, v2: boolean):
    branch v2, b1, b2
b1:
    v3: int32 = int.add v0, v1
    v4: int32 = 1int32
    yield v4, b3(v0)
b2:
    v5: int32 = 2int32
    yield v5, b3(v0)
b3(v6: int32, v7: int32):
    v8: int32 = int.add v0, v1
    return v8
}"#;

        let expected = r#"
function test(v0: int32, v1: int32, v2: boolean): int32 {
b0(v0: int32, v1: int32, v2: boolean):
    branch v2, b1, b2
b1:
    v3: int32 = int.add v0, v1
    v4: int32 = 1int32
    yield v4, b3(v0, v3)
b2:
    v5: int32 = 2int32
    v6: int32 = int.add v0, v1
    yield v5, b3(v0, v6)
b3(v7: int32, v8: int32, v9: int32):
    return v9
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&PartialRedundancyElim);
        test.assert_output(expected);
    }

    /// Non speculatable expressions are not inserted on new paths.
    #[test]
    fn test_pre_skips_non_speculatable_expression() {
        let input = r#"
function test(v0: int32, v1: int32, v2: boolean): int32 {
b0(v0: int32, v1: int32, v2: boolean):
    branch v2, b1, b2
b1:
    v3: int32 = int.div.s v0, v1
    jump b3
b2:
    jump b3
b3:
    v4: int32 = int.div.s v0, v1
    return v4
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&PartialRedundancyElim);
        test.assert_output(input);
    }

    /// Float to int casts are not speculated.
    #[test]
    fn test_pre_skips_float_to_int_cast() {
        let input = r#"
function test(v0: float32, v1: boolean): int32 {
b0(v0: float32, v1: boolean):
    branch v1, b1, b2
b1:
    v2: int32 = cast.floatToInt.s v0 -> int32
    jump b3
b2:
    jump b3
b3:
    v3: int32 = cast.floatToInt.s v0 -> int32
    return v3
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&PartialRedundancyElim);
        test.assert_output(input);
    }

    /// Calls are not considered for PRE.
    #[test]
    fn test_pre_skips_calls() {
        let input = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    branch v0, b1, b2
b1:
    v1: int32 = call getValue(): () -> int32
    jump b3
b2:
    jump b3
b3:
    v2: int32 = call getValue(): () -> int32
    return v2
}
function getValue(): int32 {
b0:
    v0: int32 = 42int32
    return v0
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&PartialRedundancyElim);
        test.assert_output(input);
    }

    /// Expressions with unavailable operands are not hoisted.
    #[test]
    fn test_pre_skips_unavailable_operands() {
        let input = r#"
function test(v0: boolean): int32 {
b0(v0: boolean):
    branch v0, b1, b2
b1:
    v1: int32 = 1int32
    jump b3
b2:
    v2: int32 = 2int32
    jump b3
b3:
    v3: int32 = int.add v1, v2
    return v3
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&PartialRedundancyElim);
        test.assert_output(input);
    }

    /// Expressions already available on all paths are left to GVN.
    #[test]
    fn test_pre_skips_fully_redundant_expression() {
        let input = r#"
function test(v0: int32, v1: int32, v2: boolean): int32 {
b0(v0: int32, v1: int32, v2: boolean):
    v3: int32 = int.add v0, v1
    branch v2, b1, b2
b1:
    jump b3
b2:
    jump b3
b3:
    v4: int32 = int.add v0, v1
    return v4
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&PartialRedundancyElim);
        test.assert_output(input);
    }
}
