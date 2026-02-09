use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::rules::common::{
    has_hyphen_separator, has_multiple_sentence_starts, is_directive_comment,
    is_non_prose_doc_line, is_separator_comment, parse_keyword_comment_with_options,
};
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
        code = "LY003",
        category = Style,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub CommentLayout,
    "Enforce comment layout"
}

impl LintRule for CommentLayout {
    fn meta(&self) -> &'static crate::LintMeta {
        CommentLayout::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Annotation>() {
            let annotation = ctx.tree.get(node_id);
            match annotation {
                // check doc comments
                ast::Annotation::Doc { node, .. } => {
                    let doc = ctx.tree.get(*node);
                    let text = ctx.strings.get(doc.string);
                    let mut has_multiple_sentence_line = false;

                    // check each prose line for multiple sentence starts
                    for line in text.as_ref().lines() {
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
                        let severity = ctx.get_effective_severity(meta, node_id);
                        if !severity.is_enabled() {
                            continue;
                        }

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
                            .with_label("split sentences across multiple lines"),
                        );
                    }

                    // check for problematic hyphen separators
                    if has_hyphen_separator(text.as_ref()) {
                        let severity = ctx.get_effective_severity(meta, node_id);
                        if !severity.is_enabled() {
                            continue;
                        }

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
                    let text = text.as_ref().trim();

                    // skip empty, directive, and separator comments
                    if text.is_empty() || is_directive_comment(text) || is_separator_comment(text) {
                        continue;
                    }

                    // require uppercase keyword and valid tags
                    if let Some(keyword_info) = parse_keyword_comment_with_options(
                        text,
                        &ctx.options.comment_keywords,
                        &ctx.options.comment_keyword_tags,
                    ) {
                        let severity = ctx.get_effective_severity(meta, node_id);
                        if !severity.is_enabled() {
                            continue;
                        }

                        // enforce uppercase keyword comments
                        if !keyword_info.keyword_is_uppercase {
                            ctx.report(
                                LintDiagnostic::new(
                                    COMMENT_LAYOUT.id,
                                    COMMENT_LAYOUT.code,
                                    COMMENT_LAYOUT.category,
                                    severity,
                                    "keyword comments should use uppercase keywords",
                                    ctx.module.file_id,
                                    ctx.tree.get_span(node_id),
                                )
                                .with_label("use NOTE, TODO, or FUGU in uppercase"),
                            );
                        }

                        // reject unknown keyword tags
                        if keyword_info.has_unknown_tag {
                            ctx.report(
                                LintDiagnostic::new(
                                    COMMENT_LAYOUT.id,
                                    COMMENT_LAYOUT.code,
                                    COMMENT_LAYOUT.category,
                                    severity,
                                    "keyword comments should use known AGENTS tags only",
                                    ctx.module.file_id,
                                    ctx.tree.get_span(node_id),
                                )
                                .with_label(
                                    known_comment_tag_label(&ctx.options.comment_keyword_tags),
                                ),
                            );
                        }

                        // enforce tags on keyword comments
                        if !keyword_info.has_known_tag && !keyword_info.has_unknown_tag {
                            ctx.report(
                                LintDiagnostic::new(
                                    COMMENT_LAYOUT.id,
                                    COMMENT_LAYOUT.code,
                                    COMMENT_LAYOUT.category,
                                    severity,
                                    "keyword comments should include at least one known tag",
                                    ctx.module.file_id,
                                    ctx.tree.get_span(node_id),
                                )
                                .with_label("add a tag like #Cleanup or #Suspicious"),
                            );
                        }
                    }

                    // check for multiple sentences on one comment line
                    if has_multiple_sentence_starts(text) {
                        let severity = ctx.get_effective_severity(meta, node_id);
                        if !severity.is_enabled() {
                            continue;
                        }

                        ctx.report(
                            LintDiagnostic::new(
                                COMMENT_LAYOUT.id,
                                COMMENT_LAYOUT.code,
                                COMMENT_LAYOUT.category,
                                severity,
                                "inline comments should stay below one sentence",
                                ctx.module.file_id,
                                ctx.tree.get_span(node_id),
                            )
                            .with_label("split into separate comments"),
                        );
                    }

                    // check for problematic hyphens
                    if has_hyphen_separator(text) {
                        let severity = ctx.get_effective_severity(meta, node_id);
                        if !severity.is_enabled() {
                            continue;
                        }
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

/// Build a diagnostic label for configured keyword tags.
fn known_comment_tag_label(tags: &[String]) -> String {
    // fall back to a generic message when no tags are configured
    if tags.is_empty() {
        return "configure known keyword tags in linter options".to_string();
    }

    format!("use one of {}", tags.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_single_sentence_allowed() {
        let test = TestProgram::for_rule_without_prelude(CommentLayout);
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
            "comment_layout/test_keyword_comment_requires_uppercase_keyword.ds",
            r#"
// todo #Cleanup: normalize this branch
const value = 1;
"#,
        );
        test.result(result).assert_lint("comment-layout");
    }

    /// Reject unknown keyword tags.
    #[test]
    fn test_keyword_comment_rejects_unknown_tag() {
        let test = TestProgram::for_rule_without_prelude(CommentLayout);
        let result = test.lint_ast(
            "comment_layout/test_keyword_comment_rejects_unknown_tag.ds",
            r#"
// TODO #Whatever: normalize this branch
const value = 1;
"#,
        );
        test.result(result).assert_lint("comment-layout");
    }

    /// Accept keyword comments with known tags.
    #[test]
    fn test_keyword_comment_accepts_known_tags() {
        let test = TestProgram::for_rule_without_prelude(CommentLayout);
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
        let result = test.lint_ast(
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
