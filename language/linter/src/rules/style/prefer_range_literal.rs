use destack_ast::{
    self as ast, AssignOperator, BinaryOperator, Declarator, Expression, Pattern, ScalarLiteral,
    StringId, UnaryOperator,
};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintFix, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Prefer range literals over C-style for loops.
    ///
    /// Use `for (i in 0..n) { ... }` instead of `for (let i = 0; i < n; i++) { ... }`.
    /// Range literals are more concise and clearly express iteration intent.
    #[lint(
        id = "prefer-range-literal",
        code = "LY052",
        category = Style,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferRangeLiteral,
    "Prefer range literal for simple counted loops"
}

impl LintRule for PreferRangeLiteral {
    fn meta(&self) -> &'static crate::LintMeta {
        PreferRangeLiteral::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expr = ctx.tree.get(node_id);

            // look for C-style for loops
            let Expression::For {
                initialization: Some(initialization_id),
                condition: Some(condition_id),
                increment: Some(increment_id),
                body,
            } = expr
            else {
                continue;
            };

            // check if this looks like a simple counting loop
            let Some(candidate) =
                counting_loop_candidate(ctx.tree, *initialization_id, *condition_id, *increment_id)
            else {
                continue;
            };

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            let mut diagnostic = LintDiagnostic::new(
                PREFER_RANGE_LITERAL.id,
                PREFER_RANGE_LITERAL.code,
                PREFER_RANGE_LITERAL.category,
                severity,
                "use range literal instead of C-style for loop",
                ctx.module.file_id,
                ctx.tree.get_span(node_id),
            )
            .with_label("use `for (i in 0..n) { ... }` instead");
            if ctx.compute_fixes
                && let Some(fix) = prefer_range_literal_fix(ctx, node_id, *body, &candidate)
            {
                diagnostic = diagnostic.with_fix(fix);
            }
            ctx.report(diagnostic);
        }
    }
}

/// A simple counting loop that can be lowered to a range loop.
struct CountingLoopCandidate {
    /// The loop variable name.
    loop_variable_name: StringId,
    /// The declaration kind for the loop variable.
    declaration_kind: ast::LetKind,
    /// The start expression for the range.
    start_expression_id: ast::LocalNodeId<Expression>,
    /// The end expression for the range.
    end_expression_id: ast::LocalNodeId<Expression>,
    /// Whether the range end is inclusive.
    is_inclusive: bool,
}

/// Extract a simple counting loop candidate from a C style for loop.
fn counting_loop_candidate(
    tree: &ast::NodeTree,
    initialization_id: ast::LocalNodeId<Expression>,
    condition_id: ast::LocalNodeId<Expression>,
    increment_id: ast::LocalNodeId<Expression>,
) -> Option<CountingLoopCandidate> {
    let initialization = tree.get(initialization_id);
    let condition = tree.get(condition_id);
    let increment = tree.get(increment_id);

    // check initialization is a let binding with integer literal
    let (declaration_kind, loop_var, start_expression_id) = match initialization {
        Expression::Let {
            kind, declarators, ..
        } if declarators.len() == 1 => {
            let declarator: &Declarator = tree.get(declarators[0]);
            let pattern = tree.get(declarator.pattern);
            let Pattern::Binding { name, .. } = pattern else {
                return None;
            };
            let Some(start_expression_id) = declarator.value else {
                return None;
            };
            let value = tree.get(start_expression_id);
            // must initialize to an integer literal
            if !matches!(value, Expression::ScalarLiteral(ScalarLiteral::Integer(_))) {
                return None;
            }
            (*kind, *name, start_expression_id)
        }
        _ => return None,
    };

    // check condition is a comparison with the loop variable
    let (end_expression_id, is_inclusive) = match condition {
        Expression::Binary {
            left,
            operator: BinaryOperator::LessThan,
            right,
        } => {
            let left_expr = tree.get(*left);
            if !is_simple_identifier(left_expr, loop_var) {
                return None;
            }
            (*right, false)
        }
        Expression::Binary {
            left,
            operator: BinaryOperator::LessThanOrEqual,
            right,
        } => {
            let left_expr = tree.get(*left);
            if !is_simple_identifier(left_expr, loop_var) {
                return None;
            }
            (*right, true)
        }
        _ => return None,
    };

    // check increment is i++ or i += 1
    let is_simple_increment = match increment {
        // i++
        Expression::Unary {
            operator: UnaryOperator::PostIncrement,
            right,
        } => {
            let right_expr = tree.get(*right);
            is_simple_identifier(right_expr, loop_var)
        }
        // ++i
        Expression::Unary {
            operator: UnaryOperator::PreIncrement,
            right,
        } => {
            let right_expr = tree.get(*right);
            is_simple_identifier(right_expr, loop_var)
        }
        // i += 1
        Expression::Assign {
            operator: AssignOperator::AddAssign,
            left,
            right,
        } => {
            let left_expr = tree.get(*left);
            let right_expr = tree.get(*right);
            let is_loop_var = is_simple_identifier(left_expr, loop_var);
            let is_one = matches!(
                right_expr,
                Expression::ScalarLiteral(ScalarLiteral::Integer(1))
            );
            is_loop_var && is_one
        }
        _ => false,
    };

    if !is_simple_increment {
        return None;
    }

    Some(CountingLoopCandidate {
        loop_variable_name: loop_var,
        declaration_kind,
        start_expression_id,
        end_expression_id,
        is_inclusive,
    })
}

/// Check if an expression is a simple identifier matching the given name.
fn is_simple_identifier(expr: &Expression, name: StringId) -> bool {
    match expr {
        Expression::Path {
            path,
            static_arguments: None,
        } => path.segments.len() == 1 && path.segments[0] == name,
        _ => false,
    }
}

/// Build an unsafe fix that rewrites a counted for loop to a range loop.
fn prefer_range_literal_fix(
    ctx: &LintModuleAstContext<'_>,
    for_expression_id: ast::LocalNodeId<Expression>,
    body_id: ast::LocalNodeId<ast::Block>,
    candidate: &CountingLoopCandidate,
) -> Option<LintFix> {
    let declaration_keyword = match candidate.declaration_kind {
        ast::LetKind::Let => "let",
        ast::LetKind::Var => "var",
        ast::LetKind::Const => "const",
    };
    let loop_variable_name = ctx.strings.get(candidate.loop_variable_name).to_string();
    let start_text = ctx.get_span_text(ctx.tree.get_span(candidate.start_expression_id));
    let end_text = ctx.get_span_text(ctx.tree.get_span(candidate.end_expression_id));
    let body_text = ctx.get_span_text(ctx.tree.get_span(body_id));
    let range_operator = if candidate.is_inclusive { "..=" } else { ".." };
    let replacement = format!(
        "for ({declaration_keyword} {loop_variable_name} in {start_text}{range_operator}{end_text}) {body_text}"
    );

    let for_span = ctx.tree.get_span(for_expression_id);
    let edits = ctx
        .edit_builder()
        .replace(for_span, replacement)
        .into_edits();
    Some(LintFix::r#unsafe("Rewrite C-style loop to range loop").with_edits(edits))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_c_style_for() {
        let test = TestProgram::for_rule_without_prelude(PreferRangeLiteral);
        let result = test.lint_ast(
            "prefer_range_literal/test_detects_c_style_for.ds",
            r#"
for (let i = 0; i < 10; i++) {
    print(i)
}
"#,
        );
        test.result(result)
            .assert_lint("prefer-range-literal")
            .assert_unsafe_fixed(
                r#"
for (let i in 0..10) {
    print(i)
}
"#,
            );
    }

    #[test]
    fn test_detects_c_style_for_increment_assign() {
        let test = TestProgram::for_rule_without_prelude(PreferRangeLiteral);
        let result = test.lint_ast(
            "prefer_range_literal/test_detects_c_style_for_increment_assign.ds",
            r#"
for (let i = 0; i < n; i += 1) {
    print(i)
}
"#,
        );
        test.result(result)
            .assert_lint("prefer-range-literal")
            .assert_unsafe_fixed(
                r#"
for (let i in 0..n) {
    print(i)
}
"#,
            );
    }

    #[test]
    fn test_fix_preserves_inclusive_range_condition() {
        let test = TestProgram::for_rule_without_prelude(PreferRangeLiteral);
        let result = test.lint_ast(
            "prefer_range_literal/test_fix_preserves_inclusive_range_condition.ds",
            r#"
for (let i = 0; i <= 10; i++) {
    print(i)
}
"#,
        );
        test.result(result)
            .assert_lint("prefer-range-literal")
            .assert_unsafe_fixed(
                r#"
for (let i in 0..=10) {
    print(i)
}
"#,
            );
    }

    #[test]
    fn test_allows_range_literal() {
        let test = TestProgram::for_rule_without_prelude(PreferRangeLiteral);
        let result = test.lint_ast(
            "prefer_range_literal/test_allows_range_literal.ds",
            r#"
for (i in 0..10) {
    print(i)
}
"#,
        );
        test.result(result).assert_no_lint("prefer-range-literal");
    }

    #[test]
    fn test_allows_for_each() {
        let test = TestProgram::for_rule_without_prelude(PreferRangeLiteral);
        let result = test.lint_ast(
            "prefer_range_literal/test_allows_for_each.ds",
            r#"
for (item in items) {
    print(item)
}
"#,
        );
        test.result(result).assert_no_lint("prefer-range-literal");
    }

    #[test]
    fn test_allows_complex_for() {
        let test = TestProgram::for_rule_without_prelude(PreferRangeLiteral);
        // non-standard increment shouldn't trigger
        let result = test.lint_ast(
            "prefer_range_literal/test_allows_complex_for.ds",
            r#"
for (let i = 0; i < 10; i += 2) {
    print(i)
}
"#,
        );
        test.result(result).assert_no_lint("prefer-range-literal");
    }
}
