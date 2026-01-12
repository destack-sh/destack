use std::collections::{HashMap, HashSet};

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
        mir::Terminator::TailCallIndirect { callee, arguments } => {
            *callee == value || arguments.contains(&value)
        }
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
        mir::Terminator::TailCallIndirect { callee, arguments } => {
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

/// Thread jumps through empty or passthrough blocks.
///
/// If a block has no instructions and either has no parameters or just forwards
/// them, predecessors can bypass it. For jump terminators, we resolve chains
/// (A->B->C becomes A->C).
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
                    ResolvedTarget::Jump { target, arguments } => (target, arguments),
                    _ => (*then_target, then_arguments.clone()),
                };
                let (new_else, new_else_args) = match else_resolved {
                    ResolvedTarget::Jump { target, arguments } => (target, arguments),
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
                    ResolvedTarget::Jump { target, arguments } => (target, arguments),
                    _ => (success.target, success.arguments.clone()),
                };
                let (new_failure, new_failure_args) = match failure_resolved {
                    ResolvedTarget::Jump { target, arguments } => (target, arguments),
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
        mir::Terminator::TailCallIndirect { callee, arguments } => {
            mir::Terminator::TailCallIndirect {
                callee: substitute(callee),
                arguments: arguments.iter().map(&substitute).collect(),
            }
        }
    }
}
