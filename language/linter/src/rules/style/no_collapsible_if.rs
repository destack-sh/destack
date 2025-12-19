use destack_ast::{self as ast, Block};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintFix, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Suggest merging nested if statements without else.
    ///
    /// When an if statement's body contains only another if statement,
    /// and neither has an else clause, they can be combined using `&&`.
    ///
    /// ```
    /// // bad
    /// if (a) {
    ///     if (b) {
    ///         doSomething()
    ///     }
    /// }
    ///
    /// // good
    /// if (a && b) {
    ///     doSomething()
    /// }
    /// ```
    #[lint(
        id = "no-collapsible-if",
        code = "LY048",
        category = Style,
        level = Ast,
        fixable = Always,
        recommended = Strict,
        stability = Stable
    )]
    pub NoCollapsibleIf,
    "Suggest merging nested if statements"
}

impl LintRule for NoCollapsibleIf {
    fn meta(&self) -> &'static crate::LintMeta {
        NoCollapsibleIf::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);

            // match outer if statement without else
            let ast::Expression::If {
                kind: ast::IfKind::If,
                condition: outer_condition_id,
                then_expression: then_expression_id,
                else_expression: None,
                ..
            } = expression
            else {
                continue;
            };

            // get the then block
            let then_block = ctx.tree.get(*then_expression_id);

            // check if the then block contains only a single if statement without else
            let Some(inner_if_id) =
                find_single_if_without_else(ctx, then_block, *then_expression_id)
            else {
                continue;
            };

            // get the inner if expression details
            let inner_if = ctx.tree.get(inner_if_id);
            let ast::Expression::If {
                condition: inner_condition_id,
                then_expression: inner_then_id,
                ..
            } = inner_if
            else {
                continue;
            };

            // build the fix: if (a) { if (b) { body } } -> if (a && b) { body }
            let outer_span = ctx.tree.get_span(node_id);
            let outer_cond_span = ctx.tree.get_span(*outer_condition_id);
            let outer_cond_text = ctx.get_span_text(outer_cond_span);
            let inner_cond_span = ctx.tree.get_span(*inner_condition_id);
            let inner_cond_text = ctx.get_span_text(inner_cond_span);
            let inner_then_span = ctx.tree.get_span(*inner_then_id);
            let inner_then_text = ctx.get_span_text(inner_then_span);

            let replacement =
                format!("if ({outer_cond_text} && {inner_cond_text}) {inner_then_text}");
            let edits = ctx
                .edit_builder()
                .replace(outer_span, replacement)
                .into_edits();
            let fix = LintFix::safe("Merge nested if statements").with_edits(edits);

            ctx.report(
                LintDiagnostic::new(
                    NO_COLLAPSIBLE_IF.id,
                    NO_COLLAPSIBLE_IF.code,
                    NO_COLLAPSIBLE_IF.category,
                    severity,
                    "nested `if` statements can be merged",
                    ctx.module.file_id,
                    outer_span,
                )
                .with_label("combine conditions using `&&`")
                .with_fix(fix),
            );
        }
    }
}

/// Check if a then expression contains only a single if statement without else.
/// Returns the inner if expression ID if found.
fn find_single_if_without_else(
    ctx: &LintModuleAstContext<'_>,
    expression: &ast::Expression,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> Option<ast::LocalNodeId<ast::Expression>> {
    // handle block wrapper
    if let ast::Expression::Block(block_id) = expression {
        let block: &Block = ctx.tree.get(*block_id);
        if block.expressions.len() != 1 {
            return None;
        }

        let inner_id = block.expressions[0];
        let single_expression = ctx.tree.get(inner_id);
        return is_if_without_else(ctx, single_expression, inner_id);
    }

    is_if_without_else(ctx, expression, expression_id)
}

/// Check if an expression is an if without else (possibly wrapped in a Statement).
/// Returns the if expression ID if it's a valid collapsible if.
fn is_if_without_else(
    ctx: &LintModuleAstContext<'_>,
    expression: &ast::Expression,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> Option<ast::LocalNodeId<ast::Expression>> {
    // unwrap statement if needed
    if let ast::Expression::Statement(inner_id) = expression {
        let inner = ctx.tree.get(*inner_id);
        return is_if_without_else(ctx, inner, *inner_id);
    }

    // check if it's an if without else
    if let ast::Expression::If {
        kind: ast::IfKind::If,
        else_expression: None,
        ..
    } = expression
    {
        return Some(expression_id);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_nested_if_without_else_detected() {
        let test = TestProgram::for_rule(NoCollapsibleIf);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(a: bool, b: bool) {
    if (a) {
        if (b) {
            doSomething()
        }
    }
}
"#,
        );
        test.result(result).assert_lint("no-collapsible-if");
    }

    #[test]
    fn test_nested_three_levels_detected() {
        let test = TestProgram::for_rule(NoCollapsibleIf);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(a: bool, b: bool, c: bool) {
    if (a) {
        if (b) {
            if (c) {
                doSomething()
            }
        }
    }
}
"#,
        );
        // should detect the outer two at least
        test.result(result).assert_lint("no-collapsible-if");
    }

    #[test]
    fn test_outer_has_else_allowed() {
        let test = TestProgram::for_rule(NoCollapsibleIf);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(a: bool, b: bool) {
    if (a) {
        if (b) {
            doSomething()
        }
    } else {
        doOther()
    }
}
"#,
        );
        test.result(result).assert_no_lint("no-collapsible-if");
    }

    #[test]
    fn test_inner_has_else_allowed() {
        let test = TestProgram::for_rule(NoCollapsibleIf);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(a: bool, b: bool) {
    if (a) {
        if (b) {
            doSomething()
        } else {
            doOther()
        }
    }
}
"#,
        );
        test.result(result).assert_no_lint("no-collapsible-if");
    }

    #[test]
    fn test_multiple_statements_in_then_allowed() {
        let test = TestProgram::for_rule(NoCollapsibleIf);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(a: bool, b: bool) {
    if (a) {
        setup()
        if (b) {
            doSomething()
        }
    }
}
"#,
        );
        test.result(result).assert_no_lint("no-collapsible-if");
    }

    #[test]
    fn test_simple_if_allowed() {
        let test = TestProgram::for_rule(NoCollapsibleIf);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(a: bool) {
    if (a) {
        doSomething()
    }
}
"#,
        );
        test.result(result).assert_no_lint("no-collapsible-if");
    }

    #[test]
    fn test_if_else_allowed() {
        let test = TestProgram::for_rule(NoCollapsibleIf);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(a: bool) {
    if (a) {
        doSomething()
    } else {
        doOther()
    }
}
"#,
        );
        test.result(result).assert_no_lint("no-collapsible-if");
    }

    #[test]
    fn test_already_combined_condition_allowed() {
        let test = TestProgram::for_rule(NoCollapsibleIf);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(a: bool, b: bool) {
    if (a && b) {
        doSomething()
    }
}
"#,
        );
        test.result(result).assert_no_lint("no-collapsible-if");
    }

    #[test]
    fn test_fix_nested_if() {
        let test = TestProgram::for_rule(NoCollapsibleIf);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(a: bool, b: bool) {
    if (a) {
        if (b) {
            doSomething()
        }
    }
}
"#,
        );
        test.result(result)
            .assert_lint("no-collapsible-if")
            .assert_safe_fixed(
                r#"
function foo(a: bool, b: bool) {
    if ((a) && (b)) {
        doSomething()
    }
}
"#,
            );
    }
}
