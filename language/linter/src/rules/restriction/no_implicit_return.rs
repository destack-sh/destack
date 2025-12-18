use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Require explicit return statements.
    ///
    /// Implicit returns (expression bodies without `return`) can be confusing
    /// in larger functions. Use explicit `return` for clarity.
    #[lint(
        id = "no-implicit-return",
        code = "LR017",
        category = Restriction,
        level = Ast,
        fixable = No,
        recommended = Off,
        stability = Stable
    )]
    pub NoImplicitReturn,
    "Require explicit return statements"
}

impl LintRule for NoImplicitReturn {
    fn meta(&self) -> &'static crate::LintMeta {
        NoImplicitReturn::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        for node_id in ctx.tree.iter_nodes::<ast::Declaration>() {
            let declaration = ctx.tree.get(node_id);
            let ast::Declaration::Function {
                body: Some(body_id),
                ..
            } = declaration
            else {
                continue;
            };

            let body = ctx.tree.get(*body_id);

            // flag functions with expression bodies (implicit return)
            if !matches!(body, ast::Expression::Block(_)) {
                ctx.report(
                    LintDiagnostic::new(
                        NO_IMPLICIT_RETURN.id,
                        NO_IMPLICIT_RETURN.code,
                        NO_IMPLICIT_RETURN.category,
                        severity,
                        "implicit return in function",
                        ctx.module.file_id,
                        ctx.tree.get_span(*body_id),
                    )
                    .with_label("use explicit `return` statement"),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_arrow_expression_body() {
        let test = TestProgram::for_rule(NoImplicitReturn);
        let result = test.lint_ast("test.ts", "const foo = () => 1;");
        test.result(result).assert_lint("no-implicit-return");
    }

    #[test]
    fn test_detects_arrow_expression_body_complex() {
        let test = TestProgram::for_rule(NoImplicitReturn);
        let result = test.lint_ast("test.ts", "const add = (a: number, b: number) => a + b;");
        test.result(result).assert_lint("no-implicit-return");
    }

    #[test]
    fn test_allows_arrow_block_body() {
        let test = TestProgram::for_rule(NoImplicitReturn);
        let result = test.lint_ast("test.ts", "const foo = () => { return 1; };");
        test.result(result).assert_no_lint("no-implicit-return");
    }

    #[test]
    fn test_allows_function_with_block() {
        let test = TestProgram::for_rule(NoImplicitReturn);
        let result = test.lint_ast("test.ts", "function foo() { return 1; }");
        test.result(result).assert_no_lint("no-implicit-return");
    }

    #[test]
    fn test_allows_void_function() {
        let test = TestProgram::for_rule(NoImplicitReturn);
        let result = test.lint_ast("test.ts", "function foo() { console.log('hi'); }");
        test.result(result).assert_no_lint("no-implicit-return");
    }
}
