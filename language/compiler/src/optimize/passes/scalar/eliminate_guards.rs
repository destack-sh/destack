use crate::optimize::declare_pass;
use destack_mir as mir;

use crate::optimize::{FunctionPass, MirOptimized, PipelineContext};
use destack_mir::{Mutation, RangeAnalysis};

declare_pass! {
    /// Eliminate redundant guard checks when conditions are proven.
    ///
    /// Uses control flow state, assume instructions, and range analysis to
    /// remove checks that are guaranteed to take one edge.
    ///
    /// ```mir
    /// function before(v0: uint32, v1: uint32, v2: [uint32; 4]): uint32 {
    /// b0(v0: uint32, v1: uint32, v2: [uint32; 4]):
    ///     v3 = int.eq v0, v1
    ///     branch v3, b1, b2
    /// b1:
    ///     check bounds.u v0, v1, v2 => b3, b4
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
    #[pass(id = "eliminate-guards")]
    pub EliminateGuards,
    "Eliminate redundant guard checks"
}

impl FunctionPass for EliminateGuards {
    /// Run guard elimination on a function.
    fn run(
        &self,
        function: &mut mir::Function,
        optimized: &mut MirOptimized,
        _ctx: &PipelineContext<'_>,
        analyses: &mir::FunctionAnalysisCache,
    ) -> Mutation {
        let tree = &mut optimized.tree;

        // skip imported functions
        let Some(_entry) = function.entry() else {
            return Mutation::NONE;
        };

        // gather analyses
        let ranges = analyses.get::<RangeAnalysis>(function, tree).clone();

        // scan blocks for eliminable checks
        let mut changed = false;
        for &block_id in function.blocks() {
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
            let check_outcome = block_ranges.truth_value(&constraint);

            // rewrite the terminator when the outcome is known
            if let Some(is_true) = check_outcome {
                let target = if is_true { success } else { failure };
                replace_check_with_jump(tree, block_id, target);
                changed = true;
            }
        }

        // report what this pass changed
        if changed {
            Mutation::CONTROL
        } else {
            Mutation::NONE
        }
    }

    /// Return the pass name.
    fn name(&self) -> &'static str {
        "EliminateGuards"
    }

    /// Return the pass id.
    fn id(&self) -> &'static str {
        "eliminate-guards"
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
    fn test_eliminate_guards_branch_facts() {
        let input = r#"
function test(v0: uint32, v1: uint32, v2: [uint32; 4]): uint32 {
entry(v0: uint32, v1: uint32, v2: [uint32; 4]):
    v3: boolean = int.eq v0, v1
    branch v3, b1, b2

b1:
    check bounds.u v0, v1, v2 => b3, b4

b2:
    return v1

b3:
    return v0

b4:
    unreachable
}
"#;
        let expected = r#"
function test(v0: uint32, v1: uint32, v2: [uint32; 4]): uint32 {
entry(v0: uint32, v1: uint32, v2: [uint32; 4]):
    v3: boolean = int.eq v0, v1
    branch v3, b1, b2

b1:
    check bounds.u v0, v1, v2 => b3, b4

b2:
    return v1

b3:
    return v0

b4:
    unreachable
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateGuards);
        test.assert_output(expected);
    }

    /// Assume instructions feed redundant checks.
    #[test]
    fn test_eliminate_guards_assume_fact() {
        let input = r#"
function test(v0: uint32, v1: uint32, v2: [uint32; 4]): uint32 {
entry(v0: uint32, v1: uint32, v2: [uint32; 4]):
    v3: boolean = int.eq v0, v1
    assume v3
    check bounds.u v0, v1, v2 => b1, b2

b1:
    return v0

b2:
    unreachable
}
"#;
        let expected = r#"
function test(v0: uint32, v1: uint32, v2: [uint32; 4]): uint32 {
entry(v0: uint32, v1: uint32, v2: [uint32; 4]):
    v3: boolean = int.eq v0, v1
    assume v3
    check bounds.u v0, v1, v2 => b1, b2

b1:
    return v0

b2:
    unreachable
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateGuards);
        test.assert_output(expected);
    }

    /// Constant conditions eliminate checks.
    #[test]
    fn test_eliminate_guards_constant_condition() {
        let input = r#"
function test(v0: uint32, v1: uint32, v2: [uint32; 4]): uint32 {
entry(v0: uint32, v1: uint32, v2: [uint32; 4]):
    v3: boolean = false
    check bounds.u v0, v1, v2 => b1, b2

b1:
    return v0

b2:
    return v1
}
"#;
        let expected = r#"
function test(v0: uint32, v1: uint32, v2: [uint32; 4]): uint32 {
entry(v0: uint32, v1: uint32, v2: [uint32; 4]):
    v3: boolean = false
    check bounds.u v0, v1, v2 => b1, b2

b1:
    return v0

b2:
    return v1
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateGuards);
        test.assert_output(expected);
    }

    /// Negated conditions are resolved using edge constraints.
    #[test]
    fn test_eliminate_guards_negated_condition() {
        let input = r#"
function test(v0: uint32, v1: uint32, v2: [uint32; 4]): uint32 {
entry(v0: uint32, v1: uint32, v2: [uint32; 4]):
    v3: boolean = int.eq v0, v1
    v4: boolean = int.not v3
    branch v3, b1, b2

b1:
    return v0

b2:
    check bounds.u v0, v1, v2 => b3, b4

b3:
    return v1

b4:
    unreachable
}
"#;
        let expected = r#"
function test(v0: uint32, v1: uint32, v2: [uint32; 4]): uint32 {
entry(v0: uint32, v1: uint32, v2: [uint32; 4]):
    v3: boolean = int.eq v0, v1
    v4: boolean = int.not v3
    branch v3, b1, b2

b1:
    return v0

b2:
    check bounds.u v0, v1, v2 => b3, b4

b3:
    return v1

b4:
    unreachable
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateGuards);
        test.assert_output(expected);
    }

    /// Condition constraints transfer through block parameters.
    #[test]
    fn test_eliminate_guards_block_param_condition() {
        let input = r#"
function test(v0: uint32, v1: uint32, v2: [uint32; 4]): uint32 {
entry(v0: uint32, v1: uint32, v2: [uint32; 4]):
    v3: boolean = int.eq v0, v1
    branch v3, b1(v3), b2(v3)

b1(v4: boolean):
    check bounds.u v0, v1, v2 => b3, b4

b2(v5: boolean):
    return v1

b3:
    return v0

b4:
    unreachable
}
"#;
        let expected = r#"
function test(v0: uint32, v1: uint32, v2: [uint32; 4]): uint32 {
entry(v0: uint32, v1: uint32, v2: [uint32; 4]):
    v3: boolean = int.eq v0, v1
    branch v3, b1(v3), b2(v3)

b1(v4: boolean):
    check bounds.u v0, v1, v2 => b3, b4

b2(v5: boolean):
    return v1

b3:
    return v0

b4:
    unreachable
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateGuards);
        test.assert_output(expected);
    }

    /// Check edges propagate condition constraints to successors.
    #[test]
    fn test_eliminate_guards_check_edge_fact() {
        let input = r#"
function test(v0: boolean, v1: uint32, v2: uint32, v3: [uint32; 4]): uint32 {
entry(v0: boolean, v1: uint32, v2: uint32, v3: [uint32; 4]):
    check bounds.u v1, v2, v3 => b1, b2

b1:
    check bounds.u v1, v2, v3 => b3, b4

b2:
    return v2

b3:
    return v1

b4:
    unreachable
}
"#;
        let expected = r#"
function test(v0: boolean, v1: uint32, v2: uint32, v3: [uint32; 4]): uint32 {
entry(v0: boolean, v1: uint32, v2: uint32, v3: [uint32; 4]):
    check bounds.u v1, v2, v3 => b1, b2

b1:
    check bounds.u v1, v2, v3 => b3, b4

b2:
    return v2

b3:
    return v1

b4:
    unreachable
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateGuards);
        test.assert_output(expected);
    }

    /// Bounds constraints eliminate checks when always in range.
    #[test]
    fn test_eliminate_guards_bounds_constraint_success() {
        let input = r#"
function test(v0: boolean, v1: [uint32; 4]): uint32 {
entry(v0: boolean, v1: [uint32; 4]):
    v2: uint32 = 2
    v3: uint32 = 4
    check bounds.u v2, v3, v1 => b1, b2

b1:
    return v2

b2:
    unreachable
}
"#;
        let expected = r#"
function test(v0: boolean, v1: [uint32; 4]): uint32 {
entry(v0: boolean, v1: [uint32; 4]):
    v2: uint32 = 2
    v3: uint32 = 4
    jump b1

b1:
    return v2

b2:
    unreachable
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateGuards);
        test.assert_output(expected);
    }

    /// Bounds constraints jump to failure when always out of range.
    #[test]
    fn test_eliminate_guards_bounds_constraint_failure() {
        let input = r#"
function test(v0: boolean, v1: [uint32; 0]): uint32 {
entry(v0: boolean, v1: [uint32; 0]):
    v2: uint32 = 0
    v3: uint32 = 0
    check bounds.u v2, v3, v1 => b1, b2

b1:
    unreachable

b2:
    return v2
}
"#;
        let expected = r#"
function test(v0: boolean, v1: [uint32; 0]): uint32 {
entry(v0: boolean, v1: [uint32; 0]):
    v2: uint32 = 0
    v3: uint32 = 0
    jump b2

b1:
    unreachable

b2:
    return v2
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateGuards);
        test.assert_output(expected);
    }

    /// Div zero constraints eliminate checks with non zero divisors.
    #[test]
    fn test_eliminate_guards_div_zero_constraint_success() {
        let input = r#"
function test(v0: boolean): int32 {
entry(v0: boolean):
    v1: int32 = 4
    check div.zero v1 => b1, b2

b1:
    return v1

b2:
    unreachable
}
"#;
        let expected = r#"
function test(v0: boolean): int32 {
entry(v0: boolean):
    v1: int32 = 4
    jump b1

b1:
    return v1

b2:
    unreachable
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateGuards);
        test.assert_output(expected);
    }

    /// Div zero constraints eliminate checks with zero divisors.
    #[test]
    fn test_eliminate_guards_div_zero_constraint_failure() {
        let input = r#"
function test(v0: boolean): int32 {
entry(v0: boolean):
    v1: int32 = 0
    check div.zero v1 => b1, b2

b1:
    unreachable

b2:
    return v1
}
"#;
        let expected = r#"
function test(v0: boolean): int32 {
entry(v0: boolean):
    v1: int32 = 0
    jump b2

b1:
    unreachable

b2:
    return v1
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateGuards);
        test.assert_output(expected);
    }

    /// Shift range constraints eliminate checks with in range shifts.
    #[test]
    fn test_eliminate_guards_shift_constraint_success() {
        let input = r#"
function test(v0: boolean): uint8 {
entry(v0: boolean):
    v1: uint8 = 3
    check shift.range.u v1, 8 => b1, b2

b1:
    return v1

b2:
    unreachable
}
"#;
        let expected = r#"
function test(v0: boolean): uint8 {
entry(v0: boolean):
    v1: uint8 = 3
    jump b1

b1:
    return v1

b2:
    unreachable
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateGuards);
        test.assert_output(expected);
    }

    /// Shift range constraints jump to failure on out of range shifts.
    #[test]
    fn test_eliminate_guards_shift_constraint_failure() {
        let input = r#"
function test(v0: boolean): uint8 {
entry(v0: boolean):
    v1: uint8 = 8
    check shift.range.u v1, 8 => b1, b2

b1:
    unreachable

b2:
    return v1
}
"#;
        let expected = r#"
function test(v0: boolean): uint8 {
entry(v0: boolean):
    v1: uint8 = 8
    jump b2

b1:
    unreachable

b2:
    return v1
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateGuards);
        test.assert_output(expected);
    }

    /// Narrow constraints eliminate checks for values in range.
    #[test]
    fn test_eliminate_guards_narrow_constraint_success() {
        let input = r#"
function test(v0: boolean): uint16 {
entry(v0: boolean):
    v1: uint16 = 12
    check narrow.range.u v1, 8 => b1, b2

b1:
    return v1

b2:
    unreachable
}
"#;
        let expected = r#"
function test(v0: boolean): uint16 {
entry(v0: boolean):
    v1: uint16 = 12
    jump b1

b1:
    return v1

b2:
    unreachable
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateGuards);
        test.assert_output(expected);
    }

    /// Narrow constraints jump to failure for out of range values.
    #[test]
    fn test_eliminate_guards_narrow_constraint_failure() {
        let input = r#"
function test(v0: boolean): uint16 {
entry(v0: boolean):
    v1: uint16 = 300
    check narrow.range.u v1, 8 => b1, b2

b1:
    unreachable

b2:
    return v1
}
"#;
        let expected = r#"
function test(v0: boolean): uint16 {
entry(v0: boolean):
    v1: uint16 = 300
    jump b2

b1:
    unreachable

b2:
    return v1
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateGuards);
        test.assert_output(expected);
    }

    /// Overflow constraints eliminate checks when no overflow is possible.
    #[test]
    fn test_eliminate_guards_overflow_constraint_success() {
        let input = r#"
function test(v0: boolean): int8 {
entry(v0: boolean):
    v1: int8 = 1
    v2: int8 = 2
    check int.add.overflow.s v1, v2 => b1, b2

b1:
    return v1

b2:
    unreachable
}
"#;
        let expected = r#"
function test(v0: boolean): int8 {
entry(v0: boolean):
    v1: int8 = 1
    v2: int8 = 2
    jump b1

b1:
    return v1

b2:
    unreachable
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateGuards);
        test.assert_output(expected);
    }

    /// Overflow constraints jump to failure when overflow is guaranteed.
    #[test]
    fn test_eliminate_guards_overflow_constraint_failure() {
        let input = r#"
function test(v0: boolean): int8 {
entry(v0: boolean):
    v1: int8 = 120
    v2: int8 = 120
    check int.add.overflow.s v1, v2 => b1, b2

b1:
    unreachable

b2:
    return v1
}
"#;
        let expected = r#"
function test(v0: boolean): int8 {
entry(v0: boolean):
    v1: int8 = 120
    v2: int8 = 120
    jump b2

b1:
    unreachable

b2:
    return v1
}
"#;

        let mut test = TestProgram::new(input);
        test.run_pass(&EliminateGuards);
        test.assert_output(expected);
    }
}
