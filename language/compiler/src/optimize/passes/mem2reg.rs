use std::collections::{HashMap, HashSet, VecDeque};

use destack_compiler_macros::declare_pass;
use destack_mir as mir;
use mir::{Instruction, Terminator};

use crate::optimize::analyses::{ControlFlowGraph, DominatorTree};
use crate::optimize::{
    AnalysisPreservation, FunctionPass, OptimizationContext, Pass, PassMetadata,
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

impl Pass for Mem2Reg {
    fn metadata(&self) -> &'static PassMetadata {
        Mem2Reg::metadata()
    }
}

impl FunctionPass for Mem2Reg {
    fn run_on_function(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::NodeTree,
        context: &OptimizationContext<'_>,
    ) -> AnalysisPreservation {
        // no locals to promote
        if function.locals.is_empty() {
            return AnalysisPreservation::all();
        }

        // find promotable locals (only LocalGet/LocalSet, no address taken)
        let promotable = find_promotable_locals(function, tree);
        if promotable.is_empty() {
            return AnalysisPreservation::all();
        }

        // ensure next_value_id is correct before allocating new values
        function.recompute_next_value_id(tree);

        // get dominator tree and CFG for the algorithm
        let cfg = context.analyses.get::<ControlFlowGraph>(function, tree);
        let domtree = context.analyses.get::<DominatorTree>(function, tree);

        // compute dominance frontiers
        let frontiers = compute_dominance_frontiers(function, &cfg, &domtree);

        // find definition blocks for each promotable local
        let def_blocks = find_definition_blocks(&promotable, function, tree);

        // compute where block parameters are needed (dominance frontier + live uses)
        let param_placements =
            compute_param_placements(&promotable, &def_blocks, &frontiers, function, tree);

        // insert block parameters for locals
        let block_params = insert_block_parameters(&param_placements, &promotable, function, tree);

        // rename variables: replace LocalGet/LocalSet with SSA values
        let entry = function.entry.expect("function has no entry block");
        rename_variables(
            &promotable,
            &block_params,
            function,
            tree,
            &cfg,
            &domtree,
            entry,
        );

        // remove promoted locals from the function
        let promoted_set: HashSet<_> = promotable.keys().copied().collect();
        function
            .locals
            .retain(|local| !promoted_set.contains(local));

        // CFG structure changed (block parameters modified)
        AnalysisPreservation::none()
    }
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
    // DF(X) = {Y : Y has a predecessor Z where X dominates Z but X does not strictly dominate Y}
    for &block in &function.blocks {
        let preds = cfg.predecessors(block);
        if preds.len() >= 2 {
            // block is a join point - check each predecessor
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
/// In block-parameter SSA, values must be explicitly passed between blocks.
/// A block needs a parameter for a local if:
/// 1. It's at a dominance frontier (multiple definitions can reach it)
/// 2. It uses the local (LocalGet before LocalSet) and is not the entry block
fn compute_param_placements(
    promotable: &HashMap<mir::LocalNodeId<mir::Local>, PromotableLocal>,
    def_blocks: &HashMap<mir::LocalNodeId<mir::Local>, HashSet<mir::LocalNodeId<mir::Block>>>,
    frontiers: &HashMap<mir::LocalNodeId<mir::Block>, HashSet<mir::LocalNodeId<mir::Block>>>,
    function: &mir::Function,
    tree: &mir::NodeTree,
) -> HashMap<mir::LocalNodeId<mir::Block>, HashSet<mir::LocalNodeId<mir::Local>>> {
    let mut param_placements: HashMap<
        mir::LocalNodeId<mir::Block>,
        HashSet<mir::LocalNodeId<mir::Local>>,
    > = function
        .blocks
        .iter()
        .map(|&b| (b, HashSet::new()))
        .collect();

    let entry = function.entry.expect("function has entry");

    // 1) add block parameters at dominance frontiers (merge points)
    for &local in promotable.keys() {
        let defs = &def_blocks[&local];
        let mut worklist: VecDeque<mir::LocalNodeId<mir::Block>> = defs.iter().copied().collect();
        let mut seen = defs.clone();

        while let Some(block) = worklist.pop_front() {
            for &frontier_block in &frontiers[&block] {
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

    // 2) add parameters for blocks that *use* a local (LocalGet before LocalSet)
    //    (and are not the entry block, which gets values from function parameters)
    for &block_id in &function.blocks {
        if block_id == entry {
            continue;
        }

        let block = tree.get(block_id);
        for local in promotable.keys() {
            // check if this block uses the local before defining it
            let mut used_before_def = false;
            for &instruction_id in &block.instructions {
                let instr = tree.get(instruction_id);
                match instr {
                    Instruction::LocalSet { local: l, .. } if l == local => {
                        // found a definition before any use - this local is not live-in
                        break;
                    }
                    Instruction::LocalGet { local: l, .. } if l == local => {
                        // found a use before any definition - this local is live-in
                        used_before_def = true;
                        break;
                    }
                    _ => {}
                }
            }
            if used_before_def {
                param_placements.get_mut(&block_id).unwrap().insert(*local);
            }
        }
    }

    param_placements
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
    block_params: &HashMap<(mir::LocalNodeId<mir::Block>, mir::LocalNodeId<mir::Local>), mir::Value>,
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

    // DFS in dominator tree order
    let mut worklist: Vec<RenameWorklistEntry> = vec![(entry, Vec::new())];

    while let Some((block_id, stack_depths)) = worklist.pop() {
        // restore stack depths from when we entered this block
        for (local, depth) in &stack_depths {
            value_stacks.get_mut(local).unwrap().truncate(*depth);
        }

        // record current stack depths for children
        let current_depths: Vec<(mir::LocalNodeId<mir::Local>, usize)> = promotable
            .keys()
            .map(|&k| (k, value_stacks[&k].len()))
            .collect();

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
                        let current_value = value_stacks[local].last().copied().unwrap_or_else(|| {
                            panic!(
                                "use of undefined local in block {block_id:?} \
                                 (local was read before being written - this is a bug in the MIR)"
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

            let instruction = tree.get(instruction_id);
            let new_instruction = substitute_instruction_uses(instruction, &substitutions);
            if new_instruction != *instruction {
                tree.replace(instruction_id, new_instruction);
            }
        }

        // apply substitutions to terminator
        let block = tree.get(block_id);
        let new_terminator = substitute_terminator_uses(&block.terminator, &substitutions);
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
    block_params: &HashMap<(mir::LocalNodeId<mir::Block>, mir::LocalNodeId<mir::Local>), mir::Value>,
    value_stacks: &HashMap<mir::LocalNodeId<mir::Local>, Vec<mir::Value>>,
    substitutions: &HashMap<mir::Value, mir::Value>,
) -> Terminator {
    match terminator {
        Terminator::Jump { target, arguments } => {
            let new_args =
                extend_arguments(*target, arguments, block_params, value_stacks, substitutions);
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
    }
}

/// Extend existing arguments with block arguments for a target block.
fn extend_arguments(
    target: mir::LocalNodeId<mir::Block>,
    existing: &[mir::Value],
    block_params: &HashMap<(mir::LocalNodeId<mir::Block>, mir::LocalNodeId<mir::Local>), mir::Value>,
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

/// Substitute uses in an instruction.
fn substitute_instruction_uses(
    instruction: &Instruction,
    substitutions: &HashMap<mir::Value, mir::Value>,
) -> Instruction {
    match instruction {
        Instruction::Binary {
            destination,
            operator,
            left,
            right,
        } => Instruction::Binary {
            destination: *destination,
            operator: *operator,
            left: resolve_value(*left, substitutions),
            right: resolve_value(*right, substitutions),
        },
        Instruction::Unary {
            destination,
            operator,
            argument,
        } => Instruction::Unary {
            destination: *destination,
            operator: *operator,
            argument: resolve_value(*argument, substitutions),
        },
        Instruction::Cast {
            destination,
            operator,
            argument,
            to_type,
        } => Instruction::Cast {
            destination: *destination,
            operator: *operator,
            argument: resolve_value(*argument, substitutions),
            to_type: *to_type,
        },
        Instruction::LocalSet { local, value } => Instruction::LocalSet {
            local: *local,
            value: resolve_value(*value, substitutions),
        },
        Instruction::Load {
            destination,
            pointer,
        } => Instruction::Load {
            destination: *destination,
            pointer: resolve_value(*pointer, substitutions),
        },
        Instruction::Store { pointer, value } => Instruction::Store {
            pointer: resolve_value(*pointer, substitutions),
            value: resolve_value(*value, substitutions),
        },
        Instruction::Drop { value } => Instruction::Drop {
            value: resolve_value(*value, substitutions),
        },
        Instruction::FieldGet {
            destination,
            aggregate,
            index,
        } => Instruction::FieldGet {
            destination: *destination,
            aggregate: resolve_value(*aggregate, substitutions),
            index: *index,
        },
        Instruction::FieldAddr {
            destination,
            aggregate,
            index,
        } => Instruction::FieldAddr {
            destination: *destination,
            aggregate: resolve_value(*aggregate, substitutions),
            index: *index,
        },
        Instruction::FieldSet {
            destination,
            aggregate,
            index,
            value,
        } => Instruction::FieldSet {
            destination: *destination,
            aggregate: resolve_value(*aggregate, substitutions),
            index: *index,
            value: resolve_value(*value, substitutions),
        },
        Instruction::ElementGet {
            destination,
            array,
            index,
        } => Instruction::ElementGet {
            destination: *destination,
            array: resolve_value(*array, substitutions),
            index: resolve_value(*index, substitutions),
        },
        Instruction::ElementAddr {
            destination,
            array,
            index,
        } => Instruction::ElementAddr {
            destination: *destination,
            array: resolve_value(*array, substitutions),
            index: resolve_value(*index, substitutions),
        },
        Instruction::ElementSet {
            destination,
            array,
            index,
            value,
        } => Instruction::ElementSet {
            destination: *destination,
            array: resolve_value(*array, substitutions),
            index: resolve_value(*index, substitutions),
            value: resolve_value(*value, substitutions),
        },
        Instruction::ManagedAllocArray {
            destination,
            element,
            length,
        } => Instruction::ManagedAllocArray {
            destination: *destination,
            element: *element,
            length: resolve_value(*length, substitutions),
        },
        Instruction::RawFree { pointer } => Instruction::RawFree {
            pointer: resolve_value(*pointer, substitutions),
        },
        // instructions with no value uses or external arguments
        Instruction::Const { .. }
        | Instruction::LocalGet { .. }
        | Instruction::GlobalAddr { .. }
        | Instruction::GlobalConst { .. }
        | Instruction::Call { .. }
        | Instruction::CallIndirect { .. }
        | Instruction::ManagedAlloc { .. }
        | Instruction::RawAlloc { .. }
        | Instruction::StackAlloc { .. }
        | Instruction::Intrinsic { .. } => instruction.clone(),
    }
}

/// Substitute uses in a terminator.
fn substitute_terminator_uses(
    terminator: &Terminator,
    substitutions: &HashMap<mir::Value, mir::Value>,
) -> Terminator {
    match terminator {
        Terminator::Jump { target, arguments } => Terminator::Jump {
            target: *target,
            arguments: arguments
                .iter()
                .map(|&v| resolve_value(v, substitutions))
                .collect(),
        },
        Terminator::Branch {
            condition,
            then_target,
            then_arguments,
            else_target,
            else_arguments,
        } => Terminator::Branch {
            condition: resolve_value(*condition, substitutions),
            then_target: *then_target,
            then_arguments: then_arguments
                .iter()
                .map(|&v| resolve_value(v, substitutions))
                .collect(),
            else_target: *else_target,
            else_arguments: else_arguments
                .iter()
                .map(|&v| resolve_value(v, substitutions))
                .collect(),
        },
        Terminator::Switch {
            value,
            default,
            default_arguments,
            cases,
        } => Terminator::Switch {
            value: resolve_value(*value, substitutions),
            default: *default,
            default_arguments: default_arguments
                .iter()
                .map(|&v| resolve_value(v, substitutions))
                .collect(),
            cases: cases
                .iter()
                .map(|c| mir::SwitchCase {
                    value: c.value,
                    target: c.target,
                    arguments: c
                        .arguments
                        .iter()
                        .map(|&v| resolve_value(v, substitutions))
                        .collect(),
                })
                .collect(),
        },
        Terminator::Return { value } => Terminator::Return {
            value: value.map(|v| resolve_value(v, substitutions)),
        },
        Terminator::Yield {
            value,
            resume,
            resume_arguments,
        } => Terminator::Yield {
            value: resolve_value(*value, substitutions),
            resume: *resume,
            resume_arguments: resume_arguments
                .iter()
                .map(|&v| resolve_value(v, substitutions))
                .collect(),
        },
        Terminator::Unreachable => Terminator::Unreachable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Simple case: single local with set then get.
    #[test]
    fn test_mem2reg_simple() {
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
        program.assert_eq(expected);
    }

    /// Local accessed across blocks via jump.
    #[test]
    fn test_mem2reg_cross_block() {
        let input = r#"function @test(v0: i32) -> i32 {
    local0: i32 ; owned, mut
block0(v0: i32):
    local.set local0, v0
    jump block1
block1:
    v1 = local.get local0
    return v1
}"#;
        // v0, v1 exist in input -> next_value_id = 2
        // new block parameter is v2
        let expected = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    jump block1(v0)
block1(v2: i32):
    return v2
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&Mem2Reg);
        program.assert_eq(expected);
    }

    /// Diamond CFG with block parameter needed at join point.
    #[test]
    fn test_mem2reg_diamond() {
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
        program.assert_eq(expected);
    }

    /// Multiple locals, all promotable.
    #[test]
    fn test_mem2reg_multiple_locals() {
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
        program.assert_eq(expected);
    }

    /// No locals to promote.
    #[test]
    fn test_mem2reg_no_locals() {
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
    fn test_mem2reg_undefined_read() {
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
    fn test_mem2reg_loop() {
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
        // block3 also needs a parameter since local is read there via local.get
        // v0-v6 exist in input -> next_value_id = 7
        // blocks processed in ID order: block1 gets v7, block3 gets v8
        let expected = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1 = iconst 0i32
    jump block1(v1)
block1(v7: i32):
    v3 = icmp_slt v7, v0
    branch v3, block2, block3(v7)
block2:
    v4 = iconst 1i32
    v5 = iadd v7, v4
    jump block1(v5)
block3(v8: i32):
    return v8
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&Mem2Reg);
        program.assert_eq(expected);
    }

    /// Multiple definitions in the same block.
    #[test]
    fn test_mem2reg_multiple_defs_same_block() {
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
        program.assert_eq(expected);
    }

    /// Existing block parameters should be preserved.
    #[test]
    fn test_mem2reg_preserve_existing_params() {
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
        // block1 already has parameter v2, gets another for local0
        // v0-v4 exist in input -> next_value_id = 5
        // new block parameter is v5
        let expected = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    jump block1(v1, v0)
block1(v2: i32, v5: i32):
    v4 = iadd v2, v5
    return v4
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&Mem2Reg);
        program.assert_eq(expected);
    }

    /// Switch terminator with local.
    #[test]
    fn test_mem2reg_switch() {
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
        program.assert_eq(expected);
    }
}
