use destack_ast::{self as ast, BinaryOperator, Expression};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintFix, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Require strict equality operators.
    ///
    /// Use `===` and `!==` instead of `==` and `!=`.
    /// Note: In Destack, `==` is typed and overloadable, so this rule
    /// may not always be applicable.
    #[lint(
        id = "eqeqeq",
        code = "LY021",
        category = Style,
        level = Ast,
        fixable = Always,
        recommended = Strict,
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

            // make fix: replace `a == b` with `a === b` or `a != b` with `a !== b`
            let expression_span = ctx.tree.get_span(node_id);
            let left_text = ctx.get_span_text(ctx.tree.get_span(*left));
            let right_text = ctx.get_span_text(ctx.tree.get_span(*right));
            let replacement = format!("{left_text} {strict_op} {right_text}");
            let edits = ctx
                .edit_builder()
                .replace(expression_span, replacement)
                .into_edits();
            let fix = LintFix::safe(format!("Replace with `{strict_op}`")).with_edits(edits);

            ctx.report(
                LintDiagnostic::new(
                    EQEQEQ.id,
                    EQEQEQ.code,
                    EQEQEQ.category,
                    severity,
                    message,
                    ctx.module.file_id,
                    expression_span,
                )
                .with_label(label)
                .with_fix(fix),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_loose_equality() {
        let test = TestProgram::for_rule_without_builtins(Eqeqeq);
        let result = test.lint_ast(
            "test.ds",
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
        let test = TestProgram::for_rule_without_builtins(Eqeqeq);
        let result = test.lint_ast(
            "test.ds",
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
        let test = TestProgram::for_rule_without_builtins(Eqeqeq);
        let result = test.lint_ast(
            "test.ds",
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
        let test = TestProgram::for_rule_without_builtins(Eqeqeq);
        let result = test.lint_ast(
            "test.ds",
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
        let test = TestProgram::for_rule_without_builtins(Eqeqeq);
        let result = test.lint_ast(
            "test.ds",
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
        let test = TestProgram::for_rule_without_builtins(Eqeqeq);
        let result = test.lint_ast(
            "test.ds",
            r#"
if (a == b) { x() }
"#,
        );
        test.result(result).assert_lint("eqeqeq").assert_safe_fixed(
            r#"
if (a === b) { x() }
"#,
        );
    }

    #[test]
    fn test_fix_loose_inequality() {
        let test = TestProgram::for_rule_without_builtins(Eqeqeq);
        let result = test.lint_ast(
            "test.ds",
            r#"
if (a != b) { x() }
"#,
        );
        test.result(result).assert_lint("eqeqeq").assert_safe_fixed(
            r#"
if (a !== b) { x() }
"#,
        );
    }
}
