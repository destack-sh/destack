use destack_ast::{self as ast, IfKind, UnaryOperator};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow negated conditions in if-else and ternary expressions.
    ///
    /// Negated conditions with else branches are harder to read.
    /// Swap the branches and remove the negation for clearer code.
    #[lint(
        id = "no-negated-condition",
        code = "LY045",
        category = Style,
        level = Ast,
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub NoNegatedCondition,
    "Disallow negated conditions"
}

impl LintRule for NoNegatedCondition {
    fn meta(&self) -> &'static crate::LintMeta {
        NoNegatedCondition::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);
            let ast::Expression::If {
                kind,
                condition,
                else_expression,
                ..
            } = expression
            else {
                continue;
            };

            // only check if there's an else branch
            let Some(_else_id) = else_expression else {
                continue;
            };

            // check if condition is negated
            if !is_negated_condition(ctx, *condition) {
                continue;
            }

            let message = match kind {
                IfKind::If => "unexpected negated condition in if-else",
                IfKind::Ternary => "unexpected negated condition in ternary",
            };
            ctx.report(
                LintDiagnostic::new(
                    NO_NEGATED_CONDITION.id,
                    NO_NEGATED_CONDITION.code,
                    NO_NEGATED_CONDITION.category,
                    severity,
                    message,
                    ctx.module.file_id,
                    ctx.tree.get_span(node_id),
                )
                .with_label("swap branches and remove negation"),
            );
        }
    }
}

/// Check if an expression is a negated condition (! operator or != comparison).
fn is_negated_condition(
    ctx: &LintModuleAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    let expression = ctx.tree.get(expression_id);

    // unwrap parentheses
    if let ast::Expression::Parenthesized { expression: inner } = expression {
        return is_negated_condition(ctx, *inner);
    }

    // check for unary not: !x
    if let ast::Expression::Unary { operator, .. } = expression
        && *operator == UnaryOperator::Not
    {
        return true;
    }

    // check for inequality: x != y or x !== y
    if let ast::Expression::Binary { operator, .. } = expression
        && matches!(
            operator,
            ast::BinaryOperator::NotEqual | ast::BinaryOperator::NotEqualStrict
        )
    {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_negated_if_with_else_detected() {
        let test = TestProgram::for_rule(NoNegatedCondition);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(x: bool) {
    if (!x) {
        doA()
    } else {
        doB()
    }
}
"#,
        );
        test.result(result).assert_lint("no-negated-condition");
    }

    #[test]
    fn test_negated_ternary_detected() {
        let test = TestProgram::for_rule(NoNegatedCondition);
        let result = test.lint_ast(
            "test.ds",
            r#"
const result = (!condition) ? 1 : 2;
"#,
        );
        test.result(result).assert_lint("no-negated-condition");
    }

    #[test]
    fn test_not_equal_with_else_detected() {
        let test = TestProgram::for_rule(NoNegatedCondition);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(x: int32) {
    if (x != 0) {
        doA()
    } else {
        doB()
    }
}
"#,
        );
        test.result(result).assert_lint("no-negated-condition");
    }

    #[test]
    fn test_strict_not_equal_with_else_detected() {
        let test = TestProgram::for_rule(NoNegatedCondition);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(x: int32) {
    if (x !== 0) {
        doA()
    } else {
        doB()
    }
}
"#,
        );
        test.result(result).assert_lint("no-negated-condition");
    }

    #[test]
    fn test_negated_if_without_else_allowed() {
        let test = TestProgram::for_rule(NoNegatedCondition);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(x: bool) {
    if (!x) {
        doA()
    }
}
"#,
        );
        test.result(result).assert_no_lint("no-negated-condition");
    }

    #[test]
    fn test_positive_condition_allowed() {
        let test = TestProgram::for_rule(NoNegatedCondition);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(x: bool) {
    if (x) {
        doA()
    } else {
        doB()
    }
}
"#,
        );
        test.result(result).assert_no_lint("no-negated-condition");
    }

    #[test]
    fn test_equality_condition_allowed() {
        let test = TestProgram::for_rule(NoNegatedCondition);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(x: int32) {
    if (x == 0) {
        doA()
    } else {
        doB()
    }
}
"#,
        );
        test.result(result).assert_no_lint("no-negated-condition");
    }

    #[test]
    fn test_parenthesized_negation_detected() {
        let test = TestProgram::for_rule(NoNegatedCondition);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(x: bool) {
    if ((!x)) {
        doA()
    } else {
        doB()
    }
}
"#,
        );
        test.result(result).assert_lint("no-negated-condition");
    }

    #[test]
    fn test_not_equal_ternary_detected() {
        let test = TestProgram::for_rule(NoNegatedCondition);
        let result = test.lint_ast(
            "test.ds",
            r#"
const result = (x != 0) ? 1 : 2;
"#,
        );
        test.result(result).assert_lint("no-negated-condition");
    }

    #[test]
    fn test_else_if_chain_allowed() {
        // negation in else-if chains is often intentional for clarity
        let test = TestProgram::for_rule(NoNegatedCondition);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(x: int32) {
    if (x > 0) {
        doA()
    } else if (x != 0) {
        doB()
    }
}
"#,
        );
        // this should be allowed since the else-if has no else branch
        test.result(result).assert_no_lint("no-negated-condition");
    }
}
