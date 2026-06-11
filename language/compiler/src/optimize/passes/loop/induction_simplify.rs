use std::collections::HashMap;

use crate::declare_mir_pass;
use destack_mir as mir;

use crate::common::mir::analysis::{ControlFlowGraph, LoopAnalysis, ScalarEvolution, Scev};
use crate::common::mir::{
    BlockParamForwarding, TypeKey, constant_is_zero, fold_binary,
    instruction_substitute_uses_in_tree, remap_instruction_memory_accesses,
    resolve_substitution_chains, terminator_arguments_for_successor, terminator_substitute_uses,
};
use crate::optimize::{AnalysisPreservation, FunctionPass, PipelineContext};

declare_mir_pass! {
    /// Simplify redundant induction variables in loop headers.
    ///
    /// Identifies header parameters with identical recurrence patterns and
    /// rewrites uses to the canonical parameter.
    ///
    /// ```mir
    /// function before(): int32 {
    /// b0:
    ///     v0 = 0int32
    ///     jump b1(v0, v0)
    /// b1(v1: int32, v2: int32):
    ///     v3 = int.add v1, v2
    ///     v4 = 1int32
    ///     v5 = int.add v1, v4
    ///     v6 = int.lt.s v5, v0
    ///     branch v6, b1(v5, v5), b2(v2)
    /// b2(v7: int32):
    ///     return v7
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function after(): int32 {
    /// b0:
    ///     v0 = 0int32
    ///     jump b1(v0)
    /// b1(v1: int32):
    ///     v3 = int.add v1, v1
    ///     v4 = 1int32
    ///     v5 = int.add v1, v4
    ///     v6 = int.lt.s v5, v0
    ///     branch v6, b1(v5), b2(v1)
    /// b2(v7: int32):
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
        tree: &mut mir::Tree,
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
    tree: &mut mir::Tree,
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
            let Some(param_value) = param.value.value() else {
                continue;
            };

            let Some(param_type) = param.ty.ty().map(|ty| TypeKey::from_type(ty, tree)) else {
                continue;
            };

            // derive a structural type key for comparisons
            let signature = param_signature(lp.header, param_index, tree, cfg, &forwarding);

            // resolve the recurrence key for the parameter
            let scev_key = scev
                .scev_for_value_in_loop(loop_index, param_value)
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
                signature_match.or(scev_match).unwrap_or(param_value)
            };
            if canonical_value != param_value {
                substitutions.insert(param_value, canonical_value);
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
            tree.set(header_id, header_block);
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
                if param
                    .value
                    .value()
                    .is_some_and(|value| substitutions.contains_key(&value))
                {
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
                tree.set(instruction_id, new_instruction);
                remap_instruction_memory_accesses(tree, instruction_id, &substitutions);
            }
        }
    }

    // apply substitutions to terminators and parameters
    for &block_id in &function.blocks {
        // read the current block
        let block = tree.get(block_id);
        let terminator_id = block.terminator;
        let terminator = tree.get(terminator_id).clone();

        // rewrite terminator uses
        let new_terminator = terminator_substitute_uses(&terminator, &substitutions);

        // drop removed parameter arguments
        let new_terminator = remove_arguments_at_indices(&new_terminator, &removed_indices);

        // filter out removed parameters
        let new_parameters: Vec<_> = block
            .parameters
            .iter()
            .filter(|param| {
                !param
                    .value
                    .value()
                    .is_some_and(|value| substitutions.contains_key(&value))
            })
            .cloned()
            .collect();

        // replace blocks that changed
        if new_terminator != terminator || new_parameters.len() != block.parameters.len() {
            let mut new_block = block.clone();
            new_block.parameters = new_parameters;
            tree.set(block_id, new_block);
            tree.set(terminator_id, new_terminator);
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
    tree: &mut mir::Tree,
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
        destination: const_value.into(),
        value: offset,
    };
    let const_id = tree.insert(const_instruction);

    // materialize the adjusted value
    let adjusted_value = function.next_typed_value_like(base_value);
    let add_instruction = mir::Instruction::Binary {
        destination: adjusted_value.into(),
        operator: mir::BinaryOperator::Add,
        left: base_value.into(),
        right: const_value.into(),
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
        mir::Terminator::Jump { target } => {
            // update jump arguments when needed
            let Some(target_block) = target.block.block() else {
                return terminator.clone();
            };

            if let Some(indices) = removed_indices.get(&target_block) {
                let new_args = filter_indices(&target.arguments, indices);
                mir::Terminator::Jump {
                    target: mir::BlockTarget {
                        block: target.block,
                        arguments: new_args,
                    },
                }
            } else {
                terminator.clone()
            }
        }
        mir::Terminator::Branch {
            condition,
            then_target,
            else_target,
        } => {
            // update then arguments when needed
            let new_then_args = if let Some(indices) = then_target
                .block
                .block()
                .and_then(|block| removed_indices.get(&block))
            {
                filter_indices(&then_target.arguments, indices)
            } else {
                then_target.arguments.clone()
            };

            // update else arguments when needed
            let new_else_args = if let Some(indices) = else_target
                .block
                .block()
                .and_then(|block| removed_indices.get(&block))
            {
                filter_indices(&else_target.arguments, indices)
            } else {
                else_target.arguments.clone()
            };

            // rebuild the branch when arguments changed
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
            // update success arguments when needed
            let new_success_args = if let Some(indices) = success
                .block
                .block()
                .and_then(|block| removed_indices.get(&block))
            {
                filter_indices(&success.arguments, indices)
            } else {
                success.arguments.clone()
            };

            // update failure arguments when needed
            let new_failure_args = if let Some(indices) = failure
                .block
                .block()
                .and_then(|block| removed_indices.get(&block))
            {
                filter_indices(&failure.arguments, indices)
            } else {
                failure.arguments.clone()
            };

            // rebuild the check when arguments changed
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
            // update default arguments when needed
            let new_default_args = if let Some(indices) = default
                .block
                .block()
                .and_then(|block| removed_indices.get(&block))
            {
                filter_indices(&default.arguments, indices)
            } else {
                default.arguments.clone()
            };

            // update case arguments when needed
            let mut new_cases = Vec::new();
            for case in cases {
                let new_args = if let Some(indices) = case
                    .target
                    .block
                    .block()
                    .and_then(|block| removed_indices.get(&block))
                {
                    filter_indices(&case.target.arguments, indices)
                } else {
                    case.target.arguments.clone()
                };

                new_cases.push(mir::SwitchCase {
                    value: case.value,
                    target: mir::BlockTarget {
                        block: case.target.block,
                        arguments: new_args,
                    },
                });
            }

            // rebuild the switch when arguments changed
            if new_default_args != default.arguments || new_cases != *cases {
                mir::Terminator::Switch {
                    value: *value,
                    default: mir::BlockTarget {
                        block: default.block,
                        arguments: new_default_args,
                    },
                    cases: new_cases,
                }
            } else {
                terminator.clone()
            }
        }
        mir::Terminator::Yield {
            value,
            resume,
            unwind,
        } => {
            let new_resume_args = if let Some(indices) = resume
                .block
                .block()
                .and_then(|block| removed_indices.get(&block))
            {
                filter_indices(&resume.arguments, indices)
            } else {
                resume.arguments.clone()
            };

            let new_unwind = unwind.as_ref().map(|unwind| {
                let arguments = if let Some(indices) = unwind
                    .block
                    .block()
                    .and_then(|block| removed_indices.get(&block))
                {
                    filter_indices(&unwind.arguments, indices)
                } else {
                    unwind.arguments.clone()
                };

                mir::BlockTarget {
                    block: unwind.block,
                    arguments,
                }
            });

            if new_resume_args != resume.arguments || new_unwind != *unwind {
                mir::Terminator::Yield {
                    value: *value,
                    resume: mir::BlockTarget {
                        block: resume.block,
                        arguments: new_resume_args,
                    },
                    unwind: new_unwind,
                }
            } else {
                terminator.clone()
            }
        }
        _ => terminator.clone(),
    }
}

/// Filter out values at the specified indices.
fn filter_indices(
    values: &[mir::ValueReference],
    indices_to_remove: &[usize],
) -> Vec<mir::ValueReference> {
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
    tree: &mir::Tree,
    cfg: &ControlFlowGraph,
    forwarding: &BlockParamForwarding,
) -> Option<ParamSignature> {
    // collect argument values from each predecessor
    let mut arguments = Vec::new();
    for &pred in cfg.predecessors(header) {
        let pred_block = tree.get(pred);
        let pred_terminator = tree.get(pred_block.terminator);
        let args = terminator_arguments_for_successor(pred_terminator, header);
        let arg = *args.get(param_index)?;
        let arg = forwarding.resolve(arg.value()?);
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
        let input = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 0int32
    jump b1(v1, v1)
b1(v2: int32, v3: int32):
    v4: int32 = int.add v2, v3
    v5: int32 = 1int32
    v6: int32 = int.add v2, v5
    v7: boolean = int.lt.s v6, v0
    branch v7, b1(v6, v6), b2(v3)
b2(v8: int32):
    return v8
}"#;

        // expected output
        let expected = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 0int32
    jump b1(v1)
b1(v2: int32):
    v3: int32 = int.add v2, v2
    v4: int32 = 1int32
    v5: int32 = int.add v2, v4
    v6: boolean = int.lt.s v5, v0
    branch v6, b1(v5), b2(v2)
b2(v7: int32):
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
        let input = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 0int32
    v2: int32 = 1int32
    v3: int32 = 2int32
    jump b1(v1, v1)
b1(v4: int32, v5: int32):
    v6: int32 = int.add v4, v2
    v7: int32 = int.add v5, v3
    v8: boolean = int.lt.s v6, v0
    branch v8, b1(v6, v7), b2(v5)
b2(v9: int32):
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
        let input = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 0int32
    jump b1(v1, v1, v1)
b1(v2: int32, v3: int32, v4: int32):
    v5: int32 = int.add v2, v3
    v6: int32 = int.add v3, v4
    v7: int32 = 1int32
    v8: int32 = int.add v2, v7
    v9: boolean = int.lt.s v8, v0
    branch v9, b1(v8, v8, v8), b2(v4)
b2(v10: int32):
    return v10
}"#;

        // expected output
        let expected = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 0int32
    jump b1(v1)
b1(v2: int32):
    v3: int32 = int.add v2, v2
    v4: int32 = int.add v2, v2
    v5: int32 = 1int32
    v6: int32 = int.add v2, v5
    v7: boolean = int.lt.s v6, v0
    branch v7, b1(v6), b2(v2)
b2(v8: int32):
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
        let input = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 0int32
    jump b1(v1, v1)
b1(v2: int32, v3: int32):
    jump b2(v2, v3)
b2(v4: int32, v5: int32):
    v6: int32 = int.add v4, v5
    v7: int32 = 1int32
    v8: int32 = int.add v4, v7
    v9: boolean = int.lt.s v8, v0
    branch v9, b2(v8, v8), b3(v5)
b3(v10: int32):
    return v10
}"#;

        // expected output
        let expected = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 0int32
    jump b1(v1, v1)
b1(v2: int32, v3: int32):
    jump b2(v2)
b2(v4: int32):
    v5: int32 = int.add v4, v4
    v6: int32 = 1int32
    v7: int32 = int.add v4, v6
    v8: boolean = int.lt.s v7, v0
    branch v8, b2(v7), b3(v4)
b3(v9: int32):
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
        let input = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 0int32
    v2: uint32 = 0uint32
    v3: int32 = 1int32
    v4: uint32 = 1uint32
    jump b1(v1, v2)
b1(v5: int32, v6: uint32):
    v7: int32 = int.add v5, v3
    v8: uint32 = int.add v6, v4
    v9: boolean = int.lt.s v7, v0
    branch v9, b1(v7, v8), b2(v5)
b2(v10: int32):
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
        let input = r#"
function test(v0: [int32; 4]): void {
b0(v0: [int32; 4]):
    v1: uint32 = 0uint32
    v2: uint32 = 1uint32
    v3: uint32 = 4uint32
    jump b1(v1, v1)
b1(v4: uint32, v5: uint32):
    v6: uint32 = int.add v4, v2
    v7: boolean = int.lt.u v6, v3
    check bounds.u v6, v3, v0 -> b1(v6, v6), b2
b2:
    return
}"#;

        // expected output
        let expected = r#"
function test(v0: [int32; 4]): void {
b0(v0: [int32; 4]):
    v1: uint32 = 0uint32
    v2: uint32 = 1uint32
    v3: uint32 = 4uint32
    jump b1(v1)
b1(v4: uint32):
    v5: uint32 = int.add v4, v2
    v6: boolean = int.lt.u v5, v3
    check bounds.u v5, v3, v0 -> b1(v5), b2
b2:
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
        let input = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 0int32
    v2: int32 = 1int32
    jump b1(v1, v1)
b1(v3: int32, v4: int32):
    v5: int32 = int.add v3, v2
    v6: int32 = int.add v4, v2
    v7: boolean = int.lt.s v5, v0
    switch v7, b2, 0 => b1(v5, v6), 1 => b2
b2:
    return v3
}"#;

        // expected output
        let expected = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 0int32
    v2: int32 = 1int32
    jump b1(v1)
b1(v3: int32):
    v4: int32 = int.add v3, v2
    v5: int32 = int.add v3, v2
    v6: boolean = int.lt.s v4, v0
    switch v6, b2, 0 => b1(v4), 1 => b2
b2:
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
        let input = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 0int32
    v2: int32 = 1int32
    v3: int32 = 4int32
    jump b1(v1, v2)
b1(v4: int32, v5: int32):
    v6: int32 = int.add v4, v2
    v7: int32 = int.add v5, v2
    v8: boolean = int.lt.s v6, v3
    branch v8, b1(v6, v7), b2(v5)
b2(v9: int32):
    return v9
}"#;
        // expected output
        let expected = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 0int32
    v2: int32 = 1int32
    v3: int32 = 4int32
    jump b1(v1)
b1(v4: int32):
    v5: int32 = 1int32
    v6: int32 = int.add v4, v5
    v7: int32 = int.add v4, v2
    v8: int32 = int.add v6, v2
    v9: boolean = int.lt.s v7, v3
    branch v9, b1(v7), b2(v6)
b2(v10: int32):
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
        let input = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 0int32
    v2: int32 = 1int32
    jump b1(v1, v1)
b1(v3: int32, v4: int32):
    v5: int32 = int.add v3, v2
    v6: int32 = int.add v4, v2
    v7: boolean = int.lt.s v5, v0
    branch v7, b1(v5, v6), b2(v4)
b2(v8: int32):
    return v8
}"#;

        // expected output
        let expected = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 0int32
    v2: int32 = 1int32
    jump b1(v1)
b1(v3: int32):
    v4: int32 = int.add v3, v2
    v5: int32 = int.add v3, v2
    v6: boolean = int.lt.s v4, v0
    branch v6, b1(v4), b2(v3)
b2(v7: int32):
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
        let input = r#"
function test(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = 0int32
    jump b1(v1, v1)
b1(v2: int32, v3: int32):
    v4: int32 = int.add v2, v3
    v5: int32 = 1int32
    v6: int32 = int.add v2, v5
    v7: boolean = int.lt.s v6, v0
    branch v7, b1(v6, v6), b2(v3)
b2(v8: int32):
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
        let param_left = &header_block.parameters[0];
        let param_right = &header_block.parameters[1];
        let signature_left = param_signature(header, 0, &test.tree, &cfg, &forwarding);
        let signature_right = param_signature(header, 1, &test.tree, &cfg, &forwarding);

        assert!(signature_left.is_some());
        assert_eq!(signature_left, signature_right);
        assert_eq!(loops.num_loops(), 1);
        assert_eq!(loops.loops()[0].header, header);

        let canonical_signatures = [CanonicalSignature {
            signature: signature_left.unwrap(),
            ty: TypeKey::from_type(
                param_left
                    .ty
                    .ty()
                    .expect("left parameter type should be concrete"),
                &test.tree,
            ),
            value: param_left
                .value
                .value()
                .expect("left parameter value should be concrete"),
        }];
        let canonical_value = signature_right.and_then(|signature| {
            let param_right_ty = TypeKey::from_type(
                param_right
                    .ty
                    .ty()
                    .expect("right parameter type should be concrete"),
                &test.tree,
            );
            canonical_signatures.iter().find_map(|entry| {
                if entry.ty == param_right_ty && entry.signature == signature {
                    Some(entry.value)
                } else {
                    None
                }
            })
        });
        assert_eq!(
            canonical_value,
            Some(
                param_left
                    .value
                    .value()
                    .expect("left parameter value should be concrete"),
            )
        );
    }
}
