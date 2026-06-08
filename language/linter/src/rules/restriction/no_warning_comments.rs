use destack_dir as dir;
use destack_repository::{LintSeverity, WarningCommentLocation};
use destack_source::Span;

use crate::rules::common::{comment_contains_warning_term, is_directive_comment};
use crate::{LintFix, LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow specified warning terms in comments.
    ///
    /// Warning comments like TODO, FIXME, and HACK indicate incomplete work.
    /// Resolve these before committing or track them in an issue tracker.
    #[lint(
        id = "no-warning-comments",
        code = "LR030",
        category = Restriction,
        level = Dir,
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

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();
        let warning_terms = ctx.options().restriction.warning_comment_terms.clone();
        let warning_location = ctx.options().restriction.warning_comment_location;
        let warning_decoration = ctx.options().restriction.warning_comment_decoration.clone();

        // iterate over all raw comments
        for comment in ctx.dir.comments().iter().copied() {
            let comment_text = dir::normalize_comment_payload(ctx.get_span_text(comment.span));
            let comment_text = comment_text.into_owned();
            if is_directive_comment(&comment_text)
                && comment_contains_warning_term(
                    &comment_text,
                    "no-warning-comments",
                    destack_repository::WarningCommentLocation::Anywhere,
                    &[],
                )
            {
                continue;
            }

            report_warning_comment(
                ctx,
                meta,
                comment.span,
                &comment_text,
                &warning_terms,
                warning_location,
                &warning_decoration,
            );
        }
    }
}

/// Report one warning comment diagnostic when the text matches configured terms.
fn report_warning_comment(
    ctx: &mut LintModuleContext<'_>,
    meta: &LintMeta,
    span: Span,
    comment_text: &str,
    warning_terms: &[String],
    warning_location: WarningCommentLocation,
    warning_decoration: &[String],
) {
    // report at most once per comment span
    for term in warning_terms {
        if !comment_contains_warning_term(comment_text, term, warning_location, warning_decoration)
        {
            continue;
        }

        let severity = ctx.get_severity(meta);
        if !severity.is_enabled() {
            break;
        }

        let mut diagnostic = LintReport::new(
            NO_WARNING_COMMENTS.id,
            NO_WARNING_COMMENTS.code,
            NO_WARNING_COMMENTS.category,
            severity,
            format!("warning comment contains `{term}`"),
            span,
        )
        .label("resolve before committing");

        // compute fixes only when requested by the runner
        if ctx.compute_fixes
            && let Some(fix) = warning_comment_fix(ctx, span)
        {
            diagnostic = diagnostic.fix(fix);
        }

        ctx.report(diagnostic);
        break;
    }
}

/// Build a suggestion by removing one warning comment.
fn warning_comment_fix(ctx: &LintModuleContext<'_>, comment_span: Span) -> Option<LintFix> {
    let edits = ctx.edit_builder().delete(comment_span).into_edits();
    Some(LintFix::suggestion("Remove warning comment").with_edits(edits))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_todo_comment() {
        let test = TestProgram::for_rule_without_prelude(NoWarningComments);
        let result = test.lint(
            "no_warning_comments/test_detects_todo_comment.ts",
            "// TODO: fix this",
        );
        test.result(result).assert_lint("no-warning-comments");
    }

    #[test]
    fn test_fix_removes_todo_comment() {
        let test = TestProgram::for_rule_without_prelude(NoWarningComments);
        let result = test.lint(
            "no_warning_comments/test_fix_removes_todo_comment.ts",
            r#"
const value = 1 // TODO: remove temporary path
"#,
        );
        test.result(result)
            .assert_lint("no-warning-comments")
            .assert_suggested_fixed(
                r#"
const value = 1;
"#,
            );
    }

    #[test]
    fn test_detects_fixme_comment() {
        let test = TestProgram::for_rule_without_prelude(NoWarningComments);
        let result = test.lint(
            "no_warning_comments/test_detects_fixme_comment.ts",
            "// FIXME: broken",
        );
        test.result(result).assert_lint("no-warning-comments");
    }

    #[test]
    fn test_reports_doc_comment_once() {
        let test = TestProgram::for_rule_without_prelude(NoWarningComments);
        let result = test.lint(
            "no_warning_comments/test_reports_doc_comment_once.ts",
            "/** TODO: document this */",
        );
        test.result(result)
            .assert_lint("no-warning-comments")
            .assert_lint_count("no-warning-comments", 1);
    }

    #[test]
    fn test_detects_hack_comment() {
        let test = TestProgram::for_rule_without_prelude(NoWarningComments);
        let result = test.lint(
            "no_warning_comments/test_detects_hack_comment.ts",
            "/* HACK: temporary workaround */",
        );
        test.result(result).assert_lint("no-warning-comments");
    }

    #[test]
    fn test_case_insensitive() {
        let test = TestProgram::for_rule_without_prelude(NoWarningComments);
        let result = test.lint(
            "no_warning_comments/test_case_insensitive.ts",
            "// todo: lowercase",
        );
        test.result(result).assert_lint("no-warning-comments");
    }

    #[test]
    fn test_allows_normal_comment() {
        let test = TestProgram::for_rule_without_prelude(NoWarningComments);
        let result = test.lint(
            "no_warning_comments/test_allows_normal_comment.ts",
            "// this is a regular comment",
        );
        test.result(result).assert_no_lint("no-warning-comments");
    }

    #[test]
    fn test_mutation_fix_removes_block_hack_comment() {
        let test = TestProgram::for_rule_without_prelude(NoWarningComments);
        let result = test.lint(
            "no_warning_comments/test_mutation_fix_removes_block_hack_comment.ts",
            r#"
/* HACK: temporary workaround */
const value = 1
"#,
        );
        test.result(result)
            .assert_lint("no-warning-comments")
            .assert_suggested_fixed(
                r#"
const value = 1;
"#,
            );
    }

    #[test]
    fn test_skips_substring_warning_terms() {
        let test = TestProgram::for_rule_without_prelude(NoWarningComments);
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
            "no_warning_comments/test_does_not_skip_non_directive_comment_with_rule_name.ts",
            r#"
// this mentions no-warning-comments but still has TODO
const value = 1;
"#,
        );
        test.result(result).assert_no_lint("no-warning-comments");
    }

    #[test]
    fn test_detects_warning_term_anywhere_when_enabled() {
        let test =
            TestProgram::for_rule_without_prelude(NoWarningComments).with_options(|options| {
                options.restriction.warning_comment_location =
                    destack_repository::WarningCommentLocation::Anywhere;
            });
        let result = test.lint(
            "no_warning_comments/test_detects_warning_term_anywhere_when_enabled.ts",
            r#"
// this mentions no-warning-comments but still has TODO
const value = 1;
"#,
        );
        test.result(result).assert_lint("no-warning-comments");
    }

    #[test]
    fn test_allows_warning_term_after_decoration_by_default() {
        let test = TestProgram::for_rule_without_prelude(NoWarningComments);
        let result = test.lint(
            "no_warning_comments/test_allows_warning_term_after_decoration_by_default.ts",
            r#"
/* *** TODO: finish this */
const value = 1;
"#,
        );
        test.result(result).assert_no_lint("no-warning-comments");
    }

    #[test]
    fn test_detects_warning_term_after_configured_decoration() {
        let test =
            TestProgram::for_rule_without_prelude(NoWarningComments).with_options(|options| {
                options.restriction.warning_comment_decoration = vec![String::from("*")];
            });
        let result = test.lint(
            "no_warning_comments/test_detects_warning_term_after_configured_decoration.ts",
            r#"
/* *** TODO: finish this */
const value = 1;
"#,
        );
        test.result(result).assert_lint("no-warning-comments");
    }
}
