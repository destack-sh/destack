use crate::optimize::declare_pass;
use destack_mir as mir;

use crate::optimize::{FunctionPass, PipelineContext};
use destack_mir::{Mutation, RangeAnalysis, constraint_truth_value};

declare_pass! {
    /// Eliminate redundant guard checks when conditions are proven.
    ///
    /// Uses control flow facts, assume instructions, and range analysis to
    /// remove checks that are guaranteed to take one edge.
    ///
    /// ```mir
    /// function before(v0: uint32, v1: uint32, v2: [uint32; 4]): uint32 {
    /// b0(v0: uint32, v1: uint32, v2: [uint32; 4]):
    ///     v3 = int.eq v0, v1
    ///     branch v3, b1, b2
    /// b1:
    ///     check bounds.u v0, v1, v2 -> b3, b4
    /// b3:
    ///     return v0
    /// b4:
    ///     unreachable
    /// b2:
    ///     return v1
    /// }
    /// ```
    /// becomes:
    /// ```mir
    /// function after(v0: uint32, v1: uint32, v2: [uint32; 4]): uint32 {
    /// b0(v0: uint32, v1: uint32, v2: [uint32; 4]):
    ///     v3 = int.eq v0, v1
    ///     branch v3, b1, b2
    /// b1:
    ///     jump b3
    /// b3:
    ///     return v0
    /// b4:
    ///     unreachable
    /// b2:
    ///     return v1
    /// }
    /// ```
    #[pass(id = "guard-eliminate")]
    pub GuardEliminate,
    "Eliminate redundant guard checks"
}

impl FunctionPass for GuardEliminate {
    /// Run guard elimination on a function.
    fn run(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::Tree,
        _ctx: &PipelineContext<'_>,
        analyses: &mir::FunctionAnalyses,
    ) -> Mutation {
        // skip imported functions
        let Some(_entry) = function.entry else {
            return Mutation::NONE;
        };

        // gather analyses
        let ranges = analyses.get::<RangeAnalysis>(function, tree).clone();

        // scan blocks for eliminable checks
        let mut changed = false;
        for &block_id in &function.blocks {
            // read the terminator
            let block = tree.get(block_id);
            let terminator = tree.get(block.terminator).clone();
            let mir::Terminator::Check {
                constraint,
                success,
                failure,
            } = terminator
            else {
                continue;
            };

            let block_ranges = ranges.exit(block_id);
            let check_outcome = constraint_truth_value(&constraint, block_ranges);

            // rewrite the terminator when the outcome is known
            if let Some(is_true) = check_outcome {
                let target = if is_true { &success } else { &failure };
                replace_check_with_jump(tree, block_id, target.clone());
                changed = true;
            }
        }

        // report what this pass changed
        if changed {
            Mutation::CONTROL_FLOW
        } else {
            Mutation::NONE
        }
    }

    /// Return the pass name.
    fn name(&self) -> &'static str {
        "GuardEliminate"
    }

    /// Return the pass id.
    fn id(&self) -> &'static str {
        "guard-eliminate"
    }
}

/// Replace a check terminator with a direct jump.
fn replace_check_with_jump(
    tree: &mut mir::Tree,
    block_id: mir::LocalNodeId<mir::Block>,
    target: mir::BlockTarget,
) {
    // build a jump terminator replacement
    let block = tree.get(block_id);
    let new_terminator = mir::Terminator::Jump { target };
    tree.set(block.terminator, new_terminator);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Dominating branch conditions eliminate redundant checks.
    #[test]
    fn test_guard_eliminate_branch_facts() {
        let input = r#"
function test(value0: uint32, value1: uint32, value2: [uint32; 4]): uint32 {
entry0(value0: uint32, value1: uint32, value2: [uint32; 4]):
    value3: boolean = int.eq value0, value1
    branch value3, block1(), block2()

block1:
    check bounds.u value0, value1, value2 -> block3(), block4()

block2:
    return value1

block3:
    return value0

block4:
    unreachable
}
"#;
        let expected = r#"
function test(value0: uint32, value1: uint32, value2: [uint32; 4]): uint32 {
entry0(value0: uint32, value1: uint32, value2: [uint32; 4]):
    value3: boolean = int.eq value0, value1
    branch value3, block1(), block2()

block1:
    check bounds.u value0, value1, value2 -> block3(), block4()

block2:
    return value1

block3:
    return value0

block4:
    unreachable
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&GuardEliminate);
        test.assert_output(expected);
    }

    /// Assume instructions feed redundant checks.
    #[test]
    fn test_guard_eliminate_assume_fact() {
        let input = r#"
function test(value0: uint32, value1: uint32, value2: [uint32; 4]): uint32 {
entry0(value0: uint32, value1: uint32, value2: [uint32; 4]):
    value3: boolean = int.eq value0, value1
    assume value3
    check bounds.u value0, value1, value2 -> block1(), block2()

block1:
    return value0

block2:
    unreachable
}
"#;
        let expected = r#"
function test(value0: uint32, value1: uint32, value2: [uint32; 4]): uint32 {
entry0(value0: uint32, value1: uint32, value2: [uint32; 4]):
    value3: boolean = int.eq value0, value1
    assume value3
    check bounds.u value0, value1, value2 -> block1(), block2()

block1:
    return value0

block2:
    unreachable
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&GuardEliminate);
        test.assert_output(expected);
    }

    /// Constant conditions eliminate checks.
    #[test]
    fn test_guard_eliminate_constant_condition() {
        let input = r#"
function test(value0: uint32, value1: uint32, value2: [uint32; 4]): uint32 {
entry0(value0: uint32, value1: uint32, value2: [uint32; 4]):
    value3: boolean = false
    check bounds.u value0, value1, value2 -> block1(), block2()

block1:
    return value0

block2:
    return value1
}
"#;
        let expected = r#"
function test(value0: uint32, value1: uint32, value2: [uint32; 4]): uint32 {
entry0(value0: uint32, value1: uint32, value2: [uint32; 4]):
    value3: boolean = false
    check bounds.u value0, value1, value2 -> block1(), block2()

block1:
    return value0

block2:
    return value1
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&GuardEliminate);
        test.assert_output(expected);
    }

    /// Negated conditions are resolved using edge facts.
    #[test]
    fn test_guard_eliminate_negated_condition() {
        let input = r#"
function test(value0: uint32, value1: uint32, value2: [uint32; 4]): uint32 {
entry0(value0: uint32, value1: uint32, value2: [uint32; 4]):
    value3: boolean = int.eq value0, value1
    value4: boolean = int.not value3
    branch value3, block1(), block2()

block1:
    return value0

block2:
    check bounds.u value0, value1, value2 -> block3(), block4()

block3:
    return value1

block4:
    unreachable
}
"#;
        let expected = r#"
function test(value0: uint32, value1: uint32, value2: [uint32; 4]): uint32 {
entry0(value0: uint32, value1: uint32, value2: [uint32; 4]):
    value3: boolean = int.eq value0, value1
    value4: boolean = int.not value3
    branch value3, block1(), block2()

block1:
    return value0

block2:
    check bounds.u value0, value1, value2 -> block3(), block4()

block3:
    return value1

block4:
    unreachable
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&GuardEliminate);
        test.assert_output(expected);
    }

    /// Condition facts transfer through block parameters.
    #[test]
    fn test_guard_eliminate_block_param_condition() {
        let input = r#"
function test(value0: uint32, value1: uint32, value2: [uint32; 4]): uint32 {
entry0(value0: uint32, value1: uint32, value2: [uint32; 4]):
    value3: boolean = int.eq value0, value1
    branch value3, block1(value3), block2(value3)

block1(value4: boolean):
    check bounds.u value0, value1, value2 -> block3(), block4()

block2(value5: boolean):
    return value1

block3:
    return value0

block4:
    unreachable
}
"#;
        let expected = r#"
function test(value0: uint32, value1: uint32, value2: [uint32; 4]): uint32 {
entry0(value0: uint32, value1: uint32, value2: [uint32; 4]):
    value3: boolean = int.eq value0, value1
    branch value3, block1(value3), block2(value3)

block1(value4: boolean):
    check bounds.u value0, value1, value2 -> block3(), block4()

block2(value5: boolean):
    return value1

block3:
    return value0

block4:
    unreachable
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&GuardEliminate);
        test.assert_output(expected);
    }

    /// Check edges propagate condition facts to successors.
    #[test]
    fn test_guard_eliminate_check_edge_fact() {
        let input = r#"
function test(value0: boolean, value1: uint32, value2: uint32, value3: [uint32; 4]): uint32 {
entry0(value0: boolean, value1: uint32, value2: uint32, value3: [uint32; 4]):
    check bounds.u value1, value2, value3 -> block1(), block2()

block1:
    check bounds.u value1, value2, value3 -> block3(), block4()

block2:
    return value2

block3:
    return value1

block4:
    unreachable
}
"#;
        let expected = r#"
function test(value0: boolean, value1: uint32, value2: uint32, value3: [uint32; 4]): uint32 {
entry0(value0: boolean, value1: uint32, value2: uint32, value3: [uint32; 4]):
    check bounds.u value1, value2, value3 -> block1(), block2()

block1:
    check bounds.u value1, value2, value3 -> block3(), block4()

block2:
    return value2

block3:
    return value1

block4:
    unreachable
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&GuardEliminate);
        test.assert_output(expected);
    }

    /// Bounds constraints eliminate checks when always in range.
    #[test]
    fn test_guard_eliminate_bounds_constraint_success() {
        let input = r#"
function test(value0: boolean, value1: [uint32; 4]): uint32 {
entry0(value0: boolean, value1: [uint32; 4]):
    value2: uint32 = 2uint32
    value3: uint32 = 4uint32
    check bounds.u value2, value3, value1 -> block1(), block2()

block1:
    return value2

block2:
    unreachable
}
"#;
        let expected = r#"
function test(value0: boolean, value1: [uint32; 4]): uint32 {
entry0(value0: boolean, value1: [uint32; 4]):
    value2: uint32 = 2uint32
    value3: uint32 = 4uint32
    jump block1()

block1:
    return value2

block2:
    unreachable
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&GuardEliminate);
        test.assert_output(expected);
    }

    /// Bounds constraints jump to failure when always out of range.
    #[test]
    fn test_guard_eliminate_bounds_constraint_failure() {
        let input = r#"
function test(value0: boolean, value1: [uint32; 0]): uint32 {
entry0(value0: boolean, value1: [uint32; 0]):
    value2: uint32 = 0uint32
    value3: uint32 = 0uint32
    check bounds.u value2, value3, value1 -> block1(), block2()

block1:
    unreachable

block2:
    return value2
}
"#;
        let expected = r#"
function test(value0: boolean, value1: [uint32; 0]): uint32 {
entry0(value0: boolean, value1: [uint32; 0]):
    value2: uint32 = 0uint32
    value3: uint32 = 0uint32
    jump block2()

block1:
    unreachable

block2:
    return value2
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&GuardEliminate);
        test.assert_output(expected);
    }

    /// Div zero constraints eliminate checks with non zero divisors.
    #[test]
    fn test_guard_eliminate_div_zero_constraint_success() {
        let input = r#"
function test(value0: boolean): int32 {
entry0(value0: boolean):
    value1: int32 = 4int32
    check zeroDivisor value1 -> block1(), block2()

block1:
    return value1

block2:
    unreachable
}
"#;
        let expected = r#"
function test(value0: boolean): int32 {
entry0(value0: boolean):
    value1: int32 = 4int32
    jump block1()

block1:
    return value1

block2:
    unreachable
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&GuardEliminate);
        test.assert_output(expected);
    }

    /// Div zero constraints eliminate checks with zero divisors.
    #[test]
    fn test_guard_eliminate_div_zero_constraint_failure() {
        let input = r#"
function test(value0: boolean): int32 {
entry0(value0: boolean):
    value1: int32 = 0int32
    check zeroDivisor value1 -> block1(), block2()

block1:
    unreachable

block2:
    return value1
}
"#;
        let expected = r#"
function test(value0: boolean): int32 {
entry0(value0: boolean):
    value1: int32 = 0int32
    jump block2()

block1:
    unreachable

block2:
    return value1
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&GuardEliminate);
        test.assert_output(expected);
    }

    /// Shift range constraints eliminate checks with in range shifts.
    #[test]
    fn test_guard_eliminate_shift_constraint_success() {
        let input = r#"
function test(value0: boolean): uint8 {
entry0(value0: boolean):
    value1: uint8 = 3uint8
    check shiftRange.u value1, 8 -> block1(), block2()

block1:
    return value1

block2:
    unreachable
}
"#;
        let expected = r#"
function test(value0: boolean): uint8 {
entry0(value0: boolean):
    value1: uint8 = 3uint8
    jump block1()

block1:
    return value1

block2:
    unreachable
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&GuardEliminate);
        test.assert_output(expected);
    }

    /// Shift range constraints jump to failure on out of range shifts.
    #[test]
    fn test_guard_eliminate_shift_constraint_failure() {
        let input = r#"
function test(value0: boolean): uint8 {
entry0(value0: boolean):
    value1: uint8 = 8uint8
    check shiftRange.u value1, 8 -> block1(), block2()

block1:
    unreachable

block2:
    return value1
}
"#;
        let expected = r#"
function test(value0: boolean): uint8 {
entry0(value0: boolean):
    value1: uint8 = 8uint8
    jump block2()

block1:
    unreachable

block2:
    return value1
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&GuardEliminate);
        test.assert_output(expected);
    }

    /// Narrow constraints eliminate checks for values in range.
    #[test]
    fn test_guard_eliminate_narrow_constraint_success() {
        let input = r#"
function test(value0: boolean): uint16 {
entry0(value0: boolean):
    value1: uint16 = 12uint16
    check narrowRange.u value1, 8 -> block1(), block2()

block1:
    return value1

block2:
    unreachable
}
"#;
        let expected = r#"
function test(value0: boolean): uint16 {
entry0(value0: boolean):
    value1: uint16 = 12uint16
    jump block1()

block1:
    return value1

block2:
    unreachable
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&GuardEliminate);
        test.assert_output(expected);
    }

    /// Narrow constraints jump to failure for out of range values.
    #[test]
    fn test_guard_eliminate_narrow_constraint_failure() {
        let input = r#"
function test(value0: boolean): uint16 {
entry0(value0: boolean):
    value1: uint16 = 300uint16
    check narrowRange.u value1, 8 -> block1(), block2()

block1:
    unreachable

block2:
    return value1
}
"#;
        let expected = r#"
function test(value0: boolean): uint16 {
entry0(value0: boolean):
    value1: uint16 = 300uint16
    jump block2()

block1:
    unreachable

block2:
    return value1
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&GuardEliminate);
        test.assert_output(expected);
    }

    /// Overflow constraints eliminate checks when no overflow is possible.
    #[test]
    fn test_guard_eliminate_overflow_constraint_success() {
        let input = r#"
function test(value0: boolean): int8 {
entry0(value0: boolean):
    value1: int8 = 1int8
    value2: int8 = 2int8
    check int.add.overflow.s value1, value2 -> block1(), block2()

block1:
    return value1

block2:
    unreachable
}
"#;
        let expected = r#"
function test(value0: boolean): int8 {
entry0(value0: boolean):
    value1: int8 = 1int8
    value2: int8 = 2int8
    jump block1()

block1:
    return value1

block2:
    unreachable
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&GuardEliminate);
        test.assert_output(expected);
    }

    /// Overflow constraints jump to failure when overflow is guaranteed.
    #[test]
    fn test_guard_eliminate_overflow_constraint_failure() {
        let input = r#"
function test(value0: boolean): int8 {
entry0(value0: boolean):
    value1: int8 = 120int8
    value2: int8 = 120int8
    check int.add.overflow.s value1, value2 -> block1(), block2()

block1:
    unreachable

block2:
    return value1
}
"#;
        let expected = r#"
function test(value0: boolean): int8 {
entry0(value0: boolean):
    value1: int8 = 120int8
    value2: int8 = 120int8
    jump block2()

block1:
    unreachable

block2:
    return value1
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&GuardEliminate);
        test.assert_output(expected);
    }
}
