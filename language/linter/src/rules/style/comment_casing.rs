use crate::LintMeta;
use destack_ast as ast;
use destack_source::Span;
use destack_workspace::LintSeverity;

use crate::rules::common::{
    first_alphabetic_character, is_directive_comment, is_doc_comment_source, is_non_prose_doc_line,
    is_separator_comment, is_separator_heading_line, parse_keyword_comment_with_options,
};
use crate::{LintAstContext, LintDiagnostic, LintFix, LintRule, declare_lint};

declare_lint! {
    /// Enforce comment casing conventions.
    ///
    /// Inline comments should begin with lowercase letters for consistency.
    /// Documentation comments should begin with uppercase (proper sentences).
    #[lint(
        id = "comment-casing",
        code = "LY002",
        category = Style,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable
    )]
    pub CommentCasing,
    "Enforce comment casing"
}

impl LintRule for CommentCasing {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        CommentCasing::meta()
    }

    /// Check module AST annotations for comment casing.
    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        // non doc comments should start with lowercase
        let inline_comments: Vec<_> = ctx
            .tree
            .comments()
            .iter()
            .copied()
            .filter(|comment| !is_doc_comment_source(ctx.get_span_text(comment.span)))
            .collect();

        for (index, comment) in inline_comments.iter().copied().enumerate() {
            let comment_text = ast::normalize_comment_payload(ctx.get_span_text(comment.span));
            let comment_text = comment_text.as_ref().trim();
            let previous_comment_text = index.checked_sub(1).map(|previous_index| {
                ast::normalize_comment_payload(
                    ctx.get_span_text(inline_comments[previous_index].span),
                )
            });
            let next_comment_text = inline_comments.get(index + 1).map(|next_comment| {
                ast::normalize_comment_payload(ctx.get_span_text(next_comment.span))
            });
            let is_separator_heading_triplet = is_separator_heading_line(
                previous_comment_text.as_deref(),
                comment_text,
                next_comment_text.as_deref(),
                ctx.options.style.comment_separator_heading_min_lines,
            );

            // skip comments that are exempt from lowercase casing
            if comment_text.is_empty()
                || is_directive_comment(comment_text)
                || is_separator_comment(comment_text)
                || is_separator_heading_triplet
                || is_separator_heading_block(
                    comment_text,
                    ctx.options.style.comment_separator_heading_min_lines,
                )
                || parse_keyword_comment_with_options(
                    comment_text,
                    &ctx.options.style.comment_keywords,
                    &ctx.options.style.comment_keyword_tags,
                )
                .is_some()
            {
                continue;
            }

            // report uppercase comment starts
            if let Some(first_character) = first_alphabetic_character(comment_text)
                && first_character.is_uppercase()
            {
                let severity = ctx.get_severity(meta);
                if !severity.is_enabled() {
                    continue;
                }

                let mut diagnostic = LintDiagnostic::new(
                    COMMENT_CASING.id,
                    COMMENT_CASING.code,
                    COMMENT_CASING.category,
                    severity,
                    "inline comment should start with lowercase",
                    ctx.module.file_id,
                    comment.span,
                )
                .with_label("use lowercase for inline comments");

                // compute fixes only when requested by the runner
                if ctx.compute_fixes
                    && let Some(fix) =
                        comment_casing_fix(ctx, comment.span, CasingFixKind::LowercaseInline)
                {
                    diagnostic = diagnostic.with_fix(fix);
                }

                ctx.report(diagnostic);
            }
        }

        // doc comments should start with uppercase
        for comment in ctx.tree.comments().iter().copied() {
            if !is_doc_comment_source(ctx.get_span_text(comment.span)) {
                continue;
            }

            // find the first prose line
            let doc_text = ast::normalize_comment_payload(ctx.get_span_text(comment.span));
            let first_line = doc_text
                .as_ref()
                .lines()
                .find(|line| !is_non_prose_doc_line(line))
                .map(|line| line.trim());
            let Some(first_line) = first_line else {
                continue;
            };

            // report lowercase doc comment starts
            if let Some(first_character) = first_alphabetic_character(first_line)
                && first_character.is_lowercase()
            {
                let severity = ctx.get_severity(meta);
                if !severity.is_enabled() {
                    continue;
                }

                let comment_span = comment.span;
                let mut diagnostic = LintDiagnostic::new(
                    COMMENT_CASING.id,
                    COMMENT_CASING.code,
                    COMMENT_CASING.category,
                    severity,
                    "doc comment should start with uppercase",
                    ctx.module.file_id,
                    comment_span,
                )
                .with_label("use uppercase for doc comments");

                // compute fixes only when requested by the runner
                if ctx.compute_fixes
                    && let Some(fix) =
                        comment_casing_fix(ctx, comment_span, CasingFixKind::UppercaseDoc)
                {
                    diagnostic = diagnostic.with_fix(fix);
                }

                ctx.report(diagnostic);
            }
        }
    }
}

/// Return true when one comment contains a separator heading block.
fn is_separator_heading_block(comment_text: &str, min_lines: usize) -> bool {
    let comment_lines: Vec<&str> = comment_text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    if comment_lines.len() < min_lines {
        return false;
    }

    let Some(first_line) = comment_lines.first() else {
        return false;
    };
    let Some(last_line) = comment_lines.last() else {
        return false;
    };

    is_separator_comment(first_line) && is_separator_comment(last_line)
}

/// The casing change for one comment annotation.
enum CasingFixKind {
    /// Lowercase the first alphabetic character in one inline comment.
    LowercaseInline,
    /// Uppercase the first alphabetic character in the first prose doc line.
    UppercaseDoc,
}

/// Build a safe fix for one comment casing violation.
fn comment_casing_fix(
    ctx: &LintAstContext<'_>,
    annotation_span: Span,
    fix_kind: CasingFixKind,
) -> Option<LintFix> {
    let annotation_text = ctx.get_span_text(annotation_span);

    // locate the first alphabetic character to rewrite
    let (offset, current_char) = match fix_kind {
        CasingFixKind::LowercaseInline => first_alphabetic_char_offset(annotation_text)?,
        CasingFixKind::UppercaseDoc => first_doc_prose_alphabetic_char_offset(annotation_text)?,
    };

    let replacement = match fix_kind {
        CasingFixKind::LowercaseInline => current_char.to_lowercase().to_string(),
        CasingFixKind::UppercaseDoc => current_char.to_uppercase().to_string(),
    };
    if replacement == current_char.to_string() {
        return None;
    }

    // rewrite only one character to avoid touching surrounding formatting
    let replacement_span = Span::new(
        annotation_span.file,
        annotation_span.start + offset as u32,
        annotation_span.start + offset as u32 + current_char.len_utf8() as u32,
    );
    let edits = ctx
        .edit_builder()
        .replace(replacement_span, replacement)
        .into_edits();
    Some(LintFix::safe("Fix comment casing").with_edits(edits))
}

/// Return the first alphabetic character offset and value.
fn first_alphabetic_char_offset(text: &str) -> Option<(usize, char)> {
    text.char_indices()
        .find(|(_, character)| character.is_alphabetic())
}

/// Return the first prose doc alphabetic character offset and value.
fn first_doc_prose_alphabetic_char_offset(text: &str) -> Option<(usize, char)> {
    let mut base_offset = 0usize;

    for line in text.split_inclusive('\n') {
        let line_without_newline = line.trim_end_matches('\n');
        let trimmed = line_without_newline.trim();
        if is_non_prose_doc_line(trimmed) {
            base_offset += line.len();
            continue;
        }

        if let Some((line_offset, character)) = line_without_newline
            .char_indices()
            .find(|(_, character)| character.is_alphabetic())
        {
            return Some((base_offset + line_offset, character));
        }

        base_offset += line.len();
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_inline_comment_lowercase_allowed() {
        let test = TestProgram::for_rule_without_prelude(CommentCasing);
        let result = test.lint_ast(
            "comment_casing/test_inline_comment_lowercase_allowed.ds",
            r#"
let x = 1 // this is fine
"#,
        );
        test.result(result).assert_no_lint("comment-casing");
    }

    #[test]
    fn test_inline_comment_uppercase_detected() {
        let test = TestProgram::for_rule_without_prelude(CommentCasing);
        let result = test.lint_ast(
            "comment_casing/test_inline_comment_uppercase_detected.ds",
            r#"
let x = 1 // This should be lowercase
"#,
        );
        test.result(result).assert_lint("comment-casing");
    }

    #[test]
    fn test_fix_lowercases_inline_comment_start() {
        let test = TestProgram::for_rule_without_prelude(CommentCasing);
        let result = test.lint_ast(
            "comment_casing/test_fix_lowercases_inline_comment_start.ds",
            r#"
let x = 1 // This should be lowercase
"#,
        );
        test.result(result)
            .assert_lint("comment-casing")
            .assert_safe_fixed(
                r#"
let x = 1; // this should be lowercase
"#,
            );
    }

    #[test]
    fn test_doc_comment_uppercase_allowed() {
        let test = TestProgram::for_rule_without_prelude(CommentCasing);
        let result = test.lint_ast(
            "comment_casing/test_doc_comment_uppercase_allowed.ds",
            r#"
/// This is proper documentation.
function foo() {}
"#,
        );
        test.result(result).assert_no_lint("comment-casing");
    }

    #[test]
    fn test_doc_comment_lowercase_detected() {
        let test = TestProgram::for_rule_without_prelude(CommentCasing);
        let result = test.lint_ast(
            "comment_casing/test_doc_comment_lowercase_detected.ds",
            r#"
/// this should be uppercase
function foo() {}
"#,
        );
        test.result(result).assert_lint("comment-casing");
    }

    #[test]
    fn test_fix_uppercases_doc_comment_start() {
        let test = TestProgram::for_rule_without_prelude(CommentCasing);
        let result = test.lint_ast(
            "comment_casing/test_fix_uppercases_doc_comment_start.ds",
            r#"
/// this should be uppercase
function foo() {}
"#,
        );
        test.result(result)
            .assert_lint("comment-casing")
            .assert_safe_fixed(
                r#"
/// This should be uppercase
function foo() {}
"#,
            );
    }

    #[test]
    fn test_special_markers_skipped() {
        let test = TestProgram::for_rule_without_prelude(CommentCasing);
        let result = test.lint_ast(
            "comment_casing/test_special_markers_skipped.ds",
            r#"
let x = 1 // @ts-ignore
let y = 2 // #region
"#,
        );
        test.result(result).assert_no_lint("comment-casing");
    }

    /// Skip uppercase keyword comments with tags.
    #[test]
    fn test_keyword_comment_skipped() {
        let test = TestProgram::for_rule_without_prelude(CommentCasing);
        let result = test.lint_ast(
            "comment_casing/test_keyword_comment_skipped.ds",
            r#"
let x = 1 // NOTE #Suspicious: this path is odd
"#,
        );
        test.result(result).assert_no_lint("comment-casing");
    }

    /// Skip uppercase separator headings.
    #[test]
    fn test_separator_heading_skipped() {
        let test = TestProgram::for_rule_without_prelude(CommentCasing);
        let result = test.lint_ast(
            "comment_casing/test_separator_heading_skipped.ds",
            r#"
// ================================================================================
// Binary operator precedence
// ================================================================================
const value = 1;
"#,
        );
        test.result(result).assert_no_lint("comment-casing");
    }

    /// Respect configurable separator heading line thresholds.
    #[test]
    fn test_separator_heading_respects_min_lines_option() {
        let test = TestProgram::for_rule_without_prelude(CommentCasing).with_options(|options| {
            options.style.comment_separator_heading_min_lines = 4;
        });
        let result = test.lint_ast(
            "comment_casing/test_separator_heading_respects_min_lines_option.ds",
            r#"
// ================================================================================
// Binary operator precedence
// ================================================================================
const value = 1;
"#,
        );
        test.result(result).assert_lint("comment-casing");
    }

    /// Detect uppercase block comments that are not separator headings.
    #[test]
    fn test_block_comment_uppercase_detected() {
        let test = TestProgram::for_rule_without_prelude(CommentCasing);
        let result = test.lint_ast(
            "comment_casing/test_block_comment_uppercase_detected.ds",
            r#"
// Build the value map
const value = 1;
"#,
        );
        test.result(result).assert_lint("comment-casing");
    }

    /// Skip non prose doc comment lines.
    #[test]
    fn test_doc_non_prose_line_skipped() {
        let test = TestProgram::for_rule_without_prelude(CommentCasing);
        let result = test.lint_ast(
            "comment_casing/test_doc_non_prose_line_skipped.ds",
            r#"
/// @returns {number}
function foo(): number {
    return 1;
}
"#,
        );
        test.result(result).assert_no_lint("comment-casing");
    }
}
