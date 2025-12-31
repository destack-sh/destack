use destack_ast::{self as ast, IfKind, UnaryOperator};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintFix, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow negated conditions in if-else and ternary expressions.
    ///
    /// Negated conditions with else branches are harder to read.
    /// Swap the branches and remove the negation for clearer code.
    #[lint(
        id = "no-negated-condition",
        code = "LY026",
        category = Style,
        level = Ast,
        fixable = Always,
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

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);
            let ast::Expression::If {
                kind,
                condition,
                then_expression,
                else_expression,
                ..
            } = expression
            else {
                continue;
            };

            // only check if there's an else branch
            let Some(else_id) = else_expression else {
                continue;
            };

            // check if condition is negated and get the positive version
            let Some(positive_condition) = get_positive_condition(ctx, *condition) else {
                continue;
            };

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            // make fix: swap branches and remove negation
            let expression_span = ctx.tree.get_span(node_id);
            let then_span = ctx.tree.get_span(*then_expression);
            let else_span = ctx.tree.get_span(*else_id);
            let then_text = ctx.get_span_text(then_span);
            let else_text = ctx.get_span_text(else_span);
            let replacement = match kind {
                IfKind::If => {
                    format!("if ({positive_condition}) {else_text} else {then_text}")
                }
                IfKind::Ternary => {
                    format!("{positive_condition} ? {else_text} : {then_text}")
                }
            };
            let edits = ctx
                .edit_builder()
                .replace(expression_span, replacement)
                .into_edits();
            let fix = LintFix::safe("Swap branches and remove negation").with_edits(edits);

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
                    expression_span,
                )
                .with_label("swap branches and remove negation")
                .with_fix(fix),
            );
        }
    }
}

/// Get the positive version of a negated condition, if the condition is negated.
/// Returns None if the condition is not negated.
fn get_positive_condition(
    ctx: &LintModuleAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> Option<String> {
    let expression = ctx.tree.get(expression_id);

    // unwrap parentheses
    if let ast::Expression::Parenthesized { expression: inner } = expression {
        return get_positive_condition(ctx, *inner);
    }

    // check for unary not: !x -> x
    if let ast::Expression::Unary { operator, right } = expression
        && *operator == UnaryOperator::Not
    {
        let inner_span = ctx.tree.get_span(*right);
        return Some(ctx.get_span_text(inner_span).to_string());
    }

    // check for inequality: x != y -> x == y, x !== y -> x === y
    if let ast::Expression::Binary {
        left,
        operator,
        right,
    } = expression
    {
        let positive_op = match operator {
            ast::BinaryOperator::NotEqual => Some("=="),
            ast::BinaryOperator::NotEqualStrict => Some("==="),
            _ => None,
        };
        if let Some(op) = positive_op {
            let left_span = ctx.tree.get_span(*left);
            let right_span = ctx.tree.get_span(*right);
            let left_text = ctx.get_span_text(left_span);
            let right_text = ctx.get_span_text(right_span);
            return Some(format!("{left_text} {op} {right_text}"));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_negated_if_with_else_detected() {
        let test = TestProgram::for_rule_without_builtins(NoNegatedCondition);
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
        let test = TestProgram::for_rule_without_builtins(NoNegatedCondition);
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
        let test = TestProgram::for_rule_without_builtins(NoNegatedCondition);
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
        let test = TestProgram::for_rule_without_builtins(NoNegatedCondition);
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
        let test = TestProgram::for_rule_without_builtins(NoNegatedCondition);
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
        let test = TestProgram::for_rule_without_builtins(NoNegatedCondition);
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
        let test = TestProgram::for_rule_without_builtins(NoNegatedCondition);
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
        let test = TestProgram::for_rule_without_builtins(NoNegatedCondition);
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
        let test = TestProgram::for_rule_without_builtins(NoNegatedCondition);
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
        let test = TestProgram::for_rule_without_builtins(NoNegatedCondition);
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

    #[test]
    fn test_fix_negated_if() {
        let test = TestProgram::for_rule_without_builtins(NoNegatedCondition);
        let result = test.lint_ast(
            "test.ds",
            r#"
if (!x) {
    doA()
} else {
    doB()
}
"#,
        );
        test.result(result)
            .assert_lint("no-negated-condition")
            .assert_safe_fixed(
                r#"
if (x) {
    doB()
} else {
    doA()
}
"#,
            );
    }

    #[test]
    fn test_fix_not_equal() {
        let test = TestProgram::for_rule_without_builtins(NoNegatedCondition);
        let result = test.lint_ast(
            "test.ds",
            r#"
if (x != 0) {
    doA()
} else {
    doB()
}
"#,
        );
        test.result(result)
            .assert_lint("no-negated-condition")
            .assert_safe_fixed(
                r#"
if (x == 0) {
    doB()
} else {
    doA()
}
"#,
            );
    }

    #[test]
    fn test_fix_ternary() {
        let test = TestProgram::for_rule_without_builtins(NoNegatedCondition);
        let result = test.lint_ast(
            "test.ds",
            r#"
const result = (!x) ? 1 : 2
"#,
        );
        test.result(result)
            .assert_lint("no-negated-condition")
            .assert_safe_fixed(
                r#"
const result = x ? 2 : 1;
"#,
            );
    }
}
