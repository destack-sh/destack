use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintFix, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow for loops that go in the wrong direction.
    ///
    /// A for loop with a counter moving in the wrong direction relative to its
    /// condition will never terminate or never execute. This is almost always a bug.
    #[lint(
        id = "for-direction",
        code = "LC001",
        category = Correctness,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Always,
        stability = Stable
    )]
    pub ForDirection,
    "Disallow for loops with incorrect direction"
}

/// Direction of loop iteration.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Direction {
    Increasing,
    Decreasing,
}

impl LintRule for ForDirection {
    fn meta(&self) -> &'static crate::LintMeta {
        ForDirection::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let ast::Expression::For {
                condition: Some(condition_id),
                increment: Some(increment_id),
                ..
            } = ctx.tree.get(node_id)
            else {
                continue;
            };

            // get direction from condition (e.g., `i < 10` -> Increasing, `i > 0` -> Decreasing)
            let condition_direction = get_condition_direction(ctx, *condition_id);
            let Some(condition_direction) = condition_direction else {
                continue;
            };

            // get direction from increment (e.g., `i++` -> Increasing, `i--` -> Decreasing)
            let increment_direction = get_increment_direction(ctx, *increment_id);
            let Some(increment_direction) = increment_direction else {
                continue;
            };

            // report if directions don't match
            if condition_direction != increment_direction {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }
                let direction_word = match condition_direction {
                    Direction::Increasing => "increase",
                    Direction::Decreasing => "decrease",
                };
                let mut diagnostic = LintDiagnostic::new(
                    FOR_DIRECTION.id,
                    FOR_DIRECTION.code,
                    FOR_DIRECTION.category,
                    severity,
                    format!("for loop counter should {direction_word} to match condition"),
                    ctx.module.file_id,
                    ctx.tree.get_span(node_id),
                )
                .with_label("counter moves in wrong direction");

                // invert increment direction when the update expression is a known form
                if ctx.compute_fixes
                    && let Some(fix) =
                        build_for_direction_fix(ctx, *increment_id, condition_direction)
                {
                    diagnostic = diagnostic.with_fix(fix);
                }

                ctx.report(diagnostic);
            }
        }
    }
}

/// Build an unsafe fix that flips increment direction for one loop update.
fn build_for_direction_fix(
    ctx: &LintModuleAstContext<'_>,
    increment_id: ast::LocalNodeId<ast::Expression>,
    expected_direction: Direction,
) -> Option<LintFix> {
    let replacement = increment_with_expected_direction(ctx, increment_id, expected_direction)?;
    let edits = ctx
        .edit_builder()
        .replace(ctx.tree.get_span(increment_id), replacement)
        .into_edits();
    Some(LintFix::r#unsafe("Flip loop update direction").with_edits(edits))
}

/// Render one increment expression with the expected direction.
fn increment_with_expected_direction(
    ctx: &LintModuleAstContext<'_>,
    increment_id: ast::LocalNodeId<ast::Expression>,
    expected_direction: Direction,
) -> Option<String> {
    let increment = ctx.tree.get(increment_id);
    match increment {
        ast::Expression::Unary { operator, right } => {
            let operand_text = ctx.get_span_text(ctx.tree.get_span(*right));
            let replacement = match (operator, expected_direction) {
                (ast::UnaryOperator::PostIncrement, Direction::Decreasing) => {
                    format!("{operand_text}--")
                }
                (ast::UnaryOperator::PostDecrement, Direction::Increasing) => {
                    format!("{operand_text}++")
                }
                (ast::UnaryOperator::PreIncrement, Direction::Decreasing) => {
                    format!("--{operand_text}")
                }
                (ast::UnaryOperator::PreDecrement, Direction::Increasing) => {
                    format!("++{operand_text}")
                }
                _ => return None,
            };
            Some(replacement)
        }
        ast::Expression::Assign {
            operator,
            left,
            right,
        } => {
            let replacement_operator =
                assign_operator_with_expected_direction(*operator, expected_direction)?;
            let left_text = ctx.get_span_text(ctx.tree.get_span(*left));
            let right_text = ctx.get_span_text(ctx.tree.get_span(*right));
            Some(format!("{left_text} {replacement_operator} {right_text}"))
        }
        ast::Expression::Parenthesized { expression } => {
            let inner = increment_with_expected_direction(ctx, *expression, expected_direction)?;
            Some(format!("({inner})"))
        }
        _ => None,
    }
}

/// Return an assignment operator matching the expected direction.
fn assign_operator_with_expected_direction(
    operator: ast::AssignOperator,
    expected_direction: Direction,
) -> Option<&'static str> {
    match (operator, expected_direction) {
        (ast::AssignOperator::AddAssign, Direction::Decreasing) => Some("-="),
        (ast::AssignOperator::WrappingAddAssign, Direction::Decreasing) => Some("-%="),
        (ast::AssignOperator::SaturatingAddAssign, Direction::Decreasing) => Some("-|="),
        (ast::AssignOperator::SubtractAssign, Direction::Increasing) => Some("+="),
        (ast::AssignOperator::WrappingSubtractAssign, Direction::Increasing) => Some("+%="),
        (ast::AssignOperator::SaturatingSubtractAssign, Direction::Increasing) => Some("+|="),
        _ => None,
    }
}

/// Determine the expected direction from the loop condition.
fn get_condition_direction(
    ctx: &LintModuleAstContext<'_>,
    condition_id: ast::LocalNodeId<ast::Expression>,
) -> Option<Direction> {
    let condition = ctx.tree.get(condition_id);
    match condition {
        ast::Expression::Binary { operator, .. } => match operator {
            // i < n or i <= n: counter should increase
            ast::BinaryOperator::LessThan | ast::BinaryOperator::LessThanOrEqual => {
                Some(Direction::Increasing)
            }
            // i > n or i >= n: counter should decrease
            ast::BinaryOperator::GreaterThan | ast::BinaryOperator::GreaterThanOrEqual => {
                Some(Direction::Decreasing)
            }
            _ => None,
        },
        ast::Expression::Parenthesized { expression } => get_condition_direction(ctx, *expression),
        _ => None,
    }
}

/// Determine the direction from the loop increment expression.
fn get_increment_direction(
    ctx: &LintModuleAstContext<'_>,
    increment_id: ast::LocalNodeId<ast::Expression>,
) -> Option<Direction> {
    let increment = ctx.tree.get(increment_id);
    match increment {
        // i++ or ++i
        ast::Expression::Unary { operator, .. } => match operator {
            ast::UnaryOperator::PostIncrement | ast::UnaryOperator::PreIncrement => {
                Some(Direction::Increasing)
            }
            ast::UnaryOperator::PostDecrement | ast::UnaryOperator::PreDecrement => {
                Some(Direction::Decreasing)
            }
            _ => None,
        },
        // i += n (assume positive n means increasing)
        ast::Expression::Assign { operator, .. } => match operator {
            ast::AssignOperator::AddAssign
            | ast::AssignOperator::WrappingAddAssign
            | ast::AssignOperator::SaturatingAddAssign => Some(Direction::Increasing),
            ast::AssignOperator::SubtractAssign
            | ast::AssignOperator::WrappingSubtractAssign
            | ast::AssignOperator::SaturatingSubtractAssign => Some(Direction::Decreasing),
            _ => None,
        },
        ast::Expression::Parenthesized { expression } => get_increment_direction(ctx, *expression),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_wrong_direction_increment() {
        let test = TestProgram::for_rule_without_prelude(ForDirection);
        let result = test.lint_ast(
            "for_direction/test_detects_wrong_direction_increment.ds",
            r#"
for (let i = 0; i < 10; i--) {
    console.log(i);
}
"#,
        );
        test.result(result).assert_lint("for-direction");
    }

    #[test]
    fn test_fix_flips_post_decrement_to_increment() {
        let test = TestProgram::for_rule_without_prelude(ForDirection);
        let result = test.lint_ast(
            "for_direction/test_fix_flips_post_decrement_to_increment.ds",
            r#"
for (let i = 0; i < 10; i--) {
    console.log(i);
}
"#,
        );
        test.result(result)
            .assert_lint("for-direction")
            .assert_has_fix("for-direction")
            .assert_unsafe_fixed(
                r#"
for (let i = 0; i < 10; i++) {
    console.log(i);
}
"#,
            );
    }

    #[test]
    fn test_detects_wrong_direction_decrement() {
        let test = TestProgram::for_rule_without_prelude(ForDirection);
        let result = test.lint_ast(
            "for_direction/test_detects_wrong_direction_decrement.ds",
            r#"
for (let i = 10; i > 0; i++) {
    console.log(i);
}
"#,
        );
        test.result(result).assert_lint("for-direction");
    }

    #[test]
    fn test_mutation_fix_flips_add_assign_to_subtract_assign() {
        let test = TestProgram::for_rule_without_prelude(ForDirection);
        let result = test.lint_ast(
            "for_direction/test_mutation_fix_flips_add_assign_to_subtract_assign.ds",
            r#"
for (let i = 10; i > 0; i += 2) {
    console.log(i);
}
"#,
        );
        test.result(result)
            .assert_lint("for-direction")
            .assert_has_fix("for-direction")
            .assert_unsafe_fixed(
                r#"
for (let i = 10; i > 0; i -= 2) {
    console.log(i);
}
"#,
            );
    }

    #[test]
    fn test_detects_wrong_direction_less_equal() {
        let test = TestProgram::for_rule_without_prelude(ForDirection);
        let result = test.lint_ast(
            "for_direction/test_detects_wrong_direction_less_equal.ds",
            r#"
for (let i = 0; i <= 10; i--) {
    console.log(i);
}
"#,
        );
        test.result(result).assert_lint("for-direction");
    }

    #[test]
    fn test_detects_wrong_direction_greater_equal() {
        let test = TestProgram::for_rule_without_prelude(ForDirection);
        let result = test.lint_ast(
            "for_direction/test_detects_wrong_direction_greater_equal.ds",
            r#"
for (let i = 10; i >= 0; i++) {
    console.log(i);
}
"#,
        );
        test.result(result).assert_lint("for-direction");
    }

    #[test]
    fn test_correct_direction_increment() {
        let test = TestProgram::for_rule_without_prelude(ForDirection);
        let result = test.lint_ast(
            "for_direction/test_correct_direction_increment.ds",
            r#"
for (let i = 0; i < 10; i++) {
    console.log(i);
}
"#,
        );
        test.result(result).assert_no_lint("for-direction");
    }

    #[test]
    fn test_correct_direction_decrement() {
        let test = TestProgram::for_rule_without_prelude(ForDirection);
        let result = test.lint_ast(
            "for_direction/test_correct_direction_decrement.ds",
            r#"
for (let i = 10; i > 0; i--) {
    console.log(i);
}
"#,
        );
        test.result(result).assert_no_lint("for-direction");
    }

    #[test]
    fn test_correct_direction_add_assign() {
        let test = TestProgram::for_rule_without_prelude(ForDirection);
        let result = test.lint_ast(
            "for_direction/test_correct_direction_add_assign.ds",
            r#"
for (let i = 0; i < 10; i += 2) {
    console.log(i);
}
"#,
        );
        test.result(result).assert_no_lint("for-direction");
    }

    #[test]
    fn test_correct_direction_subtract_assign() {
        let test = TestProgram::for_rule_without_prelude(ForDirection);
        let result = test.lint_ast(
            "for_direction/test_correct_direction_subtract_assign.ds",
            r#"
for (let i = 10; i > 0; i -= 2) {
    console.log(i);
}
"#,
        );
        test.result(result).assert_no_lint("for-direction");
    }

    #[test]
    fn test_no_condition() {
        let test = TestProgram::for_rule_without_prelude(ForDirection);
        let result = test.lint_ast(
            "for_direction/test_no_condition.ds",
            r#"
for (;;) {
    break;
}
"#,
        );
        test.result(result).assert_no_lint("for-direction");
    }
}
