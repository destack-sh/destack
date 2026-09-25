use crate as mir;
use tspp_core::{FxIndexMap, FxIndexSet};

use crate::{
    ControlTable, DefinitionTable, DominatorTable, UseTable, instruction_substitute_uses_in_tree,
    terminator_remap,
};

/// Return one block target with appended arguments.
fn block_target_with_arguments(
    tree: &mut mir::Tree,
    target: &mir::BlockTarget,
    extra_args: &[mir::Value],
) -> mir::BlockTarget {
    let mut arguments = target.arguments(tree).to_vec();
    arguments.extend_from_slice(extra_args);

    let mut target = target.clone();
    target.arguments = tree.add_values(&arguments);

    target
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
pub fn append_edge_arguments(
    tree: &mut mir::Tree,
    block_id: mir::LocalNodeId<mir::Block>,
    successor: mir::LocalNodeId<mir::Block>,
    extra_args: &[mir::Value],
) {
    // rewrite every matching edge
    let terminator_id = tree.get(block_id).terminator;
    let mut terminator = tree.get(terminator_id).clone();
    let is_changed = terminator.rewrite_successor(
        successor,
        |target, tree| block_target_with_arguments(tree, target, extra_args),
        tree,
    );

    // write back only when arguments changed
    if is_changed {
        tree.set(terminator_id, terminator);
    }
}

/// Ensure insertions happen on the correct edge when needed.
#[allow(clippy::too_many_arguments)]
pub fn ensure_edge_block(
    predecessor: mir::LocalNodeId<mir::Block>,
    successor: mir::LocalNodeId<mir::Block>,
    function: &mut mir::Function,
    tree: &mut mir::Tree,
    cfg: &ControlTable,
    edge_blocks: &mut FxIndexMap<
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
    let args = match pred_terminator.successor_arguments(tree, successor) {
        mir::EdgeArguments::Found(args) => args.to_vec(),
        _ => return predecessor,
    };

    // build the new edge block
    let edge_arguments = tree.add_values(&args);
    let edge_terminator = tree.insert(mir::Terminator::Jump {
        target: mir::BlockTarget::new(successor, edge_arguments),
    });
    let edge_block = mir::Block::new(edge_terminator);
    let edge_block_id = tree.insert(edge_block);
    insert_block_after(function, predecessor, edge_block_id, tree);

    // redirect the predecessor to the edge block
    if redirect_successor_to_edge(predecessor, successor, edge_block_id, tree) {
        edge_blocks.insert((predecessor, successor), edge_block_id);
        *changed = true;
        edge_block_id
    } else {
        predecessor
    }
}

impl mir::Edge {
    /// Split this exact edge and return its insertion block.
    pub fn split(
        self,
        function: &mut mir::Function,
        tree: &mut mir::Tree,
        edge_blocks: &mut FxIndexMap<mir::Edge, mir::LocalNodeId<mir::Block>>,
        changed: &mut bool,
    ) -> mir::LocalNodeId<mir::Block> {
        if let Some(existing) = edge_blocks.get(&self) {
            return *existing;
        }

        // capture the exact edge before mutating its terminator
        let terminator_id = tree.get(self.source).terminator;
        let terminator = tree.get(terminator_id);
        let target = terminator
            .targets(tree, self.source)
            .into_iter()
            .find_map(|(edge, target)| (edge == self).then_some(target.clone()))
            .unwrap_or_else(|| unreachable!("control-flow edge is absent from its terminator"));
        let arguments = target.arguments(tree).to_vec();
        let destination = tree.get(self.target);

        // reproduce every implicit and explicit destination parameter
        let parameters = destination
            .parameters
            .iter()
            .map(|parameter| mir::BlockParameter {
                value: function.next_typed_value(parameter.ty),
                ty: parameter.ty,
            })
            .collect::<Vec<_>>();
        let forwarded = parameters
            .iter()
            .map(|parameter| parameter.value)
            .collect::<Vec<_>>();

        // forward the complete edge state into the original destination
        let forwarded = tree.add_values(&forwarded);
        let terminator = tree.insert(mir::Terminator::Jump {
            target: mir::BlockTarget::new(self.target, forwarded),
        });
        let block = tree.insert(mir::Block::with_parameters(parameters, terminator));

        // redirect the selected edge with its original explicit arguments
        let mut terminator = tree.get(terminator_id).clone();
        let arguments = tree.add_values(&arguments);
        let target = mir::BlockTarget::new(block, arguments);
        if !terminator.replace_edge(self.successor, target, tree) {
            unreachable!("control-flow edge cannot be replaced in its terminator");
        }
        tree.set(terminator_id, terminator);
        insert_block_after(function, self.source, block, tree);

        edge_blocks.insert(self, block);
        *changed = true;

        block
    }
}

/// Count the unique successors for a terminator.
fn successor_count(tree: &mir::Tree, terminator: &mir::Terminator) -> usize {
    // track unique successors
    let mut unique = FxIndexSet::default();
    for successor in terminator.successors(tree) {
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
    tree: &mir::Tree,
) {
    let Some(body) = function.body_mut() else {
        unreachable!("cannot insert a block into a function without a body");
    };

    body.insert_block_after(predecessor, block, tree);
}

/// Redirect a successor edge to a new edge block.
fn redirect_successor_to_edge(
    block_id: mir::LocalNodeId<mir::Block>,
    successor: mir::LocalNodeId<mir::Block>,
    edge_block: mir::LocalNodeId<mir::Block>,
    tree: &mut mir::Tree,
) -> bool {
    // redirect every matching edge
    let terminator_id = tree.get(block_id).terminator;
    let mut terminator = tree.get(terminator_id).clone();
    let is_changed = terminator.rewrite_successor(
        successor,
        |_target, _tree| mir::BlockTarget::new(edge_block, mir::ValueSlice::default()),
        tree,
    );

    // update the terminator
    if is_changed {
        tree.set(terminator_id, terminator);
    }

    is_changed
}

/// Forwarding information for block parameters.
#[derive(Debug, Clone)]
pub struct BlockParamForwarding {
    /// Mapping from block parameters to their forwarded arguments.
    map: FxIndexMap<mir::Value, mir::Value>,
}

impl BlockParamForwarding {
    /// Build forwarding information for block parameters.
    pub fn build(function: &mir::Function, tree: &mir::Tree, cfg: &ControlTable) -> Self {
        // map parameters to consistent incoming values
        let mut map = FxIndexMap::default();

        // scan blocks for forwarded parameters
        for &block_id in function.blocks() {
            // read block parameters
            let block = tree.get(block_id);
            if block.parameters.is_empty() {
                continue;
            }

            // track candidates and conflicts per parameter
            let mut candidates: Vec<Option<mir::Value>> = vec![None; block.parameters.len()];
            let mut conflicts = vec![false; block.parameters.len()];
            let mut saw_pred = false;

            for pred in cfg.predecessors(block_id) {
                // read arguments for the predecessor edge
                let pred_block = tree.get(pred);
                let pred_terminator = tree.get(pred_block.terminator);
                let successor_args = pred_terminator.successor_arguments(tree, block_id);

                let args = match successor_args {
                    mir::EdgeArguments::Found(args) => args,
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
    dominator: &DominatorTable,
    root: mir::LocalNodeId<mir::Block>,
    substitutions: &FxIndexMap<mir::Value, mir::Value>,
) -> bool {
    // skip when there is nothing to substitute
    if substitutions.is_empty() {
        return false;
    }

    // track whether any changes were made
    let mut changed = false;

    // update blocks dominated by the root
    for &block_id in function.blocks() {
        // skip blocks not dominated by the root
        if !dominator.dominates(root, block_id) {
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
    uses.extend(terminator.uses(tree).iter().copied());

    uses
}

/// Return true when all block uses are available in a predecessor.
pub fn block_uses_available_in_predecessor(
    block_id: mir::LocalNodeId<mir::Block>,
    block: &mir::Block,
    tree: &mir::Tree,
    predecessor: mir::LocalNodeId<mir::Block>,
    definitions: &DefinitionTable,
    dominator: &DominatorTable,
) -> bool {
    // collect block parameter values
    let mut param_values: FxIndexSet<mir::Value> = FxIndexSet::default();

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
        let Some(definition) = definitions.definition(value) else {
            return false;
        };

        // accept function parameters and values defined inside the block
        let Some(definition_block) = definition.block() else {
            continue;
        };
        if definition_block != block_id
            && !definitions.is_available_at_exit(value, predecessor, dominator)
        {
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
    param_indices: &FxIndexMap<mir::Value, usize>,
) -> Option<mir::Value> {
    // map block parameters to predecessor arguments
    let Some(&param_index) = param_indices.get(&value) else {
        return Some(value);
    };

    // read arguments for the predecessor edge
    let predecessor_terminator = tree.get(predecessor.terminator);
    let args = match predecessor_terminator.successor_arguments(tree, block_id) {
        mir::EdgeArguments::Found(args) => args,
        _ => return None,
    };

    // return the argument at the parameter index
    args.get(param_index).copied()
}

/// Return true when a value is available in a block.
pub fn value_available_in_block(
    value: mir::Value,
    block_id: mir::LocalNodeId<mir::Block>,
    definitions: &DefinitionTable,
    dominator: &DominatorTable,
) -> bool {
    definitions.is_available_at_exit(value, block_id, dominator)
}

/// Return true when block parameters are used outside the block.
pub fn block_parameters_used_outside_block(
    block: &mir::Block,
    uses: &UseTable,
    block_id: mir::LocalNodeId<mir::Block>,
) -> bool {
    // detect parameter uses outside of the defining block
    for param in &block.parameters {
        let param = param.value;
        if uses.is_used_outside(param, block_id) {
            return true;
        }
    }

    false
}

/// Thread jumps through empty and passthrough blocks.
pub fn function_thread_jumps(function: &mir::Function, tree: &mut mir::Tree) -> bool {
    // find all empty blocks (no instructions) that can be threaded
    let mut threadable: FxIndexMap<mir::LocalNodeId<mir::Block>, ThreadableBlock> =
        FxIndexMap::default();

    // scan blocks to identify threadable candidates
    for &block_id in function.blocks() {
        // read the block
        let block = tree.get(block_id);
        let terminator = tree.get(block.terminator);

        // block must have no instructions to be threadable
        if !block.instructions.is_empty() {
            continue;
        }

        match terminator {
            mir::Terminator::Jump { target } => {
                let target_arguments = target.arguments(tree);
                if block.parameters.is_empty() && target.arguments.is_empty() {
                    threadable.insert(block_id, ThreadableBlock::Terminator(terminator.clone()));
                } else if block_is_passthrough_jump(block, target_arguments) {
                    let target = target.block;

                    threadable.insert(block_id, ThreadableBlock::Forward { target });
                }
            }
            mir::Terminator::Return { .. }
            | mir::Terminator::Abort { .. }
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
    for &block_id in function.blocks() {
        // snapshot the terminator before interning new payload slices
        let terminator_id = tree.get(block_id).terminator;
        let terminator = tree.get(terminator_id).clone();

        let new_terminator = match &terminator {
            mir::Terminator::Jump { target } => {
                let target_block = target.block;
                let target_arguments = target.arguments(tree).to_vec();

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
                let then_arguments = then_target.arguments(tree).to_vec();
                let else_arguments = else_target.arguments(tree).to_vec();

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
                let success_arguments = success.arguments(tree).to_vec();
                let failure_arguments = failure.arguments(tree).to_vec();

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
                let default_arguments = default.arguments(tree).to_vec();

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
                    let case_arguments = case.target.arguments(tree).to_vec();

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

/// Threadable block tables.
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
    threadable: &FxIndexMap<mir::LocalNodeId<mir::Block>, ThreadableBlock>,
) -> ResolvedTarget {
    // seed the traversal state
    let mut current = target;
    let mut current_args = arguments.to_vec();
    let mut visited = FxIndexSet::default();

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

/// Substitute mapped values in one terminator.
pub fn terminator_substitute_uses(
    tree: &mut mir::Tree,
    terminator: &mir::Terminator,
    substitutions: &FxIndexMap<mir::Value, mir::Value>,
) -> mir::Terminator {
    if substitutions.is_empty() {
        return terminator.clone();
    }

    let mut terminator = terminator.clone();
    let block_map = FxIndexMap::default();
    terminator_remap(tree, &mut terminator, &block_map, substitutions);

    terminator
}
