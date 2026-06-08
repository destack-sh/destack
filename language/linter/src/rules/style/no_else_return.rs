use crate::LintMeta;
use destack_dir::{self as dir, Block};
use destack_repository::LintSeverity;
use destack_source::Span;

use crate::rules::common::{expression_is_else_if_branch, source_text_contains_comment_token};
use crate::{LintFix, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow `else` blocks after `return` in `if` statements.
    ///
    /// When an `if` block ends with a `return`, the `else` is unnecessary
    /// because the remaining code will only execute if the condition is false.
    #[lint(
        id = "no-else-return",
        code = "LY017",
        category = Style,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable
    )]
    pub NoElseReturn,
    "Disallow else after return"
}

impl LintRule for NoElseReturn {
    fn meta(&self) -> &'static LintMeta {
        NoElseReturn::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.dir.iter_nodes::<dir::Expression>() {
            let expr = ctx.dir.get(node_id);
            let dir::Expression::If {
                form: dir::IfForm::If,
                then_expression,
                else_expression: Some(else_id),
                ..
            } = expr
            else {
                continue;
            };

            // skip else if chain members when the option allows them
            if ctx.options().style.no_else_return_allow_else_if
                && expression_is_else_if_branch(ctx.dir.tree(), node_id)
            {
                continue;
            }

            // skip else if alternates when the option allows them
            if ctx.options().style.no_else_return_allow_else_if
                && expression_is_else_if(ctx, *else_id)
            {
                continue;
            }

            // report when the then branch is terminal return
            if ends_with_return(ctx, *then_expression) {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }

                let else_span = ctx.dir.get_span(*else_id);
                let mut diagnostic = LintReport::new(
                    NO_ELSE_RETURN.id,
                    NO_ELSE_RETURN.code,
                    NO_ELSE_RETURN.category,
                    severity,
                    "unnecessary `else` after `return`",
                    else_span,
                )
                .label("remove the `else` and un-indent this code");

                // only apply safe fixes for block else branches without binding declarations
                if ctx.compute_fixes
                    && let Some(fix) = no_else_return_fix(ctx, node_id, *then_expression, *else_id)
                {
                    diagnostic = diagnostic.fix(fix);
                }

                ctx.report(diagnostic);
            }
        }
    }
}

/// Return true when one expression is an else if branch.
fn expression_is_else_if(
    ctx: &LintModuleContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    matches!(
        ctx.dir.get(expression_id),
        dir::Expression::If {
            form: dir::IfForm::If,
            ..
        }
    )
}

/// Build a conservative no else return fix.
fn no_else_return_fix(
    ctx: &LintModuleContext<'_>,
    if_expression_id: dir::LocalNodeId<dir::Expression>,
    then_expression_id: dir::LocalNodeId<dir::Expression>,
    else_expression_id: dir::LocalNodeId<dir::Expression>,
) -> Option<LintFix> {
    // only rewrite else blocks
    let dir::Expression::Block(else_block_id) = ctx.dir.get(else_expression_id) else {
        return None;
    };

    // avoid dropping comment text that lives inside the else block
    if source_text_contains_comment_token(ctx.get_span_text(ctx.dir.get_span(*else_block_id))) {
        return None;
    }

    // avoid lexical binding collisions from lifted else block bindings
    if block_contains_binding_declaration(ctx, *else_block_id) {
        return None;
    }

    // rewrite if then section followed by else block contents
    let if_span = ctx.dir.get_span(if_expression_id);
    let then_span = ctx.dir.get_span(then_expression_id);
    let if_then_span = Span::new(if_span.file, if_span.start, then_span.end);
    let if_then_text = ctx.get_span_text(if_then_span);
    let else_text = block_inner_text(ctx, *else_block_id)?;
    let replacement = format!("{if_then_text}\n{else_text}");
    let edits = ctx
        .edit_builder()
        .replace(if_span, replacement)
        .into_edits();

    Some(LintFix::safe("Remove unnecessary else").with_edits(edits))
}

/// Return true when one block contains binding declarations at top level.
fn block_contains_binding_declaration(
    ctx: &LintModuleContext<'_>,
    block_id: dir::LocalNodeId<dir::Block>,
) -> bool {
    let block = ctx.dir.get(block_id);
    block
        .iter_expressions()
        .any(|expression_id| expression_contains_binding_declaration(ctx, expression_id))
}

/// Return true when one expression declares bindings in local scope.
fn expression_contains_binding_declaration(
    ctx: &LintModuleContext<'_>,
    expr_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let expr = ctx.dir.get(expr_id);
    match expr {
        dir::Expression::Let { .. }
        | dir::Expression::Using { .. }
        | dir::Expression::Declaration(_) => true,
        dir::Expression::Parenthesized { expression } => {
            expression_contains_binding_declaration(ctx, *expression)
        }
        _ => false,
    }
}

/// Return the source text inside one block expression.
fn block_inner_text(
    ctx: &LintModuleContext<'_>,
    block_id: dir::LocalNodeId<dir::Block>,
) -> Option<String> {
    let block = ctx.dir.get(block_id);
    let (first_expression_id, last_expression_id) =
        (block.first_expression()?, block.last_expression()?);

    let first_span = ctx.dir.get_span(first_expression_id);
    let last_span = ctx.dir.get_span(last_expression_id);
    if first_span.file != last_span.file {
        return None;
    }

    let block_content_span = Span::new(first_span.file, first_span.start, last_span.end);
    Some(ctx.get_span_text(block_content_span).to_string())
}

fn ends_with_return(
    ctx: &LintModuleContext<'_>,
    expr_id: dir::LocalNodeId<dir::Expression>,
) -> bool {
    let expr = ctx.dir.get(expr_id);
    match expr {
        dir::Expression::Return { .. } => true,
        dir::Expression::Block(block_id) => {
            let block: &Block = ctx.dir.get(*block_id);
            if let Some(last_id) = block.last_expression() {
                ends_with_return(ctx, last_id)
            } else {
                false
            }
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_else_after_return() {
        let test = TestProgram::for_rule_without_prelude(NoElseReturn);
        let result = test.lint(
            "no_else_return/test_detects_else_after_return.ds",
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
    fn test_has_no_fix_when_else_block_contains_comment() {
        let test = TestProgram::for_rule_without_prelude(NoElseReturn);
        let result = test.lint(
            "no_else_return/test_has_no_fix_when_else_block_contains_comment.ds",
            r#"
function foo(x: boolean) {
    if (x) {
        return 1
    } else {
        // keep
        log()
    }
}
"#,
        );
        test.result(result)
            .assert_lint("no-else-return")
            .assert_has_no_fix("no-else-return");
    }

    #[test]
    fn test_allows_no_else() {
        let test = TestProgram::for_rule_without_prelude(NoElseReturn);
        let result = test.lint(
            "no_else_return/test_allows_no_else.ds",
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
        let test = TestProgram::for_rule_without_prelude(NoElseReturn);
        let result = test.lint(
            "no_else_return/test_allows_else_without_return_in_if.ds",
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
    fn test_allows_else_if_by_default() {
        let test = TestProgram::for_rule_without_prelude(NoElseReturn);
        let result = test.lint(
            "no_else_return/test_allows_else_if_by_default.ds",
            r#"
function foo(x: boolean, y: boolean) {
    if (x) {
        return 1
    } else if (y) {
        return 2
    } else {
        return 3
    }
}
"#,
        );
        test.result(result).assert_no_lint("no-else-return");
    }

    #[test]
    fn test_reports_else_if_when_disabled() {
        let test = TestProgram::for_rule_without_prelude(NoElseReturn)
            .with_options(|options| options.style.no_else_return_allow_else_if = false);
        let result = test.lint(
            "no_else_return/test_reports_else_if_when_disabled.ds",
            r#"
function foo(x: boolean, y: boolean) {
    if (x) {
        return 1
    } else if (y) {
        return 2
    } else {
        return 3
    }
}
"#,
        );
        test.result(result).assert_lint("no-else-return");
    }

    #[test]
    fn test_fix_else_after_return() {
        let test = TestProgram::for_rule_without_prelude(NoElseReturn);
        let result = test.lint(
            "no_else_return/test_fix_else_after_return.ds",
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
        return 1;
    }
    return 2;
}
"#,
            );
    }

    #[test]
    fn test_allows_else_if_chain_by_default() {
        let test = TestProgram::for_rule_without_prelude(NoElseReturn);
        let result = test.lint(
            "no_else_return/test_allows_else_if_chain_by_default.ds",
            r#"
function foo(x: int32): int32 {
    if (x > 0) {
        return 1
    } else if (x == 0) {
        return 0
    } else {
        return -1
    }
}
"#,
        );
        test.result(result).assert_no_lint("no-else-return");
    }

    #[test]
    fn test_no_fix_when_else_declares_bindings() {
        let test = TestProgram::for_rule_without_prelude(NoElseReturn);
        let result = test.lint(
            "no_else_return/test_no_fix_when_else_declares_bindings.ds",
            r#"
function foo(flag: bool): int32 {
    if (flag) {
        return 1
    } else {
        let value = 2
        return value
    }
}
"#,
        );
        test.result(result)
            .assert_lint("no-else-return")
            .assert_has_no_fix("no-else-return");
    }
}
