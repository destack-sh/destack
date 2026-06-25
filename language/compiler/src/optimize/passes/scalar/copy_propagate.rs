use std::collections::{HashMap, HashSet};

use crate::optimize::declare_pass;
use destack_mir as mir;

use crate::optimize::{FunctionPass, PipelineContext};
use destack_mir::{
    Mutation, instruction_substitute_uses_in_tree, remap_instruction_memory_accesses,
    resolve_substitution_chains, terminator_substitute_uses,
};

declare_pass! {
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
        _analyses: &mir::FunctionAnalyses,
    ) -> Mutation {
        // run copy propagation
        let changed = run_copy_propagate(function, tree);

        // report what this pass changed
        if changed {
            Mutation::VALUE
        } else {
            Mutation::NONE
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
        Vec<(mir::LocalNodeId<mir::Block>, Vec<mir::Value>)>,
    > = HashMap::new();

    // initialize all blocks with empty predecessor lists
    for &block_id in &function.blocks {
        predecessors.insert(block_id, Vec::new());
    }

    // collect predecessors and their arguments
    for &block_id in &function.blocks {
        let block = tree.get(block_id).clone();
        let terminator = tree.get(block.terminator);
        let mut record_predecessor = |target: &mir::BlockTarget| {
            let arguments = target.arguments(tree).to_vec();
            predecessors
                .get_mut(&target.block)
                .unwrap()
                .push((block_id, arguments));
        };

        match terminator {
            mir::Terminator::Error => {
                panic!("invalid MIR terminator reached optimizer");
            }
            mir::Terminator::Jump { target } => {
                record_predecessor(target);
            }
            mir::Terminator::Branch {
                then_target,
                else_target,
                ..
            } => {
                record_predecessor(then_target);
                record_predecessor(else_target);
            }
            mir::Terminator::Check {
                success, failure, ..
            } => {
                record_predecessor(success);
                record_predecessor(failure);
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
                record_predecessor(success);
                record_predecessor(failure);
            }
            mir::Terminator::Switch { default, cases, .. } => {
                record_predecessor(default);

                for case in tree.get_switch_cases(*cases) {
                    record_predecessor(&case.target);
                }
            }
            mir::Terminator::Yield { resume, unwind, .. } => {
                record_predecessor(resume);
                if let Some(unwind) = unwind {
                    record_predecessor(unwind);
                }
            }
            mir::Terminator::Call { target, unwind, .. }
            | mir::Terminator::CallIndirect { target, unwind, .. }
            | mir::Terminator::CallVirtual { target, unwind, .. }
            | mir::Terminator::CallDynamic { target, unwind, .. } => {
                record_predecessor(target);
                if let Some(unwind) = unwind {
                    record_predecessor(unwind);
                }
            }
            mir::Terminator::Return { .. }
            | mir::Terminator::Panic { .. }
            | mir::Terminator::UnwindResume
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

        let block = tree.get(block_id).clone();
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
                    let argument = args[param_idx];
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
                && true
                && true
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
                tree.set(instruction_id, new_instruction);
                remap_instruction_memory_accesses(tree, instruction_id, &substitutions);
            }
        }
    }

    // apply substitutions to terminators and parameters
    for &block_id in &function.blocks {
        let block = tree.get(block_id).clone();
        let terminator_id = block.terminator;
        let parameters = block.parameters.clone();
        let instructions = block.instructions.clone();
        let terminator = tree.get(terminator_id).clone();

        // rewrite terminator arguments
        let new_terminator = terminator_substitute_uses(tree, &terminator, &substitutions);
        let new_terminator = remove_arguments_at_indices(tree, &new_terminator, &removed_indices);

        // drop substituted parameters
        let new_parameters: Vec<_> = parameters
            .iter()
            .filter(|p| !substitutions.contains_key(&p.value))
            .cloned()
            .collect();

        // drop substituted instructions
        let new_instructions: Vec<_> = instructions
            .iter()
            .copied()
            .filter(|id| !to_remove.contains(id))
            .collect();

        // replace blocks when terminators or parameters change
        if new_terminator != terminator
            || new_parameters.len() != parameters.len()
            || new_instructions.len() != instructions.len()
        {
            let mut new_block = block;
            new_block.parameters = new_parameters;
            new_block.instructions = new_instructions;
            tree.set(terminator_id, new_terminator);
            tree.set(block_id, new_block);
        }
    }

    true
}

/// Remove arguments at specified indices from terminator's target arguments.
fn remove_arguments_at_indices(
    tree: &mut mir::Tree,
    terminator: &mir::Terminator,
    removed_indices: &HashMap<mir::LocalNodeId<mir::Block>, Vec<usize>>,
) -> mir::Terminator {
    match terminator {
        mir::Terminator::Jump { target } => {
            let (new_arguments, changed) = filter_target_arguments(tree, target, removed_indices);
            if !changed {
                return terminator.clone();
            }

            mir::Terminator::Jump {
                target: mir::BlockTarget::new(target.block, new_arguments),
            }
        }
        mir::Terminator::Branch {
            condition,
            then_target,
            else_target,
        } => {
            let (new_then_args, changed_then) =
                filter_target_arguments(tree, then_target, removed_indices);
            let (new_else_args, changed_else) =
                filter_target_arguments(tree, else_target, removed_indices);
            if changed_then || changed_else {
                mir::Terminator::Branch {
                    condition: *condition,
                    then_target: mir::BlockTarget::new(then_target.block, new_then_args),
                    else_target: mir::BlockTarget::new(else_target.block, new_else_args),
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
            let (new_success_args, changed_success) =
                filter_target_arguments(tree, success, removed_indices);
            let (new_failure_args, changed_failure) =
                filter_target_arguments(tree, failure, removed_indices);
            if changed_success || changed_failure {
                mir::Terminator::Check {
                    constraint: constraint.clone(),
                    success: mir::BlockTarget::new(success.block, new_success_args),
                    failure: mir::BlockTarget::new(failure.block, new_failure_args),
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
            let (new_default_args, changed_default) =
                filter_target_arguments(tree, default, removed_indices);
            let cases = tree.get_switch_cases(*cases).to_vec();
            let mut changed_cases = false;
            let new_cases: Vec<_> = cases
                .iter()
                .map(|case| {
                    let (new_args, changed_case) =
                        filter_target_arguments(tree, &case.target, removed_indices);
                    changed_cases |= changed_case;
                    mir::SwitchCase {
                        value: case.value,
                        target: mir::BlockTarget::new(case.target.block, new_args),
                    }
                })
                .collect();
            if !changed_default && !changed_cases {
                return terminator.clone();
            }
            let new_cases = tree.add_switch_cases(&new_cases);

            mir::Terminator::Switch {
                value: *value,
                default: mir::BlockTarget::new(default.block, new_default_args),
                cases: new_cases,
            }
        }
        mir::Terminator::Yield {
            value,
            resume,
            unwind,
        } => {
            let (new_resume_args, changed_resume) =
                filter_target_arguments(tree, resume, removed_indices);

            let new_unwind = unwind.as_ref().map(|unwind| {
                let (arguments, _) = filter_target_arguments(tree, unwind, removed_indices);

                mir::BlockTarget::new(unwind.block, arguments)
            });

            if changed_resume || new_unwind != *unwind {
                mir::Terminator::Yield {
                    value: *value,
                    resume: mir::BlockTarget::new(resume.block, new_resume_args),
                    unwind: new_unwind,
                }
            } else {
                terminator.clone()
            }
        }
        _ => terminator.clone(),
    }
}

/// Remove arguments from one block target.
fn filter_target_arguments(
    tree: &mut mir::Tree,
    target: &mir::BlockTarget,
    removed_indices: &HashMap<mir::LocalNodeId<mir::Block>, Vec<usize>>,
) -> (mir::ValueSlice, bool) {
    let Some(indices) = removed_indices.get(&target.block) else {
        return (target.arguments, false);
    };

    let arguments = target.arguments(tree);
    let arguments = filter_indices(arguments, indices);
    let arguments = tree.add_values(&arguments);

    (arguments, true)
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
        let input = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    branch v0, b1, b2

b1:
    jump b3(v0)

b2:
    jump b3(v0)

b3(v1: int32):
    return v1
}
"#;

        // v1 is always v0, so replace uses of v1 with v0 and remove the parameter
        let expected = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    branch v0, b1, b2

b1:
    jump b3

b2:
    jump b3

b3:
    return v0
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&CopyPropagate);
        test.assert_output(expected);
    }

    /// Block parameter with different values from predecessors is NOT eliminated.
    #[test]
    fn test_preserve_varying_incoming_values() {
        let input = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 1
    branch v0, b1, b2

b1:
    jump b3(v0)

b2:
    jump b3(v1)

b3(v2: int32):
    return v2
}
"#;

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
entry(v0: int32):
    jump b1(v0)

b1(v1: int32):
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
        test.run_pass(&CopyPropagate);
        test.assert_output(expected);
    }

    /// Chained copies are resolved transitively.
    #[test]
    fn test_propagate_through_chain() {
        let input = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    jump b1(v0)

b1(v1: int32):
    jump b2(v1)

b2(v2: int32):
    return v2
}
"#;

        // v1 = v0, v2 = v1 = v0
        let expected = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    jump b1

b1:
    jump b2

b2:
    return v0
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&CopyPropagate);
        test.assert_output(expected);
    }

    /// Multiple parameters, only some are copies.
    #[test]
    fn test_propagate_partial_copies() {
        let input = r#"
function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    branch v0, b1, b2

b1:
    v2: int32 = 10
    jump b3(v0, v2)

b2:
    v3: int32 = 20
    jump b3(v0, v3)

b3(v4: int32, v5: int32):
    v6: int32 = int.add v4, v5
    return v6
}
"#;

        // v4 is always v0 (copy), but v5 differs between predecessors
        let expected = r#"
function test(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    branch v0, b1, b2

b1:
    v2: int32 = 10
    jump b3(v2)

b2:
    v3: int32 = 20
    jump b3(v3)

b3(v5: int32):
    v6: int32 = int.add v0, v5
    return v6
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&CopyPropagate);
        test.assert_output(expected);
    }

    /// No copies means no changes.
    #[test]
    fn test_preserve_without_copies() {
        let input = r#"
function test(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 1
    v2: int32 = int.add v0, v1
    return v2
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&CopyPropagate);
        test.assert_unchanged(input);
    }

    /// Selects with identical arms are removed.
    #[test]
    fn test_remove_redundant_select() {
        let input = r#"
function test(v0: boolean, v1: int32): int32 {
entry(v0: boolean, v1: int32):
    v2: int32 = select v0, v1, v1
    return v2
}
"#;

        let expected = r#"
function test(v0: boolean, v1: int32): int32 {
entry(v0: boolean, v1: int32):
    return v1
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&CopyPropagate);
        test.assert_output(expected);
    }
}
