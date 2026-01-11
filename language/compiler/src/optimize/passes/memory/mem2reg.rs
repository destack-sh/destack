use std::collections::{HashMap, HashSet, VecDeque};

use destack_compiler_macros::declare_pass;
use destack_mir as mir;
use mir::{Instruction, Terminator};

use crate::optimize::analyses::{ControlFlowGraph, DominatorTree};
use crate::optimize::common::{instruction_substitute_uses_in_tree, terminator_substitute_uses};
use crate::optimize::{AnalysisPreservation, FunctionAnalyses, FunctionPass, PipelineContext};

declare_pass! {
    /// Memory to register promotion pass.
    ///
    /// Promotes local variables (stack slots) to SSA values when they:
    /// - Are only accessed by LocalGet and LocalSet (no address taken)
    /// - Have no volatile or atomic access requirements
    ///
    /// This is a traditional SSA construction pass that eliminates memory
    /// operations in favor of direct value flow through block parameters.
    ///
    /// ```mir
    /// function @before(v0: i32) -> i32 {
    ///     local0: i32 ; owned, mut
    /// block0(v0: i32):
    ///     local.set local0, v0
    ///     jump block1
    /// block1:
    ///     v1 = local.get local0
    ///     return v1
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function @after(v0: i32) -> i32 {
    /// block0(v0: i32):
    ///     jump block1(v0)
    /// block1(v1: i32):
    ///     return v1
    /// }
    /// ```
    #[pass(id = "mem2reg")]
    pub Mem2Reg,
    "Promote locals to SSA values"
}

impl FunctionPass for Mem2Reg {
    /// Run memory to register promotion on a function.
    fn run(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::NodeTree,
        _ctx: &PipelineContext<'_>,
    ) -> AnalysisPreservation {
        // skip functions without locals
        if function.locals.is_empty() {
            return AnalysisPreservation::all();
        }

        // get analyses
        let (cfg, domtree) = {
            let analyses = FunctionAnalyses::new(function, tree);
            (
                analyses.get::<ControlFlowGraph>().clone(),
                analyses.get::<DominatorTree>().clone(),
            )
        };

        // run mem2reg
        let changed = run_mem2reg(function, tree, &cfg, &domtree);

        // select preservation based on mem2reg changes
        if changed {
            AnalysisPreservation::none()
        } else {
            AnalysisPreservation::all()
        }
    }

    /// Return the pass name.
    fn name(&self) -> &'static str {
        "Mem2Reg"
    }

    /// Return the pass identifier.
    fn id(&self) -> &'static str {
        "mem2reg"
    }
}

/// Core mem2reg logic. Returns true if changes were made.
fn run_mem2reg(
    function: &mut mir::Function,
    tree: &mut mir::NodeTree,
    cfg: &ControlFlowGraph,
    domtree: &DominatorTree,
) -> bool {
    // find promotable locals (only LocalGet/LocalSet, no address taken)
    let promotable = find_promotable_locals(function, tree);
    if promotable.is_empty() {
        return false;
    }

    // ensure next_value_id is correct before allocating new values
    function.recompute_next_value_id(tree);

    // compute dominance frontiers
    let frontiers = compute_dominance_frontiers(function, cfg, domtree);

    // find definition blocks for each promotable local
    let def_blocks = find_definition_blocks(&promotable, function, tree);

    // compute local liveness for pruned SSA placement
    let local_liveness = compute_local_liveness(&promotable, function, tree);

    // compute where block parameters are needed (dominance frontier pruned by liveness)
    let param_placements = compute_parameter_placements(
        &promotable,
        &def_blocks,
        &frontiers,
        &local_liveness,
        function,
    );

    // insert block parameters for locals
    let block_params = insert_block_parameters(&param_placements, &promotable, function, tree);

    // rename variables: replace LocalGet/LocalSet with SSA values
    let entry = function.entry.expect("function has no entry block");
    rename_variables(
        &promotable,
        &block_params,
        function,
        tree,
        cfg,
        domtree,
        entry,
    );

    // remove promoted locals from the function
    let promoted_set: HashSet<_> = promotable.keys().copied().collect();
    function
        .locals
        .retain(|local| !promoted_set.contains(local));

    true
}

/// Information about a promotable local.
#[derive(Debug)]
struct PromotableLocal {
    /// The type of the local.
    ty: mir::LocalNodeId<mir::Type>,
}

/// Type alias for the renaming worklist to avoid type_complexity warning.
///
/// Each entry is (block_id, stack_depths) where stack_depths records how deep
/// each local's value stack was when entering this block (for backtracking).
type RenameWorklistEntry = (
    mir::LocalNodeId<mir::Block>,
    Vec<(mir::LocalNodeId<mir::Local>, usize)>,
);

/// Find locals that can be promoted to SSA values.
///
/// Currently all locals are promotable since the MIR only supports LocalGet/LocalSet
/// (no address-taking). If LocalAddr is added in the future, this function should
/// filter out locals whose address is taken.
fn find_promotable_locals(
    function: &mir::Function,
    tree: &mir::NodeTree,
) -> HashMap<mir::LocalNodeId<mir::Local>, PromotableLocal> {
    function
        .locals
        .iter()
        .map(|&local_id| {
            let local = tree.get(local_id);
            (local_id, PromotableLocal { ty: local.ty })
        })
        .collect()
}

/// Compute dominance frontiers for all blocks.
fn compute_dominance_frontiers(
    function: &mir::Function,
    cfg: &ControlFlowGraph,
    domtree: &DominatorTree,
) -> HashMap<mir::LocalNodeId<mir::Block>, HashSet<mir::LocalNodeId<mir::Block>>> {
    let mut frontiers: HashMap<
        mir::LocalNodeId<mir::Block>,
        HashSet<mir::LocalNodeId<mir::Block>>,
    > = HashMap::new();

    for &block in &function.blocks {
        frontiers.insert(block, HashSet::new());
    }

    // for each block, compute its dominance frontier using the standard algorithm:
    // df(x) = {y : y has a predecessor z where x dominates z but x does not strictly dominate y}
    for &block in &function.blocks {
        let preds = cfg.predecessors(block);
        if preds.len() >= 2 {
            // block is a join point, check each predecessor
            for &pred in preds {
                let mut runner = pred;
                // walk up the dominator tree until we reach block's immediate dominator
                while Some(runner) != domtree.immediate_dominator(block)
                    && runner != block
                    && domtree.immediate_dominator(runner).is_some()
                {
                    frontiers.get_mut(&runner).unwrap().insert(block);
                    runner = domtree.immediate_dominator(runner).unwrap();
                }
            }
        }
    }

    frontiers
}

/// Find which blocks contain definitions (LocalSet) for each promotable local.
fn find_definition_blocks(
    promotable: &HashMap<mir::LocalNodeId<mir::Local>, PromotableLocal>,
    function: &mir::Function,
    tree: &mir::NodeTree,
) -> HashMap<mir::LocalNodeId<mir::Local>, HashSet<mir::LocalNodeId<mir::Block>>> {
    let mut def_blocks: HashMap<
        mir::LocalNodeId<mir::Local>,
        HashSet<mir::LocalNodeId<mir::Block>>,
    > = promotable.keys().map(|&k| (k, HashSet::new())).collect();

    for &block_id in &function.blocks {
        let block = tree.get(block_id);
        for &instruction_id in &block.instructions {
            let instruction = tree.get(instruction_id);
            if let Instruction::LocalSet { local, .. } = instruction
                && promotable.contains_key(local)
            {
                def_blocks.get_mut(local).unwrap().insert(block_id);
            }
        }
    }

    def_blocks
}

/// Compute where block parameters are needed for each local.
///
/// In block parameter SSA, values must be explicitly passed between blocks.
/// A block needs a parameter for a local when it is in the dominance frontier
/// of a definition and the local is live in to that block.
fn compute_parameter_placements(
    promotable: &HashMap<mir::LocalNodeId<mir::Local>, PromotableLocal>,
    def_blocks: &HashMap<mir::LocalNodeId<mir::Local>, HashSet<mir::LocalNodeId<mir::Block>>>,
    frontiers: &HashMap<mir::LocalNodeId<mir::Block>, HashSet<mir::LocalNodeId<mir::Block>>>,
    local_liveness: &HashMap<mir::LocalNodeId<mir::Block>, HashSet<mir::LocalNodeId<mir::Local>>>,
    function: &mir::Function,
) -> HashMap<mir::LocalNodeId<mir::Block>, HashSet<mir::LocalNodeId<mir::Local>>> {
    // initialize block parameter placement map
    let mut param_placements: HashMap<
        mir::LocalNodeId<mir::Block>,
        HashSet<mir::LocalNodeId<mir::Local>>,
    > = function
        .blocks
        .iter()
        .map(|&b| (b, HashSet::new()))
        .collect();

    // add block parameters at dominance frontiers pruned by liveness
    for &local in promotable.keys() {
        let defs = &def_blocks[&local];
        let mut worklist: VecDeque<mir::LocalNodeId<mir::Block>> = defs.iter().copied().collect();
        let mut seen = defs.clone();

        while let Some(block) = worklist.pop_front() {
            for &frontier_block in &frontiers[&block] {
                let is_live = local_liveness
                    .get(&frontier_block)
                    .map(|locals| locals.contains(&local))
                    .unwrap_or(false);
                if !is_live {
                    continue;
                }

                if !param_placements[&frontier_block].contains(&local) {
                    param_placements
                        .get_mut(&frontier_block)
                        .unwrap()
                        .insert(local);
                    if !seen.contains(&frontier_block) {
                        seen.insert(frontier_block);
                        worklist.push_back(frontier_block);
                    }
                }
            }
        }
    }

    param_placements
}

/// Compute local liveness for promotable locals.
fn compute_local_liveness(
    promotable: &HashMap<mir::LocalNodeId<mir::Local>, PromotableLocal>,
    function: &mir::Function,
    tree: &mir::NodeTree,
) -> HashMap<mir::LocalNodeId<mir::Block>, HashSet<mir::LocalNodeId<mir::Local>>> {
    // initialize per block use and def sets
    let mut block_use: HashMap<
        mir::LocalNodeId<mir::Block>,
        HashSet<mir::LocalNodeId<mir::Local>>,
    > = HashMap::new();
    let mut block_def: HashMap<
        mir::LocalNodeId<mir::Block>,
        HashSet<mir::LocalNodeId<mir::Local>>,
    > = HashMap::new();

    // compute local use and def sets
    for &block_id in &function.blocks {
        let block = tree.get(block_id);
        let mut seen_defs: HashSet<mir::LocalNodeId<mir::Local>> = HashSet::new();
        let mut uses: HashSet<mir::LocalNodeId<mir::Local>> = HashSet::new();
        let mut defs: HashSet<mir::LocalNodeId<mir::Local>> = HashSet::new();

        for &instruction_id in &block.instructions {
            let instruction = tree.get(instruction_id);
            match instruction {
                Instruction::LocalGet { local, .. } if promotable.contains_key(local) => {
                    let was_defined = seen_defs.contains(local);
                    if !was_defined {
                        uses.insert(*local);
                    }
                }
                Instruction::LocalSet { local, .. } if promotable.contains_key(local) => {
                    seen_defs.insert(*local);
                    defs.insert(*local);
                }
                _ => {}
            }
        }

        block_use.insert(block_id, uses);
        block_def.insert(block_id, defs);
    }

    // initialize liveness state
    let mut live_in: HashMap<mir::LocalNodeId<mir::Block>, HashSet<mir::LocalNodeId<mir::Local>>> =
        HashMap::new();
    let mut live_out: HashMap<mir::LocalNodeId<mir::Block>, HashSet<mir::LocalNodeId<mir::Local>>> =
        HashMap::new();

    for &block_id in &function.blocks {
        live_in.insert(block_id, HashSet::new());
        live_out.insert(block_id, HashSet::new());
    }

    // run backward dataflow to fixed point
    let mut changed = true;
    while changed {
        // reset convergence flag for this iteration
        changed = false;

        // update blocks in reverse order for faster convergence
        for &block_id in function.blocks.iter().rev() {
            let block = tree.get(block_id);

            // live_out is union of successor live_in sets
            let mut new_live_out: HashSet<mir::LocalNodeId<mir::Local>> = HashSet::new();
            for successor in block.terminator.successors() {
                if let Some(successor_live_in) = live_in.get(&successor) {
                    new_live_out.extend(successor_live_in.iter().copied());
                }
            }

            // live_in = use ∪ (live_out - def)
            let block_uses = block_use.get(&block_id).expect("block use missing");
            let block_defs = block_def.get(&block_id).expect("block def missing");
            let mut new_live_in: HashSet<mir::LocalNodeId<mir::Local>> =
                new_live_out.difference(block_defs).copied().collect();
            new_live_in.extend(block_uses.iter().copied());

            // update maps when changed
            let live_in_changed = new_live_in != *live_in.get(&block_id).unwrap();
            let live_out_changed = new_live_out != *live_out.get(&block_id).unwrap();
            if live_in_changed {
                live_in.insert(block_id, new_live_in);
                changed = true;
            }
            if live_out_changed {
                live_out.insert(block_id, new_live_out);
                changed = true;
            }
        }
    }

    live_in
}

/// Insert block parameters for promoted locals.
/// Returns a mapping from (block, local) -> parameter value.
fn insert_block_parameters(
    param_placements: &HashMap<mir::LocalNodeId<mir::Block>, HashSet<mir::LocalNodeId<mir::Local>>>,
    promotable: &HashMap<mir::LocalNodeId<mir::Local>, PromotableLocal>,
    function: &mut mir::Function,
    tree: &mut mir::NodeTree,
) -> HashMap<(mir::LocalNodeId<mir::Block>, mir::LocalNodeId<mir::Local>), mir::Value> {
    let mut block_params = HashMap::new();

    // sort blocks for deterministic value allocation order
    let mut sorted_blocks: Vec<_> = param_placements.keys().copied().collect();
    sorted_blocks.sort_by_key(|b| b.id);

    for block_id in sorted_blocks {
        let locals = &param_placements[&block_id];
        if locals.is_empty() {
            continue;
        }

        let mut block = tree.get(block_id).clone();

        // sort locals for deterministic order
        let mut sorted_locals: Vec<_> = locals.iter().copied().collect();
        sorted_locals.sort_by_key(|l| l.id);

        for local in sorted_locals {
            let ty = promotable[&local].ty;
            let param_value = function.next_value();

            block.parameters.push(mir::TypedValue {
                value: param_value,
                ty,
            });

            block_params.insert((block_id, local), param_value);
        }

        tree.replace(block_id, block);
    }

    block_params
}

/// Rename variables: replace LocalGet with SSA values, LocalSet with assignments.
fn rename_variables(
    promotable: &HashMap<mir::LocalNodeId<mir::Local>, PromotableLocal>,
    block_params: &HashMap<
        (mir::LocalNodeId<mir::Block>, mir::LocalNodeId<mir::Local>),
        mir::Value,
    >,
    function: &mut mir::Function,
    tree: &mut mir::NodeTree,
    _cfg: &ControlFlowGraph,
    domtree: &DominatorTree,
    entry: mir::LocalNodeId<mir::Block>,
) {
    // current value for each local during renaming (stack for each local)
    let mut value_stacks: HashMap<mir::LocalNodeId<mir::Local>, Vec<mir::Value>> =
        promotable.keys().map(|&k| (k, Vec::new())).collect();

    // track instructions to remove (LocalGet/LocalSet for promoted locals)
    let mut instructions_to_remove: HashSet<mir::LocalNodeId<mir::Instruction>> = HashSet::new();

    // track value substitutions (LocalGet destination -> actual value)
    let mut substitutions: HashMap<mir::Value, mir::Value> = HashMap::new();

    // dfs in dominator tree order
    let mut worklist: Vec<RenameWorklistEntry> = vec![(entry, Vec::new())];
    while let Some((block_id, stack_depths)) = worklist.pop() {
        // restore stack depths from when we entered this block
        for (local, depth) in &stack_depths {
            value_stacks.get_mut(local).unwrap().truncate(*depth);
        }

        // process block parameters for this block
        for (&(param_block, local), &param_value) in block_params {
            if param_block == block_id {
                value_stacks.get_mut(&local).unwrap().push(param_value);
            }
        }

        // process instructions in this block
        let block = tree.get(block_id);
        let instruction_ids: Vec<_> = block.instructions.clone();
        for instruction_id in instruction_ids {
            let instruction = tree.get(instruction_id);
            match instruction {
                Instruction::LocalGet { destination, local } => {
                    if promotable.contains_key(local) {
                        // replace with current value
                        let current_value =
                            value_stacks[local].last().copied().unwrap_or_else(|| {
                                panic!(
                                    "use of undefined local in block {block_id:?} \
                                 (local was read before being written, this is invalid MIR)"
                                )
                            });
                        substitutions.insert(*destination, current_value);
                        instructions_to_remove.insert(instruction_id);
                    }
                }
                Instruction::LocalSet { local, value } => {
                    if promotable.contains_key(local) {
                        // apply any pending substitutions to the value
                        let actual_value = resolve_value(*value, &substitutions);
                        value_stacks.get_mut(local).unwrap().push(actual_value);
                        instructions_to_remove.insert(instruction_id);
                    }
                }
                _ => {}
            }
        }

        // update terminator to pass block arguments to successors
        let block = tree.get(block_id);
        let new_terminator = update_terminator_arguments(
            &block.terminator,
            block_id,
            block_params,
            &value_stacks,
            &substitutions,
        );

        if new_terminator != block.terminator {
            let mut new_block = block.clone();
            new_block.terminator = new_terminator;
            tree.replace(block_id, new_block);
        }

        // record current stack depths for children
        let current_depths: Vec<(mir::LocalNodeId<mir::Local>, usize)> = promotable
            .keys()
            .map(|&k| (k, value_stacks[&k].len()))
            .collect();

        // push dominator tree children onto worklist
        for &child_block in &function.blocks {
            if domtree.immediate_dominator(child_block) == Some(block_id) {
                worklist.push((child_block, current_depths.clone()));
            }
        }
    }

    // apply substitutions to all remaining instructions
    for &block_id in &function.blocks {
        let block = tree.get(block_id);
        let instruction_ids: Vec<_> = block.instructions.clone();
        for instruction_id in instruction_ids {
            if instructions_to_remove.contains(&instruction_id) {
                continue;
            }

            let instruction = tree.get(instruction_id).clone();
            let new_instruction =
                instruction_substitute_uses_in_tree(&instruction, &substitutions, tree);
            if new_instruction != instruction {
                tree.replace(instruction_id, new_instruction);
            }
        }

        // apply substitutions to terminator
        let block = tree.get(block_id);
        let new_terminator = terminator_substitute_uses(&block.terminator, &substitutions);
        if new_terminator != block.terminator {
            let mut new_block = block.clone();
            new_block.terminator = new_terminator;
            tree.replace(block_id, new_block);
        }
    }

    // remove dead instructions
    for &block_id in &function.blocks {
        let block = tree.get(block_id);
        let new_instructions: Vec<_> = block
            .instructions
            .iter()
            .filter(|&&id| !instructions_to_remove.contains(&id))
            .copied()
            .collect();

        if new_instructions.len() != block.instructions.len() {
            let mut new_block = block.clone();
            new_block.instructions = new_instructions;
            tree.replace(block_id, new_block);
        }
    }
}

/// Resolve a value through substitution chains.
fn resolve_value(value: mir::Value, substitutions: &HashMap<mir::Value, mir::Value>) -> mir::Value {
    let mut current = value;
    while let Some(&next) = substitutions.get(&current) {
        if next == current {
            break;
        }
        current = next;
    }
    current
}

/// Update terminator to add block arguments for successors.
fn update_terminator_arguments(
    terminator: &Terminator,
    _block_id: mir::LocalNodeId<mir::Block>,
    block_params: &HashMap<
        (mir::LocalNodeId<mir::Block>, mir::LocalNodeId<mir::Local>),
        mir::Value,
    >,
    value_stacks: &HashMap<mir::LocalNodeId<mir::Local>, Vec<mir::Value>>,
    substitutions: &HashMap<mir::Value, mir::Value>,
) -> Terminator {
    match terminator {
        Terminator::Jump { target, arguments } => {
            let new_args = extend_arguments(
                *target,
                arguments,
                block_params,
                value_stacks,
                substitutions,
            );
            Terminator::Jump {
                target: *target,
                arguments: new_args,
            }
        }
        Terminator::Branch {
            condition,
            then_target,
            then_arguments,
            else_target,
            else_arguments,
        } => {
            let new_then_args = extend_arguments(
                *then_target,
                then_arguments,
                block_params,
                value_stacks,
                substitutions,
            );
            let new_else_args = extend_arguments(
                *else_target,
                else_arguments,
                block_params,
                value_stacks,
                substitutions,
            );
            Terminator::Branch {
                condition: resolve_value(*condition, substitutions),
                then_target: *then_target,
                then_arguments: new_then_args,
                else_target: *else_target,
                else_arguments: new_else_args,
            }
        }
        Terminator::Check {
            condition,
            constraint,
            success,
            failure,
        } => {
            let new_success_args = extend_arguments(
                success.target,
                &success.arguments,
                block_params,
                value_stacks,
                substitutions,
            );
            let new_failure_args = extend_arguments(
                failure.target,
                &failure.arguments,
                block_params,
                value_stacks,
                substitutions,
            );
            let constraint = match constraint {
                mir::CheckConstraint::Bounds {
                    index,
                    length,
                    collection,
                    is_signed,
                } => mir::CheckConstraint::Bounds {
                    index: resolve_value(*index, substitutions),
                    length: resolve_value(*length, substitutions),
                    collection: resolve_value(*collection, substitutions),
                    is_signed: *is_signed,
                },
                mir::CheckConstraint::Null { value } => mir::CheckConstraint::Null {
                    value: resolve_value(*value, substitutions),
                },
                mir::CheckConstraint::DivZero { divisor } => mir::CheckConstraint::DivZero {
                    divisor: resolve_value(*divisor, substitutions),
                },
                mir::CheckConstraint::ShiftRange {
                    value,
                    bit_width,
                    is_signed,
                } => mir::CheckConstraint::ShiftRange {
                    value: resolve_value(*value, substitutions),
                    bit_width: *bit_width,
                    is_signed: *is_signed,
                },
                mir::CheckConstraint::Narrow {
                    value,
                    to_width,
                    is_signed,
                } => mir::CheckConstraint::Narrow {
                    value: resolve_value(*value, substitutions),
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
                    left: resolve_value(*left, substitutions),
                    right: resolve_value(*right, substitutions),
                    is_signed: *is_signed,
                },
            };
            Terminator::Check {
                condition: resolve_value(*condition, substitutions),
                constraint,
                success: mir::CheckTarget {
                    target: success.target,
                    arguments: new_success_args,
                },
                failure: mir::CheckTarget {
                    target: failure.target,
                    arguments: new_failure_args,
                },
            }
        }
        Terminator::Switch {
            value,
            default,
            default_arguments,
            cases,
        } => {
            let new_default_args = extend_arguments(
                *default,
                default_arguments,
                block_params,
                value_stacks,
                substitutions,
            );
            let new_cases: Vec<_> = cases
                .iter()
                .map(|case| mir::SwitchCase {
                    value: case.value,
                    target: case.target,
                    arguments: extend_arguments(
                        case.target,
                        &case.arguments,
                        block_params,
                        value_stacks,
                        substitutions,
                    ),
                })
                .collect();
            Terminator::Switch {
                value: resolve_value(*value, substitutions),
                default: *default,
                default_arguments: new_default_args,
                cases: new_cases,
            }
        }
        Terminator::Yield {
            value,
            resume,
            resume_arguments,
        } => {
            let new_resume_args = extend_arguments(
                *resume,
                resume_arguments,
                block_params,
                value_stacks,
                substitutions,
            );
            Terminator::Yield {
                value: resolve_value(*value, substitutions),
                resume: *resume,
                resume_arguments: new_resume_args,
            }
        }
        Terminator::Return { value } => Terminator::Return {
            value: value.map(|v| resolve_value(v, substitutions)),
        },
        Terminator::Unreachable => Terminator::Unreachable,
        Terminator::TailCall {
            function,
            arguments,
        } => Terminator::TailCall {
            function: *function,
            arguments: arguments
                .iter()
                .map(|v| resolve_value(*v, substitutions))
                .collect(),
        },
        Terminator::TailCallIndirect { callee, arguments } => Terminator::TailCallIndirect {
            callee: resolve_value(*callee, substitutions),
            arguments: arguments
                .iter()
                .map(|v| resolve_value(*v, substitutions))
                .collect(),
        },
    }
}

/// Extend existing arguments with block arguments for a target block.
fn extend_arguments(
    target: mir::LocalNodeId<mir::Block>,
    existing: &[mir::Value],
    block_params: &HashMap<
        (mir::LocalNodeId<mir::Block>, mir::LocalNodeId<mir::Local>),
        mir::Value,
    >,
    value_stacks: &HashMap<mir::LocalNodeId<mir::Local>, Vec<mir::Value>>,
    substitutions: &HashMap<mir::Value, mir::Value>,
) -> Vec<mir::Value> {
    let mut args: Vec<_> = existing
        .iter()
        .map(|&v| resolve_value(v, substitutions))
        .collect();

    // find block params for target block and add arguments
    let mut param_locals: Vec<_> = block_params
        .keys()
        .filter(|(block, _)| *block == target)
        .map(|(_, local)| *local)
        .collect();
    param_locals.sort_by_key(|l| l.id);

    for local in param_locals {
        // get current value for this local
        let value = value_stacks
            .get(&local)
            .and_then(|stack| stack.last())
            .copied()
            .unwrap_or_else(|| {
                panic!(
                    "use of undefined local in block {target:?} \
                     (local was read before being written - this is a bug in the MIR)"
                )
            });

        args.push(resolve_value(value, substitutions));
    }

    args
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Simple case: single local with set then get.
    #[test]
    fn test_promote_simple_local() {
        let input = r#"function @test(v0: i32) -> i32 {
    local0: i32 ; owned, mut
block0(v0: i32):
    local.set local0, v0
    v1 = local.get local0
    return v1
}"#;
        let expected = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    return v0
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&Mem2Reg);
        program.assert_output(expected);
    }

    /// Local accessed across blocks via jump.
    #[test]
    fn test_promote_across_blocks() {
        let input = r#"function @test(v0: i32) -> i32 {
    local0: i32 ; owned, mut
block0(v0: i32):
    local.set local0, v0
    jump block1
block1:
    v1 = local.get local0
    return v1
}"#;
        let expected = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    jump block1
block1:
    return v0
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&Mem2Reg);
        program.assert_output(expected);
    }

    /// Diamond CFG with block parameter needed at join point.
    #[test]
    fn test_promote_in_diamond_cfg() {
        let input = r#"function @test(v0: bool, v99: i32) -> i32 {
    local0: i32 ; owned, mut
block0(v0: bool, v99: i32):
    local.set local0, v99
    branch v0, block1, block2
block1:
    v1 = iconst 1i32
    local.set local0, v1
    jump block3
block2:
    v2 = iconst 2i32
    local.set local0, v2
    jump block3
block3:
    v3 = local.get local0
    return v3
}"#;
        // block3 needs a block parameter for the different values from block1/block2
        // v100 is the new block parameter value allocated by the pass
        let expected = r#"function @test(v0: bool, v99: i32) -> i32 {
block0(v0: bool, v99: i32):
    branch v0, block1, block2
block1:
    v1 = iconst 1i32
    jump block3(v1)
block2:
    v2 = iconst 2i32
    jump block3(v2)
block3(v100: i32):
    return v100
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&Mem2Reg);
        program.assert_output(expected);
    }

    /// Multiple locals, all promotable.
    #[test]
    fn test_promote_multiple_locals() {
        let input = r#"function @test(v0: i32, v1: i32) -> i32 {
    local0: i32 ; owned, mut
    local1: i32 ; owned, mut
block0(v0: i32, v1: i32):
    local.set local0, v0
    local.set local1, v1
    v2 = local.get local0
    v3 = local.get local1
    v4 = iadd v2, v3
    return v4
}"#;
        let expected = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v4 = iadd v0, v1
    return v4
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&Mem2Reg);
        program.assert_output(expected);
    }

    /// No locals to promote.
    #[test]
    fn test_preserve_without_locals() {
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    return v0
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&Mem2Reg);
        program.assert_unchanged(input);
    }

    /// Local only read (never written) - should panic.
    #[test]
    #[should_panic(expected = "use of undefined local")]
    fn test_panic_on_undefined_read() {
        let input = r#"function @test() -> i32 {
    local0: i32 ; owned, mut
block0:
    v0 = local.get local0
    return v0
}"#;
        // reading a local before writing is a bug in the MIR
        let mut program = TestProgram::new(input);
        program.run_pass(&Mem2Reg);
    }

    /// Loop with local - block parameter needed at loop header.
    #[test]
    fn test_promote_in_loop() {
        let input = r#"function @test(v0: i32) -> i32 {
    local0: i32 ; owned, mut
block0(v0: i32):
    v1 = iconst 0i32
    local.set local0, v1
    jump block1
block1:
    v2 = local.get local0
    v3 = icmp_slt v2, v0
    branch v3, block2, block3
block2:
    v4 = iconst 1i32
    v5 = iadd v2, v4
    local.set local0, v5
    jump block1
block3:
    v6 = local.get local0
    return v6
}"#;
        // block1 is at dominance frontier (join point from block0 and block2)
        // v0-v6 exist in input -> next_value_id = 7
        let expected = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 0i32
    jump block1(v1)
block1(v7: i32):
    v3 = icmp_slt v7, v0
    branch v3, block2, block3
block2:
    v4 = iconst 1i32
    v5 = iadd v7, v4
    jump block1(v5)
block3:
    return v7
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&Mem2Reg);
        program.assert_output(expected);
    }

    /// Multiple definitions in the same block.
    #[test]
    fn test_promote_with_multiple_defs() {
        let input = r#"function @test(v0: i32) -> i32 {
    local0: i32 ; owned, mut
block0(v0: i32):
    local.set local0, v0
    v1 = iconst 42i32
    local.set local0, v1
    v2 = local.get local0
    return v2
}"#;
        let expected = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 42i32
    return v1
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&Mem2Reg);
        program.assert_output(expected);
    }

    /// Existing block parameters should be preserved.
    #[test]
    fn test_preserve_existing_block_params() {
        let input = r#"function @test(v0: i32, v1: i32) -> i32 {
    local0: i32 ; owned, mut
block0(v0: i32, v1: i32):
    local.set local0, v0
    jump block1(v1)
block1(v2: i32):
    v3 = local.get local0
    v4 = iadd v2, v3
    return v4
}"#;
        // block1 keeps its existing parameter, local0 value is dominated by entry
        let expected = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    jump block1(v1)
block1(v2: i32):
    v4 = iadd v2, v0
    return v4
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&Mem2Reg);
        program.assert_output(expected);
    }

    /// Switch terminator with local.
    #[test]
    fn test_promote_with_switch() {
        let input = r#"function @test(v0: i32) -> i32 {
    local0: i32 ; owned, mut
block0(v0: i32):
    v1 = iconst 10i32
    local.set local0, v1
    switch v0, block3, 0 => block1, 1 => block2
block1:
    v2 = iconst 100i32
    local.set local0, v2
    jump block3
block2:
    v3 = iconst 200i32
    local.set local0, v3
    jump block3
block3:
    v4 = local.get local0
    return v4
}"#;
        // block3 is join point with 3 predecessors (block0, block1, block2)
        // v0-v4 exist in input -> next_value_id = 5
        // new block parameter is v5
        let expected = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 10i32
    switch v0, block3(v1), 0 => block1, 1 => block2
block1:
    v2 = iconst 100i32
    jump block3(v2)
block2:
    v3 = iconst 200i32
    jump block3(v3)
block3(v5: i32):
    return v5
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&Mem2Reg);
        program.assert_output(expected);
    }

    /// Call arguments are rewritten after local promotion.
    #[test]
    fn test_promote_call_arguments() {
        let input = r#"extern function @sink(i32) -> void
function @test() -> void {
    local0: i32 ; owned, mut
block0:
    v0 = iconst 7i32
    local.set local0, v0
    v1 = local.get local0
    call @sink(v1)
    return
}"#;
        let expected = r#"extern function @sink(i32) -> void
function @test() -> void {
block0:
    v0 = iconst 7i32
    call @sink(v0)
    return
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&Mem2Reg);
        program.assert_output(expected);
    }
}
