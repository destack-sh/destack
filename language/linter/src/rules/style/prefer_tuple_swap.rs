use crate::LintMeta;
use destack_ast::{self as ast, Expression, Pattern};
use destack_workspace::LintSeverity;

use crate::rules::common::{assign_pattern_expression, expression_path_segments};
use crate::{LintAstContext, LintDiagnostic, LintFix, LintRule, declare_lint};

declare_lint! {
    /// Prefer tuple swap form over a temporary variable.
    ///
    /// Swapping two variables using a temporary variable can be more clearly
    /// expressed using tuple destructuring assignment.
    ///
    /// ```
    /// // bad
    /// const temp = a
    /// a = b
    /// b = temp
    ///
    /// // good
    /// (a, b) = (b, a)
    /// ```
    #[lint(
        id = "prefer-tuple-swap",
        code = "LY060",
        category = Style,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Always,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferTupleSwap,
    "Prefer tuple swap form"
}

impl LintRule for PreferTupleSwap {
    fn meta(&self) -> &'static LintMeta {
        PreferTupleSwap::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        // look for blocks containing expression sequences
        for block_id in ctx.tree.iter_nodes::<ast::Block>() {
            let block = ctx.tree.get(block_id);
            let expression_ids = block.iter_expressions().collect::<Vec<_>>();

            // need at least 3 expressions for a swap pattern
            if expression_ids.len() < 3 {
                continue;
            }

            // check consecutive triples for swap pattern
            for window_start in 0..expression_ids.len().saturating_sub(2) {
                let first_id = expression_ids[window_start];
                let second_id = expression_ids[window_start + 1];
                let third_id = expression_ids[window_start + 2];
                if let Some(swap_info) = detect_swap_pattern(ctx, first_id, second_id, third_id) {
                    let severity = ctx.get_effective_severity(meta, first_id);
                    if !severity.is_enabled() {
                        continue;
                    }

                    let mut diagnostic = LintDiagnostic::new(
                        PREFER_TUPLE_SWAP.id,
                        PREFER_TUPLE_SWAP.code,
                        PREFER_TUPLE_SWAP.category,
                        severity,
                        format!(
                            "use tuple swap `({}, {}) = ({}, {})` instead of temporary variable",
                            swap_info.var_a, swap_info.var_b, swap_info.var_b, swap_info.var_a
                        ),
                        ctx.module.file_id,
                        ctx.tree.get_span(first_id),
                    )
                    .with_label("swap pattern starts here");

                    // compute fixes only when requested by the runner
                    if ctx.compute_fixes {
                        let first_span = ctx.tree.get_span(first_id);
                        let third_span = ctx.tree.get_span(third_id);
                        let replace_span = destack_source::Span::new(
                            first_span.file,
                            first_span.start,
                            third_span.end,
                        );
                        let replacement = format!(
                            "({}, {}) = ({}, {});",
                            swap_info.var_a, swap_info.var_b, swap_info.var_b, swap_info.var_a
                        );
                        let edits = ctx
                            .edit_builder()
                            .replace(replace_span, replacement)
                            .into_edits();
                        let fix = LintFix::safe("Use tuple swap assignment").with_edits(edits);
                        diagnostic = diagnostic.with_fix(fix);
                    }

                    ctx.report(diagnostic);
                }
            }
        }
    }
}

struct SwapInfo {
    var_a: String,
    var_b: String,
}

/// Detect a swap pattern across three consecutive expressions.
/// Pattern: `let temp = a; a = b; b = temp`
fn detect_swap_pattern(
    ctx: &LintAstContext<'_>,
    first_id: ast::LocalNodeId<Expression>,
    second_id: ast::LocalNodeId<Expression>,
    third_id: ast::LocalNodeId<Expression>,
) -> Option<SwapInfo> {
    let first = unwrap_statement(ctx, first_id);
    let second = unwrap_statement(ctx, second_id);
    let third = unwrap_statement(ctx, third_id);

    // first expression: let/const temp = a
    let (temp_name, var_a) = match first {
        Expression::Let { declarators, .. } => {
            if declarators.len() != 1 {
                return None;
            }
            let declarator = ctx.tree.get(declarators[0]);
            let temp_name = get_simple_binding_name(ctx, declarator.pattern)?;
            let init_id = declarator.value?;
            let var_a = get_simple_path_from_expression(ctx, init_id)?;
            (temp_name, var_a)
        }
        _ => return None,
    };

    // second expression: a = b (assignment)
    let (assigned_var, var_b) = match second {
        Expression::Assign { left, right, .. } => {
            let left_expression_id = assign_pattern_expression(ctx.tree, *left)?;
            let assigned = get_simple_path_from_expression(ctx, left_expression_id)?;
            let source = get_simple_path_from_expression(ctx, *right)?;
            (assigned, source)
        }
        _ => return None,
    };

    // check that the assigned variable is the same as var_a
    if assigned_var != var_a {
        return None;
    }

    // third expression: b = temp
    let (final_assigned, final_source) = match third {
        Expression::Assign { left, right, .. } => {
            let left_expression_id = assign_pattern_expression(ctx.tree, *left)?;
            let assigned = get_simple_path_from_expression(ctx, left_expression_id)?;
            let source = get_simple_path_from_expression(ctx, *right)?;
            (assigned, source)
        }
        _ => return None,
    };

    // check that we're assigning to var_b from temp
    if final_assigned != var_b || final_source != temp_name {
        return None;
    }

    Some(SwapInfo { var_a, var_b })
}

/// Unwrap a Statement expression wrapper if present.
fn unwrap_statement<'a>(
    ctx: &'a LintAstContext<'_>,
    expression_id: ast::LocalNodeId<Expression>,
) -> &'a Expression {
    ctx.tree.get(expression_id)
}

/// Get a simple identifier name from a binding pattern.
fn get_simple_binding_name(
    ctx: &LintAstContext<'_>,
    pattern_id: ast::LocalNodeId<Pattern>,
) -> Option<String> {
    let pattern = ctx.tree.get(pattern_id);
    match pattern {
        Pattern::Binding {
            name,
            pattern: None,
            ..
        } => Some(ctx.strings.get(*name).to_string()),
        _ => None,
    }
}

/// Get a simple path name from an expression (single identifier).
fn get_simple_path_from_expression(
    ctx: &LintAstContext<'_>,
    expression_id: ast::LocalNodeId<Expression>,
) -> Option<String> {
    let path_segments = expression_path_segments(ctx.tree, expression_id)?;
    if path_segments.len() != 1 {
        return None;
    }

    Some(ctx.strings.get(path_segments[0]).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_swap_with_const() {
        let test = TestProgram::for_rule_without_prelude(PreferTupleSwap);
        let result = test.lint_ast(
            "prefer_tuple_swap/test_detects_swap_with_const.ds",
            r#"
function swap() {
    const temp = a
    a = b
    b = temp
}
"#,
        );
        test.result(result)
            .assert_lint("prefer-tuple-swap")
            .assert_has_fix("prefer-tuple-swap");
    }

    #[test]
    fn test_detects_swap_with_let() {
        let test = TestProgram::for_rule_without_prelude(PreferTupleSwap);
        let result = test.lint_ast(
            "prefer_tuple_swap/test_detects_swap_with_let.ds",
            r#"
function swap() {
    let temp = x
    x = y
    y = temp
}
"#,
        );
        test.result(result).assert_lint("prefer-tuple-swap");
    }

    #[test]
    fn test_allows_tuple_swap() {
        let test = TestProgram::for_rule_without_prelude(PreferTupleSwap);
        let result = test.lint_ast(
            "prefer_tuple_swap/test_allows_tuple_swap.ds",
            r#"
function swap() {
    (a, b) = (b, a)
}
"#,
        );
        test.result(result).assert_no_lint("prefer-tuple-swap");
    }

    #[test]
    fn test_allows_non_swap_temp() {
        let test = TestProgram::for_rule_without_prelude(PreferTupleSwap);
        let result = test.lint_ast(
            "prefer_tuple_swap/test_allows_non_swap_temp.ds",
            r#"
function notSwap() {
    const temp = a
    a = b
    c = temp
}
"#,
        );
        // not a swap pattern: c != b
        test.result(result).assert_no_lint("prefer-tuple-swap");
    }

    #[test]
    fn test_allows_different_variables() {
        let test = TestProgram::for_rule_without_prelude(PreferTupleSwap);
        let result = test.lint_ast(
            "prefer_tuple_swap/test_allows_different_variables.ds",
            r#"
function notSwap() {
    const temp = a
    c = b
    b = temp
}
"#,
        );
        // c != a, so not a swap
        test.result(result).assert_no_lint("prefer-tuple-swap");
    }

    #[test]
    fn test_allows_complex_expressions() {
        let test = TestProgram::for_rule_without_prelude(PreferTupleSwap);
        let result = test.lint_ast(
            "prefer_tuple_swap/test_allows_complex_expressions.ds",
            r#"
function notSwap() {
    const temp = arr[0]
    arr[0] = arr[1]
    arr[1] = temp
}
"#,
        );
        // array access is not a simple path
        test.result(result).assert_no_lint("prefer-tuple-swap");
    }

    #[test]
    fn test_fix_rewrites_swap_to_tuple_assignment() {
        let test = TestProgram::for_rule_without_prelude(PreferTupleSwap);
        let result = test.lint_ast(
            "prefer_tuple_swap/test_fix_rewrites_swap_to_tuple_assignment.ds",
            r#"
function swap() {
    const temp = a
    a = b
    b = temp
}
"#,
        );
        test.result(result)
            .assert_lint("prefer-tuple-swap")
            .assert_safe_fixed(
                r#"
function swap() {
    (a, b,) = (b, a,);
}
"#,
            );
    }

    #[test]
    fn test_mutation_fix_rewrites_let_swap_to_tuple_assignment() {
        let test = TestProgram::for_rule_without_prelude(PreferTupleSwap);
        let result = test.lint_ast(
            "prefer_tuple_swap/test_mutation_fix_rewrites_let_swap_to_tuple_assignment.ds",
            r#"
function swap() {
    let temp = left
    left = right
    right = temp
}
"#,
        );
        test.result(result)
            .assert_lint("prefer-tuple-swap")
            .assert_safe_fixed(
                r#"
function swap() {
    (left, right,) = (right, left,);
}
"#,
            );
    }
}
