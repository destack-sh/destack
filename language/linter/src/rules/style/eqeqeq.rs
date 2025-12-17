use destack_ast::{self as ast, BinaryOperator, Expression};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

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
        level = Ast
    )]
    pub Eqeqeq,
    "Require strict equality operators"
}

impl LintRule for Eqeqeq {
    fn meta(&self) -> &'static crate::LintMeta {
        Eqeqeq::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expr = ctx.tree.get(node_id);

            let Expression::Binary { operator, .. } = expr else {
                continue;
            };

            match operator {
                BinaryOperator::Equal => {
                    ctx.report(
                        LintDiagnostic::new(
                            EQEQEQ.id,
                            EQEQEQ.code,
                            EQEQEQ.category,
                            severity,
                            "use `===` instead of `==`",
                            ctx.module.file_id,
                            ctx.tree.get_span(node_id),
                        )
                        .with_label("prefer strict equality"),
                    );
                }
                BinaryOperator::NotEqual => {
                    ctx.report(
                        LintDiagnostic::new(
                            EQEQEQ.id,
                            EQEQEQ.code,
                            EQEQEQ.category,
                            severity,
                            "use `!==` instead of `!=`",
                            ctx.module.file_id,
                            ctx.tree.get_span(node_id),
                        )
                        .with_label("prefer strict inequality"),
                    );
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_loose_equality() {
        let test = TestProgram::for_rule(Eqeqeq);
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
        let test = TestProgram::for_rule(Eqeqeq);
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
        let test = TestProgram::for_rule(Eqeqeq);
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
        let test = TestProgram::for_rule(Eqeqeq);
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
        let test = TestProgram::for_rule(Eqeqeq);
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
}
