use std::collections::{HashMap, HashSet, VecDeque};

use crate as mir;

use crate::{
    ControlFlowGraph, DominatorTree, instruction_substitute_uses_in_tree,
    remap_instruction_memory_accesses,
};

/// Check if a terminator uses a specific value.
///
/// Returns true if the value appears in any operand position of the terminator.
/// This includes branch conditions, return values, and block arguments.
pub fn terminator_uses(tree: &mir::Tree, terminator: &mir::Terminator, value: mir::Value) -> bool {
    tree.terminator_uses(terminator).contains(&value)
}

/// Get all values used by a terminator.
///
/// Returns a vector of all value operands in the terminator, including
/// conditions, return values, and block arguments.
pub fn terminator_used_values(tree: &mir::Tree, terminator: &mir::Terminator) -> Vec<mir::Value> {
    tree.terminator_uses(terminator).iter().copied().collect()
}

/// Get the arguments passed to a specific successor block from a terminator.
pub fn terminator_arguments_for_successor<'a>(
    tree: &'a mir::Tree,
    terminator: &mir::Terminator,
    successor: mir::LocalNodeId<mir::Block>,
) -> &'a [mir::Value] {
    match terminator {
        mir::Terminator::Jump { target } if Some(target.block) == Some(successor) => {
            tree.block_target_values(target)
        }

        mir::Terminator::Branch {
            then_target,
            else_target,
            ..
        } => {
            // then branch
            if Some(then_target.block) == Some(successor) {
                tree.block_target_values(then_target)
            }
            // else branch
            else if Some(else_target.block) == Some(successor) {
                tree.block_target_values(else_target)
            }
            // not a successor
            else {
                &[]
            }
        }
        mir::Terminator::Check {
            success, failure, ..
        } => {
            if Some(success.block) == Some(successor) {
                tree.block_target_values(success)
            } else if Some(failure.block) == Some(successor) {
                tree.block_target_values(failure)
            } else {
                &[]
            }
        }

        mir::Terminator::Switch { default, cases, .. } => {
            // default case
            if Some(default.block) == Some(successor) {
                return tree.block_target_values(default);
            }

            // numbered cases
            for case in tree.get_switch_cases(*cases) {
                if Some(case.target.block) == Some(successor) {
                    return tree.block_target_values(&case.target);
                }
            }

            &[]
        }

        mir::Terminator::Yield { resume, unwind, .. } => {
            if Some(resume.block) == Some(successor) {
                tree.block_target_values(resume)
            } else if let Some(unwind) = unwind
                && Some(unwind.block) == Some(successor)
            {
                tree.block_target_values(unwind)
            } else {
                &[]
            }
        }
        mir::Terminator::Call { target, unwind, .. }
        | mir::Terminator::CallIndirect { target, unwind, .. }
        | mir::Terminator::CallVirtual { target, unwind, .. }
        | mir::Terminator::CallDynamic { target, unwind, .. } => {
            if Some(target.block) == Some(successor) {
                tree.block_target_values(target)
            } else if let Some(unwind) = unwind
                && Some(unwind.block) == Some(successor)
            {
                tree.block_target_values(unwind)
            } else {
                &[]
            }
        }

        _ => &[],
    }
}

/// Return one block target with appended arguments.
fn block_target_with_arguments(
    tree: &mut mir::Tree,
    target: &mir::BlockTarget,
    extra_args: &[mir::Value],
) -> mir::BlockTarget {
    let mut arguments = tree.block_target_values(target).to_vec();
    arguments.extend_from_slice(extra_args);

    let mut target = target.clone();
    target.arguments = tree.add_values(&arguments);

    target
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
        for successor in tree.terminator_successors(terminator) {
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
    Consistent(&'a [mir::Value]),
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
        mir::Terminator::Jump { target } if Some(target.block) == Some(successor) => {
            let new_target = block_target_with_arguments(tree, target, extra_args);

            mir::Terminator::Jump { target: new_target }
        }
        mir::Terminator::Branch {
            condition,
            then_target,
            else_target,
        } => {
            let new_then = if Some(then_target.block) == Some(successor) {
                block_target_with_arguments(tree, then_target, extra_args)
            } else {
                then_target.clone()
            };

            let new_else = if Some(else_target.block) == Some(successor) {
                block_target_with_arguments(tree, else_target, extra_args)
            } else {
                else_target.clone()
            };

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
            let new_success = if Some(success.block) == Some(successor) {
                block_target_with_arguments(tree, success, extra_args)
            } else {
                success.clone()
            };

            let new_failure = if Some(failure.block) == Some(successor) {
                block_target_with_arguments(tree, failure, extra_args)
            } else {
                failure.clone()
            };

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
            let new_default = if Some(default.block) == Some(successor) {
                block_target_with_arguments(tree, default, extra_args)
            } else {
                default.clone()
            };

            let cases = tree.get_switch_cases(*cases).to_vec();
            let mut new_cases = Vec::with_capacity(cases.len());
            for case in &cases {
                if Some(case.target.block) == Some(successor) {
                    let new_target = block_target_with_arguments(tree, &case.target, extra_args);

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
                cases: tree.add_switch_cases(&new_cases),
            }
        }
        mir::Terminator::Yield {
            value,
            resume,
            unwind,
        } => {
            let new_resume = if Some(resume.block) == Some(successor) {
                block_target_with_arguments(tree, resume, extra_args)
            } else {
                resume.clone()
            };

            let mut new_unwind = unwind.clone();
            if let Some(unwind) = &mut new_unwind
                && Some(unwind.block) == Some(successor)
            {
                *unwind = block_target_with_arguments(tree, unwind, extra_args);
            }

            mir::Terminator::Yield {
                value: *value,
                resume: new_resume,
                unwind: new_unwind,
            }
        }
        _ => terminator.clone(),
    };

    // write back only when arguments changed
    if new_terminator != terminator {
        tree.set(terminator_id, new_terminator);
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
    let pred_multi = successor_count(tree, pred_terminator) > 1;
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
        match terminator_arguments_for_successor_checked(tree, pred_terminator, successor) {
            SuccessorArguments::Consistent(args) => args.iter().copied().collect(),
            _ => return predecessor,
        };

    // build the new edge block
    let edge_arguments = tree.add_values(&args);
    let edge_terminator = tree.insert(mir::Terminator::Jump {
        target: mir::BlockTarget::new(successor.into(), edge_arguments),
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
fn successor_count(tree: &mir::Tree, terminator: &mir::Terminator) -> usize {
    // track unique successors
    let mut unique = HashSet::new();
    for successor in tree.terminator_successors(terminator) {
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
        mir::Terminator::Jump { target } if Some(target.block) == Some(successor) => {
            mir::Terminator::Jump {
                target: mir::BlockTarget::new(
                    mir::BlockId::from(edge_block),
                    mir::ValueSlice::default(),
                ),
            }
        }
        mir::Terminator::Branch {
            condition,
            then_target,
            else_target,
        } => {
            let mut new_then = then_target.clone();
            let mut new_else = else_target.clone();

            if Some(then_target.block) == Some(successor) {
                new_then.block = mir::BlockId::from(edge_block);
                new_then.arguments = mir::ValueSlice::default();
            }

            if Some(else_target.block) == Some(successor) {
                new_else.block = mir::BlockId::from(edge_block);
                new_else.arguments = mir::ValueSlice::default();
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

            if Some(success.block) == Some(successor) {
                new_success.block = mir::BlockId::from(edge_block);
                new_success.arguments = mir::ValueSlice::default();
            }

            if Some(failure.block) == Some(successor) {
                new_failure.block = mir::BlockId::from(edge_block);
                new_failure.arguments = mir::ValueSlice::default();
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
            let cases = tree.get_switch_cases(*cases).to_vec();
            let mut new_cases = Vec::with_capacity(cases.len());
            let mut changed = false;

            if Some(default.block) == Some(successor) {
                new_default.block = mir::BlockId::from(edge_block);
                new_default.arguments = mir::ValueSlice::default();
                changed = true;
            }

            for case in &cases {
                if Some(case.target.block) == Some(successor) {
                    changed = true;
                    let mut new_target = case.target.clone();
                    new_target.block = mir::BlockId::from(edge_block);
                    new_target.arguments = mir::ValueSlice::default();

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
                cases: tree.add_switch_cases(&new_cases),
            }
        }
        mir::Terminator::Yield {
            value,
            resume,
            unwind,
        } => {
            let mut new_resume = resume.clone();
            let mut new_unwind = unwind.clone();
            let mut changed = false;

            // rewrite the resume target when it matches the successor
            if Some(resume.block) == Some(successor) {
                new_resume.block = mir::BlockId::from(edge_block);
                new_resume.arguments = mir::ValueSlice::default();
                changed = true;
            }

            // rewrite the unwind target when it matches the successor
            if let Some(unwind) = &mut new_unwind
                && Some(unwind.block) == Some(successor)
            {
                unwind.block = mir::BlockId::from(edge_block);
                unwind.arguments = mir::ValueSlice::default();
                changed = true;
            }

            if !changed {
                return false;
            }

            mir::Terminator::Yield {
                value: *value,
                resume: new_resume,
                unwind: new_unwind,
            }
        }
        _ => return false,
    };

    // update the terminator
    if new_terminator != terminator {
        tree.set(terminator_id, new_terminator);
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
            let mut candidates: Vec<Option<mir::Value>> = vec![None; block.parameters.len()];
            let mut conflicts = vec![false; block.parameters.len()];
            let mut saw_pred = false;

            for &pred in cfg.predecessors(block_id) {
                // read arguments for the predecessor edge
                let pred_block = tree.get(pred);
                let pred_terminator = tree.get(pred_block.terminator);
                let successor_args =
                    terminator_arguments_for_successor_checked(tree, pred_terminator, block_id);

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

                let param_value = param.value;

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
        while let Some(next) = self.map.get(&current) {
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
                tree.set(instruction_id, updated);
                remap_instruction_memory_accesses(tree, instruction_id, substitutions);
                changed = true;
            }
        }

        // rewrite terminator uses
        let new_terminator = terminator_substitute_uses(tree, &terminator, substitutions);

        // replace the terminator when it changes
        if new_terminator != terminator {
            let new_block = block;
            tree.set(block_id, new_block);
            tree.set(terminator_id, new_terminator);
            changed = true;
        }
    }

    changed
}

/// Get the arguments passed to a successor, reporting conflicts.
pub fn terminator_arguments_for_successor_checked<'a>(
    tree: &'a mir::Tree,
    terminator: &mir::Terminator,
    successor: mir::LocalNodeId<mir::Block>,
) -> SuccessorArguments<'a> {
    // track candidate arguments and conflicts
    let mut candidate: Option<&[mir::Value]> = None;
    let mut is_conflict = false;

    /// Record candidate arguments or flag a conflict.
    fn record_arguments<'a>(
        candidate: &mut Option<&'a [mir::Value]>,
        is_conflict: &mut bool,
        args: &'a [mir::Value],
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
            if Some(target.block) == Some(successor) {
                record_arguments(
                    &mut candidate,
                    &mut is_conflict,
                    tree.block_target_values(target),
                );
            }
        }
        mir::Terminator::Branch {
            then_target,
            else_target,
            ..
        } => {
            if Some(then_target.block) == Some(successor) {
                record_arguments(
                    &mut candidate,
                    &mut is_conflict,
                    tree.block_target_values(then_target),
                );
            }
            if Some(else_target.block) == Some(successor) {
                record_arguments(
                    &mut candidate,
                    &mut is_conflict,
                    tree.block_target_values(else_target),
                );
            }
        }
        mir::Terminator::Check {
            success, failure, ..
        } => {
            if Some(success.block) == Some(successor) {
                record_arguments(
                    &mut candidate,
                    &mut is_conflict,
                    tree.block_target_values(success),
                );
            }
            if Some(failure.block) == Some(successor) {
                record_arguments(
                    &mut candidate,
                    &mut is_conflict,
                    tree.block_target_values(failure),
                );
            }
        }
        mir::Terminator::Switch { default, cases, .. } => {
            if Some(default.block) == Some(successor) {
                record_arguments(
                    &mut candidate,
                    &mut is_conflict,
                    tree.block_target_values(default),
                );
            }
            for case in tree.get_switch_cases(*cases) {
                if Some(case.target.block) == Some(successor) {
                    record_arguments(
                        &mut candidate,
                        &mut is_conflict,
                        tree.block_target_values(&case.target),
                    );
                }
            }
        }
        mir::Terminator::Yield { resume, unwind, .. } => {
            if Some(resume.block) == Some(successor) {
                record_arguments(
                    &mut candidate,
                    &mut is_conflict,
                    tree.block_target_values(resume),
                );
            }
            if let Some(unwind) = unwind
                && Some(unwind.block) == Some(successor)
            {
                record_arguments(
                    &mut candidate,
                    &mut is_conflict,
                    tree.block_target_values(unwind),
                );
            }
        }
        mir::Terminator::Call { target, unwind, .. }
        | mir::Terminator::CallIndirect { target, unwind, .. }
        | mir::Terminator::CallVirtual { target, unwind, .. }
        | mir::Terminator::CallDynamic { target, unwind, .. } => {
            if Some(target.block) == Some(successor) {
                record_arguments(
                    &mut candidate,
                    &mut is_conflict,
                    tree.block_target_values(target),
                );
            }
            if let Some(unwind) = unwind
                && Some(unwind.block) == Some(successor)
            {
                record_arguments(
                    &mut candidate,
                    &mut is_conflict,
                    tree.block_target_values(unwind),
                );
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
        uses.extend(instruction.uses().iter().copied());
        if let Some(arguments) = instruction.argument_slice() {
            uses.extend(tree.get_values(arguments).iter().copied());
        }
    }

    // collect uses from the terminator
    let terminator = tree.get(block.terminator);
    uses.extend(tree.terminator_uses(terminator).iter().copied());

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
        param_values.insert(param.value);
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
    let args =
        match terminator_arguments_for_successor_checked(tree, predecessor_terminator, block_id) {
            SuccessorArguments::Consistent(args) => args,
            _ => return None,
        };

    // return the argument at the parameter index
    args.get(param_index).copied()
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
        let param = param.value;
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
                let target_arguments = tree.block_target_values(target);
                if block.parameters.is_empty() && target.arguments.is_empty() {
                    threadable.insert(block_id, ThreadableBlock::Terminator(terminator.clone()));
                } else if block_is_passthrough_jump(block, target_arguments) {
                    let target = target.block;

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
        // snapshot the terminator before interning new payload slices
        let terminator_id = tree.get(block_id).terminator;
        let terminator = tree.get(terminator_id).clone();

        let new_terminator = match &terminator {
            mir::Terminator::Jump { target } => {
                let target_block = target.block;
                let target_arguments = tree.block_target_values(target).to_vec();

                let resolved =
                    block_resolve_jump_target(target_block, &target_arguments, &threadable);
                match resolved {
                    ResolvedTarget::Terminator(terminator) if target.arguments.is_empty() => {
                        Some(terminator)
                    }
                    ResolvedTarget::Jump {
                        target: new_target,
                        arguments: new_arguments,
                    } if new_target != target_block || new_arguments != target_arguments => {
                        let new_arguments = tree.add_values(&new_arguments);

                        Some(mir::Terminator::Jump {
                            target: mir::BlockTarget::new(
                                mir::BlockId::from(new_target),
                                new_arguments,
                            ),
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
                let then_block = then_target.block;
                let else_block = else_target.block;
                let then_arguments = tree.block_target_values(then_target).to_vec();
                let else_arguments = tree.block_target_values(else_target).to_vec();

                let then_resolved =
                    block_resolve_jump_target(then_block, &then_arguments, &threadable);
                let else_resolved =
                    block_resolve_jump_target(else_block, &else_arguments, &threadable);

                let (new_then, new_then_args) = match then_resolved {
                    ResolvedTarget::Jump { target, arguments }
                        if arguments_match_block(target, &arguments, tree) =>
                    {
                        (target, arguments)
                    }
                    _ => (then_block, then_arguments.clone()),
                };
                let (new_else, new_else_args) = match else_resolved {
                    ResolvedTarget::Jump { target, arguments }
                        if arguments_match_block(target, &arguments, tree) =>
                    {
                        (target, arguments)
                    }
                    _ => (else_block, else_arguments.clone()),
                };

                if new_then != then_block
                    || new_else != else_block
                    || new_then_args != then_arguments
                    || new_else_args != else_arguments
                {
                    let new_then_args = tree.add_values(&new_then_args);
                    let new_else_args = tree.add_values(&new_else_args);

                    Some(mir::Terminator::Branch {
                        condition: *condition,
                        then_target: mir::BlockTarget::new(
                            mir::BlockId::from(new_then),
                            new_then_args,
                        ),
                        else_target: mir::BlockTarget::new(
                            mir::BlockId::from(new_else),
                            new_else_args,
                        ),
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
                let success_block = success.block;
                let failure_block = failure.block;
                let success_arguments = tree.block_target_values(success).to_vec();
                let failure_arguments = tree.block_target_values(failure).to_vec();

                let success_resolved =
                    block_resolve_jump_target(success_block, &success_arguments, &threadable);
                let failure_resolved =
                    block_resolve_jump_target(failure_block, &failure_arguments, &threadable);

                let (new_success, new_success_args) = match success_resolved {
                    ResolvedTarget::Jump { target, arguments }
                        if arguments_match_block(target, &arguments, tree) =>
                    {
                        (target, arguments)
                    }
                    _ => (success_block, success_arguments.clone()),
                };
                let (new_failure, new_failure_args) = match failure_resolved {
                    ResolvedTarget::Jump { target, arguments }
                        if arguments_match_block(target, &arguments, tree) =>
                    {
                        (target, arguments)
                    }
                    _ => (failure_block, failure_arguments.clone()),
                };

                if new_success != success_block
                    || new_failure != failure_block
                    || new_success_args != success_arguments
                    || new_failure_args != failure_arguments
                {
                    let new_success_args = tree.add_values(&new_success_args);
                    let new_failure_args = tree.add_values(&new_failure_args);

                    Some(mir::Terminator::Check {
                        constraint: constraint.clone(),
                        success: mir::BlockTarget::new(
                            mir::BlockId::from(new_success),
                            new_success_args,
                        ),
                        failure: mir::BlockTarget::new(
                            mir::BlockId::from(new_failure),
                            new_failure_args,
                        ),
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
                let default_block = default.block;
                let default_arguments = tree.block_target_values(default).to_vec();

                let default_resolved =
                    block_resolve_jump_target(default_block, &default_arguments, &threadable);

                // choose the resolved default target
                let (new_default, new_default_args) = match default_resolved {
                    ResolvedTarget::Jump { target, arguments }
                        if arguments_match_block(target, &arguments, tree) =>
                    {
                        (target, arguments)
                    }
                    _ => (default_block, default_arguments.clone()),
                };

                // track whether any edge changes
                let mut remapped =
                    new_default != default_block || new_default_args != default_arguments;
                let cases = tree.get_switch_cases(*cases).to_vec();
                let mut new_cases = Vec::with_capacity(cases.len());

                // resolve case edges
                for case in &cases {
                    let case_block = case.target.block;
                    let case_arguments = tree.block_target_values(&case.target).to_vec();

                    let resolved =
                        block_resolve_jump_target(case_block, &case_arguments, &threadable);

                    // choose the resolved case target
                    let (target, arguments) = match resolved {
                        ResolvedTarget::Jump { target, arguments }
                            if arguments_match_block(target, &arguments, tree) =>
                        {
                            (target, arguments)
                        }
                        _ => (case_block, case_arguments.clone()),
                    };

                    // track remapped edges
                    if target != case_block || arguments != case_arguments {
                        remapped = true;
                    }
                    let arguments = tree.add_values(&arguments);

                    new_cases.push(mir::SwitchCase {
                        value: case.value,
                        target: mir::BlockTarget::new(mir::BlockId::from(target), arguments),
                    });
                }

                // rebuild the switch when edges changed
                if remapped {
                    let new_default_args = tree.add_values(&new_default_args);
                    let new_cases = tree.add_switch_cases(&new_cases);

                    Some(mir::Terminator::Switch {
                        value: *value,
                        default: mir::BlockTarget::new(
                            mir::BlockId::from(new_default),
                            new_default_args,
                        ),
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
            tree.set(terminator_id, terminator);
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
        arguments: Vec<mir::Value>,
    },
    /// Resolved to a terminator that can be absorbed (return or unreachable).
    Terminator(mir::Terminator),
}

/// Resolve a jump target by following through threadable blocks.
fn block_resolve_jump_target(
    target: mir::LocalNodeId<mir::Block>,
    arguments: &[mir::Value],
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
                    let target = target.block;

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
    arguments: &[mir::Value],
    tree: &mir::Tree,
) -> bool {
    // require argument counts to match parameters
    let target_block = tree.get(target);
    target_block.parameters.len() == arguments.len()
}

/// Return true if a jump forwards all block parameters unchanged.
fn block_is_passthrough_jump(block: &mir::Block, arguments: &[mir::Value]) -> bool {
    // require exact parameter and argument alignment
    if block.parameters.len() != arguments.len() {
        return false;
    }

    // verify each parameter is forwarded verbatim
    let mut is_forwarding = true;
    for (param, arg) in block.parameters.iter().zip(arguments.iter()) {
        // stop when a parameter does not match
        if param.value != *arg {
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
    tree: &mut mir::Tree,
    terminator: &mir::Terminator,
    substitutions: &HashMap<mir::Value, mir::Value>,
) -> mir::Terminator {
    if substitutions.is_empty() {
        return terminator.clone();
    }

    let mut terminator = terminator.clone();
    let block_map = HashMap::new();
    crate::terminator_remap(tree, &mut terminator, &block_map, substitutions);

    terminator
}
