use crate::LintMeta;
use destack_dir as dir;
use destack_source::Span;
use destack_workspace::LintSeverity;

use crate::rules::common::{
    has_hyphen_separator, has_multiple_sentence_starts, is_directive_comment,
    is_doc_comment_source, is_non_prose_doc_line, is_separator_comment,
    parse_keyword_comment_with_options,
};
use crate::{LintFix, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Enforce comment layout conventions.
    ///
    /// Doc comments should have one sentence per line for better readability.
    /// Doc comments should be upper case sentences.
    /// Single-line inline comments should be lowercase sentences.
    /// Non-trivial logic blocks should have preceding comments.
    #[lint(
        id = "comment-layout",
        code = "LY003",
        category = Style,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable
    )]
    pub CommentLayout,
    "Enforce comment layout"
}

impl LintRule for CommentLayout {
    fn meta(&self) -> &'static LintMeta {
        CommentLayout::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();

        // check doc comments
        for comment in ctx.dir.comments().iter().copied() {
            if !is_doc_comment_source(ctx.get_span_text(comment.span)) {
                continue;
            }

            let text = dir::normalize_comment_payload(ctx.get_span_text(comment.span)).into_owned();
            let mut has_multiple_sentence_line = false;

            // check each prose line for multiple sentence starts
            for line in text.lines() {
                let trimmed = line.trim();

                // skip non prose lines
                if is_non_prose_doc_line(trimmed) {
                    continue;
                }

                // check for multiple sentences on one line
                if has_multiple_sentence_starts(trimmed) {
                    has_multiple_sentence_line = true;
                    break;
                }
            }

            // report multi sentence doc lines
            if has_multiple_sentence_line {
                let severity = ctx.get_severity(meta);
                if !severity.is_enabled() {
                    continue;
                }

                ctx.report(
                    LintReport::new(
                        COMMENT_LAYOUT.id,
                        COMMENT_LAYOUT.code,
                        COMMENT_LAYOUT.category,
                        severity,
                        "doc comment should have one sentence per line",
                        comment.span,
                    )
                    .label("split sentences across multiple lines"),
                );
            }

            // check for problematic hyphen separators
            if has_hyphen_separator(&text) {
                let severity = ctx.get_severity(meta);
                if !severity.is_enabled() {
                    continue;
                }

                ctx.report(
                    LintReport::new(
                        COMMENT_LAYOUT.id,
                        COMMENT_LAYOUT.code,
                        COMMENT_LAYOUT.category,
                        severity,
                        "prefer colons or commas over hyphens in comments",
                        comment.span,
                    )
                    .label("replace hyphen with colon or comma"),
                );
            }
        }

        // check inline comments
        for comment in ctx.dir.comments().iter().copied() {
            if is_doc_comment_source(ctx.get_span_text(comment.span)) {
                continue;
            }

            let text = dir::normalize_comment_payload(ctx.get_span_text(comment.span)).into_owned();
            let text = text.trim();

            // skip empty, directive, and separator comments
            if text.is_empty() || is_directive_comment(text) || is_separator_comment(text) {
                continue;
            }

            // require uppercase keyword and valid tags
            if let Some(keyword_info) = parse_keyword_comment_with_options(
                text,
                &ctx.options.style.comment_keywords,
                &ctx.options.style.comment_keyword_tags,
            ) {
                let severity = ctx.get_severity(meta);
                if !severity.is_enabled() {
                    continue;
                }

                // enforce uppercase keyword comments
                if !keyword_info.keyword_is_uppercase {
                    let mut diagnostic = LintReport::new(
                        COMMENT_LAYOUT.id,
                        COMMENT_LAYOUT.code,
                        COMMENT_LAYOUT.category,
                        severity,
                        "keyword comments should use uppercase keywords",
                        comment.span,
                    )
                    .label(known_comment_tag_label(&ctx.options.style.comment_keywords));

                    // compute fixes only when requested by the runner
                    if ctx.compute_fixes
                        && let Some(fix) =
                            uppercase_keyword_comment_fix(ctx, comment.span, &keyword_info.keyword)
                    {
                        diagnostic = diagnostic.fix(fix);
                    }

                    ctx.report(diagnostic);
                }

                // reject unknown keyword tags
                if keyword_info.has_unknown_tag {
                    ctx.report(
                        LintReport::new(
                            COMMENT_LAYOUT.id,
                            COMMENT_LAYOUT.code,
                            COMMENT_LAYOUT.category,
                            severity,
                            "keyword comments should use known AGENTS tags only",
                            comment.span,
                        )
                        .label(known_comment_tag_label(
                            &ctx.options.style.comment_keyword_tags,
                        )),
                    );
                }

                // enforce tags on keyword comments
                if !keyword_info.has_known_tag && !keyword_info.has_unknown_tag {
                    ctx.report(
                        LintReport::new(
                            COMMENT_LAYOUT.id,
                            COMMENT_LAYOUT.code,
                            COMMENT_LAYOUT.category,
                            severity,
                            "keyword comments should include at least one known tag",
                            comment.span,
                        )
                        .label("add a tag like #Cleanup or #Suspicious"),
                    );
                }
            }

            // check for multiple sentences on one comment line
            if has_multiple_sentence_starts(text) {
                let severity = ctx.get_severity(meta);
                if !severity.is_enabled() {
                    continue;
                }

                ctx.report(
                    LintReport::new(
                        COMMENT_LAYOUT.id,
                        COMMENT_LAYOUT.code,
                        COMMENT_LAYOUT.category,
                        severity,
                        "inline comments should stay below one sentence",
                        comment.span,
                    )
                    .label("split into separate comments"),
                );
            }

            // check for problematic hyphens
            if has_hyphen_separator(text) {
                let severity = ctx.get_severity(meta);
                if !severity.is_enabled() {
                    continue;
                }
                ctx.report(
                    LintReport::new(
                        COMMENT_LAYOUT.id,
                        COMMENT_LAYOUT.code,
                        COMMENT_LAYOUT.category,
                        severity,
                        "prefer colons or commas over hyphens in comments",
                        comment.span,
                    )
                    .label("replace hyphen with colon or comma"),
                );
            }
        }
    }
}

/// Build a diagnostic label for configured keyword tags.
fn known_comment_tag_label(tags: &[String]) -> String {
    // fall back to a generic message when no tags are configured
    if tags.is_empty() {
        return "configure known keyword tags in linter options".to_string();
    }

    format!("use one of {}", tags.join(", "))
}

/// Build a safe fix for lowercase keyword comments.
fn uppercase_keyword_comment_fix(
    ctx: &LintModuleContext<'_>,
    annotation_span: Span,
    uppercase_keyword: &str,
) -> Option<LintFix> {
    let annotation_text = ctx.get_span_text(annotation_span);
    let start_offset = annotation_text
        .char_indices()
        .find_map(|(offset, character)| character.is_alphabetic().then_some(offset))?;
    let token = &annotation_text[start_offset..];
    let end_offset = token
        .char_indices()
        .find_map(|(offset, character)| character.is_whitespace().then_some(offset))
        .unwrap_or(token.len());
    let token = &token[..end_offset];
    let token_without_colon = token.trim_end_matches(':');

    if !token_without_colon.eq_ignore_ascii_case(uppercase_keyword) {
        return None;
    }

    let suffix = &token[token_without_colon.len()..];
    let replacement = format!("{uppercase_keyword}{suffix}");
    if replacement == token {
        return None;
    }

    let token_span = Span::new(
        annotation_span.file,
        annotation_span.start + start_offset as u32,
        annotation_span.start + (start_offset + token.len()) as u32,
    );
    let edits = ctx
        .edit_builder()
        .replace(token_span, replacement)
        .into_edits();
    Some(LintFix::safe("Uppercase comment keyword").with_edits(edits))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_single_sentence_allowed() {
        let test = TestProgram::for_rule_without_prelude(CommentLayout);
        let result = test.lint(
            "comment_layout/test_single_sentence_allowed.ds",
            r#"
/// This is a single sentence.
function foo() {}
"#,
        );
        test.result(result).assert_no_lint("comment-layout");
    }

    #[test]
    fn test_multiple_sentences_on_one_line_detected() {
        let test = TestProgram::for_rule_without_prelude(CommentLayout);
        let result = test.lint(
            "comment_layout/test_multiple_sentences_on_one_line_detected.ds",
            r#"
/// This is one sentence. This is another sentence.
function foo() {}
"#,
        );
        test.result(result).assert_lint("comment-layout");
    }

    #[test]
    fn test_multiple_sentences_on_separate_lines_allowed() {
        let test = TestProgram::for_rule_without_prelude(CommentLayout);
        let result = test.lint(
            "comment_layout/test_multiple_sentences_on_separate_lines_allowed.ds",
            r#"
/// This is one sentence.
/// This is another sentence.
function foo() {}
"#,
        );
        test.result(result).assert_no_lint("comment-layout");
    }

    #[test]
    fn test_hyphen_separator_detected() {
        let test = TestProgram::for_rule_without_prelude(CommentLayout);
        let result = test.lint(
            "comment_layout/test_hyphen_separator_detected.ds",
            r#"
/// This is a comment - with a hyphen separator.
function foo() {}
"#,
        );
        test.result(result).assert_lint("comment-layout");
    }

    #[test]
    fn test_colon_separator_allowed() {
        let test = TestProgram::for_rule_without_prelude(CommentLayout);
        let result = test.lint(
            "comment_layout/test_colon_separator_allowed.ds",
            r#"
/// This is a comment: with a colon separator.
function foo() {}
"#,
        );
        test.result(result).assert_no_lint("comment-layout");
    }

    #[test]
    fn test_compound_word_hyphen_allowed() {
        let test = TestProgram::for_rule_without_prelude(CommentLayout);
        let result = test.lint(
            "comment_layout/test_compound_word_hyphen_allowed.ds",
            r#"
/// This is a well-known pattern.
function foo() {}
"#,
        );
        test.result(result).assert_no_lint("comment-layout");
    }

    #[test]
    fn test_url_hyphen_allowed() {
        let test = TestProgram::for_rule_without_prelude(CommentLayout);
        let result = test.lint(
            "comment_layout/test_url_hyphen_allowed.ds",
            r#"
/// See https://example-site.com/foo-bar.
function foo() {}
"#,
        );
        test.result(result).assert_no_lint("comment-layout");
    }

    /// Require tags on NOTE comments.
    #[test]
    fn test_keyword_comment_requires_tag() {
        let test = TestProgram::for_rule_without_prelude(CommentLayout);
        let result = test.lint(
            "comment_layout/test_keyword_comment_requires_tag.ds",
            r#"
// NOTE this should include a tag
const value = 1;
"#,
        );
        test.result(result).assert_lint("comment-layout");
    }

    /// Require uppercase keyword prefixes.
    #[test]
    fn test_keyword_comment_requires_uppercase_keyword() {
        let test = TestProgram::for_rule_without_prelude(CommentLayout);
        let result = test.lint(
            "comment_layout/test_keyword_comment_requires_uppercase_keyword.ds",
            r#"
// todo #Cleanup: normalize this branch
const value = 1;
"#,
        );
        test.result(result).assert_lint("comment-layout");
    }

    /// Auto-fix lowercase keyword comments.
    #[test]
    fn test_fix_uppercases_keyword_comment() {
        let test = TestProgram::for_rule_without_prelude(CommentLayout);
        let result = test.lint(
            "comment_layout/test_fix_uppercases_keyword_comment.ds",
            r#"
// todo #Cleanup: normalize this branch
const value = 1;
"#,
        );
        test.result(result)
            .assert_lint("comment-layout")
            .assert_safe_fixed(
                r#"
// TODO #Cleanup: normalize this branch
const value = 1;
"#,
            );
    }

    /// Reject unknown keyword tags.
    #[test]
    fn test_keyword_comment_rejects_unknown_tag() {
        let test = TestProgram::for_rule_without_prelude(CommentLayout);
        let result = test.lint(
            "comment_layout/test_keyword_comment_rejects_unknown_tag.ds",
            r#"
// TODO #Whatever: normalize this branch
const value = 1;
"#,
        );
        test.result(result).assert_lint("comment-layout");
    }

    /// Skip fixes for unknown keyword tags.
    #[test]
    fn test_no_fix_for_unknown_keyword_tag() {
        let test = TestProgram::for_rule_without_prelude(CommentLayout);
        let result = test.lint(
            "comment_layout/test_no_fix_for_unknown_keyword_tag.ds",
            r#"
// TODO #Whatever: normalize this branch
const value = 1;
"#,
        );
        test.result(result)
            .assert_lint("comment-layout")
            .assert_has_no_fix("comment-layout");
    }

    /// Accept keyword comments with known tags.
    #[test]
    fn test_keyword_comment_accepts_known_tags() {
        let test = TestProgram::for_rule_without_prelude(CommentLayout);
        let result = test.lint(
            "comment_layout/test_keyword_comment_accepts_known_tags.ds",
            r#"
// TODO #Cleanup #Performance: normalize this branch
const value = 1;
"#,
        );
        test.result(result).assert_no_lint("comment-layout");
    }

    /// Reject inline comments with multiple sentences.
    #[test]
    fn test_inline_multiple_sentences_detected() {
        let test = TestProgram::for_rule_without_prelude(CommentLayout);
        let result = test.lint(
            "comment_layout/test_inline_multiple_sentences_detected.ds",
            r#"
const value = 1; // one sentence. second sentence
"#,
        );
        test.result(result).assert_lint("comment-layout");
    }

    /// Skip non prose doc lines.
    #[test]
    fn test_non_prose_doc_lines_skipped() {
        let test = TestProgram::for_rule_without_prelude(CommentLayout);
        let result = test.lint(
            "comment_layout/test_non_prose_doc_lines_skipped.ds",
            r#"
/// Returns a value.
/// @param value
/// ```ts
/// doThing()
/// ```
function foo(value: int32): int32 {
    return value;
}
"#,
        );
        test.result(result).assert_no_lint("comment-layout");
    }
}
