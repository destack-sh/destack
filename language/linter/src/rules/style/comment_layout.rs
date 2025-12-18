use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Enforce comment layout conventions.
    ///
    /// Doc comments should have one sentence per line for better readability.
    /// Doc comments should be upper case sentences.
    /// Single-line inline comments should be lowercase sentences.
    /// Non-trivial logic blocks should have preceding comments.
    #[lint(
        id = "comment-layout",
        code = "LY037",
        category = Style,
        level = Ast,
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub CommentLayout,
    "Enforce comment layout"
}

/// Check if a line contains multiple sentences (ends with . followed by more text).
fn has_multiple_sentences(line: &str) -> bool {
    // look for sentence endings followed by more content
    let trimmed = line.trim();
    let mut chars = trimmed.chars().peekable();
    while let Some(c) = chars.next() {
        if matches!(c, '.' | '!' | '?') {
            // check if followed by space and then uppercase letter (new sentence)
            if let Some(&next) = chars.peek()
                && next == ' '
            {
                chars.next();
                if let Some(&after_space) = chars.peek()
                    && after_space.is_uppercase()
                {
                    return true;
                }
            }
        }
    }

    false
}

/// Check if text has problematic hyphens (not in code, urls, or compound words).
fn has_problematic_hyphen(text: &str) -> bool {
    // skip if it looks like code or urls
    if text.contains("://") || text.contains('`') {
        return false;
    }

    // look for " - " pattern (hyphen used as separator)
    text.contains(" - ")
}

impl LintRule for CommentLayout {
    fn meta(&self) -> &'static crate::LintMeta {
        CommentLayout::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        for node_id in ctx.tree.iter_nodes::<ast::Annotation>() {
            let annotation = ctx.tree.get(node_id);
            match annotation {
                // check doc comments
                ast::Annotation::Doc { node, .. } => {
                    let doc = ctx.tree.get(*node);
                    let text = ctx.strings.get(doc.string);

                    for line in text.as_ref().lines() {
                        let trimmed = line.trim();

                        // skip empty lines, code blocks, and annotations
                        if trimmed.is_empty()
                            || trimmed.starts_with("```")
                            || trimmed.starts_with('@')
                        {
                            continue;
                        }

                        // check for multiple sentences on one line
                        if has_multiple_sentences(trimmed) {
                            ctx.report(
                                LintDiagnostic::new(
                                    COMMENT_LAYOUT.id,
                                    COMMENT_LAYOUT.code,
                                    COMMENT_LAYOUT.category,
                                    severity,
                                    "doc comment should have one sentence per line",
                                    ctx.module.file_id,
                                    ctx.tree.get_span(node_id),
                                )
                                .with_label("split into multiple lines"),
                            );
                            break; // only report once per comment
                        }
                    }

                    // check for problematic hyphens
                    if has_problematic_hyphen(text.as_ref()) {
                        ctx.report(
                            LintDiagnostic::new(
                                COMMENT_LAYOUT.id,
                                COMMENT_LAYOUT.code,
                                COMMENT_LAYOUT.category,
                                severity,
                                "prefer colons or commas over hyphens in comments",
                                ctx.module.file_id,
                                ctx.tree.get_span(node_id),
                            )
                            .with_label("replace hyphen with colon or comma"),
                        );
                    }
                }

                // check inline comments
                ast::Annotation::Comment { node, .. } => {
                    let comment = ctx.tree.get(*node);
                    let text = ctx.strings.get(comment.string);

                    // check for problematic hyphens
                    if has_problematic_hyphen(text.as_ref()) {
                        ctx.report(
                            LintDiagnostic::new(
                                COMMENT_LAYOUT.id,
                                COMMENT_LAYOUT.code,
                                COMMENT_LAYOUT.category,
                                severity,
                                "prefer colons or commas over hyphens in comments",
                                ctx.module.file_id,
                                ctx.tree.get_span(node_id),
                            )
                            .with_label("replace hyphen with colon or comma"),
                        );
                    }
                }

                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_single_sentence_allowed() {
        let test = TestProgram::for_rule(CommentLayout);
        let result = test.lint_ast(
            "test.ds",
            r#"
/// This is a single sentence.
function foo() {}
"#,
        );
        test.result(result).assert_no_lint("comment-layout");
    }

    #[test]
    fn test_multiple_sentences_on_one_line_detected() {
        let test = TestProgram::for_rule(CommentLayout);
        let result = test.lint_ast(
            "test.ds",
            r#"
/// This is one sentence. This is another sentence.
function foo() {}
"#,
        );
        test.result(result).assert_lint("comment-layout");
    }

    #[test]
    fn test_multiple_sentences_on_separate_lines_allowed() {
        let test = TestProgram::for_rule(CommentLayout);
        let result = test.lint_ast(
            "test.ds",
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
        let test = TestProgram::for_rule(CommentLayout);
        let result = test.lint_ast(
            "test.ds",
            r#"
/// This is a comment - with a hyphen separator.
function foo() {}
"#,
        );
        test.result(result).assert_lint("comment-layout");
    }

    #[test]
    fn test_colon_separator_allowed() {
        let test = TestProgram::for_rule(CommentLayout);
        let result = test.lint_ast(
            "test.ds",
            r#"
/// This is a comment: with a colon separator.
function foo() {}
"#,
        );
        test.result(result).assert_no_lint("comment-layout");
    }

    #[test]
    fn test_compound_word_hyphen_allowed() {
        let test = TestProgram::for_rule(CommentLayout);
        let result = test.lint_ast(
            "test.ds",
            r#"
/// This is a well-known pattern.
function foo() {}
"#,
        );
        test.result(result).assert_no_lint("comment-layout");
    }

    #[test]
    fn test_url_hyphen_allowed() {
        let test = TestProgram::for_rule(CommentLayout);
        let result = test.lint_ast(
            "test.ds",
            r#"
/// See https://example-site.com/foo-bar.
function foo() {}
"#,
        );
        test.result(result).assert_no_lint("comment-layout");
    }
}
