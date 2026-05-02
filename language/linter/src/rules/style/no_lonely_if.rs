use crate::LintMeta;
use destack_ast::{self as ast, Block};
use destack_workspace::LintSeverity;

use crate::rules::common::span_has_comment;
use crate::{LintAstContext, LintFix, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow `if` as the only statement in an `else` block.
    ///
    /// If the `else` block contains only an `if` statement, it should be
    /// written as `else if` instead.
    #[lint(
        id = "no-lonely-if",
        code = "LY019",
        category = Style,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable
    )]
    pub NoLonelyIf,
    "Disallow lonely if in else"
}

impl LintRule for NoLonelyIf {
    fn meta(&self) -> &'static LintMeta {
        NoLonelyIf::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expr = ctx.tree.get(node_id);
            let ast::Expression::If {
                kind: ast::IfKind::If,
                else_expression: Some(else_id),
                ..
            } = expr
            else {
                continue;
            };

            // check if else is a block containing only an if statement
            let else_expr = ctx.tree.get(*else_id);
            let lonely_if_id = match else_expr {
                ast::Expression::Block(block_id) => {
                    let block: &Block = ctx.tree.get(*block_id);
                    if block.len() == 1 {
                        let single_expression_id = block.first_expression().unwrap();
                        let single_expr = ctx.tree.get(single_expression_id);
                        if matches!(
                            single_expr,
                            ast::Expression::If {
                                kind: ast::IfKind::If,
                                ..
                            }
                        ) {
                            Some(single_expression_id)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                ast::Expression::If {
                    kind: ast::IfKind::If,
                    ..
                } => {
                    // else expression is already an if (else if), this is fine
                    None
                }
                _ => None,
            };

            if let Some(lonely_id) = lonely_if_id {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }

                let else_span = ctx.tree.get_span(*else_id);
                let lonely_span = ctx.tree.get_span(lonely_id);
                let lonely_text = ctx.get_span_text(lonely_span);
                let mut diagnostic = LintReport::new(
                    NO_LONELY_IF.id,
                    NO_LONELY_IF.code,
                    NO_LONELY_IF.category,
                    severity,
                    "lonely `if` in `else` block",
                    lonely_span,
                )
                .label("use `else if` instead");

                // avoid rewrites when else block contains trivia
                if ctx.compute_fixes && !span_has_comment(ctx.tree, else_span) {
                    let edits = ctx
                        .edit_builder()
                        .replace(else_span, lonely_text)
                        .into_edits();
                    let fix = LintFix::safe("Convert to `else if`").with_edits(edits);
                    diagnostic = diagnostic.fix(fix);
                }

                ctx.report(diagnostic);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_lonely_if() {
        let test = TestProgram::for_rule_without_prelude(NoLonelyIf);
        let result = test.lint_ast(
            "no_lonely_if/test_detects_lonely_if.ds",
            r#"
if (a) {
    foo()
} else {
    if (b) {
        bar()
    }
}
"#,
        );
        test.result(result).assert_lint("no-lonely-if");
    }

    #[test]
    fn test_allows_else_if() {
        let test = TestProgram::for_rule_without_prelude(NoLonelyIf);
        let result = test.lint_ast(
            "no_lonely_if/test_allows_else_if.ds",
            r#"
if (a) {
    foo()
} else if (b) {
    bar()
}
"#,
        );
        test.result(result).assert_no_lint("no-lonely-if");
    }

    #[test]
    fn test_allows_else_with_multiple_statements() {
        let test = TestProgram::for_rule_without_prelude(NoLonelyIf);
        let result = test.lint_ast(
            "no_lonely_if/test_allows_else_with_multiple_statements.ds",
            r#"
if (a) {
    foo()
} else {
    setup()
    if (b) {
        bar()
    }
}
"#,
        );
        test.result(result).assert_no_lint("no-lonely-if");
    }

    #[test]
    fn test_fix_lonely_if() {
        let test = TestProgram::for_rule_without_prelude(NoLonelyIf);
        let result = test.lint_ast(
            "no_lonely_if/test_fix_lonely_if.ds",
            r#"
if (a) {
    foo()
} else {
    if (b) {
        bar()
    }
}
"#,
        );
        test.result(result)
            .assert_lint("no-lonely-if")
            .assert_safe_fixed(
                r#"
if (a) {
    foo()
} else if (b) {
    bar()
}
"#,
            );
    }

    #[test]
    fn test_no_fix_when_else_contains_comment_trivia() {
        let test = TestProgram::for_rule_without_prelude(NoLonelyIf);
        let result = test.lint_ast(
            "no_lonely_if/test_no_fix_when_else_contains_comment_trivia.ds",
            r#"
if (a) {
    foo()
} else {
    // keep branch note
    if (b) {
        bar()
    }
}
"#,
        );
        test.result(result)
            .assert_lint("no-lonely-if")
            .assert_has_no_fix("no-lonely-if");
    }
}
