use destack_ast::{self as ast, AnnotationPosition};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Enforce comment casing conventions.
    ///
    /// Inline comments should begin with lowercase letters for consistency.
    /// Documentation comments should begin with uppercase (proper sentences).
    #[lint(
        id = "comment-casing",
        code = "LY003",
        category = Style,
        level = Ast,
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub CommentCasing,
    "Enforce comment casing"
}

impl LintRule for CommentCasing {
    fn meta(&self) -> &'static crate::LintMeta {
        CommentCasing::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Annotation>() {
            let annotation = ctx.tree.get(node_id);

            match annotation {
                // inline comments should start with lowercase
                ast::Annotation::Comment { node, position } => {
                    // skip block-level comments that might be section headers
                    if matches!(
                        position,
                        AnnotationPosition::BlockPrefix | AnnotationPosition::BlockPostfix
                    ) {
                        continue;
                    }

                    let comment = ctx.tree.get(*node);
                    let text = ctx.strings.get(comment.string);
                    let text = text.as_ref().trim();

                    // skip empty comments and special markers
                    if text.is_empty() || text.starts_with(['!', '#', '@', '*', '-', '=']) {
                        continue;
                    }

                    // check first alphabetic character
                    if let Some(first_char) = text.chars().find(|c| c.is_alphabetic())
                        && first_char.is_uppercase()
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

                // doc comments should start with uppercase (proper sentences)
                ast::Annotation::Doc { node, .. } => {
                    let doc = ctx.tree.get(*node);
                    let text = ctx.strings.get(doc.string);

                    // get first non-empty line
                    let first_line = text
                        .as_ref()
                        .lines()
                        .find(|line| !line.trim().is_empty())
                        .map(|line| line.trim());

                    let Some(first_line) = first_line else {
                        continue;
                    };

                    // skip special markers and annotations
                    if first_line.starts_with(['@', '#', '*', '-', '`']) {
                        continue;
                    }

                    // check first alphabetic character
                    if let Some(first_char) = first_line.chars().find(|c| c.is_alphabetic())
                        && first_char.is_lowercase()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_inline_comment_lowercase_allowed() {
        let test = TestProgram::for_rule_without_builtins(CommentCasing);
        let result = test.lint_ast(
            "test.ds",
            r#"
let x = 1 // this is fine
"#,
        );
        test.result(result).assert_no_lint("comment-casing");
    }

    #[test]
    fn test_inline_comment_uppercase_detected() {
        let test = TestProgram::for_rule_without_builtins(CommentCasing);
        let result = test.lint_ast(
            "test.ds",
            r#"
let x = 1 // This should be lowercase
"#,
        );
        test.result(result).assert_lint("comment-casing");
    }

    #[test]
    fn test_doc_comment_uppercase_allowed() {
        let test = TestProgram::for_rule_without_builtins(CommentCasing);
        let result = test.lint_ast(
            "test.ds",
            r#"
/// This is proper documentation.
function foo() {}
"#,
        );
        test.result(result).assert_no_lint("comment-casing");
    }

    #[test]
    fn test_doc_comment_lowercase_detected() {
        let test = TestProgram::for_rule_without_builtins(CommentCasing);
        let result = test.lint_ast(
            "test.ds",
            r#"
/// this should be uppercase
function foo() {}
"#,
        );
        test.result(result).assert_lint("comment-casing");
    }

    #[test]
    fn test_special_markers_skipped() {
        let test = TestProgram::for_rule_without_builtins(CommentCasing);
        let result = test.lint_ast(
            "test.ds",
            r#"
let x = 1 // @ts-ignore
let y = 2 // #region
"#,
        );
        test.result(result).assert_no_lint("comment-casing");
    }
}
