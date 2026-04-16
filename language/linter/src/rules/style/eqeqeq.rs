use crate::LintMeta;
use destack_ast::{self as ast, BinaryOperator, Expression, ScalarLiteral, TypeExpression};
use destack_workspace::{EqeqeqMode, EqeqeqNullPolicy, LintSeverity};

use crate::rules::common::source_text_contains_comment_token;
use crate::{LintAstContext, LintDiagnostic, LintFix, LintRule, declare_lint};

declare_lint! {
    /// Require strict equality operators.
    ///
    /// Use `===` and `!==` instead of `==` and `!=`.
    #[lint(
        id = "eqeqeq",
        code = "LY010",
        category = Style,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Off,
        stability = Stable
    )]
    pub Eqeqeq,
    "Require strict equality operators"
}

impl LintRule for Eqeqeq {
    fn meta(&self) -> &'static LintMeta {
        Eqeqeq::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();
        let mode = ctx.options.style.eqeqeq_mode;
        let null_policy = if mode == EqeqeqMode::Always {
            ctx.options.style.eqeqeq_null
        } else {
            EqeqeqNullPolicy::Ignore
        };

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expr = ctx.tree.get(node_id);

            let Expression::Binary {
                left,
                operator,
                right,
            } = expr
            else {
                continue;
            };

            let left_expression = ctx.tree.get(*left);
            let right_expression = ctx.tree.get(*right);
            let is_null_check = expression_is_null_literal(ctx.tree, left_expression)
                || expression_is_null_literal(ctx.tree, right_expression);

            // loose operators
            if matches!(operator, BinaryOperator::Equal | BinaryOperator::NotEqual) {
                if loose_equality_is_allowed(
                    ctx.tree,
                    mode,
                    null_policy,
                    left_expression,
                    right_expression,
                ) {
                    continue;
                }

                let (message, label, strict_op) = match operator {
                    BinaryOperator::Equal => {
                        ("use `===` instead of `==`", "prefer strict equality", "===")
                    }
                    BinaryOperator::NotEqual => (
                        "use `!==` instead of `!=`",
                        "prefer strict inequality",
                        "!==",
                    ),
                    _ => unreachable!(),
                };

                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }

                let expression_span = ctx.tree.get_span(node_id);
                let mut diagnostic = LintDiagnostic::new(
                    EQEQEQ.id,
                    EQEQEQ.code,
                    EQEQEQ.category,
                    severity,
                    message,
                    ctx.module.file_id,
                    expression_span,
                )
                .with_label(label);

                // only apply autofix where operator replacement is semantics preserving
                if ctx.compute_fixes
                    && equality_operator_fix_is_safe(ctx.tree, left_expression, right_expression)
                    && !source_text_contains_comment_token(ctx.get_span_text(expression_span))
                {
                    let left_text = ctx.get_span_text(ctx.tree.get_span(*left));
                    let right_text = ctx.get_span_text(ctx.tree.get_span(*right));
                    let replacement = format!("{left_text} {strict_op} {right_text}");
                    let edits = ctx
                        .edit_builder()
                        .replace(expression_span, replacement)
                        .into_edits();
                    let fix =
                        LintFix::safe(format!("Replace with `{strict_op}`")).with_edits(edits);
                    diagnostic = diagnostic.with_fix(fix);
                }

                ctx.report(diagnostic);
                continue;
            }

            // strict null operators with `null: never`
            if !matches!(
                operator,
                BinaryOperator::EqualStrict | BinaryOperator::NotEqualStrict
            ) {
                continue;
            }

            if null_policy != EqeqeqNullPolicy::Never || !is_null_check {
                continue;
            }

            let (message, label) = match operator {
                BinaryOperator::EqualStrict => (
                    "use `==` instead of `===` for null checks",
                    "prefer loose null equality",
                ),
                BinaryOperator::NotEqualStrict => (
                    "use `!=` instead of `!==` for null checks",
                    "prefer loose null inequality",
                ),
                _ => unreachable!(),
            };

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            ctx.report(
                LintDiagnostic::new(
                    EQEQEQ.id,
                    EQEQEQ.code,
                    EQEQEQ.category,
                    severity,
                    message,
                    ctx.module.file_id,
                    ctx.tree.get_span(node_id),
                )
                .with_label(label),
            );
        }
    }
}

/// Return true when a loose equality operator is allowed by the active mode.
fn loose_equality_is_allowed(
    tree: &ast::NodeTree,
    mode: EqeqeqMode,
    null_policy: EqeqeqNullPolicy,
    left: &Expression,
    right: &Expression,
) -> bool {
    match mode {
        EqeqeqMode::Always => {
            (expression_is_null_literal(tree, left) || expression_is_null_literal(tree, right))
                && null_policy != EqeqeqNullPolicy::Always
        }
        EqeqeqMode::Smart => {
            expression_is_typeof(left)
                || expression_is_typeof(right)
                || expressions_have_same_literal_kind(tree, left, right)
                || expression_is_null_literal(tree, left)
                || expression_is_null_literal(tree, right)
        }
        EqeqeqMode::AllowNull => {
            expression_is_null_literal(tree, left) || expression_is_null_literal(tree, right)
        }
    }
}

/// Return true when replacing loose equality is semantics preserving.
fn equality_operator_fix_is_safe(
    tree: &ast::NodeTree,
    left: &Expression,
    right: &Expression,
) -> bool {
    expression_is_typeof(left)
        || expression_is_typeof(right)
        || expressions_have_same_literal_kind(tree, left, right)
}

/// Return true when the expression is a null literal.
fn expression_is_null_literal(tree: &ast::NodeTree, expression: &Expression) -> bool {
    let Expression::Type { value } = expression else {
        return false;
    };

    matches!(
        tree.get(*value),
        TypeExpression::Literal {
            value: ast::TypeLiteral::Null | ast::TypeLiteral::Undefined,
        }
    )
}

/// Return true when one expression is a `typeof` unary expression.
fn expression_is_typeof(expression: &Expression) -> bool {
    matches!(
        expression,
        Expression::Unary {
            operator: ast::UnaryOperator::Typeof,
            ..
        }
    )
}

/// Return true when both expressions are literals with identical runtime kind.
fn expressions_have_same_literal_kind(
    tree: &ast::NodeTree,
    left: &Expression,
    right: &Expression,
) -> bool {
    match (left, right) {
        (
            Expression::ScalarLiteral(ScalarLiteral::Boolean(_)),
            Expression::ScalarLiteral(ScalarLiteral::Boolean(_)),
        )
        | (
            Expression::ScalarLiteral(ScalarLiteral::String(_)),
            Expression::ScalarLiteral(ScalarLiteral::String(_)),
        )
        | (
            Expression::ScalarLiteral(ScalarLiteral::Integer(_)),
            Expression::ScalarLiteral(ScalarLiteral::Integer(_)),
        )
        | (
            Expression::ScalarLiteral(ScalarLiteral::Bigint(_)),
            Expression::ScalarLiteral(ScalarLiteral::Bigint(_)),
        )
        | (
            Expression::ScalarLiteral(ScalarLiteral::Float(_)),
            Expression::ScalarLiteral(ScalarLiteral::Float(_)),
        ) => true,
        (Expression::Type { value: left }, Expression::Type { value: right }) => {
            let left = tree.get(*left);
            let right = tree.get(*right);
            std::mem::discriminant(left) == std::mem::discriminant(right)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_loose_equality() {
        let test = TestProgram::for_rule_without_prelude(Eqeqeq);
        let result = test.lint_ast(
            "eqeqeq/test_detects_loose_equality.ds",
            r#"
if (a == b) {
    doSomething()
}
"#,
        );
        test.result(result).assert_lint("eqeqeq");
    }

    #[test]
    fn test_detects_loose_inequality() {
        let test = TestProgram::for_rule_without_prelude(Eqeqeq);
        let result = test.lint_ast(
            "eqeqeq/test_detects_loose_inequality.ds",
            r#"
if (a != b) {
    doSomething()
}
"#,
        );
        test.result(result).assert_lint("eqeqeq");
    }

    #[test]
    fn test_allows_strict_equality() {
        let test = TestProgram::for_rule_without_prelude(Eqeqeq);
        let result = test.lint_ast(
            "eqeqeq/test_allows_strict_equality.ds",
            r#"
if (a === b) {
    doSomething()
}
"#,
        );
        test.result(result).assert_no_lint("eqeqeq");
    }

    #[test]
    fn test_allows_strict_inequality() {
        let test = TestProgram::for_rule_without_prelude(Eqeqeq);
        let result = test.lint_ast(
            "eqeqeq/test_allows_strict_inequality.ds",
            r#"
if (a !== b) {
    doSomething()
}
"#,
        );
        test.result(result).assert_no_lint("eqeqeq");
    }

    #[test]
    fn test_allows_other_operators() {
        let test = TestProgram::for_rule_without_prelude(Eqeqeq);
        let result = test.lint_ast(
            "eqeqeq/test_allows_other_operators.ds",
            r#"
if (a < b && c > d) {
    doSomething()
}
"#,
        );
        test.result(result).assert_no_lint("eqeqeq");
    }

    #[test]
    fn test_fix_loose_equality() {
        let test = TestProgram::for_rule_without_prelude(Eqeqeq);
        let result = test.lint_ast(
            "eqeqeq/test_fix_loose_equality.ds",
            r#"
if (a == b) { x() }
"#,
        );
        test.result(result)
            .assert_lint("eqeqeq")
            .assert_has_no_fix("eqeqeq");
    }

    #[test]
    fn test_fix_loose_inequality() {
        let test = TestProgram::for_rule_without_prelude(Eqeqeq);
        let result = test.lint_ast(
            "eqeqeq/test_fix_loose_inequality.ds",
            r#"
if (a != b) { x() }
"#,
        );
        test.result(result)
            .assert_lint("eqeqeq")
            .assert_has_no_fix("eqeqeq");
    }

    #[test]
    fn test_fix_typeof_comparison() {
        let test = TestProgram::for_rule_without_prelude(Eqeqeq);
        let result = test.lint_ast(
            "eqeqeq/test_fix_typeof_comparison.ds",
            r#"
if (typeof a == "number") { x() }
"#,
        );
        test.result(result).assert_lint("eqeqeq").assert_safe_fixed(
            r#"
if (typeof a === "number") { x() }
"#,
        );
    }

    #[test]
    fn test_fix_same_literal_kinds() {
        let test = TestProgram::for_rule_without_prelude(Eqeqeq);
        let result = test.lint_ast(
            "eqeqeq/test_fix_same_literal_kinds.ds",
            r#"
if (1 == 2) { x() }
"#,
        );
        test.result(result).assert_lint("eqeqeq").assert_safe_fixed(
            r#"
if (1 === 2) { x() }
"#,
        );
    }

    #[test]
    fn test_has_no_fix_when_comparison_contains_comment() {
        let test = TestProgram::for_rule_without_prelude(Eqeqeq);
        let result = test.lint_ast(
            "eqeqeq/test_has_no_fix_when_comparison_contains_comment.ds",
            r#"
if (typeof value /* keep */ == "string") { x() }
"#,
        );
        test.result(result)
            .assert_lint("eqeqeq")
            .assert_has_no_fix("eqeqeq");
    }

    #[test]
    fn test_allows_null_check_in_allow_null_mode() {
        let test = TestProgram::for_rule_without_prelude(Eqeqeq).with_options(|options| {
            options.style.eqeqeq_mode = EqeqeqMode::AllowNull;
        });
        let result = test.lint_ast(
            "eqeqeq/test_allows_null_check_in_allow_null_mode.ds",
            r#"
if (value == null) { x() }
"#,
        );
        test.result(result).assert_no_lint("eqeqeq");
    }

    #[test]
    fn test_allows_smart_typeof_comparison() {
        let test = TestProgram::for_rule_without_prelude(Eqeqeq).with_options(|options| {
            options.style.eqeqeq_mode = EqeqeqMode::Smart;
        });
        let result = test.lint_ast(
            "eqeqeq/test_allows_smart_typeof_comparison.ds",
            r#"
if (typeof value == "string") { x() }
"#,
        );
        test.result(result).assert_no_lint("eqeqeq");
    }

    #[test]
    fn test_reports_strict_null_check_when_null_policy_is_never() {
        let test = TestProgram::for_rule_without_prelude(Eqeqeq).with_options(|options| {
            options.style.eqeqeq_null = EqeqeqNullPolicy::Never;
        });
        let result = test.lint_ast(
            "eqeqeq/test_reports_strict_null_check_when_null_policy_is_never.ds",
            r#"
if (value === null) { x() }
"#,
        );
        test.result(result)
            .assert_lint("eqeqeq")
            .assert_has_no_fix("eqeqeq");
    }

    #[test]
    fn test_ignores_loose_null_check_when_null_policy_is_ignore() {
        let test = TestProgram::for_rule_without_prelude(Eqeqeq).with_options(|options| {
            options.style.eqeqeq_null = EqeqeqNullPolicy::Ignore;
        });
        let result = test.lint_ast(
            "eqeqeq/test_ignores_loose_null_check_when_null_policy_is_ignore.ds",
            r#"
if (value == null) { x() }
"#,
        );
        test.result(result).assert_no_lint("eqeqeq");
    }
}
