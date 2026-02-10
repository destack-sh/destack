use destack_ast::{self as ast, Expression, ScalarLiteral};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintFix, LintModuleAstContext, LintRule, declare_lint};

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
    /// bad: `/foo/`
    /// good: `/foo/u` or `/foo/v`
    #[lint(
        id = "require-unicode-regexp",
        code = "LP018",
        category = Performance,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Always,
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

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();
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
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }
                let mut diagnostic = LintDiagnostic::new(
                    REQUIRE_UNICODE_REGEXP.id,
                    REQUIRE_UNICODE_REGEXP.code,
                    REQUIRE_UNICODE_REGEXP.category,
                    severity,
                    "regex should have the 'u' or 'v' flag for proper Unicode handling",
                    ctx.module.file_id,
                    ctx.tree.get_span(node_id),
                )
                .with_label("add the 'u' flag for Unicode support");

                // compute fixes only when requested by the runner
                if ctx.compute_fixes
                    && let Some(fix) = unicode_regex_fix(ctx, node_id)
                {
                    diagnostic = diagnostic.with_fix(fix);
                }

                ctx.report(diagnostic);
            }
        }
    }
}

/// Build a safe fix that appends a unicode flag to a regex literal.
fn unicode_regex_fix(
    ctx: &LintModuleAstContext<'_>,
    expression_id: ast::LocalNodeId<Expression>,
) -> Option<LintFix> {
    let expression_span = ctx.tree.get_span(expression_id);
    let expression_text = ctx.get_span_text(expression_span);
    let expression_text: &str = expression_text.as_ref();

    if expression_text.is_empty() {
        return None;
    }

    let replacement = format!("{expression_text}u");
    let edits = ctx
        .edit_builder()
        .replace(expression_span, replacement)
        .into_edits();
    Some(LintFix::safe("Add unicode regex flag").with_edits(edits))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_regex_without_unicode_flag() {
        let test = TestProgram::for_rule_without_prelude(RequireUnicodeRegexp);
        let result = test.lint_ast(
            "require_unicode_regexp/test_detects_regex_without_unicode_flag.ds",
            r#"
let re = /foo/
"#,
        );
        test.result(result).assert_lint("require-unicode-regexp");
    }

    #[test]
    fn test_fix_adds_unicode_flag_without_existing_flags() {
        let test = TestProgram::for_rule_without_prelude(RequireUnicodeRegexp);
        let result = test.lint_ast(
            "require_unicode_regexp/test_fix_adds_unicode_flag_without_existing_flags.ds",
            r#"
let re = /foo/
"#,
        );
        test.result(result)
            .assert_lint("require-unicode-regexp")
            .assert_safe_fixed(
                r#"
let re = /foo/u;
"#,
            );
    }

    #[test]
    fn test_detects_regex_with_other_flags() {
        let test = TestProgram::for_rule_without_prelude(RequireUnicodeRegexp);
        let result = test.lint_ast(
            "require_unicode_regexp/test_detects_regex_with_other_flags.ds",
            r#"
let re = /foo/gi
"#,
        );
        test.result(result).assert_lint("require-unicode-regexp");
    }

    #[test]
    fn test_fix_adds_unicode_flag_with_existing_flags() {
        let test = TestProgram::for_rule_without_prelude(RequireUnicodeRegexp);
        let result = test.lint_ast(
            "require_unicode_regexp/test_fix_adds_unicode_flag_with_existing_flags.ds",
            r#"
let re = /foo/gi
"#,
        );
        test.result(result)
            .assert_lint("require-unicode-regexp")
            .assert_safe_fixed(
                r#"
let re = /foo/giu;
"#,
            );
    }

    #[test]
    fn test_mutation_fix_adds_unicode_flag_with_single_existing_flag() {
        let test = TestProgram::for_rule_without_prelude(RequireUnicodeRegexp);
        let result = test.lint_ast(
            "require_unicode_regexp/test_mutation_fix_adds_unicode_flag_with_single_existing_flag.ds",
            r#"
let re = /foo/g
"#,
        );
        test.result(result)
            .assert_lint("require-unicode-regexp")
            .assert_safe_fixed(
                r#"
let re = /foo/gu;
"#,
            );
    }

    #[test]
    fn test_allows_regex_with_u_flag() {
        let test = TestProgram::for_rule_without_prelude(RequireUnicodeRegexp);
        let result = test.lint_ast(
            "require_unicode_regexp/test_allows_regex_with_u_flag.ds",
            r#"
let re = /foo/u
"#,
        );
        test.result(result).assert_no_lint("require-unicode-regexp");
    }

    #[test]
    fn test_allows_regex_with_u_and_other_flags() {
        let test = TestProgram::for_rule_without_prelude(RequireUnicodeRegexp);
        let result = test.lint_ast(
            "require_unicode_regexp/test_allows_regex_with_u_and_other_flags.ds",
            r#"
let re = /foo/giu
"#,
        );
        test.result(result).assert_no_lint("require-unicode-regexp");
    }

    #[test]
    fn test_allows_regex_with_v_flag() {
        let test = TestProgram::for_rule_without_prelude(RequireUnicodeRegexp);
        let result = test.lint_ast(
            "require_unicode_regexp/test_allows_regex_with_v_flag.ds",
            r#"
let re = /foo/v
"#,
        );
        test.result(result).assert_no_lint("require-unicode-regexp");
    }
}
