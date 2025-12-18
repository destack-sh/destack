use destack_ast::{self as ast, Expression};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow expressions that have no effect.
    ///
    /// Expressions that are evaluated but never used are likely mistakes or
    /// leftover code. This rule flags expressions that don't produce side effects
    /// and whose values are discarded.
    ///
    /// Bad:
    /// ```
    /// x + 1;  // computed but not used
    /// "hello";  // string literal with no effect
    /// a && b;  // result discarded
    /// ```
    ///
    /// Good:
    /// ```
    /// let y = x + 1;  // value is used
    /// console.log("hello");  // has side effect
    /// if (a && b) { ... }  // used in condition
    /// ```
    #[lint(
        id = "no-unused-expressions",
        code = "LX011",
        category = Complexity,
        level = Ast
    )]
    pub NoUnusedExpressions,
    "Disallow expressions without effect"
}

impl LintRule for NoUnusedExpressions {
    fn meta(&self) -> &'static crate::LintMeta {
        NoUnusedExpressions::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        // check each root expression in the module
        for &root_id in ctx.roots {
            check_expression_statement(ctx, severity, root_id);
        }

        // also check expressions inside blocks
        for node_id in ctx.tree.iter_nodes::<ast::Block>() {
            let block = ctx.tree.get(node_id);
            for &child_id in &block.expressions {
                check_expression_statement(ctx, severity, child_id);
            }
        }
    }
}

/// Check if an expression statement is unused and report it.
fn check_expression_statement(
    ctx: &mut LintModuleAstContext<'_>,
    severity: LintSeverity,
    expr_id: ast::LocalNodeId<Expression>,
) {
    let expression = ctx.tree.get(expr_id);

    // skip expressions that have side effects or are useful
    if !has_side_effect(ctx, expression) {
        ctx.report(
            LintDiagnostic::new(
                NO_UNUSED_EXPRESSIONS.id,
                NO_UNUSED_EXPRESSIONS.code,
                NO_UNUSED_EXPRESSIONS.category,
                severity,
                "expression has no effect",
                ctx.module.file_id,
                ctx.tree.get_span(expr_id),
            )
            .with_label("this expression does nothing"),
        );
    }
}

/// Return whether an expression has side effects.
fn has_side_effect(ctx: &LintModuleAstContext<'_>, expression: &Expression) -> bool {
    match expression {
        // expressions with "side effects"
        Expression::Declaration(_)
        | Expression::Block(_)
        | Expression::Labelled { .. }
        | Expression::Let { .. }
        | Expression::Assign { .. }
        | Expression::Call { .. }
        | Expression::New { .. }
        | Expression::Delete { .. }
        | Expression::Return { .. }
        | Expression::Break { .. }
        | Expression::Continue { .. }
        | Expression::Throw { .. }
        | Expression::For { .. }
        | Expression::ForEach { .. }
        | Expression::While { .. }
        | Expression::Loop { .. }
        | Expression::If { .. }
        | Expression::Match { .. }
        | Expression::Await { .. }
        | Expression::Yield { .. }
        | Expression::Try { .. }
        | Expression::Export { .. }
        | Expression::Import { .. }
        | Expression::Debugger
        | Expression::Error
        | Expression::Stub => true,

        // statements: check inner expression
        Expression::Statement(inner_id) => {
            let inner = ctx.tree.get(*inner_id);
            has_side_effect(ctx, inner)
        }

        // parenthesized: check inner expression
        Expression::Parenthesized { expression: inner } => {
            let inner_expr = ctx.tree.get(*inner);
            has_side_effect(ctx, inner_expr)
        }

        // maybe/must propagation: check inner for side effect
        Expression::Maybe { left, .. } | Expression::Must { left, .. } => {
            let inner_expr = ctx.tree.get(*left);
            has_side_effect(ctx, inner_expr)
        }

        // expressions without side effects
        Expression::ScalarLiteral(_)
        | Expression::TypeLiteral(_)
        | Expression::TemplateExpression { .. }
        | Expression::TaggedTemplateExpression { .. }
        | Expression::RangeExpression { .. }
        | Expression::ArrayExpression { .. }
        | Expression::TupleExpression { .. }
        | Expression::ObjectExpression { .. }
        | Expression::TreeExpression { .. }
        | Expression::SequenceExpression { .. }
        | Expression::Binary { .. }
        | Expression::Unary { .. }
        | Expression::TypeUnary { .. }
        | Expression::TypeBinary { .. }
        | Expression::Path { .. }
        | Expression::Member { .. }
        | Expression::Index { .. }
        | Expression::ReferenceOf { .. }
        | Expression::ValueOf { .. } => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_unused_literal() {
        let test = TestProgram::for_rule(NoUnusedExpressions);
        let result = test.lint_ast(
            "test.ds", r#"
5
"#,
        );
        test.result(result).assert_lint("no-unused-expressions");
    }

    #[test]
    fn test_detects_unused_string() {
        let test = TestProgram::for_rule(NoUnusedExpressions);
        let result = test.lint_ast(
            "test.ds",
            r#"
"hello"
"#,
        );
        test.result(result).assert_lint("no-unused-expressions");
    }

    #[test]
    fn test_detects_unused_binary() {
        let test = TestProgram::for_rule(NoUnusedExpressions);
        let result = test.lint_ast(
            "test.ds",
            r#"
x + 1
"#,
        );
        test.result(result).assert_lint("no-unused-expressions");
    }

    #[test]
    fn test_detects_unused_identifier() {
        let test = TestProgram::for_rule(NoUnusedExpressions);
        let result = test.lint_ast(
            "test.ds", r#"
x
"#,
        );
        test.result(result).assert_lint("no-unused-expressions");
    }

    #[test]
    fn test_allows_function_call() {
        let test = TestProgram::for_rule(NoUnusedExpressions);
        let result = test.lint_ast(
            "test.ds",
            r#"
doSomething()
"#,
        );
        test.result(result).assert_no_lint("no-unused-expressions");
    }

    #[test]
    fn test_allows_assignment() {
        let test = TestProgram::for_rule(NoUnusedExpressions);
        let result = test.lint_ast(
            "test.ds",
            r#"
x = 5
"#,
        );
        test.result(result).assert_no_lint("no-unused-expressions");
    }

    #[test]
    fn test_allows_let_binding() {
        let test = TestProgram::for_rule(NoUnusedExpressions);
        let result = test.lint_ast(
            "test.ds",
            r#"
let x = 5
"#,
        );
        test.result(result).assert_no_lint("no-unused-expressions");
    }

    #[test]
    fn test_allows_return() {
        let test = TestProgram::for_rule(NoUnusedExpressions);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo() {
    return 5
}
"#,
        );
        test.result(result).assert_no_lint("no-unused-expressions");
    }
}
