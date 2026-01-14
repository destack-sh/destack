use std::collections::{HashMap, HashSet, VecDeque};

use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::optimize::analyses::{
    AvailableExpressions, ControlFlowGraph, DominatorTree, OwnershipAnalysis,
};
use crate::optimize::common::{
    SuccessorArguments, build_use_def_maps, collect_reachable_blocks, compute_dominance_frontiers,
    instruction_has_side_effects, instruction_is_speculatable,
    terminator_arguments_for_successor_checked,
};
use crate::optimize::{
    AnalysisPreservation, ExpressionKey, FunctionAnalyses, FunctionPass, PipelineContext,
    apply_substitutions_in_function, expression_key_from_instruction, expression_key_substitute,
};

declare_pass! {
    /// Eliminate partially redundant expressions by inserting computations.
    ///
    /// This pass computes SSA like phi values for pure expressions at join points.
    /// This pass inserts missing computations on incoming edges.
    /// This pass removes redundant recomputations dominated by the new values.
    ///
    /// ```mir
    /// function @before(v0: i32, v1: i32, v2: bool) -> i32 {
    /// block0(v0: i32, v1: i32, v2: bool):
    ///     branch v2, block1, block2
    /// block1:
    ///     v3 = iadd v0, v1
    ///     jump block3
    /// block2:
    ///     jump block3
    /// block3:
    ///     v4 = iadd v0, v1
    ///     return v4
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function @after(v0: i32, v1: i32, v2: bool) -> i32 {
    /// block0(v0: i32, v1: i32, v2: bool):
    ///     branch v2, block1, block2
    /// block1:
    ///     v3 = iadd v0, v1
    ///     jump block3(v3)
    /// block2:
    ///     v4 = iadd v0, v1
    ///     jump block3(v4)
    /// block3(v5: i32):
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
        tree: &mut mir::NodeTree,
        _ctx: &PipelineContext<'_>,
    ) -> AnalysisPreservation {
        // skip imported functions
        let Some(entry) = function.entry else {
            return AnalysisPreservation::all();
        };

        // gather analyses
        let analyses = FunctionAnalyses::new(function, tree);
        let cfg = analyses.get::<ControlFlowGraph>().clone();
        let domtree = analyses.get::<DominatorTree>().clone();
        let available = analyses.get::<AvailableExpressions>().clone();
        let ownership = analyses.get::<OwnershipAnalysis>().clone();

        // run PRE
        let changed = run_pre(
            entry, function, tree, &cfg, &domtree, &available, &ownership,
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
    param: mir::TypedValue,
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
        to_type: mir::LocalNodeId<mir::Type>,
    },
    /// Select expression template.
    Select,
    /// Field access expression template.
    FieldGet { index: u32 },
    /// Element access expression template.
    ElementGet,
    /// Global constant expression template.
    GlobalConst,
}

/// Run PRE on a single function and report whether it changed.
fn run_pre(
    entry: mir::LocalNodeId<mir::Block>,
    function: &mut mir::Function,
    tree: &mut mir::NodeTree,
    cfg: &ControlFlowGraph,
    domtree: &DominatorTree,
    available: &AvailableExpressions,
    ownership: &OwnershipAnalysis,
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
            let Some(destination) = instruction.destination() else {
                continue;
            };
            let Some(value_type) = ownership.value_type(destination) else {
                continue;
            };

            // track whether the expression can be speculated
            let is_speculatable = instruction_is_speculatable(instruction);

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
            let param_value = function.next_value();
            let param = mir::TypedValue::new(param_value, value_type);
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
            block.parameters.push(placement.param);
        }
        tree.replace(block_id, block);
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
                    let inserted_value = insert_expression_in_block(
                        pred,
                        insertion_block,
                        placement.key.clone(),
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
            to_type: *to_type,
        },
        mir::Instruction::Select { .. } => ExpressionTemplate::Select,
        mir::Instruction::FieldGet { index, .. } => ExpressionTemplate::FieldGet { index: *index },
        mir::Instruction::ElementGet { .. } => ExpressionTemplate::ElementGet,
        mir::Instruction::GlobalConst { .. } => ExpressionTemplate::GlobalConst,
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
        ExpressionKey::ElementGet { array, index } => vec![*array, *index],
        ExpressionKey::GlobalConst { .. } => Vec::new(),
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
    use_def: &crate::optimize::common::UseDefMaps,
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
    use_def: &crate::optimize::common::UseDefMaps,
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
    use_def: &crate::optimize::common::UseDefMaps,
) -> bool {
    // function parameters are always available
    if function.parameters.iter().any(|param| param.value == value) {
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
    tree: &mir::NodeTree,
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
                .push(placement.param.value);
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
        let Some(destination) = instruction.destination() else {
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
    function: &mut mir::Function,
    tree: &mut mir::NodeTree,
    template: Option<&ExpressionTemplate>,
    use_def: &crate::optimize::common::UseDefMaps,
    domtree: &DominatorTree,
) -> Option<mir::Value> {
    // require a template describing how to rebuild the expression
    let template = template?;

    // require operands to be available at the insertion point
    if !operands_available_in_block(&key, availability_block, function, domtree, use_def) {
        return None;
    }

    // build the new instruction
    let destination = function.next_value();
    let instruction = build_instruction_from_key(&key, template, destination);
    let instruction_id = tree.insert(instruction);

    // insert into the block before the terminator
    let mut block = tree.get(insert_block).clone();
    block.instructions.push(instruction_id);
    tree.replace(insert_block, block);

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
                destination,
                operator: *operator,
                left: *left,
                right: *right,
            }
        }
        (ExpressionKey::Unary { argument, .. }, ExpressionTemplate::Unary { operator }) => {
            mir::Instruction::Unary {
                destination,
                operator: *operator,
                argument: *argument,
            }
        }
        (ExpressionKey::Cast { argument, .. }, ExpressionTemplate::Cast { operator, to_type }) => {
            mir::Instruction::Cast {
                destination,
                operator: *operator,
                argument: *argument,
                to_type: *to_type,
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
            destination,
            condition: *condition,
            then_value: *then_value,
            else_value: *else_value,
        },
        (ExpressionKey::FieldGet { aggregate, .. }, ExpressionTemplate::FieldGet { index }) => {
            mir::Instruction::FieldGet {
                destination,
                aggregate: *aggregate,
                index: *index,
            }
        }
        (ExpressionKey::ElementGet { array, index }, ExpressionTemplate::ElementGet) => {
            mir::Instruction::ElementGet {
                destination,
                array: *array,
                index: *index,
            }
        }
        (ExpressionKey::GlobalConst { global }, ExpressionTemplate::GlobalConst) => {
            mir::Instruction::GlobalConst {
                destination,
                global: *global,
            }
        }
        _ => panic!("mismatched expression template"),
    }
}

/// Append extra arguments to edges that target a successor block.
fn append_successor_arguments(
    tree: &mut mir::NodeTree,
    block_id: mir::LocalNodeId<mir::Block>,
    successor: mir::LocalNodeId<mir::Block>,
    extra_args: &[mir::Value],
) {
    // clone the block for terminator updates
    let mut block = tree.get(block_id).clone();

    // compute an updated terminator when this edge targets the successor
    let new_terminator = match &mut block.terminator {
        mir::Terminator::Jump { target, arguments } if *target == successor => {
            let mut new_args = arguments.clone();
            new_args.extend(extra_args.iter().copied());
            mir::Terminator::Jump {
                target: *target,
                arguments: new_args,
            }
        }
        mir::Terminator::Branch {
            condition,
            then_target,
            then_arguments,
            else_target,
            else_arguments,
        } => {
            let mut new_then = then_arguments.clone();
            let mut new_else = else_arguments.clone();
            if *then_target == successor {
                new_then.extend(extra_args.iter().copied());
            }
            if *else_target == successor {
                new_else.extend(extra_args.iter().copied());
            }
            mir::Terminator::Branch {
                condition: *condition,
                then_target: *then_target,
                then_arguments: new_then,
                else_target: *else_target,
                else_arguments: new_else,
            }
        }
        mir::Terminator::Check {
            condition,
            constraint,
            success,
            failure,
        } => {
            let mut success_args = success.arguments.clone();
            let mut failure_args = failure.arguments.clone();
            if success.target == successor {
                success_args.extend(extra_args.iter().copied());
            }
            if failure.target == successor {
                failure_args.extend(extra_args.iter().copied());
            }
            mir::Terminator::Check {
                condition: *condition,
                constraint: constraint.clone(),
                success: mir::CheckTarget {
                    target: success.target,
                    arguments: success_args,
                },
                failure: mir::CheckTarget {
                    target: failure.target,
                    arguments: failure_args,
                },
            }
        }
        mir::Terminator::Switch {
            value,
            default,
            default_arguments,
            cases,
        } => {
            let mut new_default = default_arguments.clone();
            if *default == successor {
                new_default.extend(extra_args.iter().copied());
            }

            let mut new_cases = Vec::with_capacity(cases.len());
            for case in cases {
                if case.target == successor {
                    let mut new_args = case.arguments.clone();
                    new_args.extend(extra_args.iter().copied());
                    new_cases.push(mir::SwitchCase {
                        value: case.value,
                        target: case.target,
                        arguments: new_args,
                    });
                } else {
                    new_cases.push(case.clone());
                }
            }

            mir::Terminator::Switch {
                value: *value,
                default: *default,
                default_arguments: new_default,
                cases: new_cases,
            }
        }
        mir::Terminator::Yield {
            value,
            resume,
            resume_arguments,
        } => {
            let mut new_resume = resume_arguments.clone();
            if *resume == successor {
                new_resume.extend(extra_args.iter().copied());
            }
            mir::Terminator::Yield {
                value: *value,
                resume: *resume,
                resume_arguments: new_resume,
            }
        }
        _ => block.terminator.clone(),
    };

    // write back only when arguments changed
    if new_terminator != block.terminator {
        block.terminator = new_terminator;
        tree.replace(block_id, block);
    }
}

/// Ensure insertions happen on the correct edge when needed.
fn ensure_edge_block(
    predecessor: mir::LocalNodeId<mir::Block>,
    successor: mir::LocalNodeId<mir::Block>,
    function: &mut mir::Function,
    tree: &mut mir::NodeTree,
    cfg: &ControlFlowGraph,
    edge_blocks: &mut HashMap<
        (mir::LocalNodeId<mir::Block>, mir::LocalNodeId<mir::Block>),
        mir::LocalNodeId<mir::Block>,
    >,
    changed: &mut bool,
) -> mir::LocalNodeId<mir::Block> {
    // skip non critical edges
    let pred_block = tree.get(predecessor);
    if successor_count(pred_block) <= 1 || cfg.predecessors(successor).len() <= 1 {
        return predecessor;
    }

    // reuse existing edge blocks when already split
    if let Some(existing) = edge_blocks.get(&(predecessor, successor)) {
        return *existing;
    }

    // extract the successor arguments for this edge
    let args = match terminator_arguments_for_successor_checked(&pred_block.terminator, successor) {
        SuccessorArguments::Consistent(args) => args.to_vec(),
        _ => return predecessor,
    };

    // build the new edge block
    let mut edge_block = mir::Block::new();
    edge_block.terminator = mir::Terminator::Jump {
        target: successor,
        arguments: args,
    };
    let edge_block_id = tree.insert(edge_block);
    insert_block_after(function, predecessor, edge_block_id);

    // redirect the predecessor to the edge block
    if redirect_successor_to_edge(predecessor, successor, edge_block_id, tree) {
        edge_blocks.insert((predecessor, successor), edge_block_id);
        *changed = true;
        edge_block_id
    } else {
        predecessor
    }
}

/// Count the unique successors for a block.
fn successor_count(block: &mir::Block) -> usize {
    // track unique successors
    let mut unique = HashSet::new();
    for successor in block.terminator.successors() {
        unique.insert(successor);
    }

    // return the unique count
    unique.len()
}

/// Insert a block immediately after the predecessor.
fn insert_block_after(
    function: &mut mir::Function,
    predecessor: mir::LocalNodeId<mir::Block>,
    block: mir::LocalNodeId<mir::Block>,
) {
    // insert directly after the predecessor when it exists
    if let Some(index) = function.blocks.iter().position(|id| *id == predecessor) {
        function.blocks.insert(index + 1, block);
        return;
    }

    // fallback to appending when the predecessor is not found
    function.blocks.push(block);
}

/// Redirect a successor edge to a new edge block.
fn redirect_successor_to_edge(
    block_id: mir::LocalNodeId<mir::Block>,
    successor: mir::LocalNodeId<mir::Block>,
    edge_block: mir::LocalNodeId<mir::Block>,
    tree: &mut mir::NodeTree,
) -> bool {
    // clone the terminator for updates
    let mut block = tree.get(block_id).clone();

    // build a new terminator that targets the edge block
    let new_terminator = match &block.terminator {
        mir::Terminator::Jump { target, .. } if *target == successor => mir::Terminator::Jump {
            target: edge_block,
            arguments: Vec::new(),
        },
        mir::Terminator::Branch {
            condition,
            then_target,
            then_arguments,
            else_target,
            else_arguments,
        } => {
            let mut new_then = *then_target;
            let mut new_else = *else_target;
            let mut then_args = then_arguments.clone();
            let mut else_args = else_arguments.clone();

            if *then_target == successor {
                new_then = edge_block;
                then_args = Vec::new();
            }
            if *else_target == successor {
                new_else = edge_block;
                else_args = Vec::new();
            }

            if new_then == *then_target && new_else == *else_target {
                return false;
            }

            mir::Terminator::Branch {
                condition: *condition,
                then_target: new_then,
                then_arguments: then_args,
                else_target: new_else,
                else_arguments: else_args,
            }
        }
        mir::Terminator::Check {
            condition,
            constraint,
            success,
            failure,
        } => {
            let mut new_success = success.clone();
            let mut new_failure = failure.clone();

            if success.target == successor {
                new_success.target = edge_block;
                new_success.arguments = Vec::new();
            }
            if failure.target == successor {
                new_failure.target = edge_block;
                new_failure.arguments = Vec::new();
            }

            if new_success.target == success.target && new_failure.target == failure.target {
                return false;
            }

            mir::Terminator::Check {
                condition: *condition,
                constraint: constraint.clone(),
                success: new_success,
                failure: new_failure,
            }
        }
        mir::Terminator::Switch {
            value,
            default,
            default_arguments,
            cases,
        } => {
            let mut new_default = *default;
            let mut new_default_args = default_arguments.clone();
            let mut updated = false;
            if *default == successor {
                new_default = edge_block;
                new_default_args = Vec::new();
                updated = true;
            }

            let mut new_cases = Vec::with_capacity(cases.len());
            for case in cases {
                if case.target == successor {
                    new_cases.push(mir::SwitchCase {
                        value: case.value,
                        target: edge_block,
                        arguments: Vec::new(),
                    });
                    updated = true;
                } else {
                    new_cases.push(case.clone());
                }
            }

            if !updated {
                return false;
            }

            mir::Terminator::Switch {
                value: *value,
                default: new_default,
                default_arguments: new_default_args,
                cases: new_cases,
            }
        }
        mir::Terminator::Yield {
            value,
            resume,
            resume_arguments,
        } if *resume == successor => mir::Terminator::Yield {
            value: *value,
            resume: edge_block,
            resume_arguments: Vec::new(),
        },
        _ => return false,
    };

    // apply the updated terminator when it differs
    if new_terminator != block.terminator {
        block.terminator = new_terminator;
        tree.replace(block_id, block);
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Partial redundancy on a diamond inserts the missing computation.
    #[test]
    fn test_pre_diamond_inserts_expression() {
        let input = r#"function @test(v0: i32, v1: i32, v2: bool) -> i32 {
block0(v0: i32, v1: i32, v2: bool):
    branch v2, block1, block2
block1:
    v3 = iadd v0, v1
    jump block3
block2:
    jump block3
block3:
    v4 = iadd v0, v1
    return v4
}"#;

        let expected = r#"function @test(v0: i32, v1: i32, v2: bool) -> i32 {
block0(v0: i32, v1: i32, v2: bool):
    branch v2, block1, block2
block1:
    v3 = iadd v0, v1
    jump block3(v3)
block2:
    v6 = iadd v0, v1
    jump block3(v6)
block3(v5: i32):
    return v5
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&PartialRedundancyElim);
        program.assert_output(expected);
    }

    /// Critical edges receive a split block for inserted expressions.
    #[test]
    fn test_pre_splits_critical_edge_for_insertion() {
        let input = r#"function @test(v0: i32, v1: i32, v2: bool) -> i32 {
block0(v0: i32, v1: i32, v2: bool):
    branch v2, block1, block2
block1:
    v3 = iadd v0, v1
    jump block3
block2:
    branch v2, block3, block4
block3:
    v4 = iadd v0, v1
    return v4
block4:
    return v0
}"#;

        let expected = r#"function @test(v0: i32, v1: i32, v2: bool) -> i32 {
block0(v0: i32, v1: i32, v2: bool):
    branch v2, block1, block2
block1:
    v3 = iadd v0, v1
    jump block4(v3)
block2:
    branch v2, block3, block5
block3:
    v6 = iadd v0, v1
    jump block4(v6)
block4(v5: i32):
    return v5
block5:
    return v0
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&PartialRedundancyElim);
        program.assert_output(expected);
    }

    /// Switch default edges receive inserted expressions.
    #[test]
    fn test_pre_switch_default_inserts_expression() {
        let input = r#"function @test(v0: i32, v1: i32, v2: i32) -> i32 {
block0(v0: i32, v1: i32, v2: i32):
    switch v2, block2, 0 => block1
block1:
    v3 = iadd v0, v1
    jump block2
block2:
    v4 = iadd v0, v1
    return v4
}"#;

        let expected = r#"function @test(v0: i32, v1: i32, v2: i32) -> i32 {
block0(v0: i32, v1: i32, v2: i32):
    switch v2, block1, 0 => block2
block1:
    v6 = iadd v0, v1
    jump block3(v6)
block2:
    v3 = iadd v0, v1
    jump block3(v3)
block3(v5: i32):
    return v5
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&PartialRedundancyElim);
        program.assert_output(expected);
    }

    /// Switch cases keep their existing arguments when adding expressions.
    #[test]
    fn test_pre_switch_case_appends_expression() {
        let input = r#"function @test(v0: i32, v1: i32, v2: i32) -> i32 {
block0(v0: i32, v1: i32, v2: i32):
    v3 = iconst 7i32
    switch v2, block2, 0 => block1, 1 => block3(v3)
block1:
    v4 = iadd v0, v1
    jump block3(v3)
block2:
    v5 = iconst 0i32
    return v5
block3(v6: i32):
    v7 = iadd v0, v1
    return v7
}"#;

        let expected = r#"function @test(v0: i32, v1: i32, v2: i32) -> i32 {
block0(v0: i32, v1: i32, v2: i32):
    v3 = iconst 7i32
    switch v2, block3, 0 => block2, 1 => block1
block1:
    v9 = iadd v0, v1
    jump block4(v3, v9)
block2:
    v4 = iadd v0, v1
    jump block4(v3, v4)
block3:
    v5 = iconst 0i32
    return v5
block4(v6: i32, v8: i32):
    return v8
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&PartialRedundancyElim);
        program.assert_output(expected);
    }

    /// Check edges receive inserted expressions.
    #[test]
    fn test_pre_check_inserts_expression() {
        let input = r#"function @test(v0: u32, v1: u32, v2: bool, v3: [u8; 8]) -> u32 {
block0(v0: u32, v1: u32, v2: bool, v3: [u8; 8]):
    branch v2, block1, block2
block1:
    v4 = iadd v0, v1
    v5 = icmp_ult v0, v1
    check v5, bounds.unsigned v0, v1, v3, block3, block4
block2:
    jump block3
block3:
    v6 = iadd v0, v1
    return v6
block4:
    unreachable
}"#;

        let expected = r#"function @test(v0: u32, v1: u32, v2: bool, v3: [u8; 8]) -> u32 {
block0(v0: u32, v1: u32, v2: bool, v3: [u8; 8]):
    branch v2, block1, block3
block1:
    v4 = iadd v0, v1
    v5 = icmp_ult v0, v1
    check v5, bounds.unsigned v0, v1, v3, block2, block5
block2:
    jump block4(v4)
block3:
    v8 = iadd v0, v1
    jump block4(v8)
block4(v7: u32):
    return v7
block5:
    unreachable
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&PartialRedundancyElim);
        program.assert_output(expected);
    }

    /// Check failure edges receive inserted expressions.
    #[test]
    fn test_pre_check_failure_inserts_expression() {
        let input = r#"function @test(v0: u32, v1: u32, v2: bool, v3: [u8; 8]) -> u32 {
block0(v0: u32, v1: u32, v2: bool, v3: [u8; 8]):
    branch v2, block1, block2
block1:
    v4 = iadd v0, v1
    v5 = icmp_ult v0, v1
    check v5, bounds.unsigned v0, v1, v3, block3, block4
block2:
    jump block4
block3:
    return v4
block4:
    v6 = iadd v0, v1
    return v6
}"#;

        let expected = r#"function @test(v0: u32, v1: u32, v2: bool, v3: [u8; 8]) -> u32 {
block0(v0: u32, v1: u32, v2: bool, v3: [u8; 8]):
    branch v2, block1, block3
block1:
    v4 = iadd v0, v1
    v5 = icmp_ult v0, v1
    check v5, bounds.unsigned v0, v1, v3, block4, block2
block2:
    jump block5(v4)
block3:
    v8 = iadd v0, v1
    jump block5(v8)
block4:
    return v4
block5(v7: u32):
    return v7
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&PartialRedundancyElim);
        program.assert_output(expected);
    }

    /// Yield resume edges receive inserted expressions.
    #[test]
    fn test_pre_yield_resume_inserts_expression() {
        let input = r#"function @test(v0: i32, v1: i32, v2: bool) -> i32 {
block0(v0: i32, v1: i32, v2: bool):
    branch v2, block1, block2
block1:
    v3 = iadd v0, v1
    v4 = iconst 1i32
    yield v4, block3(v0)
block2:
    v5 = iconst 2i32
    yield v5, block3(v0)
block3(v6: i32, v7: i32):
    v8 = iadd v0, v1
    return v8
}"#;

        let expected = r#"function @test(v0: i32, v1: i32, v2: bool) -> i32 {
block0(v0: i32, v1: i32, v2: bool):
    branch v2, block1, block2
block1:
    v3 = iadd v0, v1
    v4 = iconst 1i32
    yield v4, block3(v0, v3)
block2:
    v5 = iconst 2i32
    v10 = iadd v0, v1
    yield v5, block3(v0, v10)
block3(v6: i32, v7: i32, v9: i32):
    return v9
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&PartialRedundancyElim);
        program.assert_output(expected);
    }

    /// Non speculatable expressions are not inserted on new paths.
    #[test]
    fn test_pre_skips_non_speculatable_expression() {
        let input = r#"function @test(v0: i32, v1: i32, v2: bool) -> i32 {
block0(v0: i32, v1: i32, v2: bool):
    branch v2, block1, block2
block1:
    v3 = sdiv v0, v1
    jump block3
block2:
    jump block3
block3:
    v4 = sdiv v0, v1
    return v4
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&PartialRedundancyElim);
        program.assert_output(input);
    }

    /// Float to int casts are not speculated.
    #[test]
    fn test_pre_skips_float_to_int_cast() {
        let input = r#"function @test(v0: f32, v1: bool) -> i32 {
block0(v0: f32, v1: bool):
    branch v1, block1, block2
block1:
    v2 = fcvt_to_sint v0 -> i32
    jump block3
block2:
    jump block3
block3:
    v3 = fcvt_to_sint v0 -> i32
    return v3
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&PartialRedundancyElim);
        program.assert_output(input);
    }

    /// Calls are not considered for PRE.
    #[test]
    fn test_pre_skips_calls() {
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    branch v0, block1, block2
block1:
    v1 = call @get_value()
    jump block3
block2:
    jump block3
block3:
    v2 = call @get_value()
    return v2
}
function @get_value() -> i32 {
block0:
    v0 = iconst 42i32
    return v0
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&PartialRedundancyElim);
        program.assert_output(input);
    }

    /// Expressions with unavailable operands are not hoisted.
    #[test]
    fn test_pre_skips_unavailable_operands() {
        let input = r#"function @test(v0: bool) -> i32 {
block0(v0: bool):
    branch v0, block1, block2
block1:
    v1 = iconst 1i32
    jump block3
block2:
    v2 = iconst 2i32
    jump block3
block3:
    v3 = iadd v1, v2
    return v3
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&PartialRedundancyElim);
        program.assert_output(input);
    }

    /// Expressions already available on all paths are left to GVN.
    #[test]
    fn test_pre_skips_fully_redundant_expression() {
        let input = r#"function @test(v0: i32, v1: i32, v2: bool) -> i32 {
block0(v0: i32, v1: i32, v2: bool):
    v3 = iadd v0, v1
    branch v2, block1, block2
block1:
    jump block3
block2:
    jump block3
block3:
    v4 = iadd v0, v1
    return v4
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&PartialRedundancyElim);
        program.assert_output(input);
    }
}
