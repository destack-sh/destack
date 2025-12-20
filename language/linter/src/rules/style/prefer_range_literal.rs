use destack_ast::{
    self as ast, AssignOperator, BinaryOperator, Declarator, Expression, Pattern, ScalarLiteral,
    StringId, UnaryOperator,
};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Prefer range literals over C-style for loops.
    ///
    /// Use `for (i in 0..n) { ... }` instead of `for (let i = 0; i < n; i++) { ... }`.
    /// Range literals are more concise and clearly express iteration intent.
    #[lint(
        id = "prefer-range-literal",
        code = "LY030",
        category = Style,
        level = Ast,
        fixable = No,
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
                body: _,
            } = expr
            else {
                continue;
            };

            // check if this looks like a simple counting loop
            if !is_simple_counting_loop(ctx.tree, *initialization_id, *condition_id, *increment_id)
            {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            ctx.report(
                LintDiagnostic::new(
                    PREFER_RANGE_LITERAL.id,
                    PREFER_RANGE_LITERAL.code,
                    PREFER_RANGE_LITERAL.category,
                    severity,
                    "use range literal instead of C-style for loop",
                    ctx.module.file_id,
                    ctx.tree.get_span(node_id),
                )
                .with_label("use `for (i in 0..n) { ... }` instead"),
            );
        }
    }
}

/// Check if a for loop is a simple counting loop that can use a range.
#[allow(clippy::let_and_return)]
fn is_simple_counting_loop(
    tree: &ast::NodeTree,
    initialization_id: ast::LocalNodeId<Expression>,
    condition_id: ast::LocalNodeId<Expression>,
    increment_id: ast::LocalNodeId<Expression>,
) -> bool {
    let initialization = tree.get(initialization_id);
    let condition = tree.get(condition_id);
    let increment = tree.get(increment_id);

    // check initialization is a let binding with integer literal
    let loop_var = match initialization {
        Expression::Let { declarators, .. } if declarators.len() == 1 => {
            let declarator: &Declarator = tree.get(declarators[0]);
            let pattern = tree.get(declarator.pattern);
            let Pattern::Binding { name, .. } = pattern else {
                return false;
            };
            let Some(value_id) = declarator.value else {
                return false;
            };
            let value = tree.get(value_id);
            // must initialize to an integer literal
            if !matches!(value, Expression::ScalarLiteral(ScalarLiteral::Integer(_))) {
                return false;
            }
            *name
        }
        _ => return false,
    };

    // check condition is a comparison with the loop variable
    let uses_loop_var_in_condition = match condition {
        Expression::Binary {
            left,
            operator: BinaryOperator::LessThan | BinaryOperator::LessThanOrEqual,
            right: _,
        } => {
            let left_expr = tree.get(*left);
            is_simple_identifier(left_expr, loop_var)
        }
        _ => false,
    };

    if !uses_loop_var_in_condition {
        return false;
    }

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

    is_simple_increment
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_c_style_for() {
        let test = TestProgram::for_rule_without_builtins(PreferRangeLiteral);
        let result = test.lint_ast(
            "test.ds",
            r#"
for (let i = 0; i < 10; i++) {
    print(i)
}
"#,
        );
        test.result(result).assert_lint("prefer-range-literal");
    }

    #[test]
    fn test_detects_c_style_for_increment_assign() {
        let test = TestProgram::for_rule_without_builtins(PreferRangeLiteral);
        let result = test.lint_ast(
            "test.ds",
            r#"
for (let i = 0; i < n; i += 1) {
    print(i)
}
"#,
        );
        test.result(result).assert_lint("prefer-range-literal");
    }

    #[test]
    fn test_allows_range_literal() {
        let test = TestProgram::for_rule_without_builtins(PreferRangeLiteral);
        let result = test.lint_ast(
            "test.ds",
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
        let test = TestProgram::for_rule_without_builtins(PreferRangeLiteral);
        let result = test.lint_ast(
            "test.ds",
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
        let test = TestProgram::for_rule_without_builtins(PreferRangeLiteral);
        // non-standard increment shouldn't trigger
        let result = test.lint_ast(
            "test.ds",
            r#"
for (let i = 0; i < 10; i += 2) {
    print(i)
}
"#,
        );
        test.result(result).assert_no_lint("prefer-range-literal");
    }
}
