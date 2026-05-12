use std::collections::{HashMap, HashSet, VecDeque};

use crate::declare_mir_pass;
use destack_mir as mir;
use mir::{Instruction, Terminator};

use crate::common::mir::analysis::{ControlFlowGraph, DominatorTree};
use crate::common::mir::{
    compute_dominance_frontiers, instruction_substitute_uses_in_tree,
    remap_instruction_memory_accesses, terminator_substitute_uses,
};
use crate::optimize::{AnalysisPreservation, FunctionPass, PipelineContext};

declare_mir_pass! {
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
    /// function before(v0: int32): int32 {
    ///     local local0: int32, owned
    /// b0(v0: int32):
    ///     local.set local0, v0
    ///     jump b1
    /// b1:
    ///     v1 = local.get local0
    ///     return v1
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function after(v0: int32): int32 {
    /// b0(v0: int32):
    ///     jump b1(v0)
    /// b1(v1: int32):
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
        tree: &mut mir::Tree,
        ctx: &PipelineContext<'_>,
    ) -> AnalysisPreservation {
        // skip functions without locals
        if function.locals.is_empty() {
            return AnalysisPreservation::all();
        }

        // get analyses
        let (cfg, domtree) = {
            let analyses = ctx.function_analyses(function, tree);
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
    tree: &mut mir::Tree,
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
    let frontiers = compute_dominance_frontiers(&function.blocks, cfg, domtree);

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
/// Locals whose address is taken are not promotable.
fn find_promotable_locals(
    function: &mir::Function,
    tree: &mir::Tree,
) -> HashMap<mir::LocalNodeId<mir::Local>, PromotableLocal> {
    // collect locals with address taken
    let mut address_taken = HashSet::new();
    for &block_id in &function.blocks {
        let block = tree.get(block_id);
        for &instruction_id in &block.instructions {
            if let Instruction::LocalAddr { local, .. } = tree.get(instruction_id) {
                let Some(local) = local.local() else {
                    continue;
                };

                address_taken.insert(local);
            }
        }
    }

    function
        .locals
        .iter()
        .filter(|local_id| !address_taken.contains(local_id))
        .filter_map(|&local_id| {
            let local = tree.get(local_id);
            let ty = local.ty.ty()?;

            Some((local_id, PromotableLocal { ty }))
        })
        .collect()
}

/// Find which blocks contain definitions (LocalSet) for each promotable local.
fn find_definition_blocks(
    promotable: &HashMap<mir::LocalNodeId<mir::Local>, PromotableLocal>,
    function: &mir::Function,
    tree: &mir::Tree,
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
                && let Some(local) = local.local()
                && promotable.contains_key(&local)
            {
                def_blocks.get_mut(&local).unwrap().insert(block_id);
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
    tree: &mir::Tree,
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
                Instruction::LocalGet { local, .. }
                    if local
                        .local()
                        .is_some_and(|local| promotable.contains_key(&local)) =>
                {
                    let local = local.local().unwrap();
                    let was_defined = seen_defs.contains(&local);
                    if !was_defined {
                        uses.insert(local);
                    }
                }
                Instruction::LocalSet { local, .. }
                    if local
                        .local()
                        .is_some_and(|local| promotable.contains_key(&local)) =>
                {
                    let local = local.local().unwrap();
                    seen_defs.insert(local);
                    defs.insert(local);
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
            let terminator = tree.get(block.terminator);

            // live_out is union of successor live_in sets
            let mut new_live_out: HashSet<mir::LocalNodeId<mir::Local>> = HashSet::new();
            for successor in terminator.successors() {
                let Some(successor) = successor.block() else {
                    continue;
                };

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
    tree: &mut mir::Tree,
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
            let param_value = function.next_typed_value(ty);

            block.parameters.push(mir::Parameter {
                value: param_value.into(),
                ty: ty.into(),
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
    tree: &mut mir::Tree,
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
                    let Some(destination) = destination.value() else {
                        continue;
                    };
                    let Some(local) = local.local() else {
                        continue;
                    };

                    if promotable.contains_key(&local) {
                        // replace with current value
                        let current_value =
                            value_stacks[&local].last().copied().unwrap_or_else(|| {
                                panic!(
                                    "use of undefined local in block {block_id:?} \
                                 (local was read before being written, this is invalid MIR)"
                                )
                            });
                        substitutions.insert(destination, current_value);
                        instructions_to_remove.insert(instruction_id);
                    }
                }
                Instruction::LocalSet { local, value } => {
                    let Some(local) = local.local() else {
                        continue;
                    };
                    let Some(value) = value.value() else {
                        continue;
                    };

                    if promotable.contains_key(&local) {
                        // apply any pending substitutions to the value
                        let actual_value = resolve_value(value, &substitutions);
                        value_stacks.get_mut(&local).unwrap().push(actual_value);
                        instructions_to_remove.insert(instruction_id);
                    }
                }
                _ => {}
            }
        }

        // update terminator to pass block arguments to successors
        let block = tree.get(block_id);
        let terminator = tree.get(block.terminator).clone();
        let new_terminator = update_terminator_arguments(
            &terminator,
            block_id,
            block_params,
            &value_stacks,
            &substitutions,
        );

        if new_terminator != terminator {
            let new_block = block.clone();
            tree.replace(block.terminator, new_terminator);
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
                remap_instruction_memory_accesses(tree, instruction_id, &substitutions);
            }
        }

        // apply substitutions to terminator
        let block = tree.get(block_id);
        let terminator = tree.get(block.terminator).clone();
        let new_terminator = terminator_substitute_uses(&terminator, &substitutions);
        if new_terminator != terminator {
            let new_block = block.clone();
            tree.replace(block.terminator, new_terminator);
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

/// Resolve a value reference through substitution chains when concrete.
fn remap_value_reference(
    value: mir::ValueReference,
    substitutions: &HashMap<mir::Value, mir::Value>,
) -> mir::ValueReference {
    let Some(value) = value.value() else {
        return value;
    };

    resolve_value(value, substitutions).into()
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
        Terminator::Error => {
            panic!("recovered MIR terminator reached optimizer");
        }
        Terminator::Jump { target } => {
            let new_args = extend_arguments(
                target.block,
                &target.arguments,
                block_params,
                value_stacks,
                substitutions,
            );
            Terminator::Jump {
                target: mir::BlockTarget {
                    block: target.block,
                    arguments: new_args,
                },
            }
        }
        Terminator::Branch {
            condition,
            then_target,
            else_target,
        } => {
            let new_then_args = extend_arguments(
                then_target.block,
                &then_target.arguments,
                block_params,
                value_stacks,
                substitutions,
            );
            let new_else_args = extend_arguments(
                else_target.block,
                &else_target.arguments,
                block_params,
                value_stacks,
                substitutions,
            );
            Terminator::Branch {
                condition: remap_value_reference(*condition, substitutions),
                then_target: mir::BlockTarget {
                    block: then_target.block,
                    arguments: new_then_args,
                },
                else_target: mir::BlockTarget {
                    block: else_target.block,
                    arguments: new_else_args,
                },
            }
        }
        Terminator::Check {
            constraint,
            success,
            failure,
        } => {
            let new_success_args = extend_arguments(
                success.block,
                &success.arguments,
                block_params,
                value_stacks,
                substitutions,
            );
            let new_failure_args = extend_arguments(
                failure.block,
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
                    index: remap_value_reference(*index, substitutions),
                    length: remap_value_reference(*length, substitutions),
                    collection: remap_value_reference(*collection, substitutions),
                    is_signed: *is_signed,
                },
                mir::CheckConstraint::Null { value } => mir::CheckConstraint::Null {
                    value: remap_value_reference(*value, substitutions),
                },
                mir::CheckConstraint::DivZero { divisor } => mir::CheckConstraint::DivZero {
                    divisor: remap_value_reference(*divisor, substitutions),
                },
                mir::CheckConstraint::ShiftRange {
                    value,
                    bit_width,
                    is_signed,
                } => mir::CheckConstraint::ShiftRange {
                    value: remap_value_reference(*value, substitutions),
                    bit_width: *bit_width,
                    is_signed: *is_signed,
                },
                mir::CheckConstraint::Narrow {
                    value,
                    to_width,
                    is_signed,
                } => mir::CheckConstraint::Narrow {
                    value: remap_value_reference(*value, substitutions),
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
                    left: remap_value_reference(*left, substitutions),
                    right: remap_value_reference(*right, substitutions),
                    is_signed: *is_signed,
                },
                mir::CheckConstraint::Type { value, expected } => mir::CheckConstraint::Type {
                    value: remap_value_reference(*value, substitutions),
                    expected: *expected,
                },
                mir::CheckConstraint::Union { value, expected } => mir::CheckConstraint::Union {
                    value: remap_value_reference(*value, substitutions),
                    expected: *expected,
                },
                mir::CheckConstraint::ReceiverType { receiver, expected } => {
                    mir::CheckConstraint::ReceiverType {
                        receiver: remap_value_reference(*receiver, substitutions),
                        expected: *expected,
                    }
                }
                mir::CheckConstraint::Implements { receiver, expected } => {
                    mir::CheckConstraint::Implements {
                        receiver: remap_value_reference(*receiver, substitutions),
                        expected: *expected,
                    }
                }
            };
            Terminator::Check {
                constraint,
                success: mir::BlockTarget {
                    block: success.block,
                    arguments: new_success_args,
                },
                failure: mir::BlockTarget {
                    block: failure.block,
                    arguments: new_failure_args,
                },
            }
        }
        Terminator::Switch {
            value,
            default,
            cases,
        } => {
            let new_default_args = extend_arguments(
                default.block,
                &default.arguments,
                block_params,
                value_stacks,
                substitutions,
            );
            let new_cases: Vec<_> = cases
                .iter()
                .map(|case| mir::SwitchCase {
                    value: case.value,
                    target: mir::BlockTarget {
                        block: case.target.block,
                        arguments: extend_arguments(
                            case.target.block,
                            &case.target.arguments,
                            block_params,
                            value_stacks,
                            substitutions,
                        ),
                    },
                })
                .collect();
            Terminator::Switch {
                value: remap_value_reference(*value, substitutions),
                default: mir::BlockTarget {
                    block: default.block,
                    arguments: new_default_args,
                },
                cases: new_cases,
            }
        }
        Terminator::Yield { value, resume } => {
            let new_resume_args = extend_arguments(
                resume.block,
                &resume.arguments,
                block_params,
                value_stacks,
                substitutions,
            );
            Terminator::Yield {
                value: remap_value_reference(*value, substitutions),
                resume: mir::BlockTarget {
                    block: resume.block,
                    arguments: new_resume_args,
                },
            }
        }
        Terminator::Call {
            function,
            call,
            target,
        } => Terminator::Call {
            function: *function,
            call: mir::Call {
                arguments: call
                    .arguments
                    .iter()
                    .map(|value| remap_value_reference(*value, substitutions))
                    .collect(),
                ..call.clone()
            },
            target: mir::BlockTarget {
                block: target.block,
                arguments: extend_arguments(
                    target.block,
                    &target.arguments,
                    block_params,
                    value_stacks,
                    substitutions,
                ),
            },
        },
        Terminator::CallIndirect {
            callee,
            call,
            target,
        } => Terminator::CallIndirect {
            callee: remap_value_reference(*callee, substitutions),
            call: mir::Call {
                arguments: call
                    .arguments
                    .iter()
                    .map(|value| remap_value_reference(*value, substitutions))
                    .collect(),
                ..call.clone()
            },
            target: mir::BlockTarget {
                block: target.block,
                arguments: extend_arguments(
                    target.block,
                    &target.arguments,
                    block_params,
                    value_stacks,
                    substitutions,
                ),
            },
        },
        Terminator::CallClass {
            receiver,
            call,
            declaring_type,
            slot,
            declared_target,
            target,
        } => Terminator::CallClass {
            receiver: remap_value_reference(*receiver, substitutions),
            call: mir::Call {
                arguments: call
                    .arguments
                    .iter()
                    .map(|value| remap_value_reference(*value, substitutions))
                    .collect(),
                ..call.clone()
            },
            declaring_type: *declaring_type,
            slot: *slot,
            declared_target: *declared_target,
            target: mir::BlockTarget {
                block: target.block,
                arguments: extend_arguments(
                    target.block,
                    &target.arguments,
                    block_params,
                    value_stacks,
                    substitutions,
                ),
            },
        },
        Terminator::CallInterface {
            receiver,
            call,
            declaring_type,
            slot,
            target,
        } => Terminator::CallInterface {
            receiver: remap_value_reference(*receiver, substitutions),
            call: mir::Call {
                arguments: call
                    .arguments
                    .iter()
                    .map(|value| remap_value_reference(*value, substitutions))
                    .collect(),
                ..call.clone()
            },
            declaring_type: *declaring_type,
            slot: *slot,
            target: mir::BlockTarget {
                block: target.block,
                arguments: extend_arguments(
                    target.block,
                    &target.arguments,
                    block_params,
                    value_stacks,
                    substitutions,
                ),
            },
        },
        Terminator::Return { value } => Terminator::Return {
            value: value.map(|value| remap_value_reference(value, substitutions)),
        },
        Terminator::Trap { kind, payload } => Terminator::Trap {
            kind: *kind,
            payload: payload.map(|value| remap_value_reference(value, substitutions)),
        },
        Terminator::Unreachable => Terminator::Unreachable,
        Terminator::TailCall { function, call } => Terminator::TailCall {
            function: *function,
            call: mir::Call {
                arguments: call
                    .arguments
                    .iter()
                    .map(|value| remap_value_reference(*value, substitutions))
                    .collect(),
                ..call.clone()
            },
        },
        Terminator::TailCallClass {
            receiver,
            call,
            declaring_type,
            slot,
            declared_target,
        } => Terminator::TailCallClass {
            receiver: remap_value_reference(*receiver, substitutions),
            call: mir::Call {
                arguments: call
                    .arguments
                    .iter()
                    .map(|value| remap_value_reference(*value, substitutions))
                    .collect(),
                ..call.clone()
            },
            declaring_type: *declaring_type,
            slot: *slot,
            declared_target: *declared_target,
        },
        Terminator::TailCallInterface {
            receiver,
            call,
            declaring_type,
            slot,
        } => Terminator::TailCallInterface {
            receiver: remap_value_reference(*receiver, substitutions),
            call: mir::Call {
                arguments: call
                    .arguments
                    .iter()
                    .map(|value| remap_value_reference(*value, substitutions))
                    .collect(),
                ..call.clone()
            },
            declaring_type: *declaring_type,
            slot: *slot,
        },
        Terminator::TailCallIndirect { callee, call } => Terminator::TailCallIndirect {
            callee: remap_value_reference(*callee, substitutions),
            call: mir::Call {
                arguments: call
                    .arguments
                    .iter()
                    .map(|value| remap_value_reference(*value, substitutions))
                    .collect(),
                ..call.clone()
            },
        },
    }
}

/// Extend existing arguments with block arguments for a target block.
fn extend_arguments(
    target: mir::BlockReference,
    existing: &[mir::ValueReference],
    block_params: &HashMap<
        (mir::LocalNodeId<mir::Block>, mir::LocalNodeId<mir::Local>),
        mir::Value,
    >,
    value_stacks: &HashMap<mir::LocalNodeId<mir::Local>, Vec<mir::Value>>,
    substitutions: &HashMap<mir::Value, mir::Value>,
) -> Vec<mir::ValueReference> {
    let Some(target) = target.block() else {
        return existing.to_vec();
    };

    let mut args: Vec<_> = existing
        .iter()
        .copied()
        .filter_map(|value| value.value())
        .map(|value| resolve_value(value, substitutions).into())
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

        args.push(resolve_value(value, substitutions).into());
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
        let input = r#"
function test(v0: int32): int32 {
    local local0: int32, owned
b0(v0: int32):
    local.set local0, v0
    v1: int32 = local.get local0
    return v1
}"#;
        let expected = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    return v0
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&Mem2Reg);
        test.assert_output(expected);
    }

    /// Local accessed across blocks via jump.
    #[test]
    fn test_promote_across_blocks() {
        let input = r#"
function test(v0: int32): int32 {
    local local0: int32, owned
b0(v0: int32):
    local.set local0, v0
    jump b1
b1:
    v1: int32 = local.get local0
    return v1
}"#;
        let expected = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    jump b1
b1:
    return v0
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&Mem2Reg);
        test.assert_output(expected);
    }

    /// Diamond CFG with block parameter needed at join point.
    #[test]
    fn test_promote_in_diamond_cfg() {
        let input = r#"
function test(v0: boolean, v1: int32): int32 {
    local local0: int32, owned
b0(v0: boolean, v1: int32):
    local.set local0, v1
    branch v0, b1, b2
b1:
    v2: int32 = 1int32
    local.set local0, v2
    jump b3
b2:
    v3: int32 = 2int32
    local.set local0, v3
    jump b3
b3:
    v4: int32 = local.get local0
    return v4
}"#;
        // block3 needs a block parameter for the different values from block1/block2
        // v4 is the new block parameter value allocated by the pass
        let expected = r#"
function test(v0: boolean, v1: int32): int32 {
b0(v0: boolean, v1: int32):
    branch v0, b1, b2
b1:
    v2: int32 = 1int32
    jump b3(v2)
b2:
    v3: int32 = 2int32
    jump b3(v3)
b3(v4: int32):
    return v4
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&Mem2Reg);
        test.assert_output(expected);
    }

    /// Multiple locals, all promotable.
    #[test]
    fn test_promote_multiple_locals() {
        let input = r#"
function test(v0: int32, v1: int32): int32 {
    local local0: int32, owned
    local local1: int32, owned
b0(v0: int32, v1: int32):
    local.set local0, v0
    local.set local1, v1
    v2: int32 = local.get local0
    v3: int32 = local.get local1
    v4: int32 = int.add v2, v3
    return v4
}"#;
        let expected = r#"
function test(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&Mem2Reg);
        test.assert_output(expected);
    }

    /// No locals to promote.
    #[test]
    fn test_preserve_without_locals() {
        let input = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    return v0
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&Mem2Reg);
        test.assert_unchanged(input);
    }

    /// Local only read (never written) - should panic.
    #[test]
    #[should_panic(expected = "use of undefined local")]
    fn test_panic_on_undefined_read() {
        let input = r#"
function test(): int32 {
    local local0: int32, owned
b0:
    v0: int32 = local.get local0
    return v0
}"#;
        // reading a local before writing is a bug in the MIR
        let mut test = TestProgram::new(input);
        test.run_pass(&Mem2Reg);
    }

    /// Loop with local - block parameter needed at loop header.
    #[test]
    fn test_promote_in_loop() {
        let input = r#"
function test(v0: int32): int32 {
    local local0: int32, owned
b0(v0: int32):
    v1: int32 = 0int32
    local.set local0, v1
    jump b1
b1:
    v2: int32 = local.get local0
    v3: boolean = int.lt.s v2, v0
    branch v3, b2, b3
b2:
    v4: int32 = 1int32
    v5: int32 = int.add v2, v4
    local.set local0, v5
    jump b1
b3:
    v6: int32 = local.get local0
    return v6
}"#;
        // b1 is at dominance frontier (join point from b0 and b2)
        // v0-v6 exist in input -> next_value_id = 7
        let expected = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 0int32
    jump b1(v1)
b1(v2: int32):
    v3: boolean = int.lt.s v2, v0
    branch v3, b2, b3
b2:
    v4: int32 = 1int32
    v5: int32 = int.add v2, v4
    jump b1(v5)
b3:
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&Mem2Reg);
        test.assert_output(expected);
    }

    /// Multiple definitions in the same block.
    #[test]
    fn test_promote_with_multiple_defs() {
        let input = r#"
function test(v0: int32): int32 {
    local local0: int32, owned
b0(v0: int32):
    local.set local0, v0
    v1: int32 = 42int32
    local.set local0, v1
    v2: int32 = local.get local0
    return v2
}"#;
        let expected = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 42int32
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&Mem2Reg);
        test.assert_output(expected);
    }

    /// Existing block parameters should be preserved.
    #[test]
    fn test_preserve_existing_block_params() {
        let input = r#"
function test(v0: int32, v1: int32): int32 {
    local local0: int32, owned
b0(v0: int32, v1: int32):
    local.set local0, v0
    jump b1(v1)
b1(v2: int32):
    v3: int32 = local.get local0
    v4: int32 = int.add v2, v3
    return v4
}"#;
        // block1 keeps its existing parameter, local0 value is dominated by entry
        let expected = r#"
function test(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    jump b1(v1)
b1(v2: int32):
    v3: int32 = int.add v2, v0
    return v3
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&Mem2Reg);
        test.assert_output(expected);
    }

    /// Switch terminator with local.
    #[test]
    fn test_promote_with_switch() {
        let input = r#"
function test(v0: int32): int32 {
    local local0: int32, owned
b0(v0: int32):
    v1: int32 = 10int32
    local.set local0, v1
    switch v0, b3, 0 => b1, 1 => b2
b1:
    v2: int32 = 100int32
    local.set local0, v2
    jump b3
b2:
    v3: int32 = 200int32
    local.set local0, v3
    jump b3
b3:
    v4: int32 = local.get local0
    return v4
}"#;
        // b3 is join point with 3 predecessors (b0, b1, b2)
        // v0-v4 exist in input -> next_value_id = 5
        // new block parameter is v5
        let expected = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 10int32
    switch v0, b3(v1), 0 => b1, 1 => b2
b1:
    v2: int32 = 100int32
    jump b3(v2)
b2:
    v3: int32 = 200int32
    jump b3(v3)
b3(v4: int32):
    return v4
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&Mem2Reg);
        test.assert_output(expected);
    }

    /// Call arguments are rewritten after local promotion.
    #[test]
    fn test_promote_call_arguments() {
        let input = r#"
extern function sink(int32): void
function test(): void {
    local local0: int32, owned
b0:
    v0: int32 = 7int32
    local.set local0, v0
    v1: int32 = local.get local0
    call sink(v1): (int32) -> void
    return
}"#;
        let expected = r#"
extern function sink(int32): void
function test(): void {
b0:
    v0: int32 = 7int32
    call sink(v0): (int32) -> void
    return
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&Mem2Reg);
        test.assert_output(expected);
    }
}
