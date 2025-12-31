use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow variable or function declarations in nested blocks.
    ///
    /// Function and variable declarations in nested blocks can be confusing
    /// and may not behave as expected due to hoisting. Declare them at the
    /// function or module level instead.
    #[lint(
        id = "no-inner-declarations",
        code = "LU025",
        category = Suspicious,
        level = Ast,
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoInnerDeclarations,
    "Disallow declarations in nested blocks"
}

impl LintRule for NoInnerDeclarations {
    fn meta(&self) -> &'static crate::LintMeta {
        NoInnerDeclarations::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        // find function declarations inside control flow blocks
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);

            // check control flow constructs that have blocks
            let block_id = match expression {
                ast::Expression::If {
                    then_expression, ..
                } => {
                    let then_expr = ctx.tree.get(*then_expression);
                    if let ast::Expression::Block(block_id) = then_expr {
                        Some(*block_id)
                    } else {
                        None
                    }
                }
                ast::Expression::While { body, .. } => Some(*body),
                ast::Expression::For { body, .. } => Some(*body),
                ast::Expression::ForEach { body, .. } => Some(*body),
                ast::Expression::Loop { body } => Some(*body),
                _ => None,
            };
            let Some(block_id) = block_id else {
                continue;
            };

            // check children of the block
            let block = ctx.tree.get(block_id);
            for expr_id in &block.expressions {
                let expr = ctx.tree.get(*expr_id);

                // check for function declarations
                if let ast::Expression::Declaration(decl_id) = expr {
                    let declaration = ctx.tree.get(*decl_id);
                    if matches!(declaration, ast::Declaration::Function { .. }) {
                        let severity = ctx.get_effective_severity(meta, *expr_id);
                        if !severity.is_enabled() {
                            continue;
                        }

                        ctx.report(
                            LintDiagnostic::new(
                                NO_INNER_DECLARATIONS.id,
                                NO_INNER_DECLARATIONS.code,
                                NO_INNER_DECLARATIONS.category,
                                severity,
                                "function declaration in nested block",
                                ctx.module.file_id,
                                ctx.tree.get_span(*decl_id),
                            )
                            .with_label("move declaration to function or module level"),
                        );
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_function_in_if() {
        let test = TestProgram::for_rule_without_builtins(NoInnerDeclarations);
        let result = test.lint_ast(
            "test.ts",
            r#"
if (true) {
    function foo() {}
}
"#,
        );
        test.result(result).assert_lint("no-inner-declarations");
    }

    #[test]
    fn test_detects_function_in_while() {
        let test = TestProgram::for_rule_without_builtins(NoInnerDeclarations);
        let result = test.lint_ast(
            "test.ts",
            r#"
while (true) {
    function bar() {}
}
"#,
        );
        test.result(result).assert_lint("no-inner-declarations");
    }

    #[test]
    fn test_allows_top_level_function() {
        let test = TestProgram::for_rule_without_builtins(NoInnerDeclarations);
        let result = test.lint_ast("test.ts", "function foo() {}");
        test.result(result).assert_no_lint("no-inner-declarations");
    }

    #[test]
    fn test_allows_function_inside_function() {
        let test = TestProgram::for_rule_without_builtins(NoInnerDeclarations);
        let result = test.lint_ast(
            "test.ts",
            r#"
function outer() {
    function inner() {}
}
"#,
        );
        test.result(result).assert_no_lint("no-inner-declarations");
    }
}
