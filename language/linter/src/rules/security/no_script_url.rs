use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::rules::common::expression_static_string_literal_source_form;
use crate::{LintAstContext, LintMeta, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow `javascript:` URLs.
    ///
    /// Using `javascript:` URLs is a form of eval and can be a security risk
    /// as it allows arbitrary code execution. It's commonly used in XSS attacks.
    #[lint(
        id = "no-script-url",
        code = "LS008",
        category = Security,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoScriptUrl,
    "Disallow javascript: URLs"
}

impl LintRule for NoScriptUrl {
    fn meta(&self) -> &'static LintMeta {
        NoScriptUrl::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        // inspect candidate expressions
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            // check static string-like expressions
            let Some(string_id) = expression_static_string_literal_source_form(ctx.tree, node_id)
            else {
                continue;
            };

            // resolve string value
            let string_value = ctx.strings.get(string_id);
            let string_str = string_value.as_ref();

            // check for javascript: URL
            if starts_with_javascript_scheme(string_str) {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }
                ctx.report(
                    LintReport::new(
                        NO_SCRIPT_URL.id,
                        NO_SCRIPT_URL.code,
                        NO_SCRIPT_URL.category,
                        severity,
                        "javascript: URLs are a security risk",
                        ctx.tree.get_span(node_id),
                    )
                    .label("avoid using javascript: URLs"),
                );
            }
        }
    }
}

/// Return true when one string starts with the `javascript:` URL scheme.
fn starts_with_javascript_scheme(value: &str) -> bool {
    const JAVASCRIPT_SCHEME: &str = "javascript:";

    let trimmed = value.trim_start_matches(|character: char| character.is_ascii_whitespace());
    let Some(prefix) = trimmed.get(..JAVASCRIPT_SCHEME.len()) else {
        return false;
    };

    prefix.eq_ignore_ascii_case(JAVASCRIPT_SCHEME)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_javascript_url() {
        let test = TestProgram::for_rule_without_prelude(NoScriptUrl);
        let result = test.lint_ast(
            "no_script_url/test_detects_javascript_url.ds",
            r#"
let url = "javascript:alert('XSS')"
"#,
        );
        test.result(result).assert_lint("no-script-url");
    }

    #[test]
    fn test_detects_javascript_url_case_insensitive() {
        let test = TestProgram::for_rule_without_prelude(NoScriptUrl);
        let result = test.lint_ast(
            "no_script_url/test_detects_javascript_url_case_insensitive.ds",
            r#"
let url = "JavaScript:alert('XSS')"
"#,
        );
        test.result(result).assert_lint("no-script-url");
    }

    #[test]
    fn test_detects_javascript_url_with_whitespace() {
        let test = TestProgram::for_rule_without_prelude(NoScriptUrl);
        let result = test.lint_ast(
            "no_script_url/test_detects_javascript_url_with_whitespace.ds",
            r#"
let url = "  javascript:void(0)"
"#,
        );
        test.result(result).assert_lint("no-script-url");
    }

    #[test]
    fn test_allows_normal_url() {
        let test = TestProgram::for_rule_without_prelude(NoScriptUrl);
        let result = test.lint_ast(
            "no_script_url/test_allows_normal_url.ds",
            r#"
let url = "https://example.com"
"#,
        );
        test.result(result).assert_no_lint("no-script-url");
    }

    #[test]
    fn test_allows_string_containing_javascript_word() {
        let test = TestProgram::for_rule_without_prelude(NoScriptUrl);
        let result = test.lint_ast(
            "no_script_url/test_allows_string_containing_javascript_word.ds",
            r#"
let msg = "I love javascript programming"
"#,
        );
        test.result(result).assert_no_lint("no-script-url");
    }

    #[test]
    fn test_allows_data_url() {
        let test = TestProgram::for_rule_without_prelude(NoScriptUrl);
        let result = test.lint_ast(
            "no_script_url/test_allows_data_url.ds",
            r#"
let url = "data:text/html,<h1>Hello</h1>"
"#,
        );
        test.result(result).assert_no_lint("no-script-url");
    }

    #[test]
    fn test_detects_template_script_url() {
        let test = TestProgram::for_rule_without_prelude(NoScriptUrl);
        let result = test.lint_ast(
            "no_script_url/test_detects_template_script_url.ds",
            r#"
let url = `javascript:alert('XSS')`
"#,
        );
        test.result(result).assert_lint("no-script-url");
    }

    #[test]
    fn test_allows_tagged_template_script_url() {
        let test = TestProgram::for_rule_without_prelude(NoScriptUrl);
        let result = test.lint_ast(
            "no_script_url/test_allows_tagged_template_script_url.ds",
            r#"
let url = safe`javascript:alert('XSS')`
"#,
        );
        test.result(result).assert_no_lint("no-script-url");
    }
}
