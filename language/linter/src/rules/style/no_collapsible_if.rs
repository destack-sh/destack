use crate::LintMeta;
use destack_ast::{self as ast, Block};
use destack_workspace::LintSeverity;

use crate::rules::common::{expression_unwrap_parenthesized_source_form, span_has_comment};
use crate::{LintAstContext, LintDiagnostic, LintFix, LintRule, declare_lint};

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
        code = "LY015",
        category = Style,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable
    )]
    pub NoCollapsibleIf,
    "Suggest merging nested if statements"
}

impl LintRule for NoCollapsibleIf {
    fn meta(&self) -> &'static LintMeta {
        NoCollapsibleIf::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);

            // match outer if statement without else
            let ast::Expression::If {
                kind: ast::IfKind::If,
                condition: outer_condition,
                then_expression: then_expression_id,
                else_expression: None,
                ..
            } = expression
            else {
                continue;
            };
            let outer_condition_id = match outer_condition {
                ast::IfCondition::Expression { condition } => *condition,
                ast::IfCondition::Let { .. } => continue,
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
                condition: inner_condition,
                then_expression: inner_then_id,
                ..
            } = inner_if
            else {
                continue;
            };
            let inner_condition_id = match inner_condition {
                ast::IfCondition::Expression { condition } => *condition,
                ast::IfCondition::Let { .. } => continue,
            };

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            let outer_span = ctx.tree.get_span(node_id);
            let mut diagnostic = LintDiagnostic::new(
                NO_COLLAPSIBLE_IF.id,
                NO_COLLAPSIBLE_IF.code,
                NO_COLLAPSIBLE_IF.category,
                severity,
                "nested `if` statements can be merged",
                ctx.module.file_id,
                outer_span,
            )
            .with_label("combine conditions using `&&`");

            // skip fixes when nested if range includes comment trivia
            if ctx.compute_fixes && !span_has_comment(ctx.tree, outer_span) {
                let outer_cond_text = and_condition_operand_text(ctx, outer_condition_id);
                let inner_cond_text = and_condition_operand_text(ctx, inner_condition_id);
                let inner_then_span = ctx.tree.get_span(*inner_then_id);
                let inner_then_text = ctx.get_span_text(inner_then_span);

                // preserve precedence for both condition sub-expressions
                let replacement =
                    format!("if ({outer_cond_text} && {inner_cond_text}) {inner_then_text}");
                let edits = ctx
                    .edit_builder()
                    .replace(outer_span, replacement)
                    .into_edits();
                let fix = LintFix::safe("Merge nested if statements").with_edits(edits);
                diagnostic = diagnostic.with_fix(fix);
            }

            ctx.report(diagnostic);
        }
    }
}

/// Return one `&&` operand text with minimal precedence-preserving wrapping.
fn and_condition_operand_text(
    ctx: &LintAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> String {
    let expression_span = ctx.tree.get_span(expression_id);
    let expression_text = ctx.get_span_text(expression_span);
    let expression_text = strip_one_outer_parentheses(expression_text);
    let normalized_expression_id =
        expression_unwrap_parenthesized_source_form(ctx.tree, expression_id);
    let normalized_expression = ctx.tree.get(normalized_expression_id);
    if expression_needs_parentheses_for_and_operand(normalized_expression) {
        return format!("({expression_text})");
    }

    expression_text.to_string()
}

/// Strip one outer parenthesis pair when it wraps the full expression text.
fn strip_one_outer_parentheses(text: &str) -> &str {
    let trimmed = text.trim();
    if !(trimmed.starts_with('(') && trimmed.ends_with(')')) {
        return trimmed;
    }

    let mut depth: i32 = 0;
    for (offset, character) in trimmed.char_indices() {
        if character == '(' {
            depth += 1;
            continue;
        }

        if character == ')' {
            depth -= 1;
            if depth == 0 && offset + 1 != trimmed.len() {
                return trimmed;
            }
        }
    }

    if depth != 0 || trimmed.len() < 2 {
        return trimmed;
    }

    &trimmed[1..trimmed.len() - 1]
}

/// Return true when an expression needs wrapping as one `&&` operand.
fn expression_needs_parentheses_for_and_operand(expression: &ast::Expression) -> bool {
    matches!(
        expression,
        ast::Expression::If {
            kind: ast::IfKind::Ternary,
            ..
        } | ast::Expression::Assign { .. }
            | ast::Expression::Binary {
                operator: ast::BinaryOperator::Equal
                    | ast::BinaryOperator::NotEqual
                    | ast::BinaryOperator::EqualStrict
                    | ast::BinaryOperator::NotEqualStrict
                    | ast::BinaryOperator::LessThan
                    | ast::BinaryOperator::LessThanOrEqual
                    | ast::BinaryOperator::GreaterThan
                    | ast::BinaryOperator::GreaterThanOrEqual
                    | ast::BinaryOperator::Or
                    | ast::BinaryOperator::Coalesce,
                ..
            }
    )
}

/// Check if a then expression contains only a single if statement without else.
/// Returns the inner if expression ID if found.
fn find_single_if_without_else(
    ctx: &LintAstContext<'_>,
    expression: &ast::Expression,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> Option<ast::LocalNodeId<ast::Expression>> {
    // handle block wrapper
    if let ast::Expression::Block(block_id) = expression {
        let block: &Block = ctx.tree.get(*block_id);
        if block.len() != 1 {
            return None;
        }

        let inner_id = block.first_expression()?;
        let single_expression = ctx.tree.get(inner_id);
        return is_if_without_else(ctx, single_expression, inner_id);
    }

    is_if_without_else(ctx, expression, expression_id)
}

/// Check if an expression is an if without else (possibly wrapped in a Statement).
/// Returns the if expression ID if it's a valid collapsible if.
fn is_if_without_else(
    _ctx: &LintAstContext<'_>,
    expression: &ast::Expression,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> Option<ast::LocalNodeId<ast::Expression>> {
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
        let test = TestProgram::for_rule_without_prelude(NoCollapsibleIf);
        let result = test.lint_ast(
            "no_collapsible_if/test_nested_if_without_else_detected.ds",
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
        let test = TestProgram::for_rule_without_prelude(NoCollapsibleIf);
        let result = test.lint_ast(
            "no_collapsible_if/test_nested_three_levels_detected.ds",
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
        let test = TestProgram::for_rule_without_prelude(NoCollapsibleIf);
        let result = test.lint_ast(
            "no_collapsible_if/test_outer_has_else_allowed.ds",
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
        let test = TestProgram::for_rule_without_prelude(NoCollapsibleIf);
        let result = test.lint_ast(
            "no_collapsible_if/test_inner_has_else_allowed.ds",
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
        let test = TestProgram::for_rule_without_prelude(NoCollapsibleIf);
        let result = test.lint_ast(
            "no_collapsible_if/test_multiple_statements_in_then_allowed.ds",
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
        let test = TestProgram::for_rule_without_prelude(NoCollapsibleIf);
        let result = test.lint_ast(
            "no_collapsible_if/test_simple_if_allowed.ds",
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
        let test = TestProgram::for_rule_without_prelude(NoCollapsibleIf);
        let result = test.lint_ast(
            "no_collapsible_if/test_if_else_allowed.ds",
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
        let test = TestProgram::for_rule_without_prelude(NoCollapsibleIf);
        let result = test.lint_ast(
            "no_collapsible_if/test_already_combined_condition_allowed.ds",
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
        let test = TestProgram::for_rule_without_prelude(NoCollapsibleIf);
        let result = test.lint_ast(
            "no_collapsible_if/test_fix_nested_if.ds",
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
    if (a && b) {
        doSomething()
    }
}
"#,
            );
    }

    #[test]
    fn test_fix_wraps_conditions_to_preserve_precedence() {
        let test = TestProgram::for_rule_without_prelude(NoCollapsibleIf);
        let result = test.lint_ast(
            "no_collapsible_if/test_fix_wraps_conditions_to_preserve_precedence.ds",
            r#"
function foo(a: bool, b: bool, c: bool) {
    if (a || b) {
        if (c) {
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
function foo(a: bool, b: bool, c: bool) {
    if ((a || b) && c) {
        doSomething()
    }
}
"#,
            );
    }

    #[test]
    fn test_no_fix_when_nested_if_has_comments() {
        let test = TestProgram::for_rule_without_prelude(NoCollapsibleIf);
        let result = test.lint_ast(
            "no_collapsible_if/test_no_fix_when_nested_if_has_comments.ds",
            r#"
function foo(a: bool, b: bool) {
    if (a) {
        // keep nested branch note
        if (b) {
            doSomething()
        }
    }
}
"#,
        );
        test.result(result)
            .assert_lint("no-collapsible-if")
            .assert_has_no_fix("no-collapsible-if");
    }
}
