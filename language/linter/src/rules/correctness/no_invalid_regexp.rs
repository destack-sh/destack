use destack_ast::{self as ast, Expression, ScalarLiteral};
use destack_workspace::LintSeverity;
use regex_syntax::Parser;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow invalid regular expression strings.
    ///
    /// Invalid regular expressions will cause runtime errors. This rule
    /// catches syntax errors in regex literals at lint time.
    #[lint(
        id = "no-invalid-regexp",
        code = "LC019",
        category = Correctness,
        level = Ast,
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoInvalidRegexp,
    "Disallow invalid regular expressions"
}

impl LintRule for NoInvalidRegexp {
    fn meta(&self) -> &'static crate::LintMeta {
        NoInvalidRegexp::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            // check regex literals
            let expression = ctx.tree.get(node_id);
            let Expression::ScalarLiteral(ScalarLiteral::RegexString { content, .. }) = expression
            else {
                continue;
            };

            // try to parse the regex
            let regex_content = ctx.strings.get(*content);
            let regex_str = regex_content.as_ref();
            if let Err(e) = Parser::new().parse(regex_str) {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }
                ctx.report(
                    LintDiagnostic::new(
                        NO_INVALID_REGEXP.id,
                        NO_INVALID_REGEXP.code,
                        NO_INVALID_REGEXP.category,
                        severity,
                        format!("invalid regular expression: {e}"),
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label("this regex is invalid"),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_invalid_regex_unmatched_paren() {
        let test = TestProgram::for_rule_without_builtins(NoInvalidRegexp);
        let result = test.lint_ast(
            "test.ds",
            r#"
let re = /(/
"#,
        );
        test.result(result).assert_lint("no-invalid-regexp");
    }

    #[test]
    fn test_detects_invalid_regex_unmatched_bracket() {
        let test = TestProgram::for_rule_without_builtins(NoInvalidRegexp);
        let result = test.lint_ast(
            "test.ds",
            r#"
let re = /[/
"#,
        );
        test.result(result).assert_lint("no-invalid-regexp");
    }

    #[test]
    fn test_detects_invalid_regex_incomplete_escape() {
        let test = TestProgram::for_rule_without_builtins(NoInvalidRegexp);
        let result = test.lint_ast(
            "test.ds",
            r#"
let re = /\p/
"#,
        );
        // regex-syntax considers \p incomplete (missing property name)
        test.result(result).assert_lint("no-invalid-regexp");
    }

    #[test]
    fn test_allows_valid_regex() {
        let test = TestProgram::for_rule_without_builtins(NoInvalidRegexp);
        let result = test.lint_ast(
            "test.ds",
            r#"
let re = /^[a-z]+$/
"#,
        );
        test.result(result).assert_no_lint("no-invalid-regexp");
    }

    #[test]
    fn test_allows_complex_valid_regex() {
        let test = TestProgram::for_rule_without_builtins(NoInvalidRegexp);
        let result = test.lint_ast(
            "test.ds",
            r#"
let re = /(\d{1,3}\.){3}\d{1,3}/
"#,
        );
        test.result(result).assert_no_lint("no-invalid-regexp");
    }

    #[test]
    fn test_detects_invalid_repetition() {
        let test = TestProgram::for_rule_without_builtins(NoInvalidRegexp);
        let result = test.lint_ast(
            "test.ds",
            r#"
let re = /a{3,1}/
"#,
        );
        test.result(result).assert_lint("no-invalid-regexp");
    }
}
