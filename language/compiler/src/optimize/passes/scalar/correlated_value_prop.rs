use std::collections::HashMap;

use destack_compiler_macros::declare_pass;
use destack_mir as mir;

use crate::optimize::analyses::{ConstantPropagation, ControlFlowGraph, DominatorTree};
use crate::optimize::common::{
    build_use_def_maps, instruction_substitute_uses_in_tree, terminator_substitute_uses,
};
use crate::optimize::{AnalysisPreservation, FunctionAnalyses, FunctionPass, PipelineContext};

declare_pass! {
    /// Propagate equalities implied by dominating conditions.
    ///
    /// When a branch condition proves two values are equal, this pass replaces
    /// uses of one value with the other within the dominated region.
    ///
    /// ```mir
    /// function @before(v0: i32, v1: i32) -> i32 {
    /// block0(v0: i32, v1: i32):
    ///     v2 = icmp_eq v0, v1
    ///     branch v2, block1, block2
    /// block1:
    ///     v3 = iadd v0, v1
    ///     jump block3(v3)
    /// block2:
    ///     v4 = isub v0, v1
    ///     jump block3(v4)
    /// block3(v5: i32):
    ///     return v5
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function @after(v0: i32, v1: i32) -> i32 {
    /// block0(v0: i32, v1: i32):
    ///     v2 = icmp_eq v0, v1
    ///     branch v2, block1, block2
    /// block1:
    ///     v3 = iadd v0, v0
    ///     jump block3(v3)
    /// block2:
    ///     v4 = isub v0, v1
    ///     jump block3(v4)
    /// block3(v5: i32):
    ///     return v5
    /// }
    /// ```
    ///
    /// Restrictions:
    /// - Only propagates integer and pointer equality
    /// - Does not propagate float equality due to NaN and signed zero
    #[pass(id = "correlated-value-prop")]
    pub CorrelatedValueProp,
    "Propagate correlated values from dominating conditions"
}

impl FunctionPass for CorrelatedValueProp {
    fn run(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::NodeTree,
        _ctx: &PipelineContext<'_>,
    ) -> AnalysisPreservation {
        // skip imported functions
        if function.entry.is_none() {
            return AnalysisPreservation::all();
        }

        // gather analyses
        let analyses = FunctionAnalyses::new(function, tree);
        let domtree = analyses.get::<DominatorTree>().clone();
        let cfg = analyses.get::<ControlFlowGraph>().clone();
        let constants = analyses.get::<ConstantPropagation>().clone();

        // run correlated propagation
        let changed = run_correlated_value_prop(function, tree, &domtree, &cfg, &constants);
        if changed {
            AnalysisPreservation::none()
        } else {
            AnalysisPreservation::all()
        }
    }

    fn name(&self) -> &'static str {
        "CorrelatedValueProp"
    }

    fn id(&self) -> &'static str {
        "correlated-value-prop"
    }
}

// NOTE #Incomplete: extend to range constraints

/// Propagate equalities implied by conditional branches.
fn run_correlated_value_prop(
    function: &mir::Function,
    tree: &mut mir::NodeTree,
    domtree: &DominatorTree,
    cfg: &ControlFlowGraph,
    constants: &ConstantPropagation,
) -> bool {
    // build definition map for dominance checks
    let use_def = build_use_def_maps(function, tree);

    // build a lookup for condition instructions
    let value_to_instruction = build_value_instruction_map(function, tree);

    // track changes across the function
    let mut changed = false;

    // scan each block for equality conditions
    for &block_id in &function.blocks {
        // read the terminator to find a branch or check
        let block = tree.get(block_id);
        let (condition, then_target, else_target) = match &block.terminator {
            mir::Terminator::Branch {
                condition,
                then_target,
                else_target,
                ..
            } => (*condition, *then_target, *else_target),
            mir::Terminator::Check {
                condition,
                success,
                failure,
                ..
            } => (*condition, success.target, failure.target),
            _ => continue,
        };

        // extract an equality condition
        let Some(equality) = equality_condition(condition, &value_to_instruction) else {
            continue;
        };

        // decide which successor is the equality path
        let equality_block = if equality.is_equal_on_then {
            then_target
        } else {
            else_target
        };

        // require the equality block to be reached only from this branch
        if !is_single_predecessor(cfg, equality_block, block_id) {
            continue;
        }

        // use constant operands when available
        let constant_substitution =
            constant_substitution(constants, block_id, equality.left, equality.right);

        // pick a canonical replacement value
        let (canonical, replace) = if let Some((canonical, replace)) = constant_substitution {
            (canonical, replace)
        } else {
            let Some((canonical, replace)) = choose_replacement(
                function.entry,
                equality.left,
                equality.right,
                equality_block,
                &use_def.def_block,
                domtree,
            ) else {
                continue;
            };

            (canonical, replace)
        };

        // avoid self substitution
        if canonical == replace {
            continue;
        }

        // apply substitutions in dominated blocks
        let mut substitutions = HashMap::new();
        substitutions.insert(replace, canonical);
        let applied = apply_substitutions(function, tree, domtree, equality_block, &substitutions);
        changed |= applied;
    }

    changed
}

/// A branch condition that implies equality along one successor.
struct EqualityCondition {
    /// Left hand value of the comparison.
    left: mir::Value,
    /// Right hand value of the comparison.
    right: mir::Value,
    /// True when equality holds on the then edge.
    is_equal_on_then: bool,
}

/// Extract equality information from a condition value.
fn equality_condition(
    condition: mir::Value,
    value_to_instruction: &HashMap<mir::Value, mir::Instruction>,
) -> Option<EqualityCondition> {
    // look up the defining instruction
    let instruction = value_to_instruction.get(&condition)?;

    // handle direct comparisons
    if let mir::Instruction::Binary {
        operator,
        left,
        right,
        ..
    } = instruction
    {
        return match operator {
            mir::BinaryOperator::Equal => Some(EqualityCondition {
                left: *left,
                right: *right,
                is_equal_on_then: true,
            }),
            mir::BinaryOperator::NotEqual => Some(EqualityCondition {
                left: *left,
                right: *right,
                is_equal_on_then: false,
            }),
            _ => None,
        };
    }

    // handle logical negation of comparisons
    if let mir::Instruction::Unary {
        operator: mir::UnaryOperator::Not,
        argument,
        ..
    } = instruction
        && let Some(nested) = value_to_instruction.get(argument)
        && let mir::Instruction::Binary {
            operator,
            left,
            right,
            ..
        } = nested
    {
        return match operator {
            mir::BinaryOperator::Equal => Some(EqualityCondition {
                left: *left,
                right: *right,
                is_equal_on_then: false,
            }),
            mir::BinaryOperator::NotEqual => Some(EqualityCondition {
                left: *left,
                right: *right,
                is_equal_on_then: true,
            }),
            _ => None,
        };
    }

    None
}

/// Choose the value to substitute within a dominated block.
fn choose_replacement(
    entry: Option<mir::LocalNodeId<mir::Block>>,
    left: mir::Value,
    right: mir::Value,
    block: mir::LocalNodeId<mir::Block>,
    def_blocks: &HashMap<mir::Value, mir::LocalNodeId<mir::Block>>,
    domtree: &DominatorTree,
) -> Option<(mir::Value, mir::Value)> {
    // resolve definition blocks with entry fallback
    let entry_block = entry?;
    let left_def = def_blocks.get(&left).copied().unwrap_or(entry_block);
    let right_def = def_blocks.get(&right).copied().unwrap_or(entry_block);

    // determine availability in the dominated region
    let left_available = domtree.dominates(left_def, block);
    let right_available = domtree.dominates(right_def, block);

    // select a replacement value when available
    match (left_available, right_available) {
        (true, true) => {
            if left.0 <= right.0 {
                Some((left, right))
            } else {
                Some((right, left))
            }
        }
        (true, false) => Some((left, right)),
        (false, true) => Some((right, left)),
        (false, false) => None,
    }
}

/// Apply a substitution map to all blocks dominated by the root.
fn apply_substitutions(
    function: &mir::Function,
    tree: &mut mir::NodeTree,
    domtree: &DominatorTree,
    root: mir::LocalNodeId<mir::Block>,
    substitutions: &HashMap<mir::Value, mir::Value>,
) -> bool {
    // track the set of blocks to visit
    let mut changed = false;
    let mut blocks = Vec::new();
    for &block_id in &function.blocks {
        if domtree.dominates(root, block_id) {
            blocks.push(block_id);
        }
    }

    // apply substitutions in each dominated block
    for block_id in blocks {
        let mut block = tree.get(block_id).clone();

        // rewrite instructions with new uses
        for &instruction_id in &block.instructions {
            let instruction = tree.get(instruction_id).clone();
            let updated = instruction_substitute_uses_in_tree(&instruction, substitutions, tree);
            if updated != instruction {
                tree.replace(instruction_id, updated);
                changed = true;
            }
        }

        // rewrite terminator uses
        let new_terminator = terminator_substitute_uses(&block.terminator, substitutions);
        if new_terminator != block.terminator {
            block.terminator = new_terminator;
            tree.replace(block_id, block);
            changed = true;
        }
    }

    changed
}

/// Build a map from values to their defining instructions.
fn build_value_instruction_map(
    function: &mir::Function,
    tree: &mir::NodeTree,
) -> HashMap<mir::Value, mir::Instruction> {
    // collect all instruction definitions
    let mut map = HashMap::new();
    for &block_id in &function.blocks {
        let block = tree.get(block_id);
        for &instruction_id in &block.instructions {
            let instruction = tree.get(instruction_id);
            if let Some(destination) = instruction.destination() {
                map.insert(destination, instruction.clone());
            }
        }
    }

    map
}

/// Pick a constant operand as the canonical value when safe.
fn constant_substitution(
    constants: &ConstantPropagation,
    block_id: mir::LocalNodeId<mir::Block>,
    left: mir::Value,
    right: mir::Value,
) -> Option<(mir::Value, mir::Value)> {
    // read the constant map for this block
    let block_constants = constants.exit(block_id);
    let left_constant = block_constants.get(left);
    let right_constant = block_constants.get(right);

    // prefer the constant operand when only one side is constant
    match (left_constant, right_constant) {
        (Some(constant), None) if constant_is_integer_like(constant) => Some((left, right)),
        (None, Some(constant)) if constant_is_integer_like(constant) => Some((right, left)),
        _ => None,
    }
}

/// Check whether a constant is safe to propagate as an equality operand.
fn constant_is_integer_like(constant: &mir::Constant) -> bool {
    // restrict to integer like scalars for equality propagation
    matches!(
        constant,
        mir::Constant::Boolean { .. }
            | mir::Constant::Int { .. }
            | mir::Constant::UInt { .. }
            | mir::Constant::Char { .. }
    )
}

/// Check whether a block has a single predecessor and it matches the expected block.
fn is_single_predecessor(
    cfg: &ControlFlowGraph,
    block: mir::LocalNodeId<mir::Block>,
    expected: mir::LocalNodeId<mir::Block>,
) -> bool {
    // read predecessor list
    let predecessors = cfg.predecessors(block);

    // require all predecessors to be the expected block
    !predecessors.is_empty() && predecessors.iter().all(|pred| *pred == expected)
}

#[cfg(test)]
mod tests {
    use crate::optimize::common::tests::TestProgram;
    use crate::optimize::passes::CorrelatedValueProp;

    /// Equality branches substitute the dominated value.
    #[test]
    fn test_cvp_substitutes_equal_values() {
        let input = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = icmp_eq v0, v1
    branch v2, block1, block2
block1:
    v3 = iadd v0, v1
    jump block3(v3)
block2:
    v4 = isub v0, v1
    jump block3(v4)
block3(v5: i32):
    return v5
}"#;
        let expected = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = icmp_eq v0, v1
    branch v2, block1, block2
block1:
    v3 = iadd v0, v0
    jump block3(v3)
block2:
    v4 = isub v0, v1
    jump block3(v4)
block3(v5: i32):
    return v5
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&CorrelatedValueProp);
        program.assert_output(expected);
    }

    /// Not equal conditions propagate equality on the false edge.
    #[test]
    fn test_cvp_inverts_not_equal() {
        let input = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = icmp_ne v0, v1
    branch v2, block1, block2
block1:
    v3 = iadd v0, v1
    return v3
block2:
    v4 = isub v0, v1
    return v4
}"#;
        let expected = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = icmp_ne v0, v1
    branch v2, block1, block2
block1:
    v3 = iadd v0, v1
    return v3
block2:
    v4 = isub v0, v0
    return v4
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&CorrelatedValueProp);
        program.assert_output(expected);
    }

    /// Constant equalities substitute the non constant operand.
    #[test]
    fn test_cvp_prefers_constant_operand() {
        let input = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = iconst 7i32
    v3 = icmp_eq v0, v2
    branch v3, block1, block2
block1:
    v4 = iadd v0, v1
    return v4
block2:
    return v0
}"#;
        let expected = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = iconst 7i32
    v3 = icmp_eq v0, v2
    branch v3, block1, block2
block1:
    v4 = iadd v2, v1
    return v4
block2:
    return v0
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&CorrelatedValueProp);
        program.assert_output(expected);
    }

    /// Float equality is not substituted.
    #[test]
    fn test_cvp_skips_float_equal() {
        let input = r#"function @test(v0: f64, v1: f64) -> f64 {
block0(v0: f64, v1: f64):
    v2 = fcmp_eq v0, v1
    branch v2, block1, block2
block1:
    v3 = fadd v0, v1
    return v3
block2:
    v4 = fsub v0, v1
    return v4
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&CorrelatedValueProp);
        program.assert_output(input);
    }

    /// Substitution flows into blocks dominated by the equality edge.
    #[test]
    fn test_cvp_propagates_into_dominated_blocks() {
        let input = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = icmp_eq v0, v1
    branch v2, block1, block2
block1:
    jump block3
block2:
    return v0
block3:
    v3 = iadd v0, v1
    return v3
}"#;
        let expected = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = icmp_eq v0, v1
    branch v2, block1, block2
block1:
    jump block3
block2:
    return v0
block3:
    v3 = iadd v0, v0
    return v3
}"#;

        let mut program = TestProgram::new(input);
        program.run_pass(&CorrelatedValueProp);
        program.assert_output(expected);
    }

    /// Negated equality conditions substitute on the else edge.
    #[test]
    fn test_cvp_handles_negated_equal() {
        // source program
        let input = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = icmp_eq v0, v1
    v3 = bnot v2
    branch v3, block1, block2
block1:
    v4 = isub v0, v1
    return v4
block2:
    v5 = iadd v0, v1
    return v5
}"#;
        // expected output
        let expected = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = icmp_eq v0, v1
    v3 = bnot v2
    branch v3, block1, block2
block1:
    v4 = isub v0, v1
    return v4
block2:
    v5 = iadd v0, v0
    return v5
}"#;

        // run the pass and verify output
        let mut program = TestProgram::new(input);
        program.run_pass(&CorrelatedValueProp);
        program.assert_output(expected);
    }

    /// Check terminators propagate equality on the success edge.
    #[test]
    fn test_cvp_handles_check_terminator() {
        // source program
        let input = r#"function @test(v0: u32, v1: u32, v2: [u32; 4]) -> u32 {
block0(v0: u32, v1: u32, v2: [u32; 4]):
    v3 = icmp_eq v0, v1
    check v3, bounds.unsigned v0, v1, v2, block1, block2
block1:
    v4 = iadd v0, v1
    return v4
block2:
    return v0
}"#;
        // expected output
        let expected = r#"function @test(v0: u32, v1: u32, v2: [u32; 4]) -> u32 {
block0(v0: u32, v1: u32, v2: [u32; 4]):
    v3 = icmp_eq v0, v1
    check v3, bounds.unsigned v0, v1, v2, block1, block2
block1:
    v4 = iadd v0, v0
    return v4
block2:
    return v0
}"#;

        // run the pass and verify output
        let mut program = TestProgram::new(input);
        program.run_pass(&CorrelatedValueProp);
        program.assert_output(expected);
    }

    /// Equality blocks with extra predecessors are not substituted.
    #[test]
    fn test_cvp_requires_single_predecessor() {
        // source program
        let input = r#"function @test(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2 = icmp_eq v0, v1
    branch v2, block1, block2
block1:
    v3 = iadd v0, v1
    return v3
block2:
    jump block1
}"#;

        // run the pass and verify output
        let mut program = TestProgram::new(input);
        program.run_pass(&CorrelatedValueProp);
        program.assert_output(input);
    }
}
