use std::collections::{HashMap, HashSet, VecDeque};

use crate::optimize::declare_pass;
use destack_mir as mir;

use crate::optimize::{FunctionPass, PipelineContext};
use destack_mir::{
    ControlFlowGraph, DominatorTree, Mutation, compute_dominance_frontiers,
    instruction_substitute_uses_in_tree, remap_instruction_memory_accesses,
    terminator_substitute_uses,
};

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
    /// function before(v0: int32): int32 {
    ///     local l0: int32
    /// b0(v0: int32):
    ///     local.set l0, v0
    ///     jump b1
    /// b1:
    ///     v1 = local.get l0
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
        _ctx: &PipelineContext<'_>,
        analyses: &mir::FunctionAnalyses,
    ) -> Mutation {
        // skip functions without locals
        if function.locals.is_empty() {
            return Mutation::NONE;
        }

        // get analyses
        let (cfg, domtree) = {
            (
                analyses.get::<ControlFlowGraph>(function, tree).clone(),
                analyses.get::<DominatorTree>(function, tree).clone(),
            )
        };

        // run mem2reg
        let changed = run_mem2reg(function, tree, &cfg, &domtree);

        // report what this pass changed
        if changed {
            Mutation::VALUES
        } else {
            Mutation::NONE
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
            if let mir::Instruction::LocalAddr { local, .. } = tree.get(instruction_id) {
                address_taken.insert(*local);
            }
        }
    }

    function
        .locals
        .iter()
        .filter(|local_id| !address_taken.contains(local_id))
        .map(|&local_id| {
            let local = tree.get(local_id);
            let ty = local.ty;

            (local_id, PromotableLocal { ty })
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
            if let mir::Instruction::LocalSet { local, .. } = instruction
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
                mir::Instruction::LocalGet { local, .. } if promotable.contains_key(local) => {
                    let was_defined = seen_defs.contains(local);
                    if !was_defined {
                        uses.insert(*local);
                    }
                }
                mir::Instruction::LocalSet { local, .. } if promotable.contains_key(local) => {
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
            let terminator = tree.get(block.terminator);

            // live_out is union of successor live_in sets
            let mut new_live_out: HashSet<mir::LocalNodeId<mir::Local>> = HashSet::new();
            for successor in tree.terminator_successors(terminator) {
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
/// Returns a mapping from (block, local) => parameter value.
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

            block.parameters.push(mir::BlockParameter {
                value: param_value.into(),
                ty: ty.into(),
            });

            block_params.insert((block_id, local), param_value);
        }

        tree.set(block_id, block);
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
                mir::Instruction::LocalGet { destination, local } => {
                    let destination = *destination;

                    if promotable.contains_key(local) {
                        // replace with current value
                        let current_value =
                            value_stacks[local].last().copied().unwrap_or_else(|| {
                                panic!(
                                    "use of undefined local in block {block_id:?} \
                                 (local was read before being written, this is invalid MIR)"
                                )
                            });
                        substitutions.insert(destination, current_value);
                        instructions_to_remove.insert(instruction_id);
                    }
                }
                mir::Instruction::LocalSet { local, value } => {
                    let value = *value;

                    if promotable.contains_key(local) {
                        // apply any pending substitutions to the value
                        let actual_value = resolve_value(value, &substitutions);
                        value_stacks.get_mut(local).unwrap().push(actual_value);
                        instructions_to_remove.insert(instruction_id);
                    }
                }
                _ => {}
            }
        }

        // update terminator to pass block arguments to successors
        let terminator_id = tree.get(block_id).terminator;
        let terminator = tree.get(terminator_id).clone();
        let new_terminator = update_terminator_arguments(
            tree,
            &terminator,
            block_id,
            block_params,
            &value_stacks,
            &substitutions,
        );

        if new_terminator != terminator {
            tree.set(terminator_id, new_terminator);
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
                tree.set(instruction_id, new_instruction);
                remap_instruction_memory_accesses(tree, instruction_id, &substitutions);
            }
        }

        // apply substitutions to terminator
        let terminator_id = tree.get(block_id).terminator;
        let terminator = tree.get(terminator_id).clone();
        let new_terminator = terminator_substitute_uses(tree, &terminator, &substitutions);
        if new_terminator != terminator {
            tree.set(terminator_id, new_terminator);
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
            tree.set(block_id, new_block);
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
    value: mir::Value,
    substitutions: &HashMap<mir::Value, mir::Value>,
) -> mir::Value {
    resolve_value(value, substitutions)
}

/// Update terminator to add block arguments for successors.
fn update_terminator_arguments(
    tree: &mut mir::Tree,
    terminator: &mir::Terminator,
    _block_id: mir::LocalNodeId<mir::Block>,
    block_params: &HashMap<
        (mir::LocalNodeId<mir::Block>, mir::LocalNodeId<mir::Local>),
        mir::Value,
    >,
    value_stacks: &HashMap<mir::LocalNodeId<mir::Local>, Vec<mir::Value>>,
    substitutions: &HashMap<mir::Value, mir::Value>,
) -> mir::Terminator {
    match terminator {
        mir::Terminator::Error => {
            panic!("recovered MIR terminator reached optimizer");
        }
        mir::Terminator::Jump { target } => {
            let new_args = extend_arguments(
                target.block,
                tree.get_values(target.arguments),
                block_params,
                value_stacks,
                substitutions,
            );
            mir::Terminator::Jump {
                target: mir::BlockTarget::new(target.block, tree.add_values(&new_args)),
            }
        }
        mir::Terminator::Branch {
            condition,
            then_target,
            else_target,
        } => {
            let new_then_args = extend_arguments(
                then_target.block,
                tree.get_values(then_target.arguments),
                block_params,
                value_stacks,
                substitutions,
            );
            let new_else_args = extend_arguments(
                else_target.block,
                tree.get_values(else_target.arguments),
                block_params,
                value_stacks,
                substitutions,
            );
            mir::Terminator::Branch {
                condition: remap_value_reference(*condition, substitutions),
                then_target: mir::BlockTarget::new(
                    then_target.block,
                    tree.add_values(&new_then_args),
                ),
                else_target: mir::BlockTarget::new(
                    else_target.block,
                    tree.add_values(&new_else_args),
                ),
            }
        }
        mir::Terminator::Check {
            constraint,
            success,
            failure,
        } => {
            let new_success_args = extend_arguments(
                success.block,
                tree.get_values(success.arguments),
                block_params,
                value_stacks,
                substitutions,
            );
            let new_failure_args = extend_arguments(
                failure.block,
                tree.get_values(failure.arguments),
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
                mir::CheckConstraint::Variant { value, expected } => {
                    mir::CheckConstraint::Variant {
                        value: remap_value_reference(*value, substitutions),
                        expected: expected.clone(),
                    }
                }
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
            mir::Terminator::Check {
                constraint,
                success: mir::BlockTarget::new(success.block, tree.add_values(&new_success_args)),
                failure: mir::BlockTarget::new(failure.block, tree.add_values(&new_failure_args)),
            }
        }
        mir::Terminator::NewZeroedTry {
            layout,
            success,
            failure,
        } => mir::Terminator::NewZeroedTry {
            layout: *layout,
            success: extend_target(tree, success, block_params, value_stacks, substitutions),
            failure: extend_target(tree, failure, block_params, value_stacks, substitutions),
        },
        mir::Terminator::NewUninitTry {
            layout,
            success,
            failure,
        } => mir::Terminator::NewUninitTry {
            layout: *layout,
            success: extend_target(tree, success, block_params, value_stacks, substitutions),
            failure: extend_target(tree, failure, block_params, value_stacks, substitutions),
        },
        mir::Terminator::NewSliceZeroedTry {
            element,
            length,
            success,
            failure,
        } => mir::Terminator::NewSliceZeroedTry {
            element: *element,
            length: remap_value_reference(*length, substitutions),
            success: extend_target(tree, success, block_params, value_stacks, substitutions),
            failure: extend_target(tree, failure, block_params, value_stacks, substitutions),
        },
        mir::Terminator::NewSliceUninitTry {
            element,
            length,
            success,
            failure,
        } => mir::Terminator::NewSliceUninitTry {
            element: *element,
            length: remap_value_reference(*length, substitutions),
            success: extend_target(tree, success, block_params, value_stacks, substitutions),
            failure: extend_target(tree, failure, block_params, value_stacks, substitutions),
        },
        mir::Terminator::Switch {
            value,
            default,
            cases,
        } => {
            let new_default_args = extend_arguments(
                default.block,
                tree.get_values(default.arguments),
                block_params,
                value_stacks,
                substitutions,
            );
            let cases = tree.get_switch_cases(*cases).to_vec();
            let new_cases: Vec<_> = cases
                .iter()
                .map(|case| mir::SwitchCase {
                    value: case.value,
                    target: extend_target(
                        tree,
                        &case.target,
                        block_params,
                        value_stacks,
                        substitutions,
                    ),
                })
                .collect();
            mir::Terminator::Switch {
                value: remap_value_reference(*value, substitutions),
                default: mir::BlockTarget::new(default.block, tree.add_values(&new_default_args)),
                cases: tree.add_switch_cases(&new_cases),
            }
        }
        mir::Terminator::Yield {
            value,
            resume,
            unwind,
        } => {
            let new_resume_args = extend_arguments(
                resume.block,
                tree.get_values(resume.arguments),
                block_params,
                value_stacks,
                substitutions,
            );
            mir::Terminator::Yield {
                value: remap_value_reference(*value, substitutions),
                resume: mir::BlockTarget::new(resume.block, tree.add_values(&new_resume_args)),
                unwind: unwind.as_ref().map(|unwind| {
                    extend_target(tree, unwind, block_params, value_stacks, substitutions)
                }),
            }
        }
        mir::Terminator::Call {
            function,
            call,
            target,
            unwind,
        } => mir::Terminator::Call {
            function: *function,
            call: remap_call(tree, call, substitutions),
            target: extend_target(tree, target, block_params, value_stacks, substitutions),
            unwind: unwind.as_ref().map(|unwind| {
                extend_target(tree, unwind, block_params, value_stacks, substitutions)
            }),
        },
        mir::Terminator::CallIndirect {
            callee,
            call,
            target,
            unwind,
        } => mir::Terminator::CallIndirect {
            callee: remap_value_reference(*callee, substitutions),
            call: remap_call(tree, call, substitutions),
            target: extend_target(tree, target, block_params, value_stacks, substitutions),
            unwind: unwind.as_ref().map(|unwind| {
                extend_target(tree, unwind, block_params, value_stacks, substitutions)
            }),
        },
        mir::Terminator::CallVirtual {
            receiver,
            call,
            class,
            slot,
            target,
            unwind,
        } => mir::Terminator::CallVirtual {
            receiver: remap_value_reference(*receiver, substitutions),
            call: remap_call(tree, call, substitutions),
            class: *class,
            slot: *slot,
            target: extend_target(tree, target, block_params, value_stacks, substitutions),
            unwind: unwind.as_ref().map(|unwind| {
                extend_target(tree, unwind, block_params, value_stacks, substitutions)
            }),
        },
        mir::Terminator::CallDynamic {
            receiver,
            call,
            constraint,
            slot,
            target,
            unwind,
        } => mir::Terminator::CallDynamic {
            receiver: remap_value_reference(*receiver, substitutions),
            call: remap_call(tree, call, substitutions),
            constraint: *constraint,
            slot: *slot,
            target: extend_target(tree, target, block_params, value_stacks, substitutions),
            unwind: unwind.as_ref().map(|unwind| {
                extend_target(tree, unwind, block_params, value_stacks, substitutions)
            }),
        },
        mir::Terminator::Return { value } => mir::Terminator::Return {
            value: value.map(|value| remap_value_reference(value, substitutions)),
        },
        mir::Terminator::Trap { kind, payload } => mir::Terminator::Trap {
            kind: *kind,
            payload: payload.map(|value| remap_value_reference(value, substitutions)),
        },
        mir::Terminator::Panic { payload } => mir::Terminator::Panic {
            payload: payload.map(|value| remap_value_reference(value, substitutions)),
        },
        mir::Terminator::UnwindResume => mir::Terminator::UnwindResume,
        mir::Terminator::Unreachable => mir::Terminator::Unreachable,
        mir::Terminator::TailCall { function, call } => mir::Terminator::TailCall {
            function: *function,
            call: remap_call(tree, call, substitutions),
        },
        mir::Terminator::TailCallVirtual {
            receiver,
            call,
            class,
            slot,
        } => mir::Terminator::TailCallVirtual {
            receiver: remap_value_reference(*receiver, substitutions),
            call: remap_call(tree, call, substitutions),
            class: *class,
            slot: *slot,
        },
        mir::Terminator::TailCallDynamic {
            receiver,
            call,
            constraint,
            slot,
        } => mir::Terminator::TailCallDynamic {
            receiver: remap_value_reference(*receiver, substitutions),
            call: remap_call(tree, call, substitutions),
            constraint: *constraint,
            slot: *slot,
        },
        mir::Terminator::TailCallIndirect { callee, call } => mir::Terminator::TailCallIndirect {
            callee: remap_value_reference(*callee, substitutions),
            call: remap_call(tree, call, substitutions),
        },
    }
}

/// Extend one block target with promoted local arguments.
fn extend_target(
    tree: &mut mir::Tree,
    target: &mir::BlockTarget,
    block_params: &HashMap<
        (mir::LocalNodeId<mir::Block>, mir::LocalNodeId<mir::Local>),
        mir::Value,
    >,
    value_stacks: &HashMap<mir::LocalNodeId<mir::Local>, Vec<mir::Value>>,
    substitutions: &HashMap<mir::Value, mir::Value>,
) -> mir::BlockTarget {
    let arguments = extend_arguments(
        target.block,
        tree.get_values(target.arguments),
        block_params,
        value_stacks,
        substitutions,
    );

    mir::BlockTarget::new(target.block, tree.add_values(&arguments))
}

/// Remap one compact call argument slice.
fn remap_call(
    tree: &mut mir::Tree,
    call: &mir::Call<mir::ValueSlice>,
    substitutions: &HashMap<mir::Value, mir::Value>,
) -> mir::Call<mir::ValueSlice> {
    let arguments: Vec<_> = tree
        .get_values(call.arguments)
        .iter()
        .copied()
        .map(|value| remap_value_reference(value, substitutions))
        .collect();

    mir::Call {
        arguments: tree.add_values(&arguments),
        ..call.clone()
    }
}

/// Extend existing arguments with block arguments for a target block.
fn extend_arguments(
    target: mir::BlockId,
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
        .copied()
        .map(|value| resolve_value(value, substitutions))
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
        let input = r#"
function test(v0: int32): int32 {
    local l0: int32

entry(v0: int32):
    local.set l0, v0
    v1: int32 = local.get l0
    return v1
}
"#;
        let expected = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    return v0
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&Mem2Reg);
        test.assert_output(expected);
    }

    /// Local accessed across blocks via jump.
    #[test]
    fn test_promote_across_blocks() {
        let input = r#"
function test(v0: int32): int32 {
    local l0: int32

entry(v0: int32):
    local.set l0, v0
    jump b1

b1:
    v1: int32 = local.get l0
    return v1
}
"#;
        let expected = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    jump b1

b1:
    return v0
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&Mem2Reg);
        test.assert_output(expected);
    }

    /// Diamond CFG with block parameter needed at join point.
    #[test]
    fn test_promote_in_diamond_cfg() {
        let input = r#"
function test(v0: boolean, v1: int32): int32 {
    local l0: int32

entry(v0: boolean, v1: int32):
    local.set l0, v1
    branch v0, b1, b2

b1:
    v2: int32 = 1
    local.set l0, v2
    jump b3

b2:
    v3: int32 = 2
    local.set l0, v3
    jump b3

b3:
    v4: int32 = local.get l0
    return v4
}
"#;
        // block3 needs a block parameter for the different values from block1/block2
        // v4 is the new block parameter value allocated by the pass
        let expected = r#"
function test(v0: boolean, v1: int32): int32 {
entry(v0: boolean, v1: int32):
    branch v0, b1, b2

b1:
    v2: int32 = 1
    jump b3(v2)

b2:
    v3: int32 = 2
    jump b3(v3)

b3(v5: int32):
    return v5
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&Mem2Reg);
        test.assert_output(expected);
    }

    /// Multiple locals, all promotable.
    #[test]
    fn test_promote_multiple_locals() {
        let input = r#"
function test(v0: int32, v1: int32): int32 {
    local l0: int32
    local l1: int32

entry(v0: int32, v1: int32):
    local.set l0, v0
    local.set l1, v1
    v2: int32 = local.get l0
    v3: int32 = local.get l1
    v4: int32 = int.add v2, v3
    return v4
}
"#;
        let expected = r#"
function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v4: int32 = int.add v0, v1
    return v4
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&Mem2Reg);
        test.assert_output(expected);
    }

    /// No locals to promote.
    #[test]
    fn test_preserve_without_locals() {
        let input = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    return v0
}
"#;

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
    local l0: int32

entry:
    v0: int32 = local.get l0
    return v0
}
"#;
        // reading a local before writing is a bug in the MIR
        let mut test = TestProgram::new(input);
        test.run_pass(&Mem2Reg);
    }

    /// Loop with local - block parameter needed at loop header.
    #[test]
    fn test_promote_in_loop() {
        let input = r#"
function test(v0: int32): int32 {
    local l0: int32

entry(v0: int32):
    v1: int32 = 0
    local.set l0, v1
    jump b1

b1:
    v2: int32 = local.get l0
    v3: boolean = int.lt.s v2, v0
    branch v3, b2, b3

b2:
    v4: int32 = 1
    v5: int32 = int.add v2, v4
    local.set l0, v5
    jump b1

b3:
    v6: int32 = local.get l0
    return v6
}
"#;
        // b1 is at dominance frontier (join point from b0 and b2)
        // v0-v6 exist in input -> next_value_id = 7
        let expected = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 0
    jump b1(v1)

b1(v7: int32):
    v3: boolean = int.lt.s v7, v0
    branch v3, b2, b3

b2:
    v4: int32 = 1
    v5: int32 = int.add v7, v4
    jump b1(v5)

b3:
    return v7
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&Mem2Reg);
        test.assert_output(expected);
    }

    /// Multiple definitions in the same block.
    #[test]
    fn test_promote_with_multiple_defs() {
        let input = r#"
function test(v0: int32): int32 {
    local l0: int32

entry(v0: int32):
    local.set l0, v0
    v1: int32 = 42
    local.set l0, v1
    v2: int32 = local.get l0
    return v2
}
"#;
        let expected = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 42
    return v1
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&Mem2Reg);
        test.assert_output(expected);
    }

    /// Existing block parameters should be preserved.
    #[test]
    fn test_preserve_existing_block_params() {
        let input = r#"
function test(v0: int32, v1: int32): int32 {
    local l0: int32

entry(v0: int32, v1: int32):
    local.set l0, v0
    jump b1(v1)

b1(v2: int32):
    v3: int32 = local.get l0
    v4: int32 = int.add v2, v3
    return v4
}
"#;
        // block1 keeps its existing parameter, l0 value is dominated by entry
        let expected = r#"
function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    jump b1(v1)

b1(v2: int32):
    v4: int32 = int.add v2, v0
    return v4
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&Mem2Reg);
        test.assert_output(expected);
    }

    /// Switch terminator with local.
    #[test]
    fn test_promote_with_switch() {
        let input = r#"
function test(v0: int32): int32 {
    local l0: int32

entry(v0: int32):
    v1: int32 = 10
    local.set l0, v1
    switch v0, b3, 0 => b1, 1 => b2

b1:
    v2: int32 = 100
    local.set l0, v2
    jump b3

b2:
    v3: int32 = 200
    local.set l0, v3
    jump b3

b3:
    v4: int32 = local.get l0
    return v4
}
"#;
        // b3 is join point with 3 predecessors (b0, b1, b2)
        // v0-v4 exist in input -> next_value_id = 5
        // new block parameter is v5
        let expected = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 10
    switch v0, b3(v1), 0 => b1, 1 => b2

b1:
    v2: int32 = 100
    jump b3(v2)

b2:
    v3: int32 = 200
    jump b3(v3)

b3(v5: int32):
    return v5
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&Mem2Reg);
        test.assert_output(expected);
    }

    /// Call arguments are rewritten after local promotion.
    #[test]
    fn test_promote_call_arguments() {
        let input = r#"
external function sink(int32): void

function test(): void {
    local l0: int32

entry:
    v0: int32 = 7
    local.set l0, v0
    v1: int32 = local.get l0
    call sink(v1)
    return
}
"#;
        let expected = r#"
external function sink(int32): void

function test(): void {
entry:
    v0: int32 = 7
    call sink(v0)
    return
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&Mem2Reg);
        test.assert_output(expected);
    }
}
