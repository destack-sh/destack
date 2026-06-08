use crate::LintMeta;
use destack_dir::{self as dir, IfForm, UnaryOperator};
use destack_repository::LintSeverity;

use crate::{LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow negated conditions in if-else and ternary expressions.
    ///
    /// Negated conditions with else branches are harder to read.
    /// Swap the branches and remove the negation for clearer code.
    #[lint(
        id = "no-negated-condition",
        code = "LY020",
        category = Style,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub NoNegatedCondition,
    "Disallow negated conditions"
}

impl LintRule for NoNegatedCondition {
    fn meta(&self) -> &'static LintMeta {
        NoNegatedCondition::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.dir.iter_nodes::<dir::Expression>() {
            let expression = ctx.dir.get(node_id);
            let dir::Expression::If {
                form,
                condition,
                then_expression: _,
                else_expression,
                ..
            } = expression
            else {
                continue;
            };

            // only check if the branch shape is eligible
            if !has_negated_condition_context(ctx, *form, *else_expression) {
                continue;
            }

            // check if condition is negated
            let condition_id = match condition {
                dir::IfCondition::Expression { condition } => *condition,
                dir::IfCondition::Let { .. } => continue,
            };
            if !is_negated_condition(ctx, condition_id) {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            let expression_span = ctx.dir.get_span(node_id);
            let message = match form {
                IfForm::If => "unexpected negated condition in if-else",
                IfForm::Ternary => "unexpected negated condition in ternary",
            };
            ctx.report(
                LintReport::new(
                    NO_NEGATED_CONDITION.id,
                    NO_NEGATED_CONDITION.code,
                    NO_NEGATED_CONDITION.category,
                    severity,
                    message,
                    expression_span,
                )
                .label("prefer positive conditions"),
            );
        }
    }
}

/// Return true when the expression kind and else branch should be checked.
fn has_negated_condition_context(
    ctx: &LintModuleContext<'_>,
    form: IfForm,
    else_expression: Option<dir::LocalNodeId<dir::Expression>>,
) -> bool {
    match form {
        // match `if (...) ... else ...` but skip `else if` chains
        IfForm::If => {
            let Some(else_expression_id) = else_expression else {
                return false;
            };
            !matches!(
                ctx.dir.get(else_expression_id),
                dir::Expression::If {
                    form: dir::IfForm::If,
                    ..
                }
            )
        }
        // match ternary expressions that always include an alternate branch
        IfForm::Ternary => else_expression.is_some(),
    }
}

/// Return true when a condition expression is negated.
fn is_negated_condition(
    ctx: &LintModuleContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let expression = ctx.dir.get(expression_id);

    // unwrap parentheses
    if let dir::Expression::Parenthesized { expression: inner } = expression {
        return is_negated_condition(ctx, *inner);
    }

    // match unary not
    if let dir::Expression::Unary { operator, right } = expression
        && *operator == UnaryOperator::Not
    {
        let _ = right;
        return true;
    }

    // match inequality operators
    matches!(
        expression,
        dir::Expression::Binary {
            operator: dir::BinaryOperator::NotEqual | dir::BinaryOperator::NotEqualStrict,
            ..
        }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_negated_if_with_else_detected() {
        let test = TestProgram::for_rule_without_prelude(NoNegatedCondition);
        let result = test.lint(
            "no_negated_condition/test_negated_if_with_else_detected.ds",
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
        let test = TestProgram::for_rule_without_prelude(NoNegatedCondition);
        let result = test.lint(
            "no_negated_condition/test_negated_ternary_detected.ds",
            r#"
const result = (!condition) ? 1 : 2;
"#,
        );
        test.result(result).assert_lint("no-negated-condition");
    }

    #[test]
    fn test_not_equal_with_else_detected() {
        let test = TestProgram::for_rule_without_prelude(NoNegatedCondition);
        let result = test.lint(
            "no_negated_condition/test_not_equal_with_else_detected.ds",
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
        let test = TestProgram::for_rule_without_prelude(NoNegatedCondition);
        let result = test.lint(
            "no_negated_condition/test_strict_not_equal_with_else_detected.ds",
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
        let test = TestProgram::for_rule_without_prelude(NoNegatedCondition);
        let result = test.lint(
            "no_negated_condition/test_negated_if_without_else_allowed.ds",
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
        let test = TestProgram::for_rule_without_prelude(NoNegatedCondition);
        let result = test.lint(
            "no_negated_condition/test_positive_condition_allowed.ds",
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
        let test = TestProgram::for_rule_without_prelude(NoNegatedCondition);
        let result = test.lint(
            "no_negated_condition/test_equality_condition_allowed.ds",
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
        let test = TestProgram::for_rule_without_prelude(NoNegatedCondition);
        let result = test.lint(
            "no_negated_condition/test_parenthesized_negation_detected.ds",
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
        let test = TestProgram::for_rule_without_prelude(NoNegatedCondition);
        let result = test.lint(
            "no_negated_condition/test_not_equal_ternary_detected.ds",
            r#"
const result = (x != 0) ? 1 : 2;
"#,
        );
        test.result(result).assert_lint("no-negated-condition");
    }

    #[test]
    fn test_else_if_chain_allowed() {
        let test = TestProgram::for_rule_without_prelude(NoNegatedCondition);
        let result = test.lint(
            "no_negated_condition/test_else_if_chain_allowed.ds",
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
    fn test_no_fix_negated_if() {
        let test = TestProgram::for_rule_without_prelude(NoNegatedCondition);
        let result = test.lint(
            "no_negated_condition/test_fix_negated_if.ds",
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
            .assert_has_no_fix("no-negated-condition");
    }

    #[test]
    fn test_no_fix_not_equal() {
        let test = TestProgram::for_rule_without_prelude(NoNegatedCondition);
        let result = test.lint(
            "no_negated_condition/test_fix_not_equal.ds",
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
            .assert_has_no_fix("no-negated-condition");
    }

    #[test]
    fn test_no_fix_ternary() {
        let test = TestProgram::for_rule_without_prelude(NoNegatedCondition);
        let result = test.lint(
            "no_negated_condition/test_fix_ternary.ds",
            r#"
const result = (!x) ? 1 : 2
"#,
        );
        test.result(result)
            .assert_lint("no-negated-condition")
            .assert_has_no_fix("no-negated-condition");
    }

    #[test]
    fn test_negated_if_with_else_if_is_allowed() {
        let test = TestProgram::for_rule_without_prelude(NoNegatedCondition);
        let result = test.lint(
            "no_negated_condition/test_negated_if_with_else_if_is_allowed.ds",
            r#"
function foo(x: bool, y: bool) {
    if (!x) {
        doA()
    } else if (y) {
        doB()
    }
}
"#,
        );
        test.result(result).assert_no_lint("no-negated-condition");
    }
}
