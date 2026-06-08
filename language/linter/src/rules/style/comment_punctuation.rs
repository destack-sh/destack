use crate::LintMeta;
use destack_dir as dir;
use destack_repository::LintSeverity;
use destack_source::Span;

use crate::rules::common::{
    has_doc_terminal_punctuation, is_directive_comment, is_doc_comment_source,
    is_non_prose_doc_line, is_separator_comment, parse_keyword_comment_with_options,
};
use crate::{LintFix, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Enforce comment punctuation conventions.
    ///
    /// Inline comments should avoid trailing periods.
    /// Documentation comment prose lines should end with punctuation.
    #[lint(
        id = "comment-punctuation",
        code = "LY004",
        category = Style,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable
    )]
    pub CommentPunctuation,
    "Enforce comment punctuation"
}

impl LintRule for CommentPunctuation {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        CommentPunctuation::meta()
    }

    /// Check module source annotations for punctuation consistency.
    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();

        // inline comments should not end with periods
        for comment in ctx.dir.comments().iter().copied() {
            if is_doc_comment_source(ctx.get_span_text(comment.span)) {
                continue;
            }

            let text = dir::normalize_comment_payload(ctx.get_span_text(comment.span));
            let text = text.as_ref().trim();

            // skip comments that have explicit exceptions
            if text.is_empty()
                || is_directive_comment(text)
                || is_separator_comment(text)
                || parse_keyword_comment_with_options(
                    text,
                    &ctx.options().style.comment_keywords,
                    &ctx.options().style.comment_keyword_tags,
                )
                .is_some()
            {
                continue;
            }

            // report trailing periods
            if text.ends_with('.') {
                let severity = ctx.get_severity(meta);
                if !severity.is_enabled() {
                    continue;
                }

                let mut diagnostic = LintReport::new(
                    COMMENT_PUNCTUATION.id,
                    COMMENT_PUNCTUATION.code,
                    COMMENT_PUNCTUATION.category,
                    severity,
                    "inline comment should not end with a period",
                    comment.span,
                )
                .label("remove trailing period");

                // compute fixes only when requested by the runner
                if ctx.compute_fixes
                    && let Some(fix) = inline_comment_trailing_period_fix(ctx, comment.span)
                {
                    diagnostic = diagnostic.fix(fix);
                }

                ctx.report(diagnostic);
            }
        }

        // doc comments should end each prose line with punctuation
        for comment in ctx.dir.comments().iter().copied() {
            if !is_doc_comment_source(ctx.get_span_text(comment.span)) {
                continue;
            }

            let text = dir::normalize_comment_payload(ctx.get_span_text(comment.span));
            let mut has_missing_punctuation = false;

            // inspect each prose line
            for line in text.as_ref().lines() {
                if is_non_prose_doc_line(line) {
                    continue;
                }

                if has_doc_terminal_punctuation(line) {
                    continue;
                }

                has_missing_punctuation = true;
                break;
            }

            // report missing punctuation once per comment
            if has_missing_punctuation {
                let severity = ctx.get_severity(meta);
                if !severity.is_enabled() {
                    continue;
                }

                ctx.report(
                    LintReport::new(
                        COMMENT_PUNCTUATION.id,
                        COMMENT_PUNCTUATION.code,
                        COMMENT_PUNCTUATION.category,
                        severity,
                        "doc comment lines should end with punctuation",
                        comment.span,
                    )
                    .label("add punctuation to each sentence line"),
                );
            }
        }
    }
}

/// Build a safe fix that removes one trailing period from an inline comment.
fn inline_comment_trailing_period_fix(
    ctx: &LintModuleContext<'_>,
    comment_span: Span,
) -> Option<LintFix> {
    let annotation_text = ctx.get_span_text(comment_span);

    // find the last non-whitespace character in the comment text
    let mut last_non_whitespace = None;
    for (offset, character) in annotation_text.char_indices() {
        if !character.is_whitespace() {
            last_non_whitespace = Some((offset, character));
        }
    }
    let (offset, character) = last_non_whitespace?;
    if character != '.' {
        return None;
    }

    let period_span = Span::new(
        comment_span.file,
        comment_span.start + offset as u32,
        comment_span.start + offset as u32 + 1,
    );
    let edits = ctx.edit_builder().delete(period_span).into_edits();
    Some(LintFix::safe("Remove trailing comment period").with_edits(edits))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Allow inline comments without trailing periods.
    #[test]
    fn test_inline_no_period_allowed() {
        let test = TestProgram::for_rule_without_prelude(CommentPunctuation);
        let result = test.lint(
            "comment_punctuation/test_inline_no_period_allowed.ds",
            r#"
let x = 1 // increment counter
"#,
        );
        test.result(result).assert_no_lint("comment-punctuation");
    }

    /// Detect trailing periods on inline comments.
    #[test]
    fn test_inline_with_period_detected() {
        let test = TestProgram::for_rule_without_prelude(CommentPunctuation);
        let result = test.lint(
            "comment_punctuation/test_inline_with_period_detected.ds",
            r#"
let x = 1 // increment counter.
"#,
        );
        test.result(result).assert_lint("comment-punctuation");
    }

    /// Remove trailing periods from inline comments.
    #[test]
    fn test_fix_removes_inline_trailing_period() {
        let test = TestProgram::for_rule_without_prelude(CommentPunctuation);
        let result = test.lint(
            "comment_punctuation/test_fix_removes_inline_trailing_period.ds",
            r#"
let x = 1 // increment counter.
"#,
        );
        test.result(result)
            .assert_lint("comment-punctuation")
            .assert_safe_fixed(
                r#"
let x = 1; // increment counter
"#,
            );
    }

    /// Allow doc comments with periods.
    #[test]
    fn test_doc_with_period_allowed() {
        let test = TestProgram::for_rule_without_prelude(CommentPunctuation);
        let result = test.lint(
            "comment_punctuation/test_doc_with_period_allowed.ds",
            r#"
/// Increments the counter.
function foo() {}
"#,
        );
        test.result(result).assert_no_lint("comment-punctuation");
    }

    /// Detect missing punctuation in doc comments.
    #[test]
    fn test_doc_without_punctuation_detected() {
        let test = TestProgram::for_rule_without_prelude(CommentPunctuation);
        let result = test.lint(
            "comment_punctuation/test_doc_without_punctuation_detected.ds",
            r#"
/// Increments the counter
function foo() {}
"#,
        );
        test.result(result).assert_lint("comment-punctuation");
    }

    /// Avoid auto-fixing missing punctuation in doc comments.
    #[test]
    fn test_no_fix_for_missing_doc_punctuation() {
        let test = TestProgram::for_rule_without_prelude(CommentPunctuation);
        let result = test.lint(
            "comment_punctuation/test_no_fix_for_missing_doc_punctuation.ds",
            r#"
/// Increments the counter
function foo() {}
"#,
        );
        test.result(result)
            .assert_lint("comment-punctuation")
            .assert_has_no_fix("comment-punctuation");
    }

    /// Allow doc comment lines ending with question marks.
    #[test]
    fn test_doc_with_question_allowed() {
        let test = TestProgram::for_rule_without_prelude(CommentPunctuation);
        let result = test.lint(
            "comment_punctuation/test_doc_with_question_allowed.ds",
            r#"
/// Is this valid?
function foo() {}
"#,
        );
        test.result(result).assert_no_lint("comment-punctuation");
    }

    /// Allow doc comment lines ending with exclamation marks.
    #[test]
    fn test_doc_with_exclamation_allowed() {
        let test = TestProgram::for_rule_without_prelude(CommentPunctuation);
        let result = test.lint(
            "comment_punctuation/test_doc_with_exclamation_allowed.ds",
            r#"
/// Do not call this!
function foo() {}
"#,
        );
        test.result(result).assert_no_lint("comment-punctuation");
    }

    /// Skip NOTE style comments with keyword tags.
    #[test]
    fn test_keyword_comment_skipped() {
        let test = TestProgram::for_rule_without_prelude(CommentPunctuation);
        let result = test.lint(
            "comment_punctuation/test_keyword_comment_skipped.ds",
            r#"
let x = 1 // NOTE #Cleanup: remove fallback
"#,
        );
        test.result(result).assert_no_lint("comment-punctuation");
    }

    /// Enforce punctuation for each doc prose line.
    #[test]
    fn test_doc_each_line_requires_punctuation() {
        let test = TestProgram::for_rule_without_prelude(CommentPunctuation);
        let result = test.lint(
            "comment_punctuation/test_doc_each_line_requires_punctuation.ds",
            r#"
/// Send a message.
/// Include metadata
function foo() {}
"#,
        );
        test.result(result).assert_lint("comment-punctuation");
    }

    /// Skip non prose lines inside doc comments.
    #[test]
    fn test_doc_non_prose_lines_ignored() {
        let test = TestProgram::for_rule_without_prelude(CommentPunctuation);
        let result = test.lint(
            "comment_punctuation/test_doc_non_prose_lines_ignored.ds",
            r#"
/// Send a message.
/// @param message
/// - one
/// - two
function foo(message: string) {}
"#,
        );
        test.result(result).assert_no_lint("comment-punctuation");
    }

    /// Allow doc lines ending with a colon.
    #[test]
    fn test_doc_line_with_colon_allowed() {
        let test = TestProgram::for_rule_without_prelude(CommentPunctuation);
        let result = test.lint(
            "comment_punctuation/test_doc_line_with_colon_allowed.ds",
            r#"
/// Send a message:
/// Use the default transport.
function foo() {}
"#,
        );
        test.result(result).assert_no_lint("comment-punctuation");
    }

    /// Skip separator comments for punctuation checks.
    #[test]
    fn test_separator_comment_skipped() {
        let test = TestProgram::for_rule_without_prelude(CommentPunctuation);
        let result = test.lint(
            "comment_punctuation/test_separator_comment_skipped.ds",
            r#"
// ================================================================================
const value = 1;
"#,
        );
        test.result(result).assert_no_lint("comment-punctuation");
    }
}
