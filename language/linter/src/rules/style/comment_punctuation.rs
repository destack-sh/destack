use destack_ast::{self as ast, AnnotationPosition};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Enforce comment punctuation conventions.
    ///
    /// Short inline comments should not end with a period.
    /// Doc comments should end with proper punctuation.
    #[lint(
        id = "comment-punctuation",
        code = "LY005",
        category = Style,
        level = Ast,
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub CommentPunctuation,
    "Enforce comment punctuation"
}

/// Check if text is a short comment (single sentence, < ~80 chars).
fn is_short_comment(text: &str) -> bool {
    let trimmed = text.trim();
    // short if no newlines and under typical line length
    !trimmed.contains('\n') && trimmed.len() < 80
}

impl LintRule for CommentPunctuation {
    fn meta(&self) -> &'static crate::LintMeta {
        CommentPunctuation::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Annotation>() {
            let annotation = ctx.tree.get(node_id);

            match annotation {
                // inline comments: short ones should not end with period
                ast::Annotation::Comment { node, position } => {
                    // skip block-level comments
                    if matches!(
                        position,
                        AnnotationPosition::BlockPrefix | AnnotationPosition::BlockPostfix
                    ) {
                        continue;
                    }

                    let comment = ctx.tree.get(*node);
                    let text = ctx.strings.get(comment.string);
                    let trimmed = text.as_ref().trim();

                    // skip empty or special comments
                    if trimmed.is_empty() || trimmed.starts_with(['@', '#', '!', '*']) {
                        continue;
                    }

                    // short inline comments should not end with period
                    if is_short_comment(trimmed) && trimmed.ends_with('.') {
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
                                "short inline comment should not end with period",
                                ctx.module.file_id,
                                ctx.tree.get_span(node_id),
                            )
                            .with_label("remove trailing period"),
                        );
                    }
                }

                // doc comments should end with proper punctuation
                ast::Annotation::Doc { node, .. } => {
                    let doc = ctx.tree.get(*node);
                    let text = ctx.strings.get(doc.string);

                    // get last non-empty, non-annotation line
                    let last_content_line = text
                        .as_ref()
                        .lines()
                        .rev()
                        .find(|line| {
                            let trimmed = line.trim();
                            !trimmed.is_empty()
                                && !trimmed.starts_with('@')
                                && !trimmed.starts_with("```")
                        })
                        .map(|line| line.trim());

                    let Some(last_line) = last_content_line else {
                        continue;
                    };

                    // skip code snippets and special markers
                    if last_line.starts_with('`') || last_line.starts_with('*') {
                        continue;
                    }

                    // doc comments should end with punctuation
                    if !last_line.ends_with(['.', '!', '?', ':', ')']) {
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
                                "doc comment should end with punctuation",
                                ctx.module.file_id,
                                ctx.tree.get_span(node_id),
                            )
                            .with_label("add period or other punctuation"),
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
    fn test_inline_no_period_allowed() {
        let test = TestProgram::for_rule_without_builtins(CommentPunctuation);
        let result = test.lint_ast(
            "test.ds",
            r#"
let x = 1 // increment counter
"#,
        );
        test.result(result).assert_no_lint("comment-punctuation");
    }

    #[test]
    fn test_inline_with_period_detected() {
        let test = TestProgram::for_rule_without_builtins(CommentPunctuation);
        let result = test.lint_ast(
            "test.ds",
            r#"
let x = 1 // increment counter.
"#,
        );
        test.result(result).assert_lint("comment-punctuation");
    }

    #[test]
    fn test_doc_with_period_allowed() {
        let test = TestProgram::for_rule_without_builtins(CommentPunctuation);
        let result = test.lint_ast(
            "test.ds",
            r#"
/// Increments the counter.
function foo() {}
"#,
        );
        test.result(result).assert_no_lint("comment-punctuation");
    }

    #[test]
    fn test_doc_without_punctuation_detected() {
        let test = TestProgram::for_rule_without_builtins(CommentPunctuation);
        let result = test.lint_ast(
            "test.ds",
            r#"
/// Increments the counter
function foo() {}
"#,
        );
        test.result(result).assert_lint("comment-punctuation");
    }

    #[test]
    fn test_doc_with_question_allowed() {
        let test = TestProgram::for_rule_without_builtins(CommentPunctuation);
        let result = test.lint_ast(
            "test.ds",
            r#"
/// Is this valid?
function foo() {}
"#,
        );
        test.result(result).assert_no_lint("comment-punctuation");
    }

    #[test]
    fn test_doc_with_exclamation_allowed() {
        let test = TestProgram::for_rule_without_builtins(CommentPunctuation);
        let result = test.lint_ast(
            "test.ds",
            r#"
/// Do not call this!
function foo() {}
"#,
        );
        test.result(result).assert_no_lint("comment-punctuation");
    }
}
