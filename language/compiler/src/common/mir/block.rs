use std::collections::{HashMap, HashSet, VecDeque};

use destack_mir as mir;

use crate::common::mir::analysis::{ControlFlowGraph, DominatorTree};
use crate::common::mir::{instruction_substitute_uses_in_tree, remap_instruction_memory_accesses};

/// Check if a terminator uses a specific value.
///
/// Returns true if the value appears in any operand position of the terminator.
/// This includes branch conditions, return values, and block arguments.
pub fn terminator_uses(term: &mir::Terminator, value: mir::Value) -> bool {
    term.uses().contains(&mir::ValueReference::Value(value))
}

/// Get all values used by a terminator.
///
/// Returns a vector of all value operands in the terminator, including
/// conditions, return values, and block arguments.
pub fn terminator_used_values(term: &mir::Terminator) -> Vec<mir::Value> {
    term.uses()
        .iter()
        .filter_map(|value| value.value())
        .collect()
}

/// Get the arguments passed to a specific successor block from a terminator.
pub fn terminator_arguments_for_successor(
    terminator: &mir::Terminator,
    successor: mir::LocalNodeId<mir::Block>,
) -> &[mir::ValueReference] {
    match terminator {
        mir::Terminator::Jump { target } if target.block.block() == Some(successor) => {
            &target.arguments
        }

        mir::Terminator::Branch {
            then_target,
            else_target,
            ..
        } => {
            // then branch
            if then_target.block.block() == Some(successor) {
                &then_target.arguments
            }
            // else branch
            else if else_target.block.block() == Some(successor) {
                &else_target.arguments
            }
            // not a successor
            else {
                &[]
            }
        }
        mir::Terminator::Check {
            success, failure, ..
        } => {
            if success.block.block() == Some(successor) {
                &success.arguments
            } else if failure.block.block() == Some(successor) {
                &failure.arguments
            } else {
                &[]
            }
        }

        mir::Terminator::Switch { default, cases, .. } => {
            // default case
            if default.block.block() == Some(successor) {
                return &default.arguments;
            }

            // numbered cases
            for case in cases {
                if case.target.block.block() == Some(successor) {
                    return &case.target.arguments;
                }
            }

            &[]
        }

        mir::Terminator::Yield { resume, .. } if resume.block.block() == Some(successor) => {
            &resume.arguments
        }
        mir::Terminator::Call { target, .. }
        | mir::Terminator::CallIndirect { target, .. }
        | mir::Terminator::CallVirtual { target, .. }
        | mir::Terminator::CallDynamic { target, .. }
            if target.block.block() == Some(successor) =>
        {
            &target.arguments
        }

        _ => &[],
    }
}

/// Collect blocks reachable from the entry in function order.
pub fn collect_reachable_blocks(
    function: &mir::Function,
    tree: &mir::Tree,
    entry: mir::LocalNodeId<mir::Block>,
) -> Vec<mir::LocalNodeId<mir::Block>> {
    // seed worklist with entry
    let mut worklist = VecDeque::new();
    let mut visited = HashSet::new();
    worklist.push_back(entry);
    visited.insert(entry);

    // bfs over successors
    while let Some(block) = worklist.pop_front() {
        let block_data = tree.get(block);
        let terminator = tree.get(block_data.terminator);
        for successor in terminator.successors() {
            let Some(successor) = successor.block() else {
                continue;
            };
            if visited.insert(successor) {
                worklist.push_back(successor);
            }
        }
    }

    // preserve function block order
    function
        .blocks
        .iter()
        .copied()
        .filter(|block| visited.contains(block))
        .collect()
}

/// Compute dominance frontiers for a list of blocks.
pub fn compute_dominance_frontiers(
    blocks: &[mir::LocalNodeId<mir::Block>],
    cfg: &ControlFlowGraph,
    domtree: &DominatorTree,
) -> HashMap<mir::LocalNodeId<mir::Block>, HashSet<mir::LocalNodeId<mir::Block>>> {
    // initialize frontiers for each block
    let mut frontiers: HashMap<
        mir::LocalNodeId<mir::Block>,
        HashSet<mir::LocalNodeId<mir::Block>>,
    > = blocks
        .iter()
        .copied()
        .map(|block| (block, HashSet::new()))
        .collect();

    // compute dominance frontiers with the standard algorithm
    for &block in blocks {
        let preds = cfg.predecessors(block);
        if preds.len() < 2 {
            continue;
        }

        let idom = domtree.immediate_dominator(block);
        for &pred in preds {
            let mut runner = pred;
            while Some(runner) != idom
                && runner != block
                && domtree.immediate_dominator(runner).is_some()
            {
                if let Some(frontier) = frontiers.get_mut(&runner) {
                    frontier.insert(block);
                }

                runner = domtree.immediate_dominator(runner).unwrap();
            }
        }
    }

    frontiers
}

/// Result of collecting arguments for a successor edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuccessorArguments<'a> {
    /// No edge to the successor was found.
    Missing,
    /// A single consistent argument list for all matching edges.
    Consistent(&'a [mir::ValueReference]),
    /// Multiple edges to the successor disagree on arguments.
    Conflict,
}

/// Edge splitting policy for inserting edge blocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeSplitPolicy {
    /// Split only critical edges.
    CriticalOnly,
    /// Split when the predecessor has multiple successors.
    PredecessorMultiSuccessor,
}

/// Append extra arguments to edges that target a successor block.
pub fn append_successor_arguments(
    tree: &mut mir::Tree,
    block_id: mir::LocalNodeId<mir::Block>,
    successor: mir::LocalNodeId<mir::Block>,
    extra_args: &[mir::Value],
) {
    // clone the terminator for updates
    let terminator_id = tree.get(block_id).terminator;
    let terminator = tree.get(terminator_id).clone();

    // compute an updated terminator when this edge targets the successor
    let new_terminator = match &terminator {
        mir::Terminator::Jump { target } if target.block.block() == Some(successor) => {
            let mut new_target = target.clone();
            new_target
                .arguments
                .extend(extra_args.iter().copied().map(mir::ValueReference::from));

            mir::Terminator::Jump { target: new_target }
        }
        mir::Terminator::Branch {
            condition,
            then_target,
            else_target,
        } => {
            let mut new_then = then_target.clone();
            let mut new_else = else_target.clone();

            if then_target.block.block() == Some(successor) {
                new_then
                    .arguments
                    .extend(extra_args.iter().copied().map(mir::ValueReference::from));
            }

            if else_target.block.block() == Some(successor) {
                new_else
                    .arguments
                    .extend(extra_args.iter().copied().map(mir::ValueReference::from));
            }

            mir::Terminator::Branch {
                condition: *condition,
                then_target: new_then,
                else_target: new_else,
            }
        }
        mir::Terminator::Check {
            constraint,
            success,
            failure,
        } => {
            let mut new_success = success.clone();
            let mut new_failure = failure.clone();

            if success.block.block() == Some(successor) {
                new_success
                    .arguments
                    .extend(extra_args.iter().copied().map(mir::ValueReference::from));
            }

            if failure.block.block() == Some(successor) {
                new_failure
                    .arguments
                    .extend(extra_args.iter().copied().map(mir::ValueReference::from));
            }

            mir::Terminator::Check {
                constraint: constraint.clone(),
                success: new_success,
                failure: new_failure,
            }
        }
        mir::Terminator::Switch {
            value,
            default,
            cases,
        } => {
            let mut new_default = default.clone();
            if default.block.block() == Some(successor) {
                new_default
                    .arguments
                    .extend(extra_args.iter().copied().map(mir::ValueReference::from));
            }

            let mut new_cases = Vec::with_capacity(cases.len());
            for case in cases {
                if case.target.block.block() == Some(successor) {
                    let mut new_target = case.target.clone();
                    new_target
                        .arguments
                        .extend(extra_args.iter().copied().map(mir::ValueReference::from));

                    new_cases.push(mir::SwitchCase {
                        value: case.value,
                        target: new_target,
                    });
                } else {
                    new_cases.push(case.clone());
                }
            }

            mir::Terminator::Switch {
                value: *value,
                default: new_default,
                cases: new_cases,
            }
        }
        mir::Terminator::Yield { value, resume } => {
            let mut new_resume = resume.clone();
            if resume.block.block() == Some(successor) {
                new_resume
                    .arguments
                    .extend(extra_args.iter().copied().map(mir::ValueReference::from));
            }

            mir::Terminator::Yield {
                value: *value,
                resume: new_resume,
            }
        }
        _ => terminator.clone(),
    };

    // write back only when arguments changed
    if new_terminator != terminator {
        tree.replace(terminator_id, new_terminator);
    }
}

/// Ensure insertions happen on the correct edge when needed.
// allow many arguments to keep the call sites explicit
#[allow(clippy::too_many_arguments)]
pub fn ensure_edge_block(
    predecessor: mir::LocalNodeId<mir::Block>,
    successor: mir::LocalNodeId<mir::Block>,
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    cfg: &ControlFlowGraph,
    edge_blocks: &mut HashMap<
        (mir::LocalNodeId<mir::Block>, mir::LocalNodeId<mir::Block>),
        mir::LocalNodeId<mir::Block>,
    >,
    policy: EdgeSplitPolicy,
    changed: &mut bool,
) -> mir::LocalNodeId<mir::Block> {
    // skip edges that do not require splitting
    let pred_block = tree.get(predecessor);
    let pred_terminator = tree.get(pred_block.terminator);
    let pred_multi = successor_count(pred_terminator) > 1;
    let succ_multi = cfg.predecessors(successor).len() > 1;
    let should_split = match policy {
        EdgeSplitPolicy::CriticalOnly => pred_multi && succ_multi,
        EdgeSplitPolicy::PredecessorMultiSuccessor => pred_multi,
    };
    if !should_split {
        return predecessor;
    }

    // reuse existing edge blocks when already split
    if let Some(existing) = edge_blocks.get(&(predecessor, successor)) {
        return *existing;
    }

    // extract the successor arguments for this edge
    let args: Vec<mir::Value> =
        match terminator_arguments_for_successor_checked(pred_terminator, successor) {
            SuccessorArguments::Consistent(args) => {
                args.iter().filter_map(|value| value.value()).collect()
            }
            _ => return predecessor,
        };

    // build the new edge block
    let edge_terminator = tree.insert(mir::Terminator::Jump {
        target: mir::BlockTarget {
            block: successor.into(),
            arguments: args.into_iter().map(mir::ValueReference::from).collect(),
        },
    });
    let edge_block = mir::Block::new(edge_terminator);
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

/// Count the unique successors for a terminator.
fn successor_count(terminator: &mir::Terminator) -> usize {
    // track unique successors
    let mut unique = HashSet::new();
    for successor in terminator.successors() {
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
    tree: &mut mir::Tree,
) -> bool {
    // clone the terminator for updates
    let terminator_id = tree.get(block_id).terminator;
    let terminator = tree.get(terminator_id).clone();

    // build a new terminator that targets the edge block
    let new_terminator = match &terminator {
        mir::Terminator::Jump { target } if target.block.block() == Some(successor) => {
            mir::Terminator::Jump {
                target: mir::BlockTarget {
                    block: mir::BlockReference::from(edge_block),
                    arguments: Vec::new(),
                },
            }
        }
        mir::Terminator::Branch {
            condition,
            then_target,
            else_target,
        } => {
            let mut new_then = then_target.clone();
            let mut new_else = else_target.clone();

            if then_target.block.block() == Some(successor) {
                new_then.block = mir::BlockReference::from(edge_block);
                new_then.arguments = Vec::new();
            }

            if else_target.block.block() == Some(successor) {
                new_else.block = mir::BlockReference::from(edge_block);
                new_else.arguments = Vec::new();
            }

            if new_then == *then_target && new_else == *else_target {
                return false;
            }

            mir::Terminator::Branch {
                condition: *condition,
                then_target: new_then,
                else_target: new_else,
            }
        }
        mir::Terminator::Check {
            constraint,
            success,
            failure,
        } => {
            let mut new_success = success.clone();
            let mut new_failure = failure.clone();

            if success.block.block() == Some(successor) {
                new_success.block = mir::BlockReference::from(edge_block);
                new_success.arguments = Vec::new();
            }

            if failure.block.block() == Some(successor) {
                new_failure.block = mir::BlockReference::from(edge_block);
                new_failure.arguments = Vec::new();
            }

            if new_success.block == success.block && new_failure.block == failure.block {
                return false;
            }

            mir::Terminator::Check {
                constraint: constraint.clone(),
                success: new_success,
                failure: new_failure,
            }
        }
        mir::Terminator::Switch {
            value,
            default,
            cases,
        } => {
            let mut new_default = default.clone();
            let mut new_cases = Vec::with_capacity(cases.len());
            let mut changed = false;

            if default.block.block() == Some(successor) {
                new_default.block = mir::BlockReference::from(edge_block);
                new_default.arguments = Vec::new();
                changed = true;
            }

            for case in cases {
                if case.target.block.block() == Some(successor) {
                    changed = true;
                    let mut new_target = case.target.clone();
                    new_target.block = mir::BlockReference::from(edge_block);
                    new_target.arguments = Vec::new();

                    new_cases.push(mir::SwitchCase {
                        value: case.value,
                        target: new_target,
                    });
                } else {
                    new_cases.push(case.clone());
                }
            }

            if !changed {
                return false;
            }

            mir::Terminator::Switch {
                value: *value,
                default: new_default,
                cases: new_cases,
            }
        }
        mir::Terminator::Yield { value, resume } => {
            // rewrite the resume target when it matches the successor
            let new_resume = if resume.block.block() == Some(successor) {
                let mut new_resume = resume.clone();
                new_resume.block = mir::BlockReference::from(edge_block);
                new_resume.arguments = Vec::new();

                new_resume
            } else {
                return false;
            };

            mir::Terminator::Yield {
                value: *value,
                resume: new_resume,
            }
        }
        _ => return false,
    };

    // update the terminator
    if new_terminator != terminator {
        tree.replace(terminator_id, new_terminator);
        true
    } else {
        false
    }
}

/// Forwarding information for block parameters.
#[derive(Debug, Clone)]
pub struct BlockParamForwarding {
    /// Mapping from block parameters to their forwarded arguments.
    map: HashMap<mir::Value, mir::Value>,
}

impl BlockParamForwarding {
    /// Build forwarding information for block parameters.
    pub fn build(function: &mir::Function, tree: &mir::Tree, cfg: &ControlFlowGraph) -> Self {
        // map parameters to consistent incoming values
        let mut map = HashMap::new();

        // scan blocks for forwarded parameters
        for &block_id in &function.blocks {
            // read block parameters
            let block = tree.get(block_id);
            if block.parameters.is_empty() {
                continue;
            }

            // track candidates and conflicts per parameter
            let mut candidates: Vec<Option<mir::ValueReference>> =
                vec![None; block.parameters.len()];
            let mut conflicts = vec![false; block.parameters.len()];
            let mut saw_pred = false;

            for &pred in cfg.predecessors(block_id) {
                // read arguments for the predecessor edge
                let pred_block = tree.get(pred);
                let pred_terminator = tree.get(pred_block.terminator);
                let successor_args =
                    terminator_arguments_for_successor_checked(pred_terminator, block_id);

                let args = match successor_args {
                    SuccessorArguments::Consistent(args) => args,
                    _ => {
                        conflicts.fill(true);
                        continue;
                    }
                };

                // mark the presence of a predecessor
                saw_pred = true;

                // reject mismatched arity
                if args.len() != block.parameters.len() {
                    conflicts.fill(true);
                    continue;
                }

                // reconcile candidates
                for (index, arg) in args.iter().enumerate() {
                    if conflicts[index] {
                        continue;
                    }

                    match candidates[index] {
                        None => candidates[index] = Some(*arg),
                        Some(existing) if existing == *arg => {}
                        _ => conflicts[index] = true,
                    }
                }
            }

            // skip blocks with no predecessors
            if !saw_pred {
                continue;
            }

            // record non conflicting mappings
            for (index, param) in block.parameters.iter().enumerate() {
                if conflicts[index] {
                    continue;
                }

                let Some(candidate) = candidates[index] else {
                    continue;
                };

                let Some(param_value) = param.value.value() else {
                    continue;
                };
                let Some(candidate) = candidate.value() else {
                    continue;
                };

                if candidate != param_value {
                    map.insert(param_value, candidate);
                }
            }
        }

        Self { map }
    }

    /// Resolve a value through forwarding chains.
    pub fn resolve(&self, value: mir::Value) -> mir::Value {
        // walk forwarding chains
        let mut current = value;
        let mut visited: Vec<mir::Value> = Vec::new();

        // follow forwarding links
        loop {
            let Some(next) = self.map.get(&current) else {
                break;
            };

            if visited.contains(&current) {
                break;
            }

            visited.push(current);
            current = *next;
        }

        current
    }
}

/// Apply substitutions to blocks dominated by the root.
pub fn apply_substitutions_in_dominated_blocks(
    function: &mir::Function,
    tree: &mut mir::Tree,
    domtree: &DominatorTree,
    root: mir::LocalNodeId<mir::Block>,
    substitutions: &HashMap<mir::Value, mir::Value>,
) -> bool {
    // skip when there is nothing to substitute
    if substitutions.is_empty() {
        return false;
    }

    // track whether any changes were made
    let mut changed = false;

    // update blocks dominated by the root
    for &block_id in &function.blocks {
        // skip blocks not dominated by the root
        if !domtree.dominates(root, block_id) {
            continue;
        }

        // snapshot block instructions and terminator
        let block = tree.get(block_id).clone();
        let instruction_ids = block.instructions.clone();
        let terminator_id = block.terminator;
        let terminator = tree.get(terminator_id).clone();

        // rewrite instructions in place
        for instruction_id in instruction_ids {
            // substitute values in the instruction
            let instruction = tree.get(instruction_id).clone();
            let updated = instruction_substitute_uses_in_tree(&instruction, substitutions, tree);

            // replace when a rewrite occurred
            if updated != instruction {
                tree.replace(instruction_id, updated);
                remap_instruction_memory_accesses(tree, instruction_id, substitutions);
                changed = true;
            }
        }

        // rewrite terminator uses
        let new_terminator = terminator_substitute_uses(&terminator, substitutions);

        // replace the terminator when it changes
        if new_terminator != terminator {
            let new_block = block;
            tree.replace(block_id, new_block);
            tree.replace(terminator_id, new_terminator);
            changed = true;
        }
    }

    changed
}

/// Get the arguments passed to a successor, reporting conflicts.
pub fn terminator_arguments_for_successor_checked(
    terminator: &mir::Terminator,
    successor: mir::LocalNodeId<mir::Block>,
) -> SuccessorArguments<'_> {
    // track candidate arguments and conflicts
    let mut candidate: Option<&[mir::ValueReference]> = None;
    let mut is_conflict = false;

    /// Record candidate arguments or flag a conflict.
    fn record_arguments<'a>(
        candidate: &mut Option<&'a [mir::ValueReference]>,
        is_conflict: &mut bool,
        args: &'a [mir::ValueReference],
    ) {
        if let Some(existing) = *candidate {
            if existing != args {
                *is_conflict = true;
            }
        } else {
            *candidate = Some(args);
        }
    }

    // scan terminator edges
    match terminator {
        mir::Terminator::Jump { target } => {
            if target.block.block() == Some(successor) {
                record_arguments(&mut candidate, &mut is_conflict, &target.arguments);
            }
        }
        mir::Terminator::Branch {
            then_target,
            else_target,
            ..
        } => {
            if then_target.block.block() == Some(successor) {
                record_arguments(&mut candidate, &mut is_conflict, &then_target.arguments);
            }
            if else_target.block.block() == Some(successor) {
                record_arguments(&mut candidate, &mut is_conflict, &else_target.arguments);
            }
        }
        mir::Terminator::Check {
            success, failure, ..
        } => {
            if success.block.block() == Some(successor) {
                record_arguments(&mut candidate, &mut is_conflict, &success.arguments);
            }
            if failure.block.block() == Some(successor) {
                record_arguments(&mut candidate, &mut is_conflict, &failure.arguments);
            }
        }
        mir::Terminator::Switch { default, cases, .. } => {
            if default.block.block() == Some(successor) {
                record_arguments(&mut candidate, &mut is_conflict, &default.arguments);
            }
            for case in cases {
                if case.target.block.block() == Some(successor) {
                    record_arguments(&mut candidate, &mut is_conflict, &case.target.arguments);
                }
            }
        }
        mir::Terminator::Yield { resume, .. } => {
            if resume.block.block() == Some(successor) {
                record_arguments(&mut candidate, &mut is_conflict, &resume.arguments);
            }
        }
        _ => {}
    }

    if is_conflict {
        SuccessorArguments::Conflict
    } else if let Some(args) = candidate {
        SuccessorArguments::Consistent(args)
    } else {
        SuccessorArguments::Missing
    }
}

/// Collect all SSA values used by a block.
pub fn collect_block_uses(block: &mir::Block, tree: &mir::Tree) -> Vec<mir::Value> {
    // prepare the use list
    let mut uses = Vec::new();

    // collect uses from instructions
    for &instruction_id in &block.instructions {
        let instruction = tree.get(instruction_id);
        uses.extend(instruction.uses().iter().filter_map(|value| value.value()));
        if let Some(arguments) = instruction.argument_slice() {
            uses.extend(
                tree.get_arguments(arguments)
                    .iter()
                    .filter_map(|value| value.value()),
            );
        }
    }

    // collect uses from the terminator
    let terminator = tree.get(block.terminator);
    uses.extend(terminator.uses().iter().filter_map(|value| value.value()));

    uses
}

/// Return true when all block uses are available in a predecessor.
pub fn block_uses_available_in_predecessor(
    block_id: mir::LocalNodeId<mir::Block>,
    block: &mir::Block,
    tree: &mir::Tree,
    predecessor: mir::LocalNodeId<mir::Block>,
    value_def_blocks: &HashMap<mir::Value, mir::LocalNodeId<mir::Block>>,
    domtree: &DominatorTree,
) -> bool {
    // collect block parameter values
    let mut param_values: HashSet<mir::Value> = HashSet::new();

    for param in &block.parameters {
        let Some(param) = param.value.value() else {
            continue;
        };
        param_values.insert(param);
    }

    // ensure all uses are defined before the predecessor
    let uses = collect_block_uses(block, tree);
    for value in uses {
        // skip values provided by block parameters
        if param_values.contains(&value) {
            continue;
        }

        // require a known definition
        let Some(def_block) = value_def_blocks.get(&value) else {
            return false;
        };

        // skip values defined inside the block
        if *def_block == block_id {
            continue;
        }

        // require dominance at the predecessor
        if !domtree.dominates(*def_block, predecessor) {
            return false;
        }
    }

    true
}

/// Resolve a block parameter value for a predecessor edge when needed.
pub fn resolve_edge_value(
    value: mir::Value,
    block_id: mir::LocalNodeId<mir::Block>,
    predecessor: &mir::Block,
    tree: &mir::Tree,
    param_indices: &HashMap<mir::Value, usize>,
) -> Option<mir::Value> {
    // map block parameters to predecessor arguments
    let Some(&param_index) = param_indices.get(&value) else {
        return Some(value);
    };

    // read arguments for the predecessor edge
    let predecessor_terminator = tree.get(predecessor.terminator);
    let args = match terminator_arguments_for_successor_checked(predecessor_terminator, block_id) {
        SuccessorArguments::Consistent(args) => args,
        _ => return None,
    };

    // return the argument at the parameter index
    args.get(param_index)
        .copied()
        .and_then(|argument| argument.value())
}

/// Return true when a value is available in a block.
pub fn value_available_in_block(
    value: mir::Value,
    block_id: mir::LocalNodeId<mir::Block>,
    def_blocks: &HashMap<mir::Value, mir::LocalNodeId<mir::Block>>,
    function_params: &HashSet<mir::Value>,
    domtree: &DominatorTree,
) -> bool {
    // accept function parameters
    if function_params.contains(&value) {
        return true;
    }

    // require a definition block for the value
    let Some(def_block) = def_blocks.get(&value) else {
        return false;
    };

    // ensure the definition dominates the block
    domtree.dominates(*def_block, block_id)
}

/// Return true when block parameters are used outside the block.
pub fn block_parameters_used_outside_block(
    block: &mir::Block,
    use_blocks: &HashMap<mir::Value, Vec<mir::LocalNodeId<mir::Block>>>,
    block_id: mir::LocalNodeId<mir::Block>,
) -> bool {
    // detect parameter uses outside of the defining block
    for param in &block.parameters {
        let Some(param) = param.value.value() else {
            continue;
        };
        let Some(uses) = use_blocks.get(&param) else {
            continue;
        };

        if uses.iter().any(|use_block| *use_block != block_id) {
            return true;
        }
    }

    false
}

/// Thread jumps through empty or passthrough blocks.
///
/// If a block has no instructions and either has no parameters or just forwards
/// them, predecessors can bypass it. For jump terminators, we resolve chains
/// (A to B to C becomes A to C).
///
/// Returns true if any changes were made.
pub fn function_thread_jumps(function: &mir::Function, tree: &mut mir::Tree) -> bool {
    // find all empty blocks (no instructions) that can be threaded
    let mut threadable: HashMap<mir::LocalNodeId<mir::Block>, ThreadableBlock> = HashMap::new();

    // scan blocks to identify threadable candidates
    for &block_id in &function.blocks {
        // read the block
        let block = tree.get(block_id);
        let terminator = tree.get(block.terminator);

        // block must have no instructions to be threadable
        if !block.instructions.is_empty() {
            continue;
        }

        match terminator {
            mir::Terminator::Jump { target } => {
                if block.parameters.is_empty() && target.arguments.is_empty() {
                    threadable.insert(block_id, ThreadableBlock::Terminator(terminator.clone()));
                } else if block_is_passthrough_jump(block, &target.arguments) {
                    let Some(target) = target.block.block() else {
                        continue;
                    };

                    threadable.insert(block_id, ThreadableBlock::Forward { target });
                }
            }
            mir::Terminator::Return { .. }
            | mir::Terminator::Trap { .. }
            | mir::Terminator::Unreachable => {
                if block.parameters.is_empty() {
                    threadable.insert(block_id, ThreadableBlock::Terminator(terminator.clone()));
                }
            }
            _ => {}
        }
    }

    // skip when there are no threadable blocks
    if threadable.is_empty() {
        return false;
    }

    // rewrite terminators to bypass threadable blocks
    let mut changed = false;
    for &block_id in &function.blocks {
        // read the block
        let block = tree.get(block_id);
        let terminator = tree.get(block.terminator);

        let new_terminator = match terminator {
            mir::Terminator::Jump { target } => {
                let Some(target_block) = target.block.block() else {
                    continue;
                };

                let resolved =
                    block_resolve_jump_target(target_block, &target.arguments, &threadable);
                match resolved {
                    ResolvedTarget::Terminator(terminator) if target.arguments.is_empty() => {
                        Some(terminator)
                    }
                    ResolvedTarget::Jump {
                        target: new_target,
                        arguments: new_arguments,
                    } if new_target != target_block || new_arguments != target.arguments => {
                        Some(mir::Terminator::Jump {
                            target: mir::BlockTarget {
                                block: mir::BlockReference::from(new_target),
                                arguments: new_arguments,
                            },
                        })
                    }
                    _ => None,
                }
            }
            mir::Terminator::Branch {
                condition,
                then_target,
                else_target,
            } => {
                let Some(then_block) = then_target.block.block() else {
                    continue;
                };
                let Some(else_block) = else_target.block.block() else {
                    continue;
                };

                let then_resolved =
                    block_resolve_jump_target(then_block, &then_target.arguments, &threadable);
                let else_resolved =
                    block_resolve_jump_target(else_block, &else_target.arguments, &threadable);

                let (new_then, new_then_args) = match then_resolved {
                    ResolvedTarget::Jump { target, arguments }
                        if arguments_match_block(target, &arguments, tree) =>
                    {
                        (target, arguments)
                    }
                    _ => (then_block, then_target.arguments.clone()),
                };
                let (new_else, new_else_args) = match else_resolved {
                    ResolvedTarget::Jump { target, arguments }
                        if arguments_match_block(target, &arguments, tree) =>
                    {
                        (target, arguments)
                    }
                    _ => (else_block, else_target.arguments.clone()),
                };

                if new_then != then_block
                    || new_else != else_block
                    || new_then_args != then_target.arguments
                    || new_else_args != else_target.arguments
                {
                    Some(mir::Terminator::Branch {
                        condition: *condition,
                        then_target: mir::BlockTarget {
                            block: mir::BlockReference::from(new_then),
                            arguments: new_then_args,
                        },
                        else_target: mir::BlockTarget {
                            block: mir::BlockReference::from(new_else),
                            arguments: new_else_args,
                        },
                    })
                } else {
                    None
                }
            }
            mir::Terminator::Check {
                constraint,
                success,
                failure,
            } => {
                let Some(success_block) = success.block.block() else {
                    continue;
                };
                let Some(failure_block) = failure.block.block() else {
                    continue;
                };

                let success_resolved =
                    block_resolve_jump_target(success_block, &success.arguments, &threadable);
                let failure_resolved =
                    block_resolve_jump_target(failure_block, &failure.arguments, &threadable);

                let (new_success, new_success_args) = match success_resolved {
                    ResolvedTarget::Jump { target, arguments }
                        if arguments_match_block(target, &arguments, tree) =>
                    {
                        (target, arguments)
                    }
                    _ => (success_block, success.arguments.clone()),
                };
                let (new_failure, new_failure_args) = match failure_resolved {
                    ResolvedTarget::Jump { target, arguments }
                        if arguments_match_block(target, &arguments, tree) =>
                    {
                        (target, arguments)
                    }
                    _ => (failure_block, failure.arguments.clone()),
                };

                if new_success != success_block
                    || new_failure != failure_block
                    || new_success_args != success.arguments
                    || new_failure_args != failure.arguments
                {
                    Some(mir::Terminator::Check {
                        constraint: constraint.clone(),
                        success: mir::BlockTarget {
                            block: mir::BlockReference::from(new_success),
                            arguments: new_success_args,
                        },
                        failure: mir::BlockTarget {
                            block: mir::BlockReference::from(new_failure),
                            arguments: new_failure_args,
                        },
                    })
                } else {
                    None
                }
            }
            mir::Terminator::Switch {
                value,
                default,
                cases,
            } => {
                // resolve the default edge
                let Some(default_block) = default.block.block() else {
                    continue;
                };

                let default_resolved =
                    block_resolve_jump_target(default_block, &default.arguments, &threadable);

                // choose the resolved default target
                let (new_default, new_default_args) = match default_resolved {
                    ResolvedTarget::Jump { target, arguments }
                        if arguments_match_block(target, &arguments, tree) =>
                    {
                        (target, arguments)
                    }
                    _ => (default_block, default.arguments.clone()),
                };

                // track whether any edge changes
                let mut remapped =
                    new_default != default_block || new_default_args != default.arguments;
                let mut new_cases = Vec::with_capacity(cases.len());

                // resolve case edges
                for case in cases {
                    let Some(case_block) = case.target.block.block() else {
                        continue;
                    };

                    let resolved =
                        block_resolve_jump_target(case_block, &case.target.arguments, &threadable);

                    // choose the resolved case target
                    let (target, arguments) = match resolved {
                        ResolvedTarget::Jump { target, arguments }
                            if arguments_match_block(target, &arguments, tree) =>
                        {
                            (target, arguments)
                        }
                        _ => (case_block, case.target.arguments.clone()),
                    };

                    // track remapped edges
                    if target != case_block || arguments != case.target.arguments {
                        remapped = true;
                    }

                    new_cases.push(mir::SwitchCase {
                        value: case.value,
                        target: mir::BlockTarget {
                            block: mir::BlockReference::from(target),
                            arguments,
                        },
                    });
                }

                // rebuild the switch when edges changed
                if remapped {
                    Some(mir::Terminator::Switch {
                        value: *value,
                        default: mir::BlockTarget {
                            block: mir::BlockReference::from(new_default),
                            arguments: new_default_args,
                        },
                        cases: new_cases,
                    })
                } else {
                    None
                }
            }
            _ => None,
        };

        // update the terminator when it changes
        if let Some(terminator) = new_terminator {
            tree.replace(block.terminator, terminator);
            changed = true;
        }
    }

    changed
}

/// Threadable block metadata.
enum ThreadableBlock {
    /// A terminator that can be absorbed by predecessors.
    Terminator(mir::Terminator),
    /// A passthrough jump that forwards its parameters unchanged.
    Forward {
        /// The target block for forwarding.
        target: mir::LocalNodeId<mir::Block>,
    },
}

/// Result of resolving a jump target through threadable blocks.
enum ResolvedTarget {
    /// Resolved to a final jump target with updated arguments.
    Jump {
        /// The final target block.
        target: mir::LocalNodeId<mir::Block>,
        /// The arguments to pass to the target.
        arguments: Vec<mir::ValueReference>,
    },
    /// Resolved to a terminator that can be absorbed (return or unreachable).
    Terminator(mir::Terminator),
}

/// Resolve a jump target by following through threadable blocks.
fn block_resolve_jump_target(
    target: mir::LocalNodeId<mir::Block>,
    arguments: &[mir::ValueReference],
    threadable: &HashMap<mir::LocalNodeId<mir::Block>, ThreadableBlock>,
) -> ResolvedTarget {
    // seed the traversal state
    let mut current = target;
    let mut current_args = arguments.to_vec();
    let mut visited = HashSet::new();

    // follow threadable blocks until a terminal target is found
    loop {
        // stop on cycles to avoid infinite loops
        if !visited.insert(current) {
            return ResolvedTarget::Jump {
                target: current,
                arguments: current_args,
            };
        }

        // stop when the block is not threadable
        let Some(threadable_block) = threadable.get(&current) else {
            return ResolvedTarget::Jump {
                target: current,
                arguments: current_args,
            };
        };

        match threadable_block {
            ThreadableBlock::Forward {
                target: next_target,
            } => {
                current = *next_target;
            }
            ThreadableBlock::Terminator(terminator) => match terminator {
                mir::Terminator::Jump { target } if target.arguments.is_empty() => {
                    let Some(target) = target.block.block() else {
                        return ResolvedTarget::Terminator(terminator.clone());
                    };

                    current = target;
                    current_args = Vec::new();
                }
                _ => {
                    return ResolvedTarget::Terminator(terminator.clone());
                }
            },
        }
    }
}

/// Return true when the argument list matches the target block parameters.
fn arguments_match_block(
    target: mir::LocalNodeId<mir::Block>,
    arguments: &[mir::ValueReference],
    tree: &mir::Tree,
) -> bool {
    // require argument counts to match parameters
    let target_block = tree.get(target);
    target_block.parameters.len() == arguments.len()
}

/// Return true if a jump forwards all block parameters unchanged.
fn block_is_passthrough_jump(block: &mir::Block, arguments: &[mir::ValueReference]) -> bool {
    // require exact parameter and argument alignment
    if block.parameters.len() != arguments.len() {
        return false;
    }

    // verify each parameter is forwarded verbatim
    let mut is_forwarding = true;
    for (param, arg) in block.parameters.iter().zip(arguments.iter()) {
        // stop when a parameter does not match
        if param.value.value() != arg.value() {
            is_forwarding = false;
            break;
        }
    }

    is_forwarding
}

/// Substitute values in a terminator according to the given map.
///
/// Creates a new terminator with value references replaced according to the substitution map.
/// Values not in the map are left unchanged.
pub fn terminator_substitute_uses(
    terminator: &mir::Terminator,
    substitutions: &HashMap<mir::Value, mir::Value>,
) -> mir::Terminator {
    if substitutions.is_empty() {
        return terminator.clone();
    }

    let substitute = |v: mir::ValueReference| -> mir::ValueReference {
        let Some(value) = v.value() else {
            return v;
        };
        let value = *substitutions.get(&value).unwrap_or(&value);

        mir::ValueReference::from(value)
    };

    let target_with_values = |target: &mir::BlockTarget| mir::BlockTarget {
        block: target.block,
        arguments: target.arguments.iter().copied().map(&substitute).collect(),
    };

    match terminator {
        mir::Terminator::Error => {
            panic!("recovered MIR terminator reached optimizer");
        }
        mir::Terminator::Return { value } => mir::Terminator::Return {
            value: value.map(substitute),
        },
        mir::Terminator::Jump { target } => mir::Terminator::Jump {
            target: mir::BlockTarget {
                block: target.block,
                arguments: target.arguments.iter().copied().map(substitute).collect(),
            },
        },
        mir::Terminator::Branch {
            condition,
            then_target,
            else_target,
        } => mir::Terminator::Branch {
            condition: substitute(*condition),
            then_target: mir::BlockTarget {
                block: then_target.block,
                arguments: then_target
                    .arguments
                    .iter()
                    .copied()
                    .map(substitute)
                    .collect(),
            },
            else_target: mir::BlockTarget {
                block: else_target.block,
                arguments: else_target
                    .arguments
                    .iter()
                    .copied()
                    .map(substitute)
                    .collect(),
            },
        },
        mir::Terminator::Check {
            constraint,
            success,
            failure,
        } => {
            let constraint = match constraint {
                mir::CheckConstraint::Bounds {
                    index,
                    length,
                    collection,
                    is_signed,
                } => mir::CheckConstraint::Bounds {
                    index: substitute(*index),
                    length: substitute(*length),
                    collection: substitute(*collection),
                    is_signed: *is_signed,
                },
                mir::CheckConstraint::Null { value } => mir::CheckConstraint::Null {
                    value: substitute(*value),
                },
                mir::CheckConstraint::DivZero { divisor } => mir::CheckConstraint::DivZero {
                    divisor: substitute(*divisor),
                },
                mir::CheckConstraint::ShiftRange {
                    value,
                    bit_width,
                    is_signed,
                } => mir::CheckConstraint::ShiftRange {
                    value: substitute(*value),
                    bit_width: *bit_width,
                    is_signed: *is_signed,
                },
                mir::CheckConstraint::Narrow {
                    value,
                    to_width,
                    is_signed,
                } => mir::CheckConstraint::Narrow {
                    value: substitute(*value),
                    to_width: *to_width,
                    is_signed: *is_signed,
                },
                mir::CheckConstraint::Overflow {
                    operator,
                    left,
                    right,
                    is_signed,
                } => mir::CheckConstraint::Overflow {
                    operator: *operator,
                    left: substitute(*left),
                    right: substitute(*right),
                    is_signed: *is_signed,
                },
                mir::CheckConstraint::Type { value, expected } => mir::CheckConstraint::Type {
                    value: substitute(*value),
                    expected: *expected,
                },
                mir::CheckConstraint::Variant { value, expected } => {
                    mir::CheckConstraint::Variant {
                        value: substitute(*value),
                        expected: expected.clone(),
                    }
                }
                mir::CheckConstraint::ReceiverType { receiver, expected } => {
                    mir::CheckConstraint::ReceiverType {
                        receiver: substitute(*receiver),
                        expected: *expected,
                    }
                }
                mir::CheckConstraint::Implements { receiver, expected } => {
                    mir::CheckConstraint::Implements {
                        receiver: substitute(*receiver),
                        expected: *expected,
                    }
                }
            };

            let success = mir::BlockTarget {
                block: success.block,
                arguments: success.arguments.iter().copied().map(substitute).collect(),
            };

            let failure = mir::BlockTarget {
                block: failure.block,
                arguments: failure.arguments.iter().copied().map(substitute).collect(),
            };

            mir::Terminator::Check {
                constraint: constraint.clone(),
                success,
                failure,
            }
        }
        mir::Terminator::NewZeroedTry {
            layout,
            success,
            failure,
        } => mir::Terminator::NewZeroedTry {
            layout: *layout,
            success: mir::BlockTarget {
                block: success.block,
                arguments: success.arguments.iter().copied().map(substitute).collect(),
            },
            failure: target_with_values(failure),
        },
        mir::Terminator::NewUninitTry {
            layout,
            success,
            failure,
        } => mir::Terminator::NewUninitTry {
            layout: *layout,
            success: mir::BlockTarget {
                block: success.block,
                arguments: success.arguments.iter().copied().map(substitute).collect(),
            },
            failure: target_with_values(failure),
        },
        mir::Terminator::NewSliceZeroedTry {
            element,
            length,
            success,
            failure,
        } => mir::Terminator::NewSliceZeroedTry {
            element: *element,
            length: substitute(*length),
            success: mir::BlockTarget {
                block: success.block,
                arguments: success.arguments.iter().copied().map(substitute).collect(),
            },
            failure: target_with_values(failure),
        },
        mir::Terminator::NewSliceUninitTry {
            element,
            length,
            success,
            failure,
        } => mir::Terminator::NewSliceUninitTry {
            element: *element,
            length: substitute(*length),
            success: mir::BlockTarget {
                block: success.block,
                arguments: success.arguments.iter().copied().map(substitute).collect(),
            },
            failure: target_with_values(failure),
        },
        mir::Terminator::Switch {
            value,
            default,
            cases,
        } => mir::Terminator::Switch {
            value: substitute(*value),
            default: mir::BlockTarget {
                block: default.block,
                arguments: default.arguments.iter().copied().map(substitute).collect(),
            },
            cases: cases
                .iter()
                .map(|case| mir::SwitchCase {
                    value: case.value,
                    target: mir::BlockTarget {
                        block: case.target.block,
                        arguments: case
                            .target
                            .arguments
                            .iter()
                            .copied()
                            .map(substitute)
                            .collect(),
                    },
                })
                .collect(),
        },
        mir::Terminator::Yield { value, resume } => mir::Terminator::Yield {
            value: substitute(*value),
            resume: mir::BlockTarget {
                block: resume.block,
                arguments: resume.arguments.iter().copied().map(substitute).collect(),
            },
        },
        mir::Terminator::Call {
            function,
            call,
            target,
            unwind,
        } => mir::Terminator::Call {
            function: *function,
            call: mir::Call {
                arguments: call.arguments.iter().copied().map(substitute).collect(),
                ..call.clone()
            },
            target: mir::BlockTarget {
                block: target.block,
                arguments: target.arguments.iter().copied().map(substitute).collect(),
            },
            unwind: unwind.as_ref().map(&target_with_values),
        },
        mir::Terminator::CallIndirect {
            callee,
            call,
            target,
            unwind,
        } => mir::Terminator::CallIndirect {
            callee: substitute(*callee),
            call: mir::Call {
                arguments: call.arguments.iter().copied().map(substitute).collect(),
                ..call.clone()
            },
            target: mir::BlockTarget {
                block: target.block,
                arguments: target.arguments.iter().copied().map(substitute).collect(),
            },
            unwind: unwind.as_ref().map(&target_with_values),
        },
        mir::Terminator::CallVirtual {
            receiver,
            call,
            class,
            slot,
            target,
            unwind,
        } => mir::Terminator::CallVirtual {
            receiver: substitute(*receiver),
            call: mir::Call {
                arguments: call.arguments.iter().copied().map(substitute).collect(),
                ..call.clone()
            },
            class: *class,
            slot: *slot,
            target: mir::BlockTarget {
                block: target.block,
                arguments: target.arguments.iter().copied().map(substitute).collect(),
            },
            unwind: unwind.as_ref().map(&target_with_values),
        },
        mir::Terminator::CallDynamic {
            receiver,
            call,
            constraint,
            slot,
            target,
            unwind,
        } => mir::Terminator::CallDynamic {
            receiver: substitute(*receiver),
            call: mir::Call {
                arguments: call.arguments.iter().copied().map(substitute).collect(),
                ..call.clone()
            },
            constraint: *constraint,
            slot: *slot,
            target: mir::BlockTarget {
                block: target.block,
                arguments: target.arguments.iter().copied().map(substitute).collect(),
            },
            unwind: unwind.as_ref().map(&target_with_values),
        },
        mir::Terminator::Trap { kind, payload } => mir::Terminator::Trap {
            kind: *kind,
            payload: payload.map(substitute),
        },
        mir::Terminator::Panic { payload } => mir::Terminator::Panic {
            payload: payload.map(substitute),
        },
        mir::Terminator::ResumePanic => mir::Terminator::ResumePanic,
        mir::Terminator::Unreachable => mir::Terminator::Unreachable,
        mir::Terminator::TailCall { function, call } => mir::Terminator::TailCall {
            function: *function,
            call: mir::Call {
                arguments: call.arguments.iter().copied().map(substitute).collect(),
                ..call.clone()
            },
        },
        mir::Terminator::TailCallVirtual {
            receiver,
            call,
            class,
            slot,
        } => mir::Terminator::TailCallVirtual {
            receiver: substitute(*receiver),
            call: mir::Call {
                arguments: call.arguments.iter().copied().map(substitute).collect(),
                ..call.clone()
            },
            class: *class,
            slot: *slot,
        },
        mir::Terminator::TailCallDynamic {
            receiver,
            call,
            constraint,
            slot,
        } => mir::Terminator::TailCallDynamic {
            receiver: substitute(*receiver),
            call: mir::Call {
                arguments: call.arguments.iter().copied().map(substitute).collect(),
                ..call.clone()
            },
            constraint: *constraint,
            slot: *slot,
        },
        mir::Terminator::TailCallIndirect { callee, call } => mir::Terminator::TailCallIndirect {
            callee: substitute(*callee),
            call: mir::Call {
                arguments: call.arguments.iter().copied().map(substitute).collect(),
                ..call.clone()
            },
        },
    }
}
