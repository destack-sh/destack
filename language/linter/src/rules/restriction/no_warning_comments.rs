use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::rules::common::{comment_contains_warning_term, is_directive_comment};
use crate::{LintAstContext, LintDiagnostic, LintFix, LintMeta, LintRule, declare_lint};

declare_lint! {
    /// Disallow specified warning terms in comments.
    ///
    /// Warning comments like TODO, FIXME, and HACK indicate incomplete work.
    /// Resolve these before committing or track them in an issue tracker.
    #[lint(
        id = "no-warning-comments",
        code = "LR030",
        category = Restriction,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Always,
        recommended = Strict,
        stability = Stable
    )]
    pub NoWarningComments,
    "Disallow warning comments"
}

impl LintRule for NoWarningComments {
    fn meta(&self) -> &'static LintMeta {
        NoWarningComments::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();
        let warning_terms = &ctx.options.warning_comment_terms;

        // iterate over all comment trivia records
        for trivia in ctx.tree.comment_trivia().iter().copied() {
            let comment_text = ast::normalize_comment_payload(ctx.get_span_text(trivia.span));
            if is_directive_comment(&comment_text)
                && comment_contains_warning_term(&comment_text, "no-warning-comments")
            {
                continue;
            }

            // check for warning terms in the comment
            for term in warning_terms {
                if comment_contains_warning_term(&comment_text, term) {
                    let severity = ctx.get_effective_severity(meta, trivia.comment);
                    if !severity.is_enabled() {
                        break;
                    }
                    let mut diagnostic = LintDiagnostic::new(
                        NO_WARNING_COMMENTS.id,
                        NO_WARNING_COMMENTS.code,
                        NO_WARNING_COMMENTS.category,
                        severity,
                        format!("warning comment contains `{term}`"),
                        ctx.module.file_id,
                        trivia.span,
                    )
                    .with_label("resolve before committing");

                    // compute fixes only when requested by the runner
                    if ctx.compute_fixes
                        && let Some(fix) = warning_comment_fix(ctx, trivia.span)
                    {
                        diagnostic = diagnostic.with_fix(fix);
                    }

                    ctx.report(diagnostic);
                    break; // only report once per comment
                }
            }
        }
    }
}

/// Build a safe fix by removing one warning comment.
fn warning_comment_fix(
    ctx: &LintAstContext<'_>,
    comment_span: destack_source::Span,
) -> Option<LintFix> {
    let edits = ctx.edit_builder().delete(comment_span).into_edits();
    Some(LintFix::safe("Remove warning comment").with_edits(edits))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_todo_comment() {
        let test = TestProgram::for_rule_without_prelude(NoWarningComments);
        let result = test.lint_ast(
            "no_warning_comments/test_detects_todo_comment.ts",
            "// TODO: fix this",
        );
        test.result(result).assert_lint("no-warning-comments");
    }

    #[test]
    fn test_fix_removes_todo_comment() {
        let test = TestProgram::for_rule_without_prelude(NoWarningComments);
        let result = test.lint_ast(
            "no_warning_comments/test_fix_removes_todo_comment.ts",
            r#"
const value = 1 // TODO: remove temporary path
"#,
        );
        test.result(result)
            .assert_lint("no-warning-comments")
            .assert_safe_fixed(
                r#"
const value = 1;
"#,
            );
    }

    #[test]
    fn test_detects_fixme_comment() {
        let test = TestProgram::for_rule_without_prelude(NoWarningComments);
        let result = test.lint_ast(
            "no_warning_comments/test_detects_fixme_comment.ts",
            "// FIXME: broken",
        );
        test.result(result).assert_lint("no-warning-comments");
    }

    #[test]
    fn test_detects_hack_comment() {
        let test = TestProgram::for_rule_without_prelude(NoWarningComments);
        let result = test.lint_ast(
            "no_warning_comments/test_detects_hack_comment.ts",
            "/* HACK: temporary workaround */",
        );
        test.result(result).assert_lint("no-warning-comments");
    }

    #[test]
    fn test_case_insensitive() {
        let test = TestProgram::for_rule_without_prelude(NoWarningComments);
        let result = test.lint_ast(
            "no_warning_comments/test_case_insensitive.ts",
            "// todo: lowercase",
        );
        test.result(result).assert_lint("no-warning-comments");
    }

    #[test]
    fn test_allows_normal_comment() {
        let test = TestProgram::for_rule_without_prelude(NoWarningComments);
        let result = test.lint_ast(
            "no_warning_comments/test_allows_normal_comment.ts",
            "// this is a regular comment",
        );
        test.result(result).assert_no_lint("no-warning-comments");
    }

    #[test]
    fn test_mutation_fix_removes_block_hack_comment() {
        let test = TestProgram::for_rule_without_prelude(NoWarningComments);
        let result = test.lint_ast(
            "no_warning_comments/test_mutation_fix_removes_block_hack_comment.ts",
            r#"
/* HACK: temporary workaround */
const value = 1
"#,
        );
        test.result(result)
            .assert_lint("no-warning-comments")
            .assert_safe_fixed(
                r#"
const value = 1;
"#,
            );
    }

    #[test]
    fn test_skips_substring_warning_terms() {
        let test = TestProgram::for_rule_without_prelude(NoWarningComments);
        let result = test.lint_ast(
            "no_warning_comments/test_skips_substring_warning_terms.ts",
            r#"
// TodoMVC integration documentation
const value = 1;
"#,
        );
        test.result(result).assert_no_lint("no-warning-comments");
    }

    #[test]
    fn test_skips_no_warning_comments_directive_comment() {
        let test = TestProgram::for_rule_without_prelude(NoWarningComments);
        let result = test.lint_ast(
            "no_warning_comments/test_skips_no_warning_comments_directive_comment.ts",
            r#"
// eslint-disable-next-line no-warning-comments TODO
const value = 1;
"#,
        );
        test.result(result).assert_no_lint("no-warning-comments");
    }

    #[test]
    fn test_does_not_skip_non_directive_comment_with_rule_name() {
        let test = TestProgram::for_rule_without_prelude(NoWarningComments);
        let result = test.lint_ast(
            "no_warning_comments/test_does_not_skip_non_directive_comment_with_rule_name.ts",
            r#"
// this mentions no-warning-comments but still has TODO
const value = 1;
"#,
        );
        test.result(result).assert_lint("no-warning-comments");
    }
}
