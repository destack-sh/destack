use std::collections::{HashMap, HashSet, VecDeque};

use destack_mir as mir;

use crate::optimize::analyses::{ControlFlowGraph, DominatorTree};
use crate::optimize::common::instruction_substitute_uses_in_tree;

/// Check if a terminator uses a specific value.
///
/// Returns true if the value appears in any operand position of the terminator.
/// This includes branch conditions, return values, and block arguments.
pub fn terminator_uses(term: &mir::Terminator, value: mir::Value) -> bool {
    match term {
        mir::Terminator::Return { value: Some(v) } => *v == value,
        mir::Terminator::Return { value: None } | mir::Terminator::Unreachable => false,
        mir::Terminator::Jump { arguments, .. } => arguments.contains(&value),
        mir::Terminator::Branch {
            condition,
            then_arguments,
            else_arguments,
            ..
        } => {
            *condition == value
                || then_arguments.contains(&value)
                || else_arguments.contains(&value)
        }
        mir::Terminator::Check {
            condition,
            constraint,
            success,
            failure,
        } => {
            *condition == value
                || constraint.uses().contains(&value)
                || success.arguments.contains(&value)
                || failure.arguments.contains(&value)
        }
        mir::Terminator::Switch {
            value: v,
            cases,
            default_arguments,
            ..
        } => {
            *v == value
                || cases.iter().any(|c| c.arguments.contains(&value))
                || default_arguments.contains(&value)
        }
        mir::Terminator::Yield {
            value: v,
            resume_arguments,
            ..
        } => *v == value || resume_arguments.contains(&value),
        mir::Terminator::TailCall { arguments, .. } => arguments.contains(&value),
        mir::Terminator::TailCallVirtual {
            receiver,
            arguments,
            ..
        }
        | mir::Terminator::TailCallInterface {
            receiver,
            arguments,
            ..
        } => *receiver == value || arguments.contains(&value),
        mir::Terminator::TailCallIndirect {
            callee, arguments, ..
        } => *callee == value || arguments.contains(&value),
    }
}

/// Get all values used by a terminator.
///
/// Returns a vector of all value operands in the terminator, including
/// conditions, return values, and block arguments.
pub fn terminator_used_values(term: &mir::Terminator) -> Vec<mir::Value> {
    match term {
        mir::Terminator::Return { value: Some(v) } => vec![*v],
        mir::Terminator::Return { value: None } | mir::Terminator::Unreachable => vec![],
        mir::Terminator::Jump { arguments, .. } => arguments.clone(),
        mir::Terminator::Branch {
            condition,
            then_arguments,
            else_arguments,
            ..
        } => {
            let mut values = vec![*condition];
            values.extend(then_arguments.iter().copied());
            values.extend(else_arguments.iter().copied());
            values
        }
        mir::Terminator::Check {
            condition,
            constraint,
            success,
            failure,
        } => {
            let mut values = vec![*condition];
            values.extend(constraint.uses().iter().copied());
            values.extend(success.arguments.iter().copied());
            values.extend(failure.arguments.iter().copied());
            values
        }
        mir::Terminator::Switch {
            value,
            cases,
            default_arguments,
            ..
        } => {
            let mut values = vec![*value];
            for case in cases {
                values.extend(case.arguments.iter().copied());
            }
            values.extend(default_arguments.iter().copied());
            values
        }
        mir::Terminator::Yield {
            value,
            resume_arguments,
            ..
        } => {
            let mut values = vec![*value];
            values.extend(resume_arguments.iter().copied());
            values
        }
        mir::Terminator::TailCall { arguments, .. } => arguments.clone(),
        mir::Terminator::TailCallVirtual {
            receiver,
            arguments,
            ..
        }
        | mir::Terminator::TailCallInterface {
            receiver,
            arguments,
            ..
        } => {
            let mut values = vec![*receiver];
            values.extend(arguments.iter().copied());
            values
        }
        mir::Terminator::TailCallIndirect {
            callee, arguments, ..
        } => {
            let mut values = vec![*callee];
            values.extend(arguments.iter().copied());
            values
        }
    }
}

/// Get the arguments passed to a specific successor block from a terminator.
pub fn terminator_arguments_for_successor(
    terminator: &mir::Terminator,
    successor: mir::LocalNodeId<mir::Block>,
) -> &[mir::Value] {
    match terminator {
        mir::Terminator::Jump { target, arguments } if *target == successor => arguments,

        mir::Terminator::Branch {
            then_target,
            then_arguments,
            else_target,
            else_arguments,
            ..
        } => {
            // then branch
            if *then_target == successor {
                then_arguments
            }
            // else branch
            else if *else_target == successor {
                else_arguments
            }
            // not a successor
            else {
                &[]
            }
        }
        mir::Terminator::Check {
            success, failure, ..
        } => {
            if success.target == successor {
                &success.arguments
            } else if failure.target == successor {
                &failure.arguments
            } else {
                &[]
            }
        }

        mir::Terminator::Switch {
            default,
            default_arguments,
            cases,
            ..
        } => {
            // default case
            if *default == successor {
                return default_arguments;
            }

            // numbered cases
            for case in cases {
                if case.target == successor {
                    return &case.arguments;
                }
            }

            &[]
        }

        mir::Terminator::Yield {
            resume,
            resume_arguments,
            ..
        } if *resume == successor => resume_arguments,

        _ => &[],
    }
}

/// Collect blocks reachable from the entry in function order.
pub fn collect_reachable_blocks(
    function: &mir::Function,
    tree: &mir::NodeTree,
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
        for successor in block_data.terminator.successors() {
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
// allow many arguments to keep the call sites explicit
#[allow(clippy::too_many_arguments)]
pub fn ensure_edge_block(
    predecessor: mir::LocalNodeId<mir::Block>,
    successor: mir::LocalNodeId<mir::Block>,
    function: &mut mir::Function,
    tree: &mut mir::NodeTree,
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
    let pred_multi = successor_count(pred_block) > 1;
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
            let mut new_default_arguments = default_arguments.clone();
            let mut new_cases = Vec::with_capacity(cases.len());
            let mut changed = false;

            if *default == successor {
                new_default = edge_block;
                new_default_arguments = Vec::new();
                changed = true;
            }

            for case in cases {
                if case.target == successor {
                    changed = true;
                    new_cases.push(mir::SwitchCase {
                        value: case.value,
                        target: edge_block,
                        arguments: Vec::new(),
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
                default_arguments: new_default_arguments,
                cases: new_cases,
            }
        }
        mir::Terminator::Yield {
            value,
            resume,
            resume_arguments: _,
        } => {
            // rewrite the resume target when it matches the successor
            let (new_resume, new_args) = if *resume == successor {
                (edge_block, Vec::new())
            } else {
                return false;
            };

            mir::Terminator::Yield {
                value: *value,
                resume: new_resume,
                resume_arguments: new_args,
            }
        }
        _ => return false,
    };

    // update the terminator
    if new_terminator != block.terminator {
        block.terminator = new_terminator;
        tree.replace(block_id, block);
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
    pub fn build(function: &mir::Function, tree: &mir::NodeTree, cfg: &ControlFlowGraph) -> Self {
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
                let successor_args = terminator_arguments_for_successor_checked(
                    &tree.get(pred).terminator,
                    block_id,
                );

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

                if candidate != param.value {
                    map.insert(param.value, candidate);
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
    tree: &mut mir::NodeTree,
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
        let terminator = block.terminator.clone();

        // rewrite instructions in place
        for instruction_id in instruction_ids {
            // substitute values in the instruction
            let instruction = tree.get(instruction_id).clone();
            let updated = instruction_substitute_uses_in_tree(&instruction, substitutions, tree);

            // replace when a rewrite occurred
            if updated != instruction {
                tree.replace(instruction_id, updated);
                changed = true;
            }
        }

        // rewrite terminator uses
        let new_terminator = terminator_substitute_uses(&terminator, substitutions);

        // replace the terminator when it changes
        if new_terminator != terminator {
            let mut new_block = block;
            new_block.terminator = new_terminator;
            tree.replace(block_id, new_block);
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
        mir::Terminator::Jump { target, arguments } => {
            if *target == successor {
                record_arguments(&mut candidate, &mut is_conflict, arguments);
            }
        }
        mir::Terminator::Branch {
            then_target,
            then_arguments,
            else_target,
            else_arguments,
            ..
        } => {
            if *then_target == successor {
                record_arguments(&mut candidate, &mut is_conflict, then_arguments);
            }
            if *else_target == successor {
                record_arguments(&mut candidate, &mut is_conflict, else_arguments);
            }
        }
        mir::Terminator::Check {
            success, failure, ..
        } => {
            if success.target == successor {
                record_arguments(&mut candidate, &mut is_conflict, &success.arguments);
            }
            if failure.target == successor {
                record_arguments(&mut candidate, &mut is_conflict, &failure.arguments);
            }
        }
        mir::Terminator::Switch {
            default,
            default_arguments,
            cases,
            ..
        } => {
            if *default == successor {
                record_arguments(&mut candidate, &mut is_conflict, default_arguments);
            }
            for case in cases {
                if case.target == successor {
                    record_arguments(&mut candidate, &mut is_conflict, &case.arguments);
                }
            }
        }
        mir::Terminator::Yield {
            resume,
            resume_arguments,
            ..
        } => {
            if *resume == successor {
                record_arguments(&mut candidate, &mut is_conflict, resume_arguments);
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
pub fn collect_block_uses(block: &mir::Block, tree: &mir::NodeTree) -> Vec<mir::Value> {
    // prepare the use list
    let mut uses = Vec::new();

    // collect uses from instructions
    for &instruction_id in &block.instructions {
        let instruction = tree.get(instruction_id);
        uses.extend(instruction.uses());
        if let Some(arguments) = instruction.argument_slice() {
            uses.extend(tree.get_arguments(arguments).iter().copied());
        }
    }

    // collect uses from the terminator
    match &block.terminator {
        mir::Terminator::Jump { arguments, .. } => {
            // record jump arguments
            uses.extend(arguments.iter().copied());
        }
        mir::Terminator::Branch {
            condition,
            then_arguments,
            else_arguments,
            ..
        } => {
            // record branch condition and arguments
            uses.push(*condition);
            uses.extend(then_arguments.iter().copied());
            uses.extend(else_arguments.iter().copied());
        }
        mir::Terminator::Check {
            condition,
            success,
            failure,
            ..
        } => {
            // record check condition and arguments
            uses.push(*condition);
            uses.extend(success.arguments.iter().copied());
            uses.extend(failure.arguments.iter().copied());
        }
        mir::Terminator::Switch {
            value,
            default_arguments,
            cases,
            ..
        } => {
            // record switch condition and arguments
            uses.push(*value);
            uses.extend(default_arguments.iter().copied());
            for case in cases {
                uses.extend(case.arguments.iter().copied());
            }
        }
        mir::Terminator::Yield {
            value,
            resume_arguments,
            ..
        } => {
            // record yield value and resume arguments
            uses.push(*value);
            uses.extend(resume_arguments.iter().copied());
        }
        mir::Terminator::Return { value } => {
            // record return value
            if let Some(value) = value {
                uses.push(*value);
            }
        }
        mir::Terminator::Unreachable
        | mir::Terminator::TailCall { .. }
        | mir::Terminator::TailCallVirtual { .. }
        | mir::Terminator::TailCallInterface { .. }
        | mir::Terminator::TailCallIndirect { .. } => {}
    }

    uses
}

/// Return true when all block uses are available in a predecessor.
pub fn block_uses_available_in_predecessor(
    block_id: mir::LocalNodeId<mir::Block>,
    block: &mir::Block,
    tree: &mir::NodeTree,
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
    param_indices: &HashMap<mir::Value, usize>,
) -> Option<mir::Value> {
    // map block parameters to predecessor arguments
    let Some(&param_index) = param_indices.get(&value) else {
        return Some(value);
    };

    // read arguments for the predecessor edge
    let args = match terminator_arguments_for_successor_checked(&predecessor.terminator, block_id) {
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
        let Some(uses) = use_blocks.get(&param.value) else {
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
pub fn function_thread_jumps(function: &mir::Function, tree: &mut mir::NodeTree) -> bool {
    // find all empty blocks (no instructions) that can be threaded
    let mut threadable: HashMap<mir::LocalNodeId<mir::Block>, ThreadableBlock> = HashMap::new();

    // scan blocks to identify threadable candidates
    for &block_id in &function.blocks {
        // read the block
        let block = tree.get(block_id);

        // block must have no instructions to be threadable
        if !block.instructions.is_empty() {
            continue;
        }

        match &block.terminator {
            mir::Terminator::Jump { target, arguments } => {
                if block.parameters.is_empty() && arguments.is_empty() {
                    threadable.insert(
                        block_id,
                        ThreadableBlock::Terminator(block.terminator.clone()),
                    );
                } else if block_is_passthrough_jump(block, arguments) {
                    threadable.insert(block_id, ThreadableBlock::Forward { target: *target });
                }
            }
            mir::Terminator::Return { .. } | mir::Terminator::Unreachable => {
                if block.parameters.is_empty() {
                    threadable.insert(
                        block_id,
                        ThreadableBlock::Terminator(block.terminator.clone()),
                    );
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

        let new_terminator = match &block.terminator {
            mir::Terminator::Jump { target, arguments } => {
                let resolved = block_resolve_jump_target(*target, arguments, &threadable);
                match resolved {
                    ResolvedTarget::Terminator(terminator) if arguments.is_empty() => {
                        Some(terminator)
                    }
                    ResolvedTarget::Jump {
                        target: new_target,
                        arguments: new_arguments,
                    } if new_target != *target || &new_arguments != arguments => {
                        Some(mir::Terminator::Jump {
                            target: new_target,
                            arguments: new_arguments,
                        })
                    }
                    _ => None,
                }
            }
            mir::Terminator::Branch {
                condition,
                then_target,
                then_arguments,
                else_target,
                else_arguments,
            } => {
                let then_resolved =
                    block_resolve_jump_target(*then_target, then_arguments, &threadable);
                let else_resolved =
                    block_resolve_jump_target(*else_target, else_arguments, &threadable);

                let (new_then, new_then_args) = match then_resolved {
                    ResolvedTarget::Jump { target, arguments }
                        if arguments_match_block(target, &arguments, tree) =>
                    {
                        (target, arguments)
                    }
                    _ => (*then_target, then_arguments.clone()),
                };
                let (new_else, new_else_args) = match else_resolved {
                    ResolvedTarget::Jump { target, arguments }
                        if arguments_match_block(target, &arguments, tree) =>
                    {
                        (target, arguments)
                    }
                    _ => (*else_target, else_arguments.clone()),
                };

                if new_then != *then_target
                    || new_else != *else_target
                    || new_then_args != *then_arguments
                    || new_else_args != *else_arguments
                {
                    Some(mir::Terminator::Branch {
                        condition: *condition,
                        then_target: new_then,
                        then_arguments: new_then_args,
                        else_target: new_else,
                        else_arguments: new_else_args,
                    })
                } else {
                    None
                }
            }
            mir::Terminator::Check {
                condition,
                constraint,
                success,
                failure,
            } => {
                let success_resolved =
                    block_resolve_jump_target(success.target, &success.arguments, &threadable);
                let failure_resolved =
                    block_resolve_jump_target(failure.target, &failure.arguments, &threadable);

                let (new_success, new_success_args) = match success_resolved {
                    ResolvedTarget::Jump { target, arguments }
                        if arguments_match_block(target, &arguments, tree) =>
                    {
                        (target, arguments)
                    }
                    _ => (success.target, success.arguments.clone()),
                };
                let (new_failure, new_failure_args) = match failure_resolved {
                    ResolvedTarget::Jump { target, arguments }
                        if arguments_match_block(target, &arguments, tree) =>
                    {
                        (target, arguments)
                    }
                    _ => (failure.target, failure.arguments.clone()),
                };

                if new_success != success.target
                    || new_failure != failure.target
                    || new_success_args != success.arguments
                    || new_failure_args != failure.arguments
                {
                    Some(mir::Terminator::Check {
                        condition: *condition,
                        constraint: constraint.clone(),
                        success: mir::CheckTarget {
                            target: new_success,
                            arguments: new_success_args,
                        },
                        failure: mir::CheckTarget {
                            target: new_failure,
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
                default_arguments,
                cases,
            } => {
                // resolve the default edge
                let default_resolved =
                    block_resolve_jump_target(*default, default_arguments, &threadable);

                // choose the resolved default target
                let (new_default, new_default_args) = match default_resolved {
                    ResolvedTarget::Jump { target, arguments }
                        if arguments_match_block(target, &arguments, tree) =>
                    {
                        (target, arguments)
                    }
                    _ => (*default, default_arguments.clone()),
                };

                // track whether any edge changes
                let mut remapped =
                    new_default != *default || new_default_args != *default_arguments;
                let mut new_cases = Vec::with_capacity(cases.len());

                // resolve case edges
                for case in cases {
                    let resolved =
                        block_resolve_jump_target(case.target, &case.arguments, &threadable);

                    // choose the resolved case target
                    let (target, arguments) = match resolved {
                        ResolvedTarget::Jump { target, arguments }
                            if arguments_match_block(target, &arguments, tree) =>
                        {
                            (target, arguments)
                        }
                        _ => (case.target, case.arguments.clone()),
                    };

                    // track remapped edges
                    if target != case.target || arguments != case.arguments {
                        remapped = true;
                    }

                    new_cases.push(mir::SwitchCase {
                        value: case.value,
                        target,
                        arguments,
                    });
                }

                // rebuild the switch when edges changed
                if remapped {
                    Some(mir::Terminator::Switch {
                        value: *value,
                        default: new_default,
                        default_arguments: new_default_args,
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
            let mut new_block = block.clone();
            new_block.terminator = terminator;
            tree.replace(block_id, new_block);
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
                mir::Terminator::Jump {
                    target: next_target,
                    arguments,
                } if arguments.is_empty() => {
                    current = *next_target;
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
    tree: &mir::NodeTree,
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
    terminator: &mir::Terminator,
    substitutions: &HashMap<mir::Value, mir::Value>,
) -> mir::Terminator {
    if substitutions.is_empty() {
        return terminator.clone();
    }

    let substitute = |v: &mir::Value| -> mir::Value { *substitutions.get(v).unwrap_or(v) };

    match terminator {
        mir::Terminator::Return { value } => mir::Terminator::Return {
            value: value.map(|v| substitute(&v)),
        },
        mir::Terminator::Jump { target, arguments } => mir::Terminator::Jump {
            target: *target,
            arguments: arguments.iter().map(&substitute).collect(),
        },
        mir::Terminator::Branch {
            condition,
            then_target,
            then_arguments,
            else_target,
            else_arguments,
        } => mir::Terminator::Branch {
            condition: substitute(condition),
            then_target: *then_target,
            then_arguments: then_arguments.iter().map(&substitute).collect(),
            else_target: *else_target,
            else_arguments: else_arguments.iter().map(&substitute).collect(),
        },
        mir::Terminator::Check {
            condition,
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
                    index: substitute(index),
                    length: substitute(length),
                    collection: substitute(collection),
                    is_signed: *is_signed,
                },
                mir::CheckConstraint::Null { value } => mir::CheckConstraint::Null {
                    value: substitute(value),
                },
                mir::CheckConstraint::DivZero { divisor } => mir::CheckConstraint::DivZero {
                    divisor: substitute(divisor),
                },
                mir::CheckConstraint::ShiftRange {
                    value,
                    bit_width,
                    is_signed,
                } => mir::CheckConstraint::ShiftRange {
                    value: substitute(value),
                    bit_width: *bit_width,
                    is_signed: *is_signed,
                },
                mir::CheckConstraint::Narrow {
                    value,
                    to_width,
                    is_signed,
                } => mir::CheckConstraint::Narrow {
                    value: substitute(value),
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
                    left: substitute(left),
                    right: substitute(right),
                    is_signed: *is_signed,
                },
                mir::CheckConstraint::Type { value, expected } => mir::CheckConstraint::Type {
                    value: substitute(value),
                    expected: *expected,
                },
                mir::CheckConstraint::Union { value, expected } => mir::CheckConstraint::Union {
                    value: substitute(value),
                    expected: *expected,
                },
                mir::CheckConstraint::Vtable { receiver, expected } => {
                    mir::CheckConstraint::Vtable {
                        receiver: substitute(receiver),
                        expected: *expected,
                    }
                }
                mir::CheckConstraint::Itab { receiver, expected } => mir::CheckConstraint::Itab {
                    receiver: substitute(receiver),
                    expected: *expected,
                },
            };

            let success = mir::CheckTarget {
                target: success.target,
                arguments: success.arguments.iter().map(&substitute).collect(),
            };

            let failure = mir::CheckTarget {
                target: failure.target,
                arguments: failure.arguments.iter().map(&substitute).collect(),
            };

            mir::Terminator::Check {
                condition: substitute(condition),
                constraint: constraint.clone(),
                success,
                failure,
            }
        }
        mir::Terminator::Switch {
            value,
            default,
            default_arguments,
            cases,
        } => mir::Terminator::Switch {
            value: substitute(value),
            default: *default,
            default_arguments: default_arguments.iter().map(&substitute).collect(),
            cases: cases
                .iter()
                .map(|case| mir::SwitchCase {
                    value: case.value,
                    target: case.target,
                    arguments: case.arguments.iter().map(&substitute).collect(),
                })
                .collect(),
        },
        mir::Terminator::Yield {
            value,
            resume,
            resume_arguments,
        } => mir::Terminator::Yield {
            value: substitute(value),
            resume: *resume,
            resume_arguments: resume_arguments.iter().map(&substitute).collect(),
        },
        mir::Terminator::Unreachable => mir::Terminator::Unreachable,
        mir::Terminator::TailCall {
            function,
            arguments,
        } => mir::Terminator::TailCall {
            function: *function,
            arguments: arguments.iter().map(&substitute).collect(),
        },
        mir::Terminator::TailCallVirtual {
            receiver,
            arguments,
            declaring_type,
            slot_id,
            declared_target,
            signature,
        } => mir::Terminator::TailCallVirtual {
            receiver: substitute(receiver),
            arguments: arguments.iter().map(&substitute).collect(),
            declaring_type: *declaring_type,
            slot_id: *slot_id,
            declared_target: *declared_target,
            signature: *signature,
        },
        mir::Terminator::TailCallInterface {
            receiver,
            arguments,
            declaring_type,
            slot_id,
            declared_target,
            signature,
        } => mir::Terminator::TailCallInterface {
            receiver: substitute(receiver),
            arguments: arguments.iter().map(&substitute).collect(),
            declaring_type: *declaring_type,
            slot_id: *slot_id,
            declared_target: *declared_target,
            signature: *signature,
        },
        mir::Terminator::TailCallIndirect {
            callee,
            arguments,
            signature,
        } => mir::Terminator::TailCallIndirect {
            callee: substitute(callee),
            arguments: arguments.iter().map(&substitute).collect(),
            signature: *signature,
        },
    }
}
