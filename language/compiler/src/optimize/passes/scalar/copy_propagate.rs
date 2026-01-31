use std::collections::{HashMap, HashSet};

use destack_compiler_macros::declare_pass;
use destack_mir as mir;
use mir::Terminator;

use crate::optimize::{
    AnalysisPreservation, FunctionPass, PipelineContext, instruction_substitute_uses_in_tree,
    remap_instruction_memory_accesses, resolve_substitution_chains, terminator_substitute_uses,
};

declare_pass! {
    /// Copy propagation pass.
    ///
    /// Replaces uses of block parameters that are copies of another value.
    /// A block parameter is a "copy" when all predecessors pass the same value.
    ///
    /// ```mir
    /// function @before(v0: i32) -> i32 {
    /// block0(v0: i32):
    ///     jump block1(v0)
    /// block1(v1: i32):
    ///     jump block2(v1)
    /// block2(v2: i32):
    ///     return v2
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function @after(v0: i32) -> i32 {
    /// block0(v0: i32):
    ///     jump block1
    /// block1:
    ///     jump block2
    /// block2:
    ///     return v0
    /// }
    /// ```
    #[pass(id = "copy-propagate")]
    pub CopyPropagate,
    "Propagate copies through block parameters"
}

impl FunctionPass for CopyPropagate {
    fn run(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::NodeTree,
        _ctx: &PipelineContext<'_>,
    ) -> AnalysisPreservation {
        // run copy propagation
        let changed = run_copy_propagate(function, tree);

        // preserve analyses when nothing changed
        if changed {
            AnalysisPreservation::none()
        } else {
            AnalysisPreservation::all()
        }
    }

    fn name(&self) -> &'static str {
        "CopyPropagate"
    }

    fn id(&self) -> &'static str {
        "copy-propagate"
    }
}

/// Core copy propagation logic.
#[allow(clippy::type_complexity)]
fn run_copy_propagate(function: &mut mir::Function, tree: &mut mir::NodeTree) -> bool {
    // build predecessor map: block -> list of (predecessor_block, arguments passed)
    let mut predecessors: HashMap<
        mir::LocalNodeId<mir::Block>,
        Vec<(mir::LocalNodeId<mir::Block>, Vec<mir::Value>)>,
    > = HashMap::new();

    // initialize all blocks with empty predecessor lists
    for &block_id in &function.blocks {
        predecessors.insert(block_id, Vec::new());
    }

    // collect predecessors and their arguments
    for &block_id in &function.blocks {
        let block = tree.get(block_id);
        match &block.terminator {
            Terminator::Jump { target, arguments } => {
                predecessors
                    .get_mut(target)
                    .unwrap()
                    .push((block_id, arguments.clone()));
            }
            Terminator::Branch {
                then_target,
                then_arguments,
                else_target,
                else_arguments,
                ..
            } => {
                predecessors
                    .get_mut(then_target)
                    .unwrap()
                    .push((block_id, then_arguments.clone()));
                predecessors
                    .get_mut(else_target)
                    .unwrap()
                    .push((block_id, else_arguments.clone()));
            }
            Terminator::Check {
                success, failure, ..
            } => {
                predecessors
                    .get_mut(&success.target)
                    .unwrap()
                    .push((block_id, success.arguments.clone()));
                predecessors
                    .get_mut(&failure.target)
                    .unwrap()
                    .push((block_id, failure.arguments.clone()));
            }
            Terminator::Switch {
                default,
                default_arguments,
                cases,
                ..
            } => {
                predecessors
                    .get_mut(default)
                    .unwrap()
                    .push((block_id, default_arguments.clone()));
                for case in cases {
                    predecessors
                        .get_mut(&case.target)
                        .unwrap()
                        .push((block_id, case.arguments.clone()));
                }
            }
            Terminator::Yield {
                resume,
                resume_arguments,
                ..
            } => {
                predecessors
                    .get_mut(resume)
                    .unwrap()
                    .push((block_id, resume_arguments.clone()));
            }
            Terminator::Return { .. }
            | Terminator::Unreachable
            | Terminator::TailCall { .. }
            | Terminator::TailCallVirtual { .. }
            | Terminator::TailCallInterface { .. }
            | Terminator::TailCallIndirect { .. } => {}
        }
    }

    // find copy parameters
    let mut substitutions: HashMap<mir::Value, mir::Value> = HashMap::new();
    let mut to_remove: HashSet<mir::LocalNodeId<mir::Instruction>> = HashSet::new();
    for &block_id in &function.blocks {
        // skip entry block parameters
        if function.entry == Some(block_id) {
            continue;
        }

        let block = tree.get(block_id);
        let preds = &predecessors[&block_id];

        // skip blocks without predecessors or parameters
        if preds.is_empty() || block.parameters.is_empty() {
            continue;
        }

        for (param_idx, param) in block.parameters.iter().enumerate() {
            let param_value = param.value;

            let mut incoming_values: Vec<mir::Value> = Vec::new();
            for (_pred_block, args) in preds {
                if param_idx < args.len() {
                    incoming_values.push(args[param_idx]);
                }
            }

            // require every predecessor to pass an argument
            if incoming_values.len() != preds.len() {
                continue;
            }

            // record copies when all incoming values match
            if let Some(&first) = incoming_values.first()
                && incoming_values.iter().all(|&v| v == first)
                && first != param_value
            {
                substitutions.insert(param_value, first);
            }
        }
    }

    // collect select based copies
    for &block_id in &function.blocks {
        let block = tree.get(block_id);
        for &instruction_id in &block.instructions {
            let instruction = tree.get(instruction_id);
            if let mir::Instruction::Select {
                destination,
                then_value,
                else_value,
                ..
            } = instruction
                && then_value == else_value
            {
                substitutions.insert(*destination, *then_value);
                to_remove.insert(instruction_id);
            }
        }
    }

    // nothing to do
    if substitutions.is_empty() && to_remove.is_empty() {
        return false;
    }

    // collapse transitive substitutions
    let substitutions = resolve_substitution_chains(substitutions);

    // collect removed indices
    let mut removed_indices: HashMap<mir::LocalNodeId<mir::Block>, Vec<usize>> = HashMap::new();
    for &block_id in &function.blocks {
        let block = tree.get(block_id);
        let indices: Vec<usize> = block
            .parameters
            .iter()
            .enumerate()
            .filter_map(|(idx, p)| {
                // record indices that will be removed
                if substitutions.contains_key(&p.value) {
                    Some(idx)
                } else {
                    None
                }
            })
            .collect();
        if !indices.is_empty() {
            removed_indices.insert(block_id, indices);
        }
    }

    // apply substitutions to instructions
    for &block_id in &function.blocks {
        let instruction_ids: Vec<_> = tree.get(block_id).instructions.clone();
        for instruction_id in instruction_ids {
            // skip instructions that will be removed
            if to_remove.contains(&instruction_id) {
                continue;
            }

            let instruction = tree.get(instruction_id).clone();
            let new_instruction =
                instruction_substitute_uses_in_tree(&instruction, &substitutions, tree);

            // replace instructions when substitutions apply
            if new_instruction != instruction {
                tree.replace(instruction_id, new_instruction);
                remap_instruction_memory_accesses(tree, instruction_id, &substitutions);
            }
        }
    }

    // apply substitutions to terminators and parameters
    for &block_id in &function.blocks {
        let block = tree.get(block_id);
        let new_terminator = terminator_substitute_uses(&block.terminator, &substitutions);
        let new_terminator = remove_arguments_at_indices(&new_terminator, &removed_indices);
        let new_parameters: Vec<_> = block
            .parameters
            .iter()
            .filter(|p| !substitutions.contains_key(&p.value))
            .cloned()
            .collect();
        let new_instructions: Vec<_> = block
            .instructions
            .iter()
            .copied()
            .filter(|id| !to_remove.contains(id))
            .collect();

        // replace blocks when terminators or parameters change
        if new_terminator != block.terminator
            || new_parameters.len() != block.parameters.len()
            || new_instructions.len() != block.instructions.len()
        {
            let mut new_block = block.clone();
            new_block.terminator = new_terminator;
            new_block.parameters = new_parameters;
            new_block.instructions = new_instructions;
            tree.replace(block_id, new_block);
        }
    }

    true
}

/// Remove arguments at specified indices from terminator's target arguments.
fn remove_arguments_at_indices(
    terminator: &Terminator,
    removed_indices: &HashMap<mir::LocalNodeId<mir::Block>, Vec<usize>>,
) -> Terminator {
    match terminator {
        Terminator::Jump { target, arguments } => {
            if let Some(indices) = removed_indices.get(target) {
                let new_args = filter_indices(arguments, indices);
                Terminator::Jump {
                    target: *target,
                    arguments: new_args,
                }
            } else {
                terminator.clone()
            }
        }
        Terminator::Branch {
            condition,
            then_target,
            then_arguments,
            else_target,
            else_arguments,
        } => {
            let new_then_args = if let Some(indices) = removed_indices.get(then_target) {
                filter_indices(then_arguments, indices)
            } else {
                then_arguments.clone()
            };
            let new_else_args = if let Some(indices) = removed_indices.get(else_target) {
                filter_indices(else_arguments, indices)
            } else {
                else_arguments.clone()
            };
            if new_then_args != *then_arguments || new_else_args != *else_arguments {
                Terminator::Branch {
                    condition: *condition,
                    then_target: *then_target,
                    then_arguments: new_then_args,
                    else_target: *else_target,
                    else_arguments: new_else_args,
                }
            } else {
                terminator.clone()
            }
        }
        Terminator::Check {
            condition,
            constraint,
            success,
            failure,
        } => {
            let new_success_args = if let Some(indices) = removed_indices.get(&success.target) {
                filter_indices(&success.arguments, indices)
            } else {
                success.arguments.clone()
            };
            let new_failure_args = if let Some(indices) = removed_indices.get(&failure.target) {
                filter_indices(&failure.arguments, indices)
            } else {
                failure.arguments.clone()
            };
            if new_success_args != success.arguments || new_failure_args != failure.arguments {
                Terminator::Check {
                    condition: *condition,
                    constraint: constraint.clone(),
                    success: mir::CheckTarget {
                        target: success.target,
                        arguments: new_success_args,
                    },
                    failure: mir::CheckTarget {
                        target: failure.target,
                        arguments: new_failure_args,
                    },
                }
            } else {
                terminator.clone()
            }
        }
        Terminator::Switch {
            value,
            default,
            default_arguments,
            cases,
        } => {
            let new_default_args = if let Some(indices) = removed_indices.get(default) {
                filter_indices(default_arguments, indices)
            } else {
                default_arguments.clone()
            };
            let new_cases: Vec<_> = cases
                .iter()
                .map(|case| {
                    let new_args = if let Some(indices) = removed_indices.get(&case.target) {
                        filter_indices(&case.arguments, indices)
                    } else {
                        case.arguments.clone()
                    };
                    mir::SwitchCase {
                        value: case.value,
                        target: case.target,
                        arguments: new_args,
                    }
                })
                .collect();
            Terminator::Switch {
                value: *value,
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
            if let Some(indices) = removed_indices.get(resume) {
                let new_args = filter_indices(resume_arguments, indices);
                Terminator::Yield {
                    value: *value,
                    resume: *resume,
                    resume_arguments: new_args,
                }
            } else {
                terminator.clone()
            }
        }
        _ => terminator.clone(),
    }
}

/// Filter out elements at the given indices.
fn filter_indices(values: &[mir::Value], indices_to_remove: &[usize]) -> Vec<mir::Value> {
    values
        .iter()
        .enumerate()
        .filter_map(|(idx, &v)| {
            if indices_to_remove.contains(&idx) {
                None
            } else {
                Some(v)
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Block parameter that receives the same value from all predecessors is eliminated.
    #[test]
    fn test_propagate_uniform_incoming_value() {
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    branch v0, block1, block2
block1:
    jump block3(v0)
block2:
    jump block3(v0)
block3(v1: i32):
    return v1
}"#;

        // v1 is always v0, so replace uses of v1 with v0 and remove the parameter
        let expected = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    branch v0, block1, block2
block1:
    jump block3
block2:
    jump block3
block3:
    return v0
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&CopyPropagate);
        test.assert_output(expected);
    }

    /// Block parameter with different values from predecessors is NOT eliminated.
    #[test]
    fn test_preserve_varying_incoming_values() {
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1: i32 = iconst 1i32
    branch v0, block1, block2
block1:
    jump block3(v0)
block2:
    jump block3(v1)
block3(v2: i32):
    return v2
}"#;

        // v2 gets different values from different predecessors, so no change

        let mut test = TestProgram::new(input);
        test.run_pass(&CopyPropagate);
        test.assert_unchanged(input);
    }

    /// Single predecessor block parameter is a trivial copy.
    #[test]
    fn test_propagate_single_predecessor() {
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    jump block1(v0)
block1(v1: i32):
    return v1
}"#;

        let expected = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    jump block1
block1:
    return v0
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&CopyPropagate);
        test.assert_output(expected);
    }

    /// Chained copies are resolved transitively.
    #[test]
    fn test_propagate_through_chain() {
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    jump block1(v0)
block1(v1: i32):
    jump block2(v1)
block2(v2: i32):
    return v2
}"#;

        // v1 = v0, v2 = v1 = v0
        let expected = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    jump block1
block1:
    jump block2
block2:
    return v0
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&CopyPropagate);
        test.assert_output(expected);
    }

    /// Multiple parameters, only some are copies.
    #[test]
    fn test_propagate_partial_copies() {
        let input = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    branch v0, block1, block2
block1:
    v2: i32 = iconst 10i32
    jump block3(v0, v2)
block2:
    v3: i32 = iconst 20i32
    jump block3(v0, v3)
block3(v4: i32, v5: i32):
    v6: i32 = iadd v4, v5
    return v6
}"#;

        // v4 is always v0 (copy), but v5 differs between predecessors
        let expected = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    branch v0, block1, block2
block1:
    v2: i32 = iconst 10i32
    jump block3(v2)
block2:
    v3: i32 = iconst 20i32
    jump block3(v3)
block3(v4: i32):
    v5: i32 = iadd v0, v4
    return v5
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&CopyPropagate);
        test.assert_output(expected);
    }

    /// No copies means no changes.
    #[test]
    fn test_preserve_without_copies() {
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1: i32 = iconst 1i32
    v2: i32 = iadd v0, v1
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&CopyPropagate);
        test.assert_unchanged(input);
    }

    /// Selects with identical arms are removed.
    #[test]
    fn test_remove_redundant_select() {
        let input = r#"function @test(v0: bool, v1: i32) -> i32 {
block0(v0: bool, v1: i32):
    v2: i32 = select v0, v1, v1
    return v2
}"#;

        let expected = r#"function @test(v0: bool, v1: i32) -> i32 {
block0(v0: bool, v1: i32):
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&CopyPropagate);
        test.assert_output(expected);
    }
}
