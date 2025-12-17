use destack_ast::{self as ast, ScalarLiteral};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Prefer `loop` over `while(true)` or `for(;;)`.
    ///
    /// Use the explicit `loop` keyword for infinite loops instead of
    /// `while(true)`, `while(1)`, or `for(;;)` patterns.
    #[lint(
        id = "prefer-loop",
        code = "LY031",
        category = Style,
        level = Ast
    )]
    pub PreferLoop,
    "Prefer explicit `loop` for infinite loops"
}

impl LintRule for PreferLoop {
    fn meta(&self) -> &'static crate::LintMeta {
        PreferLoop::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);

            let is_infinite_loop = match expression {
                // while (true) or while (1)
                ast::Expression::While { condition, .. } => is_always_true(ctx.tree, *condition),
                // for (;;)
                ast::Expression::For {
                    initialization,
                    condition,
                    increment,
                    ..
                } => initialization.is_none() && condition.is_none() && increment.is_none(),
                _ => false,
            };

            if !is_infinite_loop {
                continue;
            }

            let span = ctx.tree.get_span(node_id);
            ctx.report(
                LintDiagnostic::new(
                    PREFER_LOOP.id,
                    PREFER_LOOP.code,
                    PREFER_LOOP.category,
                    severity,
                    "use `loop` instead of infinite loop pattern",
                    ctx.module.file_id,
                    span,
                )
                .with_label("replace with `loop { ... }`"),
            );
        }
    }
}

/// Check if an expression is always truthy (true or 1).
fn is_always_true(tree: &ast::NodeTree, expr_id: ast::LocalNodeId<ast::Expression>) -> bool {
    let expr = tree.get(expr_id);

    // unwrap parentheses
    if let ast::Expression::Parenthesized { expression } = expr {
        return is_always_true(tree, *expression);
    }

    match expr {
        ast::Expression::ScalarLiteral(ScalarLiteral::Boolean(true)) => true,
        ast::Expression::ScalarLiteral(ScalarLiteral::Integer(1)) => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_while_true() {
        let test = TestProgram::for_rule(PreferLoop);
        let result = test.lint_ast(
            "test.ds",
            r#"
while (true) {
    break
}
"#,
        );
        test.result(result).assert_lint("prefer-loop");
    }

    #[test]
    fn test_detects_while_one() {
        let test = TestProgram::for_rule(PreferLoop);
        let result = test.lint_ast(
            "test.ds",
            r#"
while (1) {
    break
}
"#,
        );
        test.result(result).assert_lint("prefer-loop");
    }

    #[test]
    fn test_detects_for_empty() {
        let test = TestProgram::for_rule(PreferLoop);
        let result = test.lint_ast(
            "test.ds",
            r#"
for (;;) {
    break
}
"#,
        );
        test.result(result).assert_lint("prefer-loop");
    }

    #[test]
    fn test_allows_loop() {
        let test = TestProgram::for_rule(PreferLoop);
        let result = test.lint_ast(
            "test.ds",
            r#"
loop {
    break
}
"#,
        );
        test.result(result).assert_no_lint("prefer-loop");
    }

    #[test]
    fn test_allows_while_condition() {
        let test = TestProgram::for_rule(PreferLoop);
        let result = test.lint_ast(
            "test.ds",
            r#"
while (x > 0) {
    x = x - 1
}
"#,
        );
        test.result(result).assert_no_lint("prefer-loop");
    }

    #[test]
    fn test_allows_for_with_condition() {
        let test = TestProgram::for_rule(PreferLoop);
        let result = test.lint_ast(
            "test.ds",
            r#"
for (let i = 0; i < 10; i++) {
    console.log(i)
}
"#,
        );
        test.result(result).assert_no_lint("prefer-loop");
    }

    #[test]
    fn test_allows_for_with_partial() {
        let test = TestProgram::for_rule(PreferLoop);
        // for loop with just initialization is not infinite
        let result = test.lint_ast(
            "test.ds",
            r#"
for (let i = 0;;) {
    if (i > 10) break
    i++
}
"#,
        );
        test.result(result).assert_no_lint("prefer-loop");
    }
}
