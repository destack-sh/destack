use std::collections::HashMap;

use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::optimize::analyses::{ControlFlowGraph, LoopAnalysis, ScalarEvolution, Scev};
use crate::optimize::common::{
    BlockParamForwarding, TypeKey, constant_is_zero, fold_binary,
    instruction_substitute_uses_in_tree, resolve_substitution_chains,
    terminator_arguments_for_successor, terminator_substitute_uses,
};
use crate::optimize::{AnalysisPreservation, FunctionPass, PipelineContext};

declare_pass! {
    /// Simplify redundant induction variables in loop headers.
    ///
    /// Identifies header parameters with identical recurrence patterns and
    /// rewrites uses to the canonical parameter.
    ///
    /// ```mir
    /// function @before() -> i32 {
    /// block0:
    ///     v0 = iconst 0i32
    ///     jump block1(v0, v0)
    /// block1(v1: i32, v2: i32):
    ///     v3 = iadd v1, v2
    ///     v4 = iconst 1i32
    ///     v5 = iadd v1, v4
    ///     v6 = icmp_slt v5, v0
    ///     branch v6, block1(v5, v5), block2(v2)
    /// block2(v7: i32):
    ///     return v7
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function @after() -> i32 {
    /// block0:
    ///     v0 = iconst 0i32
    ///     jump block1(v0)
    /// block1(v1: i32):
    ///     v3 = iadd v1, v1
    ///     v4 = iconst 1i32
    ///     v5 = iadd v1, v4
    ///     v6 = icmp_slt v5, v0
    ///     branch v6, block1(v5), block2(v1)
    /// block2(v7: i32):
    ///     return v7
    /// }
    /// ```
    #[pass(id = "induction-simplify")]
    pub InductionVariableSimplify,
    "Simplify redundant induction variables"
}

impl FunctionPass for InductionVariableSimplify {
    /// Run the induction variable simplification pass.
    fn run(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::NodeTree,
        ctx: &PipelineContext<'_>,
    ) -> AnalysisPreservation {
        // skip imported functions
        if function.entry.is_none() {
            return AnalysisPreservation::all();
        }

        // gather analyses
        let analyses = ctx.function_analyses(function, tree);
        let loops = analyses.get::<LoopAnalysis>().clone();
        let scev = analyses.get::<ScalarEvolution>().clone();
        let cfg = analyses.get::<ControlFlowGraph>().clone();

        // skip when no loops are present
        if loops.num_loops() == 0 {
            return AnalysisPreservation::all();
        }

        // run the simplification pass
        function.recompute_next_value_id(tree);
        let changed = run_induction_simplify(function, tree, &loops, &scev, &cfg);
        if changed {
            AnalysisPreservation::none()
        } else {
            AnalysisPreservation::all()
        }
    }

    /// Return the display name for this pass.
    fn name(&self) -> &'static str {
        "InductionVariableSimplify"
    }

    /// Return the stable id for this pass.
    fn id(&self) -> &'static str {
        "induction-simplify"
    }
}

/// Canonical recurrence entry for deduplication.
#[derive(Debug, Clone)]
struct CanonicalScev {
    /// Recurrence expression.
    scev: Scev,
    /// Type for the recurrence value.
    ty: TypeKey,
    /// Canonical value to keep.
    value: mir::Value,
}

/// Canonical signature entry for deduplication.
#[derive(Debug, Clone)]
struct CanonicalSignature {
    /// Signature of incoming values for this parameter.
    signature: ParamSignature,
    /// Type for the recurrence value.
    ty: TypeKey,
    /// Canonical value to keep.
    value: mir::Value,
}

/// Signature for a header parameter's incoming arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ParamSignature {
    /// Arguments keyed by predecessor block.
    arguments: Vec<(mir::LocalNodeId<mir::Block>, mir::Value)>,
}

/// Run induction variable simplification for a function.
fn run_induction_simplify(
    function: &mut mir::Function,
    tree: &mut mir::NodeTree,
    loops: &LoopAnalysis,
    scev: &ScalarEvolution,
    cfg: &ControlFlowGraph,
) -> bool {
    // collect substitutions for redundant induction variables
    let mut substitutions: HashMap<mir::Value, mir::Value> = HashMap::new();

    // collect header instruction insertions
    let mut header_inserts: HashMap<
        mir::LocalNodeId<mir::Block>,
        Vec<mir::LocalNodeId<mir::Instruction>>,
    > = HashMap::new();

    // build forwarding information for header parameters
    let forwarding = BlockParamForwarding::build(function, tree, cfg);

    // scan loops for redundant recurrences
    for (loop_index, lp) in loops.loops().iter().enumerate() {
        // read header parameters
        let header_parameters = tree.get(lp.header).parameters.clone();

        // skip headers without parameters
        if header_parameters.is_empty() {
            continue;
        }

        // track canonical signatures and recurrences per loop
        let mut canonical_signatures: Vec<CanonicalSignature> = Vec::new();
        let mut canonical_scevs: Vec<CanonicalScev> = Vec::new();

        // scan header parameters
        for (param_index, param) in header_parameters.iter().enumerate() {
            // derive a structural type key for comparisons
            let param_type = TypeKey::from_type(param.ty, tree);

            // resolve the parameter signature
            let signature = param_signature(lp.header, param_index, tree, cfg, &forwarding);

            // resolve the recurrence key for the parameter
            let scev_key = scev
                .scev_for_value_in_loop(loop_index, param.value)
                .and_then(|scev_value| {
                    let scev_value = scev_value.clone();
                    let Scev::AddRec { loop_header, .. } = &scev_value else {
                        return None;
                    };
                    if *loop_header != lp.header {
                        return None;
                    }
                    Some(scev_value)
                });

            // find a canonical value using the signature
            let signature_match = signature.as_ref().and_then(|signature| {
                canonical_signatures.iter().find_map(|entry| {
                    if entry.ty == param_type && entry.signature == *signature {
                        Some(entry.value)
                    } else {
                        None
                    }
                })
            });

            // fall back to recurrence equivalence when needed
            let scev_match = if signature_match.is_none() {
                scev_key.as_ref().and_then(|scev_key| {
                    canonical_scevs.iter().find_map(|entry| {
                        if entry.ty == param_type && entry.scev == *scev_key {
                            Some(entry.value)
                        } else {
                            None
                        }
                    })
                })
            } else {
                None
            };

            // attempt to match affine offsets when recurrences differ by a constant
            let offset_match = if signature_match.is_none() && scev_match.is_none() {
                scev_key.as_ref().and_then(|scev_key| {
                    canonical_scevs.iter().find_map(|entry| {
                        if entry.ty != param_type {
                            return None;
                        }

                        let offset = affine_offset_for_scev(&entry.scev, scev_key)?;
                        if constant_is_zero(Some(&offset)) {
                            return None;
                        }

                        Some((entry.value, offset))
                    })
                })
            } else {
                None
            };

            // record substitutions for redundant parameters
            let canonical_value = if let Some((base_value, offset)) = offset_match {
                insert_offset_value(
                    function,
                    tree,
                    lp.header,
                    base_value,
                    offset,
                    &mut header_inserts,
                )
            } else {
                signature_match.or(scev_match).unwrap_or(param.value)
            };
            if canonical_value != param.value {
                substitutions.insert(param.value, canonical_value);
            }

            // record canonical signature entries
            if let Some(signature) = signature {
                let has_signature = canonical_signatures
                    .iter()
                    .any(|entry| entry.ty == param_type && entry.signature == signature);
                if !has_signature {
                    canonical_signatures.push(CanonicalSignature {
                        signature,
                        ty: param_type.clone(),
                        value: canonical_value,
                    });
                }
            }

            // record canonical recurrence entries
            if let Some(scev_key) = scev_key {
                let has_scev = canonical_scevs
                    .iter()
                    .any(|entry| entry.ty == param_type && entry.scev == scev_key);
                if !has_scev {
                    canonical_scevs.push(CanonicalScev {
                        scev: scev_key,
                        ty: param_type,
                        value: canonical_value,
                    });
                }
            }
        }
    }

    // insert derived offset instructions at loop headers
    if !header_inserts.is_empty() {
        let mut headers: Vec<_> = header_inserts.keys().copied().collect();
        headers.sort();

        for header_id in headers {
            let inserts = header_inserts.remove(&header_id).unwrap_or_default();
            if inserts.is_empty() {
                continue;
            }

            let mut header_block = tree.get(header_id).clone();
            let mut new_instructions = inserts;
            new_instructions.extend(header_block.instructions.iter().copied());
            header_block.instructions = new_instructions;
            tree.replace(header_id, header_block);
        }
    }

    // nothing to do
    if substitutions.is_empty() {
        return false;
    }

    // resolve substitution chains
    let substitutions = resolve_substitution_chains(substitutions);

    // collect removed parameter indices
    let mut removed_indices: HashMap<mir::LocalNodeId<mir::Block>, Vec<usize>> = HashMap::new();
    for &block_id in &function.blocks {
        // collect indices for parameters that will be removed
        let block = tree.get(block_id);
        let indices: Vec<usize> = block
            .parameters
            .iter()
            .enumerate()
            .filter_map(|(index, param)| {
                if substitutions.contains_key(&param.value) {
                    Some(index)
                } else {
                    None
                }
            })
            .collect();

        // track blocks with removable parameters
        if !indices.is_empty() {
            removed_indices.insert(block_id, indices);
        }
    }

    // apply substitutions to instructions
    for &block_id in &function.blocks {
        // collect instruction ids to avoid borrow issues
        let instruction_ids: Vec<_> = tree.get(block_id).instructions.clone();

        // rewrite instruction operands
        for instruction_id in instruction_ids {
            let instruction = tree.get(instruction_id).clone();
            let new_instruction =
                instruction_substitute_uses_in_tree(&instruction, &substitutions, tree);

            // replace instructions that changed
            if new_instruction != instruction {
                tree.replace(instruction_id, new_instruction);
            }
        }
    }

    // apply substitutions to terminators and parameters
    for &block_id in &function.blocks {
        // read the current block
        let block = tree.get(block_id);

        // rewrite terminator uses
        let new_terminator = terminator_substitute_uses(&block.terminator, &substitutions);

        // drop removed parameter arguments
        let new_terminator = remove_arguments_at_indices(&new_terminator, &removed_indices);

        // filter out removed parameters
        let new_parameters: Vec<_> = block
            .parameters
            .iter()
            .filter(|param| !substitutions.contains_key(&param.value))
            .cloned()
            .collect();

        // replace blocks that changed
        if new_terminator != block.terminator || new_parameters.len() != block.parameters.len() {
            let mut new_block = block.clone();
            new_block.terminator = new_terminator;
            new_block.parameters = new_parameters;
            tree.replace(block_id, new_block);
        }
    }

    true
}

/// Compute constant offsets between two affine recurrences.
fn affine_offset_for_scev(base: &Scev, candidate: &Scev) -> Option<mir::Constant> {
    // require matching add recurrences
    let Scev::AddRec {
        start: base_start,
        step: base_step,
        loop_header: base_header,
    } = base
    else {
        return None;
    };
    let Scev::AddRec {
        start: cand_start,
        step: cand_step,
        loop_header: cand_header,
    } = candidate
    else {
        return None;
    };

    // require a shared loop header and step
    if base_header != cand_header || base_step.as_ref() != cand_step.as_ref() {
        return None;
    }

    // require constant starts
    let base_const = scev_constant(base_start)?;
    let cand_const = scev_constant(cand_start)?;

    // compute the offset between starts
    fold_binary(
        mir::BinaryOperator::Subtract,
        cand_const.clone(),
        base_const.clone(),
    )
}

/// Extract a constant from a SCEV expression.
fn scev_constant(scev: &Scev) -> Option<&mir::Constant> {
    match scev {
        Scev::Constant(constant) => Some(constant),
        _ => None,
    }
}

/// Insert an offset adjustment at a loop header.
fn insert_offset_value(
    function: &mut mir::Function,
    tree: &mut mir::NodeTree,
    header: mir::LocalNodeId<mir::Block>,
    base_value: mir::Value,
    offset: mir::Constant,
    header_inserts: &mut HashMap<
        mir::LocalNodeId<mir::Block>,
        Vec<mir::LocalNodeId<mir::Instruction>>,
    >,
) -> mir::Value {
    // materialize the offset constant
    let const_value = function.next_typed_value_like(base_value);
    let const_instruction = mir::Instruction::Const {
        destination: const_value,
        value: offset,
    };
    let const_id = tree.insert(const_instruction);

    // materialize the adjusted value
    let adjusted_value = function.next_typed_value_like(base_value);
    let add_instruction = mir::Instruction::Binary {
        destination: adjusted_value,
        operator: mir::BinaryOperator::Add,
        left: base_value,
        right: const_value,
    };
    let add_id = tree.insert(add_instruction);

    // schedule instructions for insertion
    let inserts = header_inserts.entry(header).or_default();
    inserts.push(const_id);
    inserts.push(add_id);

    adjusted_value
}

/// Remove arguments at specified indices from terminator targets.
fn remove_arguments_at_indices(
    terminator: &mir::Terminator,
    removed_indices: &HashMap<mir::LocalNodeId<mir::Block>, Vec<usize>>,
) -> mir::Terminator {
    // rewrite terminators that target blocks with removed parameters
    match terminator {
        mir::Terminator::Jump { target, arguments } => {
            // update jump arguments when needed
            if let Some(indices) = removed_indices.get(target) {
                let new_args = filter_indices(arguments, indices);
                mir::Terminator::Jump {
                    target: *target,
                    arguments: new_args,
                }
            } else {
                terminator.clone()
            }
        }
        mir::Terminator::Branch {
            condition,
            then_target,
            then_arguments,
            else_target,
            else_arguments,
        } => {
            // update then arguments when needed
            let new_then_args = if let Some(indices) = removed_indices.get(then_target) {
                filter_indices(then_arguments, indices)
            } else {
                then_arguments.clone()
            };

            // update else arguments when needed
            let new_else_args = if let Some(indices) = removed_indices.get(else_target) {
                filter_indices(else_arguments, indices)
            } else {
                else_arguments.clone()
            };

            // rebuild the branch when arguments changed
            if new_then_args != *then_arguments || new_else_args != *else_arguments {
                mir::Terminator::Branch {
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
        mir::Terminator::Check {
            condition,
            constraint,
            success,
            failure,
        } => {
            // update success arguments when needed
            let new_success_args = if let Some(indices) = removed_indices.get(&success.target) {
                filter_indices(&success.arguments, indices)
            } else {
                success.arguments.clone()
            };

            // update failure arguments when needed
            let new_failure_args = if let Some(indices) = removed_indices.get(&failure.target) {
                filter_indices(&failure.arguments, indices)
            } else {
                failure.arguments.clone()
            };

            // rebuild the check when arguments changed
            if new_success_args != success.arguments || new_failure_args != failure.arguments {
                mir::Terminator::Check {
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
        mir::Terminator::Switch {
            value,
            default,
            default_arguments,
            cases,
        } => {
            // update default arguments when needed
            let new_default_args = if let Some(indices) = removed_indices.get(default) {
                filter_indices(default_arguments, indices)
            } else {
                default_arguments.clone()
            };

            // update case arguments when needed
            let mut new_cases = Vec::new();
            for case in cases {
                let new_args = if let Some(indices) = removed_indices.get(&case.target) {
                    filter_indices(&case.arguments, indices)
                } else {
                    case.arguments.clone()
                };

                new_cases.push(mir::SwitchCase {
                    value: case.value,
                    target: case.target,
                    arguments: new_args,
                });
            }

            // rebuild the switch when arguments changed
            if new_default_args != *default_arguments || new_cases != *cases {
                mir::Terminator::Switch {
                    value: *value,
                    default: *default,
                    default_arguments: new_default_args,
                    cases: new_cases,
                }
            } else {
                terminator.clone()
            }
        }
        mir::Terminator::Yield {
            value,
            resume,
            resume_arguments,
        } => {
            // update resume arguments when needed
            if let Some(indices) = removed_indices.get(resume) {
                let new_args = filter_indices(resume_arguments, indices);
                mir::Terminator::Yield {
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

/// Filter out values at the specified indices.
fn filter_indices(values: &[mir::Value], indices_to_remove: &[usize]) -> Vec<mir::Value> {
    // collect values that are not removed
    let mut filtered = Vec::with_capacity(values.len());

    for (index, value) in values.iter().enumerate() {
        // skip values at removed indices
        if indices_to_remove.contains(&index) {
            continue;
        }

        filtered.push(*value);
    }

    filtered
}

/// Build a header parameter signature based on predecessor arguments.
fn param_signature(
    header: mir::LocalNodeId<mir::Block>,
    param_index: usize,
    tree: &mir::NodeTree,
    cfg: &ControlFlowGraph,
    forwarding: &BlockParamForwarding,
) -> Option<ParamSignature> {
    // collect argument values from each predecessor
    let mut arguments = Vec::new();
    for &pred in cfg.predecessors(header) {
        let args = terminator_arguments_for_successor(&tree.get(pred).terminator, header);
        let arg = *args.get(param_index)?;
        let arg = forwarding.resolve(arg);
        arguments.push((pred, arg));
    }

    // ensure stable ordering for comparisons
    arguments.sort_by_key(|(pred, _)| *pred);

    Some(ParamSignature { arguments })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Redundant induction parameters are removed.
    #[test]
    fn test_simplify_redundant_induction_params() {
        // source test
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1: i32 = iconst 0i32
    jump block1(v1, v1)
block1(v2: i32, v3: i32):
    v4: i32 = iadd v2, v3
    v5: i32 = iconst 1i32
    v6: i32 = iadd v2, v5
    v7: bool = icmp_slt v6, v0
    branch v7, block1(v6, v6), block2(v3)
block2(v8: i32):
    return v8
}"#;

        // expected output
        let expected = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1: i32 = iconst 0i32
    jump block1(v1)
block1(v2: i32):
    v3: i32 = iadd v2, v2
    v4: i32 = iconst 1i32
    v5: i32 = iadd v2, v4
    v6: bool = icmp_slt v5, v0
    branch v6, block1(v5), block2(v2)
block2(v7: i32):
    return v7
}"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&InductionVariableSimplify);
        test.assert_output(expected);
    }

    /// Distinct induction parameters are preserved.
    #[test]
    fn test_preserve_distinct_induction_params() {
        // source test
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1: i32 = iconst 0i32
    v2: i32 = iconst 1i32
    v3: i32 = iconst 2i32
    jump block1(v1, v1)
block1(v4: i32, v5: i32):
    v6: i32 = iadd v4, v2
    v7: i32 = iadd v5, v3
    v8: bool = icmp_slt v6, v0
    branch v8, block1(v6, v7), block2(v5)
block2(v9: i32):
    return v9
}"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&InductionVariableSimplify);
        test.assert_unchanged(input);
    }

    /// Multiple redundant induction parameters are merged.
    #[test]
    fn test_simplify_multiple_redundant_params() {
        // source test
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1: i32 = iconst 0i32
    jump block1(v1, v1, v1)
block1(v2: i32, v3: i32, v4: i32):
    v5: i32 = iadd v2, v3
    v6: i32 = iadd v3, v4
    v7: i32 = iconst 1i32
    v8: i32 = iadd v2, v7
    v9: bool = icmp_slt v8, v0
    branch v9, block1(v8, v8, v8), block2(v4)
block2(v10: i32):
    return v10
}"#;

        // expected output
        let expected = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1: i32 = iconst 0i32
    jump block1(v1)
block1(v2: i32):
    v3: i32 = iadd v2, v2
    v4: i32 = iadd v2, v2
    v5: i32 = iconst 1i32
    v6: i32 = iadd v2, v5
    v7: bool = icmp_slt v6, v0
    branch v7, block1(v6), block2(v2)
block2(v8: i32):
    return v8
}"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&InductionVariableSimplify);
        test.assert_output(expected);
    }

    /// Forwarded predecessor arguments still permit signature matching.
    #[test]
    fn test_simplify_forwarded_signature_match() {
        // source test
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1: i32 = iconst 0i32
    jump block1(v1, v1)
block1(v2: i32, v3: i32):
    jump block2(v2, v3)
block2(v4: i32, v5: i32):
    v6: i32 = iadd v4, v5
    v7: i32 = iconst 1i32
    v8: i32 = iadd v4, v7
    v9: bool = icmp_slt v8, v0
    branch v9, block2(v8, v8), block3(v5)
block3(v10: i32):
    return v10
}"#;

        // expected output
        let expected = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1: i32 = iconst 0i32
    jump block1(v1, v1)
block1(v2: i32, v3: i32):
    jump block2(v2)
block2(v4: i32):
    v5: i32 = iadd v4, v4
    v6: i32 = iconst 1i32
    v7: i32 = iadd v4, v6
    v8: bool = icmp_slt v7, v0
    branch v8, block2(v7), block3(v4)
block3(v9: i32):
    return v9
}"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&InductionVariableSimplify);
        test.assert_output(expected);
    }

    /// Recurrences with different types are preserved.
    #[test]
    fn test_preserve_different_typed_recurrences() {
        // source test
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1: i32 = iconst 0i32
    v2: u32 = iconst 0u32
    v3: i32 = iconst 1i32
    v4: u32 = iconst 1u32
    jump block1(v1, v2)
block1(v5: i32, v6: u32):
    v7: i32 = iadd v5, v3
    v8: u32 = iadd v6, v4
    v9: bool = icmp_slt v7, v0
    branch v9, block1(v7, v8), block2(v5)
block2(v10: i32):
    return v10
}"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&InductionVariableSimplify);
        test.assert_unchanged(input);
    }

    /// Redundant parameters are removed from check terminators.
    #[test]
    fn test_simplify_check_terminator() {
        // source test
        let input = r#"function @test(v0: [i32; 4]) -> void {
block0(v0: [i32; 4]):
    v1: u32 = iconst 0u32
    v2: u32 = iconst 1u32
    v3: u32 = iconst 4u32
    jump block1(v1, v1)
block1(v4: u32, v5: u32):
    v6: u32 = iadd v4, v2
    v7: bool = icmp_ult v6, v3
    check v7, bounds.unsigned v6, v3, v0, block1(v6, v6), block2
block2:
    return
}"#;

        // expected output
        let expected = r#"function @test(v0: [i32; 4]) -> void {
block0(v0: [i32; 4]):
    v1: u32 = iconst 0u32
    v2: u32 = iconst 1u32
    v3: u32 = iconst 4u32
    jump block1(v1)
block1(v4: u32):
    v5: u32 = iadd v4, v2
    v6: bool = icmp_ult v5, v3
    check v6, bounds.unsigned v5, v3, v0, block1(v5), block2
block2:
    return
}"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&InductionVariableSimplify);
        test.assert_output(expected);
    }

    /// Redundant parameters are removed from switch terminators.
    #[test]
    fn test_simplify_switch_terminator() {
        // source test
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1: i32 = iconst 0i32
    v2: i32 = iconst 1i32
    jump block1(v1, v1)
block1(v3: i32, v4: i32):
    v5: i32 = iadd v3, v2
    v6: i32 = iadd v4, v2
    v7: bool = icmp_slt v5, v0
    switch v7, block2, 0 => block1(v5, v6), 1 => block2
block2:
    return v3
}"#;

        // expected output
        let expected = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1: i32 = iconst 0i32
    v2: i32 = iconst 1i32
    jump block1(v1)
block1(v3: i32):
    v4: i32 = iadd v3, v2
    v5: i32 = iadd v3, v2
    v6: bool = icmp_slt v4, v0
    switch v6, block2, 0 => block1(v4), 1 => block2
block2:
    return v3
}"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&InductionVariableSimplify);
        test.assert_output(expected);
    }

    /// Affine offset induction variables are rewritten to a canonical base.
    #[test]
    fn test_simplify_affine_offset_induction() {
        // source test
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1: i32 = iconst 0i32
    v2: i32 = iconst 1i32
    v3: i32 = iconst 4i32
    jump block1(v1, v2)
block1(v4: i32, v5: i32):
    v6: i32 = iadd v4, v2
    v7: i32 = iadd v5, v2
    v8: bool = icmp_slt v6, v3
    branch v8, block1(v6, v7), block2(v5)
block2(v9: i32):
    return v9
}"#;
        // expected output
        let expected = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1: i32 = iconst 0i32
    v2: i32 = iconst 1i32
    v3: i32 = iconst 4i32
    jump block1(v1)
block1(v4: i32):
    v5: i32 = iconst 1i32
    v6: i32 = iadd v4, v5
    v7: i32 = iadd v4, v2
    v8: i32 = iadd v6, v2
    v9: bool = icmp_slt v7, v3
    branch v9, block1(v7), block2(v6)
block2(v10: i32):
    return v10
}"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&InductionVariableSimplify);
        test.assert_output(expected);
    }

    /// Equivalent recurrences merge even with distinct latch values.
    #[test]
    fn test_simplify_equivalent_recurrence_distinct_latch_values() {
        // source test
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1: i32 = iconst 0i32
    v2: i32 = iconst 1i32
    jump block1(v1, v1)
block1(v3: i32, v4: i32):
    v5: i32 = iadd v3, v2
    v6: i32 = iadd v4, v2
    v7: bool = icmp_slt v5, v0
    branch v7, block1(v5, v6), block2(v4)
block2(v8: i32):
    return v8
}"#;

        // expected output
        let expected = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1: i32 = iconst 0i32
    v2: i32 = iconst 1i32
    jump block1(v1)
block1(v3: i32):
    v4: i32 = iadd v3, v2
    v5: i32 = iadd v3, v2
    v6: bool = icmp_slt v4, v0
    branch v6, block1(v4), block2(v3)
block2(v7: i32):
    return v7
}"#;

        // run the pass and verify output
        let mut test = TestProgram::new(input);
        test.run_pass(&InductionVariableSimplify);
        test.assert_output(expected);
    }

    /// Parameter signatures identify equivalent header arguments.
    #[test]
    fn test_param_signature_equivalence() {
        // source test
        let input = r#"function @test(v0: i32) -> i32 {
block0(v0: i32):
    v1: i32 = iconst 0i32
    jump block1(v1, v1)
block1(v2: i32, v3: i32):
    v4: i32 = iadd v2, v3
    v5: i32 = iconst 1i32
    v6: i32 = iadd v2, v5
    v7: bool = icmp_slt v6, v0
    branch v7, block1(v6, v6), block2(v3)
block2(v8: i32):
    return v8
}"#;

        // resolve header signatures
        let test = TestProgram::new(input);
        let function_id = test.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = test.tree.get(function_id);
        let analyses = test.function_analyses(function);
        let cfg = analyses.get::<ControlFlowGraph>();
        let loops = analyses.get::<LoopAnalysis>();
        let forwarding = BlockParamForwarding::build(function, &test.tree, &cfg);
        let header = function.blocks[1];
        let header_block = test.tree.get(header);
        let param_left = header_block.parameters[0];
        let param_right = header_block.parameters[1];
        let signature_left = param_signature(header, 0, &test.tree, &cfg, &forwarding);
        let signature_right = param_signature(header, 1, &test.tree, &cfg, &forwarding);

        assert!(signature_left.is_some());
        assert_eq!(signature_left, signature_right);
        assert_eq!(loops.num_loops(), 1);
        assert_eq!(loops.loops()[0].header, header);

        let canonical_signatures = [CanonicalSignature {
            signature: signature_left.unwrap(),
            ty: TypeKey::from_type(param_left.ty, &test.tree),
            value: param_left.value,
        }];
        let canonical_value = signature_right.and_then(|signature| {
            let param_right_ty = TypeKey::from_type(param_right.ty, &test.tree);
            canonical_signatures.iter().find_map(|entry| {
                if entry.ty == param_right_ty && entry.signature == signature {
                    Some(entry.value)
                } else {
                    None
                }
            })
        });
        assert_eq!(canonical_value, Some(param_left.value));
    }
}
