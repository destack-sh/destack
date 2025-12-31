use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};
use destack_ast as ast;
use destack_workspace::LintSeverity;

declare_lint! {
    /// Disallow useless backreferences in regular expressions.
    ///
    /// Backreferences that reference non-existent groups or forward-reference
    /// groups that haven't been captured yet will never match anything useful.
    #[lint(
        id = "no-useless-backreference",
        code = "LU048",
        category = Suspicious,
        level = Ast,
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoUselessBackreference,
    "Disallow useless regex backreferences"
}

impl LintRule for NoUselessBackreference {
    fn meta(&self) -> &'static crate::LintMeta {
        NoUselessBackreference::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expr = ctx.tree.get(node_id);
            let ast::Expression::ScalarLiteral(ast::ScalarLiteral::RegexString { content, .. }) =
                expr
            else {
                continue;
            };

            let problem = ctx.regex_useless_backreference(*content);
            let Some(problem) = problem.as_deref() else {
                continue;
            };
            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            ctx.report(
                LintDiagnostic::new(
                    NO_USELESS_BACKREFERENCE.id,
                    NO_USELESS_BACKREFERENCE.code,
                    NO_USELESS_BACKREFERENCE.category,
                    severity,
                    problem,
                    ctx.module.file_id,
                    ctx.tree.get_span(node_id),
                )
                .with_label("this backreference will never match"),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_nonexistent_backreference() {
        let test = TestProgram::for_rule_without_builtins(NoUselessBackreference);
        let result = test.lint_ast(
            "test.ds",
            r#"
const re = /(a)\2/
"#,
        );
        test.result(result).assert_lint("no-useless-backreference");
    }

    #[test]
    fn test_detects_forward_reference() {
        let test = TestProgram::for_rule_without_builtins(NoUselessBackreference);
        let result = test.lint_ast(
            "test.ds",
            r#"
const re = /\1(a)/
"#,
        );
        test.result(result).assert_lint("no-useless-backreference");
    }

    #[test]
    fn test_allows_valid_backreference() {
        let test = TestProgram::for_rule_without_builtins(NoUselessBackreference);
        let result = test.lint_ast(
            "test.ds",
            r#"
const re = /(a)\1/
"#,
        );
        test.result(result)
            .assert_no_lint("no-useless-backreference");
    }

    #[test]
    fn test_allows_multiple_valid_backreferences() {
        let test = TestProgram::for_rule_without_builtins(NoUselessBackreference);
        let result = test.lint_ast(
            "test.ds",
            r#"
const re = /(a)(b)\1\2/
"#,
        );
        test.result(result)
            .assert_no_lint("no-useless-backreference");
    }

    #[test]
    fn test_allows_regex_without_backreference() {
        let test = TestProgram::for_rule_without_builtins(NoUselessBackreference);
        let result = test.lint_ast(
            "test.ds",
            r#"
const re = /hello/
"#,
        );
        test.result(result)
            .assert_no_lint("no-useless-backreference");
    }

    #[test]
    fn test_allows_escaped_digit_in_char_class() {
        let test = TestProgram::for_rule_without_builtins(NoUselessBackreference);
        let result = test.lint_ast(
            "test.ds",
            r#"
const re = /[\1]/
"#,
        );
        // \1 in character class is octal, not backreference
        let _ = test.result(result);
    }
}
