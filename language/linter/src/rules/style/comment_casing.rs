use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::rules::common::{
    first_alphabetic_character, is_directive_comment, is_non_prose_doc_line, is_separator_comment,
    parse_keyword_comment_with_options,
};
use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

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
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub CommentCasing,
    "Enforce comment casing"
}

impl LintRule for CommentCasing {
    /// Return lint metadata.
    fn meta(&self) -> &'static crate::LintMeta {
        CommentCasing::meta()
    }

    /// Check module AST annotations for comment casing.
    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Annotation>() {
            let annotation = ctx.tree.get(node_id);

            match annotation {
                // non doc comments should start with lowercase
                ast::Annotation::Comment { node, .. } => {
                    let comment = ctx.tree.get(*node);
                    let comment_text = ctx.strings.get(comment.string);
                    let comment_text = comment_text.as_ref().trim();

                    // skip comments that are exempt from lowercase casing
                    if comment_text.is_empty()
                        || is_directive_comment(comment_text)
                        || is_separator_comment(comment_text)
                        || is_separator_heading_block(
                            comment_text,
                            ctx.options.comment_separator_heading_min_lines,
                        )
                        || parse_keyword_comment_with_options(
                            comment_text,
                            &ctx.options.comment_keywords,
                            &ctx.options.comment_keyword_tags,
                        )
                        .is_some()
                    {
                        continue;
                    }

                    // report uppercase comment starts
                    if let Some(first_character) = first_alphabetic_character(comment_text)
                        && first_character.is_uppercase()
                    {
                        let severity = ctx.get_effective_severity(meta, node_id);
                        if !severity.is_enabled() {
                            continue;
                        }

                        ctx.report(
                            LintDiagnostic::new(
                                COMMENT_CASING.id,
                                COMMENT_CASING.code,
                                COMMENT_CASING.category,
                                severity,
                                "inline comment should start with lowercase",
                                ctx.module.file_id,
                                ctx.tree.get_span(node_id),
                            )
                            .with_label("use lowercase for inline comments"),
                        );
                    }
                }

                // doc comments should start with uppercase
                ast::Annotation::Doc { node, .. } => {
                    let doc = ctx.tree.get(*node);
                    let doc_text = ctx.strings.get(doc.string);

                    // find the first prose line
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
                        let severity = ctx.get_effective_severity(meta, node_id);
                        if !severity.is_enabled() {
                            continue;
                        }

                        ctx.report(
                            LintDiagnostic::new(
                                COMMENT_CASING.id,
                                COMMENT_CASING.code,
                                COMMENT_CASING.category,
                                severity,
                                "doc comment should start with uppercase",
                                ctx.module.file_id,
                                ctx.tree.get_span(node_id),
                            )
                            .with_label("use uppercase for doc comments"),
                        );
                    }
                }

                _ => {}
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
            options.comment_separator_heading_min_lines = 4;
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
