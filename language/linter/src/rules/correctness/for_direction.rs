use destack_ast::{AssignOperator, BinaryOperator, Expression, UnaryOperator};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow for loops that go in the wrong direction.
    ///
    /// A for loop with a counter moving in the wrong direction relative to its
    /// condition will never terminate or never execute. This is almost always a bug.
    #[lint(
        id = "for-direction",
        code = "LC004",
        category = Correctness,
        level = Ast
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

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        for node_id in ctx.tree.iter_nodes::<Expression>() {
            let Expression::For {
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
                let direction_word = match condition_direction {
                    Direction::Increasing => "increase",
                    Direction::Decreasing => "decrease",
                };
                ctx.report(
                    LintDiagnostic::new(
                        FOR_DIRECTION.id,
                        FOR_DIRECTION.code,
                        FOR_DIRECTION.category,
                        severity,
                        format!("for loop counter should {direction_word} to match condition"),
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label("counter moves in wrong direction"),
                );
            }
        }
    }
}

/// Determine the expected direction from the loop condition.
fn get_condition_direction(
    ctx: &LintModuleAstContext<'_>,
    condition_id: destack_ast::LocalNodeId<Expression>,
) -> Option<Direction> {
    let condition = ctx.tree.get(condition_id);
    match condition {
        Expression::Binary { operator, .. } => match operator {
            // i < n or i <= n: counter should increase
            BinaryOperator::LessThan | BinaryOperator::LessThanOrEqual => {
                Some(Direction::Increasing)
            }
            // i > n or i >= n: counter should decrease
            BinaryOperator::GreaterThan | BinaryOperator::GreaterThanOrEqual => {
                Some(Direction::Decreasing)
            }
            _ => None,
        },
        Expression::Parenthesized { expression } => get_condition_direction(ctx, *expression),
        _ => None,
    }
}

/// Determine the direction from the loop increment expression.
fn get_increment_direction(
    ctx: &LintModuleAstContext<'_>,
    increment_id: destack_ast::LocalNodeId<Expression>,
) -> Option<Direction> {
    let increment = ctx.tree.get(increment_id);
    match increment {
        // i++ or ++i
        Expression::Unary { operator, .. } => match operator {
            UnaryOperator::PostIncrement | UnaryOperator::PreIncrement => {
                Some(Direction::Increasing)
            }
            UnaryOperator::PostDecrement | UnaryOperator::PreDecrement => {
                Some(Direction::Decreasing)
            }
            _ => None,
        },
        // i += n (assume positive n means increasing)
        Expression::Assign { operator, .. } => match operator {
            AssignOperator::AddAssign
            | AssignOperator::WrappingAddAssign
            | AssignOperator::SaturatingAddAssign => Some(Direction::Increasing),
            AssignOperator::SubtractAssign
            | AssignOperator::WrappingSubtractAssign
            | AssignOperator::SaturatingSubtractAssign => Some(Direction::Decreasing),
            _ => None,
        },
        Expression::Parenthesized { expression } => get_increment_direction(ctx, *expression),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_wrong_direction_increment() {
        let test = TestProgram::for_rule(ForDirection);
        let result = test.lint_ast(
            "test.ds",
            r#"
for (let i = 0; i < 10; i--) {
    console.log(i);
}
"#,
        );
        test.result(result).assert_lint("for-direction");
    }

    #[test]
    fn test_detects_wrong_direction_decrement() {
        let test = TestProgram::for_rule(ForDirection);
        let result = test.lint_ast(
            "test.ds",
            r#"
for (let i = 10; i > 0; i++) {
    console.log(i);
}
"#,
        );
        test.result(result).assert_lint("for-direction");
    }

    #[test]
    fn test_detects_wrong_direction_less_equal() {
        let test = TestProgram::for_rule(ForDirection);
        let result = test.lint_ast(
            "test.ds",
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
        let test = TestProgram::for_rule(ForDirection);
        let result = test.lint_ast(
            "test.ds",
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
        let test = TestProgram::for_rule(ForDirection);
        let result = test.lint_ast(
            "test.ds",
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
        let test = TestProgram::for_rule(ForDirection);
        let result = test.lint_ast(
            "test.ds",
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
        let test = TestProgram::for_rule(ForDirection);
        let result = test.lint_ast(
            "test.ds",
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
        let test = TestProgram::for_rule(ForDirection);
        let result = test.lint_ast(
            "test.ds",
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
        let test = TestProgram::for_rule(ForDirection);
        let result = test.lint_ast(
            "test.ds",
            r#"
for (;;) {
    break;
}
"#,
        );
        test.result(result).assert_no_lint("for-direction");
    }
}
