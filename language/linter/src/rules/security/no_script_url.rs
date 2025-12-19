use destack_ast::{self as ast, Expression, ScalarLiteral};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow `javascript:` URLs.
    ///
    /// Using `javascript:` URLs is a form of eval and can be a security risk
    /// as it allows arbitrary code execution. It's commonly used in XSS attacks.
    #[lint(
        id = "no-script-url",
        code = "LS001",
        category = Security,
        level = Ast,
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoScriptUrl,
    "Disallow javascript: URLs"
}

impl LintRule for NoScriptUrl {
    fn meta(&self) -> &'static crate::LintMeta {
        NoScriptUrl::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);

            // check string literals
            let Expression::ScalarLiteral(ScalarLiteral::String(string_id)) = expression else {
                continue;
            };

            let string_value = ctx.strings.get(*string_id);
            let string_str = string_value.as_ref();

            // check for javascript: URL (case-insensitive)
            let trimmed = string_str.trim();
            if trimmed.to_lowercase().starts_with("javascript:") {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }
                ctx.report(
                    LintDiagnostic::new(
                        NO_SCRIPT_URL.id,
                        NO_SCRIPT_URL.code,
                        NO_SCRIPT_URL.category,
                        severity,
                        "javascript: URLs are a security risk",
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label("avoid using javascript: URLs"),
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
    fn test_detects_javascript_url() {
        let test = TestProgram::for_rule(NoScriptUrl);
        let result = test.lint_ast(
            "test.ds",
            r#"
let url = "javascript:alert('XSS')"
"#,
        );
        test.result(result).assert_lint("no-script-url");
    }

    #[test]
    fn test_detects_javascript_url_case_insensitive() {
        let test = TestProgram::for_rule(NoScriptUrl);
        let result = test.lint_ast(
            "test.ds",
            r#"
let url = "JavaScript:alert('XSS')"
"#,
        );
        test.result(result).assert_lint("no-script-url");
    }

    #[test]
    fn test_detects_javascript_url_with_whitespace() {
        let test = TestProgram::for_rule(NoScriptUrl);
        let result = test.lint_ast(
            "test.ds",
            r#"
let url = "  javascript:void(0)"
"#,
        );
        test.result(result).assert_lint("no-script-url");
    }

    #[test]
    fn test_allows_normal_url() {
        let test = TestProgram::for_rule(NoScriptUrl);
        let result = test.lint_ast(
            "test.ds",
            r#"
let url = "https://example.com"
"#,
        );
        test.result(result).assert_no_lint("no-script-url");
    }

    #[test]
    fn test_allows_string_containing_javascript_word() {
        let test = TestProgram::for_rule(NoScriptUrl);
        let result = test.lint_ast(
            "test.ds",
            r#"
let msg = "I love javascript programming"
"#,
        );
        test.result(result).assert_no_lint("no-script-url");
    }

    #[test]
    fn test_allows_data_url() {
        let test = TestProgram::for_rule(NoScriptUrl);
        let result = test.lint_ast(
            "test.ds",
            r#"
let url = "data:text/html,<h1>Hello</h1>"
"#,
        );
        test.result(result).assert_no_lint("no-script-url");
    }
}
