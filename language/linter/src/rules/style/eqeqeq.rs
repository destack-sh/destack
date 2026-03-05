use destack_ast::{self as ast, BinaryOperator, Expression, ScalarLiteral, TypeLiteral};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintFix, LintModuleAstContext, LintRule, declare_lint};

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
    fn meta(&self) -> &'static crate::LintMeta {
        Eqeqeq::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

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

            let (message, label, strict_op) = match operator {
                BinaryOperator::Equal => {
                    ("use `===` instead of `==`", "prefer strict equality", "===")
                }
                BinaryOperator::NotEqual => (
                    "use `!==` instead of `!=`",
                    "prefer strict inequality",
                    "!==",
                ),
                _ => continue,
            };

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            let expression_span = ctx.tree.get_span(node_id);
            let left_expression = ctx.tree.get(*left);
            let right_expression = ctx.tree.get(*right);

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
            if ctx.compute_fixes && equality_operator_fix_is_safe(left_expression, right_expression)
            {
                let left_text = ctx.get_span_text(ctx.tree.get_span(*left));
                let right_text = ctx.get_span_text(ctx.tree.get_span(*right));
                let replacement = format!("{left_text} {strict_op} {right_text}");
                let edits = ctx
                    .edit_builder()
                    .replace(expression_span, replacement)
                    .into_edits();
                let fix = LintFix::safe(format!("Replace with `{strict_op}`")).with_edits(edits);
                diagnostic = diagnostic.with_fix(fix);
            }

            ctx.report(diagnostic);
        }
    }
}

/// Return true when replacing loose equality is semantics preserving.
fn equality_operator_fix_is_safe(left: &Expression, right: &Expression) -> bool {
    expression_is_typeof(left)
        || expression_is_typeof(right)
        || expressions_have_same_literal_kind(left, right)
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
fn expressions_have_same_literal_kind(left: &Expression, right: &Expression) -> bool {
    matches!(
        (left, right),
        (
            Expression::ScalarLiteral(ScalarLiteral::Boolean(_)),
            Expression::ScalarLiteral(ScalarLiteral::Boolean(_))
        ) | (
            Expression::ScalarLiteral(ScalarLiteral::String(_)),
            Expression::ScalarLiteral(ScalarLiteral::String(_))
        ) | (
            Expression::ScalarLiteral(ScalarLiteral::Integer(_)),
            Expression::ScalarLiteral(ScalarLiteral::Integer(_))
        ) | (
            Expression::ScalarLiteral(ScalarLiteral::Bigint(_)),
            Expression::ScalarLiteral(ScalarLiteral::Bigint(_))
        ) | (
            Expression::ScalarLiteral(ScalarLiteral::Float(_)),
            Expression::ScalarLiteral(ScalarLiteral::Float(_))
        ) | (
            Expression::TypeLiteral(TypeLiteral::Null),
            Expression::TypeLiteral(TypeLiteral::Null)
        ) | (
            Expression::TypeLiteral(TypeLiteral::Undefined),
            Expression::TypeLiteral(TypeLiteral::Undefined)
        )
    )
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
}
