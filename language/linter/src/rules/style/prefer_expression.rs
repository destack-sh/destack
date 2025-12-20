use destack_ast::{
    AssignOperator, Block, Declarator, Expression, IfKind, LocalNodeId, NodeTree, Pattern,
};
use destack_base::StringId;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Prefer expression-based if over statement-based pattern.
    ///
    /// Detects patterns where a variable is declared without initialization
    /// and then immediately assigned in both branches of an if statement.
    /// Such patterns should use expression-based constructs instead.
    #[lint(
        id = "prefer-expression",
        code = "LY032",
        category = Style,
        level = Ast,
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferExpression,
    "Prefer expression-based if over statement pattern"
}

impl LintRule for PreferExpression {
    fn meta(&self) -> &'static crate::LintMeta {
        PreferExpression::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for root_id in ctx.roots.iter() {
            check_expression(ctx, meta, ctx.tree, *root_id);
        }
    }
}

/// Recursively check expressions for the pattern.
fn check_expression(
    ctx: &mut LintModuleAstContext<'_>,
    meta: &'static crate::LintMeta,
    tree: &NodeTree,
    expr_id: LocalNodeId<Expression>,
) {
    let expression = tree.get(expr_id);

    // check blocks for the pattern
    if let Expression::Block(block_id) = expression {
        check_block(ctx, meta, tree, *block_id);
    }

    // recursively check nested expressions
    match expression {
        Expression::Block(block_id) => {
            let block = tree.get(*block_id);
            for &child_id in &block.expressions {
                check_expression(ctx, meta, tree, child_id);
            }
        }
        Expression::If {
            then_expression,
            else_expression,
            ..
        } => {
            check_expression(ctx, meta, tree, *then_expression);
            if let Some(else_id) = else_expression {
                check_expression(ctx, meta, tree, *else_id);
            }
        }
        Expression::Declaration(declaration_id) => {
            let declaration = tree.get(*declaration_id);
            if let destack_ast::Declaration::Function { body, .. } = declaration
                && let Some(body_id) = body
            {
                check_expression(ctx, meta, tree, *body_id);
            }
        }
        _ => {}
    }
}

/// Check a block for the uninitialized-let-then-if pattern.
fn check_block(
    ctx: &mut LintModuleAstContext<'_>,
    meta: &'static crate::LintMeta,
    tree: &NodeTree,
    block_id: LocalNodeId<Block>,
) {
    let block = tree.get(block_id);
    let expressions = &block.expressions;

    // need at least 2 consecutive expressions
    for window_index in 0..expressions.len().saturating_sub(1) {
        let let_expr_id = expressions[window_index];
        let if_expr_id = expressions[window_index + 1];

        // check if we have a let followed by an if
        if let Some(variable_name) = get_uninitialized_let(tree, let_expr_id)
            && check_if_assigns_to_variable(tree, if_expr_id, variable_name)
        {
            let severity = ctx.get_effective_severity(meta, let_expr_id);
            if !severity.is_enabled() {
                continue;
            }

            ctx.report(
                LintDiagnostic::new(
                    PREFER_EXPRESSION.id,
                    PREFER_EXPRESSION.code,
                    PREFER_EXPRESSION.category,
                    severity,
                    "prefer expression-based if over statement pattern",
                    ctx.module.file_id,
                    tree.get_span(let_expr_id),
                )
                .with_label("use `const x = if (condition) { a } else { b }` instead"),
            );
        }
    }
}

/// Check if an expression is a let binding with no initializer, return the variable name.
fn get_uninitialized_let(tree: &NodeTree, expr_id: LocalNodeId<Expression>) -> Option<StringId> {
    let expression = tree.get(expr_id);

    // unwrap Statement wrapper if present
    let expression = if let Expression::Statement(inner_id) = expression {
        tree.get(*inner_id)
    } else {
        expression
    };

    let Expression::Let { declarators, .. } = expression else {
        return None;
    };

    // only handle single declarator for now
    if declarators.len() != 1 {
        return None;
    }

    let declarator_id = declarators[0];
    let declarator: &Declarator = tree.get(declarator_id);

    // must have no initializer
    if declarator.value.is_some() {
        return None;
    }

    // extract the simple variable name from the pattern
    get_simple_identifier_from_pattern(tree, declarator.pattern)
}

/// Extract a simple identifier name from a pattern (if it's a simple binding).
fn get_simple_identifier_from_pattern(
    tree: &NodeTree,
    pattern_id: LocalNodeId<Pattern>,
) -> Option<StringId> {
    let pattern = tree.get(pattern_id);

    match pattern {
        Pattern::Binding {
            name,
            pattern: None,
            ..
        } => Some(*name),
        _ => None,
    }
}

/// Check if an if expression assigns to the given variable in both branches.
fn check_if_assigns_to_variable(
    tree: &NodeTree,
    expr_id: LocalNodeId<Expression>,
    variable_name: StringId,
) -> bool {
    let expression = tree.get(expr_id);

    // unwrap Statement wrapper if present
    let expression = if let Expression::Statement(inner_id) = expression {
        tree.get(*inner_id)
    } else {
        expression
    };

    let Expression::If {
        kind: IfKind::If,
        then_expression,
        else_expression: Some(else_expression),
        ..
    } = expression
    else {
        return false;
    };

    // both branches must be blocks that assign to the variable
    block_assigns_to_variable(tree, *then_expression, variable_name)
        && block_assigns_to_variable(tree, *else_expression, variable_name)
}

/// Check if a block (or expression) assigns to the given variable.
fn block_assigns_to_variable(
    tree: &NodeTree,
    expr_id: LocalNodeId<Expression>,
    variable_name: StringId,
) -> bool {
    let expression = tree.get(expr_id);

    match expression {
        Expression::Block(block_id) => {
            let block = tree.get(*block_id);
            // check if any expression in the block assigns to the variable
            // for simplicity, we check if the first expression does
            if let Some(&first_expr_id) = block.expressions.first() {
                assigns_to_variable(tree, first_expr_id, variable_name)
            } else {
                false
            }
        }
        _ => assigns_to_variable(tree, expr_id, variable_name),
    }
}

/// Check if an expression is an assignment to the given variable.
fn assigns_to_variable(
    tree: &NodeTree,
    expr_id: LocalNodeId<Expression>,
    variable_name: StringId,
) -> bool {
    let expression = tree.get(expr_id);

    // unwrap Statement wrapper if present
    let expression = if let Expression::Statement(inner_id) = expression {
        tree.get(*inner_id)
    } else {
        expression
    };

    let Expression::Assign {
        left,
        operator: AssignOperator::Assign,
        ..
    } = expression
    else {
        return false;
    };

    // check if the left side is a simple identifier reference
    let left_expr = tree.get(*left);
    if let Expression::Path { path, .. } = left_expr {
        // simple identifier is a path with a single segment
        path.segments.len() == 1 && path.segments[0] == variable_name
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_uninitialized_let_with_if_assignment() {
        let test = TestProgram::for_rule_without_builtins(PreferExpression);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(condition: boolean) {
    let x;
    if (condition) {
        x = 1;
    } else {
        x = 2;
    }
}
"#,
        );
        test.result(result).assert_lint("prefer-expression");
    }

    #[test]
    fn test_allows_initialized_let() {
        let test = TestProgram::for_rule_without_builtins(PreferExpression);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(condition: boolean) {
    let x = 0;
    if (condition) {
        x = 1;
    } else {
        x = 2;
    }
}
"#,
        );
        test.result(result).assert_no_lint("prefer-expression");
    }

    #[test]
    fn test_allows_expression_based_if() {
        let test = TestProgram::for_rule_without_builtins(PreferExpression);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(condition: boolean) {
    const x = if (condition) { 1 } else { 2 };
}
"#,
        );
        test.result(result).assert_no_lint("prefer-expression");
    }

    #[test]
    fn test_allows_if_without_else() {
        let test = TestProgram::for_rule_without_builtins(PreferExpression);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(condition: boolean) {
    let x;
    if (condition) {
        x = 1;
    }
}
"#,
        );
        test.result(result).assert_no_lint("prefer-expression");
    }

    #[test]
    fn test_allows_if_not_assigning_to_same_variable() {
        let test = TestProgram::for_rule_without_builtins(PreferExpression);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(condition: boolean) {
    let x;
    let y;
    if (condition) {
        x = 1;
    } else {
        y = 2;
    }
}
"#,
        );
        test.result(result).assert_no_lint("prefer-expression");
    }

    #[test]
    fn test_allows_non_adjacent_statements() {
        let test = TestProgram::for_rule_without_builtins(PreferExpression);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(condition: boolean) {
    let x;
    console.log("something");
    if (condition) {
        x = 1;
    } else {
        x = 2;
    }
}
"#,
        );
        test.result(result).assert_no_lint("prefer-expression");
    }
}
