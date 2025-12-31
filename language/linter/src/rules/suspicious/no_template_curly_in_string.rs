use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow template literal syntax in regular strings.
    ///
    /// Using `"${x}"` in a regular string instead of a template literal
    /// `` `${x}` `` won't interpolate the variable and is likely a mistake.
    #[lint(
        id = "no-template-curly-in-string",
        code = "LU042",
        category = Suspicious,
        level = Ast,
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoTemplateCurlyInString,
    "Disallow template syntax in regular strings"
}

impl LintRule for NoTemplateCurlyInString {
    fn meta(&self) -> &'static crate::LintMeta {
        NoTemplateCurlyInString::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expr = ctx.tree.get(node_id);
            let ast::Expression::ScalarLiteral(ast::ScalarLiteral::String(string_id)) = expr else {
                continue;
            };

            let string_value = ctx.strings.get(*string_id);
            let string_str = string_value.as_ref();

            // check for ${...} patterns in the string
            if contains_template_syntax(string_str) {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }

                ctx.report(
                    LintDiagnostic::new(
                        NO_TEMPLATE_CURLY_IN_STRING.id,
                        NO_TEMPLATE_CURLY_IN_STRING.code,
                        NO_TEMPLATE_CURLY_IN_STRING.category,
                        severity,
                        "template syntax in regular string",
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label("use a template literal `...` instead of \"...\""),
                );
            }
        }
    }
}

/// Check if a string contains template literal syntax like ${...}.
fn contains_template_syntax(s: &str) -> bool {
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '$' && chars.peek() == Some(&'{') {
            // found ${, check for matching }
            chars.next(); // consume {
            let mut depth = 1;
            for c in chars.by_ref() {
                match c {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            return true;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_template_in_double_quoted_string() {
        let test = TestProgram::for_rule_without_builtins(NoTemplateCurlyInString);
        let result = test.lint_ast(
            "test.ds",
            r#"
const x = "Hello ${name}"
"#,
        );
        test.result(result)
            .assert_lint("no-template-curly-in-string");
    }

    #[test]
    fn test_detects_multiple_templates() {
        let test = TestProgram::for_rule_without_builtins(NoTemplateCurlyInString);
        let result = test.lint_ast(
            "test.ds",
            r#"
const x = "${a} + ${b} = ${c}"
"#,
        );
        test.result(result)
            .assert_lint("no-template-curly-in-string");
    }

    #[test]
    fn test_allows_template_literal() {
        let test = TestProgram::for_rule_without_builtins(NoTemplateCurlyInString);
        let result = test.lint_ast(
            "test.ds",
            r#"
const x = `Hello ${name}`
"#,
        );
        test.result(result)
            .assert_no_lint("no-template-curly-in-string");
    }

    #[test]
    fn test_allows_regular_string() {
        let test = TestProgram::for_rule_without_builtins(NoTemplateCurlyInString);
        let result = test.lint_ast(
            "test.ds",
            r#"
const x = "Hello world"
"#,
        );
        test.result(result)
            .assert_no_lint("no-template-curly-in-string");
    }

    #[test]
    fn test_allows_dollar_without_brace() {
        let test = TestProgram::for_rule_without_builtins(NoTemplateCurlyInString);
        let result = test.lint_ast(
            "test.ds",
            r#"
const x = "Price: $100"
"#,
        );
        test.result(result)
            .assert_no_lint("no-template-curly-in-string");
    }

    #[test]
    fn test_allows_incomplete_template() {
        let test = TestProgram::for_rule_without_builtins(NoTemplateCurlyInString);
        let result = test.lint_ast(
            "test.ds",
            r#"
const x = "${unclosed"
"#,
        );
        test.result(result)
            .assert_no_lint("no-template-curly-in-string");
    }
}
