use destack_ast as ast;
use destack_source::Span;
use destack_workspace::LintSeverity;

use crate::rules::common::{
    has_doc_terminal_punctuation, is_directive_comment, is_non_prose_doc_line,
    is_separator_comment, parse_keyword_comment_with_options,
};
use crate::{LintDiagnostic, LintFix, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Enforce comment punctuation conventions.
    ///
    /// Inline comments should avoid trailing periods.
    /// Documentation comment prose lines should end with punctuation.
    #[lint(
        id = "comment-punctuation",
        code = "LY004",
        category = Style,
        level = Ast,
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
    fn meta(&self) -> &'static crate::LintMeta {
        CommentPunctuation::meta()
    }

    /// Check module AST annotations for punctuation consistency.
    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Annotation>() {
            let annotation = ctx.tree.get(node_id);

            match annotation {
                // inline comments should not end with periods
                ast::Annotation::Comment { node, .. } => {
                    let comment = ctx.tree.get(*node);
                    let text = ctx.strings.get(comment.string);
                    let text = text.as_ref().trim();

                    // skip comments that have explicit exceptions
                    if text.is_empty()
                        || is_directive_comment(text)
                        || is_separator_comment(text)
                        || parse_keyword_comment_with_options(
                            text,
                            &ctx.options.comment_keywords,
                            &ctx.options.comment_keyword_tags,
                        )
                        .is_some()
                    {
                        continue;
                    }

                    // report trailing periods
                    if text.ends_with('.') {
                        let severity = ctx.get_effective_severity(meta, node_id);
                        if !severity.is_enabled() {
                            continue;
                        }

                        let mut diagnostic = LintDiagnostic::new(
                            COMMENT_PUNCTUATION.id,
                            COMMENT_PUNCTUATION.code,
                            COMMENT_PUNCTUATION.category,
                            severity,
                            "inline comment should not end with a period",
                            ctx.module.file_id,
                            ctx.tree.get_span(node_id),
                        )
                        .with_label("remove trailing period");

                        // compute fixes only when requested by the runner
                        if ctx.compute_fixes
                            && let Some(fix) = inline_comment_trailing_period_fix(ctx, node_id)
                        {
                            diagnostic = diagnostic.with_fix(fix);
                        }

                        ctx.report(diagnostic);
                    }
                }

                // doc comments should end each prose line with punctuation
                ast::Annotation::Doc { node, .. } => {
                    let doc = ctx.tree.get(*node);
                    let text = ctx.strings.get(doc.string);
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
                        let severity = ctx.get_effective_severity(meta, node_id);
                        if !severity.is_enabled() {
                            continue;
                        }

                        ctx.report(
                            LintDiagnostic::new(
                                COMMENT_PUNCTUATION.id,
                                COMMENT_PUNCTUATION.code,
                                COMMENT_PUNCTUATION.category,
                                severity,
                                "doc comment lines should end with punctuation",
                                ctx.module.file_id,
                                ctx.tree.get_span(node_id),
                            )
                            .with_label("add punctuation to each sentence line"),
                        );
                    }
                }

                _ => {}
            }
        }
    }
}

/// Build a safe fix that removes one trailing period from an inline comment.
fn inline_comment_trailing_period_fix(
    ctx: &LintModuleAstContext<'_>,
    annotation_id: ast::LocalNodeId<ast::Annotation>,
) -> Option<LintFix> {
    let annotation_span = ctx.tree.get_span(annotation_id);
    let annotation_text = ctx.get_span_text(annotation_span);

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
        annotation_span.file,
        annotation_span.start + offset as u32,
        annotation_span.start + offset as u32 + 1,
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
            "comment_punctuation/test_separator_comment_skipped.ds",
            r#"
// ================================================================================
const value = 1;
"#,
        );
        test.result(result).assert_no_lint("comment-punctuation");
    }
}
