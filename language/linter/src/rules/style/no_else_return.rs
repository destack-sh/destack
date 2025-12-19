use destack_ast::{self as ast, Block};
use destack_source::Span;
use destack_workspace::LintSeverity;

use crate::rules::common::expression_get_block_span_str;
use crate::{LintDiagnostic, LintFix, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow `else` blocks after `return` in `if` statements.
    ///
    /// When an `if` block ends with a `return`, the `else` is unnecessary
    /// because the remaining code will only execute if the condition is false.
    #[lint(
        id = "no-else-return",
        code = "LY014",
        category = Style,
        level = Ast,
        fixable = Always,
        recommended = Strict,
        stability = Stable
    )]
    pub NoElseReturn,
    "Disallow else after return"
}

impl LintRule for NoElseReturn {
    fn meta(&self) -> &'static crate::LintMeta {
        NoElseReturn::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expr = ctx.tree.get(node_id);
            let ast::Expression::If {
                kind: ast::IfKind::If,
                then_expression,
                else_expression: Some(else_id),
                ..
            } = expr
            else {
                continue;
            };

            // check if the then block ends with a return
            if ends_with_return(ctx, *then_expression) {
                let if_span = ctx.tree.get_span(node_id);
                let then_span = ctx.tree.get_span(*then_expression);
                let else_span = ctx.tree.get_span(*else_id);

                // build replacement: if-then on one line, else content on next
                let if_then_span = Span::new(if_span.file, if_span.start, then_span.end);
                let if_then_text = ctx.get_span_text(if_then_span);
                let else_text = expression_get_block_span_str(ctx, *else_id);
                let replacement = format!("{if_then_text}\n{else_text}");
                let edits = ctx
                    .edit_builder()
                    .replace(if_span, replacement)
                    .into_edits();
                let fix = LintFix::safe("Remove unnecessary else").with_edits(edits);

                ctx.report(
                    LintDiagnostic::new(
                        NO_ELSE_RETURN.id,
                        NO_ELSE_RETURN.code,
                        NO_ELSE_RETURN.category,
                        severity,
                        "unnecessary `else` after `return`",
                        ctx.module.file_id,
                        else_span,
                    )
                    .with_label("remove the `else` and un-indent this code")
                    .with_fix(fix),
                );
            }
        }
    }
}

fn ends_with_return(
    ctx: &LintModuleAstContext<'_>,
    expr_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    let expr = ctx.tree.get(expr_id);
    match expr {
        ast::Expression::Return { .. } => true,
        ast::Expression::Block(block_id) => {
            let block: &Block = ctx.tree.get(*block_id);
            if let Some(&last_id) = block.expressions.last() {
                ends_with_return(ctx, last_id)
            } else {
                false
            }
        }
        ast::Expression::Statement(inner_id) => ends_with_return(ctx, *inner_id),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_else_after_return() {
        let test = TestProgram::for_rule(NoElseReturn);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(x: boolean) {
    if (x) {
        return 1
    } else {
        return 2
    }
}
"#,
        );
        test.result(result).assert_lint("no-else-return");
    }

    #[test]
    fn test_allows_no_else() {
        let test = TestProgram::for_rule(NoElseReturn);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(x: boolean) {
    if (x) {
        return 1
    }
    return 2
}
"#,
        );
        test.result(result).assert_no_lint("no-else-return");
    }

    #[test]
    fn test_allows_else_without_return_in_if() {
        let test = TestProgram::for_rule(NoElseReturn);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(x: boolean) {
    if (x) {
        console.log("yes")
    } else {
        console.log("no")
    }
}
"#,
        );
        test.result(result).assert_no_lint("no-else-return");
    }

    #[test]
    fn test_fix_else_after_return() {
        let test = TestProgram::for_rule(NoElseReturn);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(x: bool): int32 {
    if (x) {
        return 1
    } else {
        return 2
    }
}
"#,
        );
        test.result(result)
            .assert_lint("no-else-return")
            .assert_safe_fixed(
                r#"
function foo(x: bool): int32 {
    if (x) {
        return 1
    }
    return 2
}
"#,
            );
    }
}
