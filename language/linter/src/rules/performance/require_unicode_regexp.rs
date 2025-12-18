use destack_ast::{self as ast, Expression, ScalarLiteral};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Require `u` or `v` flag on regular expressions.
    ///
    /// The `u` (unicode) flag enables correct handling of Unicode characters
    /// in regular expressions. Without it, patterns may not match characters
    /// outside the Basic Multilingual Plane correctly, and character classes
    /// like `\w` won't match non-ASCII letters.
    ///
    /// The `v` flag (unicodeSets) is a more powerful alternative that also
    /// enables set notation and properties of strings.
    ///
    /// Bad: `/foo/`
    /// Good: `/foo/u` or `/foo/v`
    #[lint(
        id = "require-unicode-regexp",
        code = "LP003",
        category = Performance,
        level = Ast,
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub RequireUnicodeRegexp,
    "Require unicode flag on regex"
}

impl LintRule for RequireUnicodeRegexp {
    fn meta(&self) -> &'static crate::LintMeta {
        RequireUnicodeRegexp::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            // check regex literals
            let expression = ctx.tree.get(node_id);
            let Expression::ScalarLiteral(ScalarLiteral::RegexString { flags, .. }) = expression
            else {
                continue;
            };

            // check if flags contain 'u' or 'v'
            let has_unicode_flag = if let Some(flags_id) = flags {
                let flags_str = ctx.strings.get(*flags_id);
                let flags_ref = flags_str.as_ref();
                flags_ref.contains('u') || flags_ref.contains('v')
            } else {
                false
            };
            if !has_unicode_flag {
                ctx.report(
                    LintDiagnostic::new(
                        REQUIRE_UNICODE_REGEXP.id,
                        REQUIRE_UNICODE_REGEXP.code,
                        REQUIRE_UNICODE_REGEXP.category,
                        severity,
                        "regex should have the 'u' or 'v' flag for proper Unicode handling",
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label("add the 'u' flag for Unicode support"),
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
    fn test_detects_regex_without_unicode_flag() {
        let test = TestProgram::for_rule(RequireUnicodeRegexp);
        let result = test.lint_ast(
            "test.ds",
            r#"
let re = /foo/
"#,
        );
        test.result(result).assert_lint("require-unicode-regexp");
    }

    #[test]
    fn test_detects_regex_with_other_flags() {
        let test = TestProgram::for_rule(RequireUnicodeRegexp);
        let result = test.lint_ast(
            "test.ds",
            r#"
let re = /foo/gi
"#,
        );
        test.result(result).assert_lint("require-unicode-regexp");
    }

    #[test]
    fn test_allows_regex_with_u_flag() {
        let test = TestProgram::for_rule(RequireUnicodeRegexp);
        let result = test.lint_ast(
            "test.ds",
            r#"
let re = /foo/u
"#,
        );
        test.result(result).assert_no_lint("require-unicode-regexp");
    }

    #[test]
    fn test_allows_regex_with_u_and_other_flags() {
        let test = TestProgram::for_rule(RequireUnicodeRegexp);
        let result = test.lint_ast(
            "test.ds",
            r#"
let re = /foo/giu
"#,
        );
        test.result(result).assert_no_lint("require-unicode-regexp");
    }

    #[test]
    fn test_allows_regex_with_v_flag() {
        let test = TestProgram::for_rule(RequireUnicodeRegexp);
        let result = test.lint_ast(
            "test.ds",
            r#"
let re = /foo/v
"#,
        );
        test.result(result).assert_no_lint("require-unicode-regexp");
    }
}
