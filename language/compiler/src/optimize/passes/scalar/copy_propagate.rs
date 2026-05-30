use std::collections::{HashMap, HashSet};

use crate::declare_mir_pass;
use destack_mir as mir;

use crate::optimize::{
    AnalysisPreservation, FunctionPass, PipelineContext, instruction_substitute_uses_in_tree,
    remap_instruction_memory_accesses, resolve_substitution_chains, terminator_substitute_uses,
};

declare_mir_pass! {
    /// Copy propagation pass.
    ///
    /// Replaces uses of block parameters that are copies of another value.
    /// A block parameter is a "copy" when all predecessors pass the same value.
    ///
    /// ```mir
    /// function before(v0: int32): int32 {
    /// b0(v0: int32):
    ///     jump b1(v0)
    /// b1(v1: int32):
    ///     jump b2(v1)
    /// b2(v2: int32):
    ///     return v2
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function after(v0: int32): int32 {
    /// b0(v0: int32):
    ///     jump b1
    /// b1:
    ///     jump b2
    /// b2:
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
        tree: &mut mir::Tree,
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
fn run_copy_propagate(function: &mut mir::Function, tree: &mut mir::Tree) -> bool {
    // build predecessor map: block -> list of (predecessor_block, arguments passed)
    let mut predecessors: HashMap<
        mir::LocalNodeId<mir::Block>,
        Vec<(mir::LocalNodeId<mir::Block>, Vec<mir::ValueReference>)>,
    > = HashMap::new();

    // initialize all blocks with empty predecessor lists
    for &block_id in &function.blocks {
        predecessors.insert(block_id, Vec::new());
    }

    // collect predecessors and their arguments
    for &block_id in &function.blocks {
        let block = tree.get(block_id);
        let terminator = tree.get(block.terminator);
        match terminator {
            mir::Terminator::Error => {}
            mir::Terminator::Jump { target } => {
                if let Some(target_block) = target.block.block() {
                    predecessors
                        .get_mut(&target_block)
                        .unwrap()
                        .push((block_id, target.arguments.clone()));
                }
            }
            mir::Terminator::Branch {
                then_target,
                else_target,
                ..
            } => {
                if let Some(target_block) = then_target.block.block() {
                    predecessors
                        .get_mut(&target_block)
                        .unwrap()
                        .push((block_id, then_target.arguments.clone()));
                }

                if let Some(target_block) = else_target.block.block() {
                    predecessors
                        .get_mut(&target_block)
                        .unwrap()
                        .push((block_id, else_target.arguments.clone()));
                }
            }
            mir::Terminator::Check {
                success, failure, ..
            } => {
                if let Some(target_block) = success.block.block() {
                    predecessors
                        .get_mut(&target_block)
                        .unwrap()
                        .push((block_id, success.arguments.clone()));
                }

                if let Some(target_block) = failure.block.block() {
                    predecessors
                        .get_mut(&target_block)
                        .unwrap()
                        .push((block_id, failure.arguments.clone()));
                }
            }
            mir::Terminator::NewZeroedTry {
                success, failure, ..
            }
            | mir::Terminator::NewUninitTry {
                success, failure, ..
            }
            | mir::Terminator::NewSliceZeroedTry {
                success, failure, ..
            }
            | mir::Terminator::NewSliceUninitTry {
                success, failure, ..
            } => {
                if let Some(target_block) = success.block.block() {
                    let mut arguments = Vec::with_capacity(success.arguments.len() + 1);
                    arguments.push(mir::ValueReference::Missing);
                    arguments.extend(success.arguments.iter().copied());

                    predecessors
                        .get_mut(&target_block)
                        .unwrap()
                        .push((block_id, arguments));
                }

                if let Some(target_block) = failure.block.block() {
                    predecessors
                        .get_mut(&target_block)
                        .unwrap()
                        .push((block_id, failure.arguments.clone()));
                }
            }
            mir::Terminator::Switch { default, cases, .. } => {
                if let Some(target_block) = default.block.block() {
                    predecessors
                        .get_mut(&target_block)
                        .unwrap()
                        .push((block_id, default.arguments.clone()));
                }

                for case in cases {
                    if let Some(target_block) = case.target.block.block() {
                        predecessors
                            .get_mut(&target_block)
                            .unwrap()
                            .push((block_id, case.target.arguments.clone()));
                    }
                }
            }
            mir::Terminator::Yield { resume, .. } => {
                if let Some(target_block) = resume.block.block() {
                    predecessors
                        .get_mut(&target_block)
                        .unwrap()
                        .push((block_id, resume.arguments.clone()));
                }
            }
            mir::Terminator::Call { target, unwind, .. }
            | mir::Terminator::CallIndirect { target, unwind, .. }
            | mir::Terminator::CallVirtual { target, unwind, .. }
            | mir::Terminator::CallDynamic { target, unwind, .. } => {
                if let Some(target_block) = target.block.block() {
                    predecessors
                        .get_mut(&target_block)
                        .unwrap()
                        .push((block_id, target.arguments.clone()));
                }
                if let Some(unwind) = unwind
                    && let Some(target_block) = unwind.block.block()
                {
                    predecessors
                        .get_mut(&target_block)
                        .unwrap()
                        .push((block_id, unwind.arguments.clone()));
                }
            }
            mir::Terminator::Return { .. }
            | mir::Terminator::Panic { .. }
            | mir::Terminator::ResumePanic
            | mir::Terminator::Trap { .. }
            | mir::Terminator::Unreachable
            | mir::Terminator::TailCall { .. }
            | mir::Terminator::TailCallVirtual { .. }
            | mir::Terminator::TailCallDynamic { .. }
            | mir::Terminator::TailCallIndirect { .. } => {}
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
            let Some(param_value) = param.value.value() else {
                continue;
            };

            let mut incoming_values: Vec<mir::Value> = Vec::new();
            for (_pred_block, args) in preds {
                if param_idx < args.len() {
                    let Some(argument) = args[param_idx].value() else {
                        incoming_values.clear();
                        break;
                    };
                    incoming_values.push(argument);
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
                && let Some(destination) = destination.value()
                && let Some(then_value) = then_value.value()
            {
                substitutions.insert(destination, then_value);
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
                if p.value
                    .value()
                    .is_some_and(|value| substitutions.contains_key(&value))
                {
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
        let terminator = tree.get(block.terminator).clone();
        let new_terminator = terminator_substitute_uses(&terminator, &substitutions);
        let new_terminator = remove_arguments_at_indices(&new_terminator, &removed_indices);
        let new_parameters: Vec<_> = block
            .parameters
            .iter()
            .filter(|p| {
                !p.value
                    .value()
                    .is_some_and(|value| substitutions.contains_key(&value))
            })
            .cloned()
            .collect();
        let new_instructions: Vec<_> = block
            .instructions
            .iter()
            .copied()
            .filter(|id| !to_remove.contains(id))
            .collect();

        // replace blocks when terminators or parameters change
        if new_terminator != terminator
            || new_parameters.len() != block.parameters.len()
            || new_instructions.len() != block.instructions.len()
        {
            let mut new_block = block.clone();
            new_block.parameters = new_parameters;
            new_block.instructions = new_instructions;
            tree.replace(block.terminator, new_terminator);
            tree.replace(block_id, new_block);
        }
    }

    true
}

/// Remove arguments at specified indices from terminator's target arguments.
fn remove_arguments_at_indices(
    terminator: &mir::Terminator,
    removed_indices: &HashMap<mir::LocalNodeId<mir::Block>, Vec<usize>>,
) -> mir::Terminator {
    match terminator {
        mir::Terminator::Jump { target } => {
            let Some(target_block) = target.block.block() else {
                return terminator.clone();
            };
            let Some(indices) = removed_indices.get(&target_block) else {
                return terminator.clone();
            };

            let new_arguments = filter_indices(&target.arguments, indices);
            if new_arguments == target.arguments {
                return terminator.clone();
            }

            mir::Terminator::Jump {
                target: mir::BlockTarget {
                    block: target.block,
                    arguments: new_arguments,
                },
            }
        }
        mir::Terminator::Branch {
            condition,
            then_target,
            else_target,
        } => {
            let new_then_args = then_target
                .block
                .block()
                .and_then(|block| removed_indices.get(&block))
                .map(|indices| filter_indices(&then_target.arguments, indices))
                .unwrap_or_else(|| then_target.arguments.clone());
            let new_else_args = else_target
                .block
                .block()
                .and_then(|block| removed_indices.get(&block))
                .map(|indices| filter_indices(&else_target.arguments, indices))
                .unwrap_or_else(|| else_target.arguments.clone());
            if new_then_args != then_target.arguments || new_else_args != else_target.arguments {
                mir::Terminator::Branch {
                    condition: *condition,
                    then_target: mir::BlockTarget {
                        block: then_target.block,
                        arguments: new_then_args,
                    },
                    else_target: mir::BlockTarget {
                        block: else_target.block,
                        arguments: new_else_args,
                    },
                }
            } else {
                terminator.clone()
            }
        }
        mir::Terminator::Check {
            constraint,
            success,
            failure,
        } => {
            let new_success_args = success
                .block
                .block()
                .and_then(|block| removed_indices.get(&block))
                .map(|indices| filter_indices(&success.arguments, indices))
                .unwrap_or_else(|| success.arguments.clone());
            let new_failure_args = failure
                .block
                .block()
                .and_then(|block| removed_indices.get(&block))
                .map(|indices| filter_indices(&failure.arguments, indices))
                .unwrap_or_else(|| failure.arguments.clone());
            if new_success_args != success.arguments || new_failure_args != failure.arguments {
                mir::Terminator::Check {
                    constraint: constraint.clone(),
                    success: mir::BlockTarget {
                        block: success.block,
                        arguments: new_success_args,
                    },
                    failure: mir::BlockTarget {
                        block: failure.block,
                        arguments: new_failure_args,
                    },
                }
            } else {
                terminator.clone()
            }
        }
        mir::Terminator::Switch {
            value,
            default,
            cases,
        } => {
            let new_default_args = default
                .block
                .block()
                .and_then(|block| removed_indices.get(&block))
                .map(|indices| filter_indices(&default.arguments, indices))
                .unwrap_or_else(|| default.arguments.clone());
            let new_cases: Vec<_> = cases
                .iter()
                .map(|case| {
                    let new_args = case
                        .target
                        .block
                        .block()
                        .and_then(|block| removed_indices.get(&block))
                        .map(|indices| filter_indices(&case.target.arguments, indices))
                        .unwrap_or_else(|| case.target.arguments.clone());
                    mir::SwitchCase {
                        value: case.value,
                        target: mir::BlockTarget {
                            block: case.target.block,
                            arguments: new_args,
                        },
                    }
                })
                .collect();
            mir::Terminator::Switch {
                value: *value,
                default: mir::BlockTarget {
                    block: default.block,
                    arguments: new_default_args,
                },
                cases: new_cases,
            }
        }
        mir::Terminator::Yield { value, resume } => {
            if let Some(indices) = resume
                .block
                .block()
                .and_then(|block| removed_indices.get(&block))
            {
                let new_args = filter_indices(&resume.arguments, indices);
                mir::Terminator::Yield {
                    value: *value,
                    resume: mir::BlockTarget {
                        block: resume.block,
                        arguments: new_args,
                    },
                }
            } else {
                terminator.clone()
            }
        }
        _ => terminator.clone(),
    }
}

/// Filter out elements at the given indices.
fn filter_indices(
    values: &[mir::ValueReference],
    indices_to_remove: &[usize],
) -> Vec<mir::ValueReference> {
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
        let input = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    branch v0, b1, b2
b1:
    jump b3(v0)
b2:
    jump b3(v0)
b3(v1: int32):
    return v1
}"#;

        // v1 is always v0, so replace uses of v1 with v0 and remove the parameter
        let expected = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    branch v0, b1, b2
b1:
    jump b3
b2:
    jump b3
b3:
    return v0
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&CopyPropagate);
        test.assert_output(expected);
    }

    /// Block parameter with different values from predecessors is NOT eliminated.
    #[test]
    fn test_preserve_varying_incoming_values() {
        let input = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 1int32
    branch v0, b1, b2
b1:
    jump b3(v0)
b2:
    jump b3(v1)
b3(v2: int32):
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
        let input = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    jump b1(v0)
b1(v1: int32):
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
        test.run_pass(&CopyPropagate);
        test.assert_output(expected);
    }

    /// Chained copies are resolved transitively.
    #[test]
    fn test_propagate_through_chain() {
        let input = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    jump b1(v0)
b1(v1: int32):
    jump b2(v1)
b2(v2: int32):
    return v2
}"#;

        // v1 = v0, v2 = v1 = v0
        let expected = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    jump b1
b1:
    jump b2
b2:
    return v0
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&CopyPropagate);
        test.assert_output(expected);
    }

    /// Multiple parameters, only some are copies.
    #[test]
    fn test_propagate_partial_copies() {
        let input = r#"
function test(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    branch v0, b1, b2
b1:
    v2: int32 = 10int32
    jump b3(v0, v2)
b2:
    v3: int32 = 20int32
    jump b3(v0, v3)
b3(v4: int32, v5: int32):
    v6: int32 = int.add v4, v5
    return v6
}"#;

        // v4 is always v0 (copy), but v5 differs between predecessors
        let expected = r#"
function test(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    branch v0, b1, b2
b1:
    v2: int32 = 10int32
    jump b3(v2)
b2:
    v3: int32 = 20int32
    jump b3(v3)
b3(v4: int32):
    v5: int32 = int.add v0, v4
    return v5
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&CopyPropagate);
        test.assert_output(expected);
    }

    /// No copies means no changes.
    #[test]
    fn test_preserve_without_copies() {
        let input = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 1int32
    v2: int32 = int.add v0, v1
    return v2
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&CopyPropagate);
        test.assert_unchanged(input);
    }

    /// Selects with identical arms are removed.
    #[test]
    fn test_remove_redundant_select() {
        let input = r#"
function test(v0: boolean, v1: int32): int32 {
b0(v0: boolean, v1: int32):
    v2: int32 = select v0, v1, v1
    return v2
}"#;

        let expected = r#"
function test(v0: boolean, v1: int32): int32 {
b0(v0: boolean, v1: int32):
    return v1
}"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&CopyPropagate);
        test.assert_output(expected);
    }
}
