use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::rules::common::{declaration_at_allowed_root, declaration_expression};
use crate::{LintDiagnostic, LintMeta, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow function declarations in nested blocks.
    ///
    /// Function and variable declarations in nested blocks can be confusing
    /// and may not behave as expected due to hoisting. Declare them at the
    /// function or module level instead.
    #[lint(
        id = "no-inner-declarations",
        code = "LU020",
        category = Suspicious,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoInnerDeclarations,
    "Disallow function declarations in nested blocks"
}

impl LintRule for NoInnerDeclarations {
    fn meta(&self) -> &'static LintMeta {
        NoInnerDeclarations::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        // inspect declarations for nested function definitions
        for declaration_id in ctx.tree.iter_nodes::<ast::Declaration>() {
            let declaration = ctx.tree.get(declaration_id);
            if !matches!(declaration, ast::Declaration::Function { .. }) {
                continue;
            }

            // require one declaration wrapper expression
            let Some(declaration_expression_id) =
                declaration_expression(ctx.tree, &ctx.parents, declaration_id)
            else {
                continue;
            };

            // allow declaration roots at module, function, and static block boundaries
            if declaration_at_allowed_root(ctx.tree, &ctx.parents, declaration_expression_id) {
                continue;
            }

            // skip disabled diagnostics
            let severity = ctx.get_effective_severity(meta, declaration_id);
            if !severity.is_enabled() {
                continue;
            }

            // report one nested declaration diagnostic
            ctx.report(
                LintDiagnostic::new(
                    NO_INNER_DECLARATIONS.id,
                    NO_INNER_DECLARATIONS.code,
                    NO_INNER_DECLARATIONS.category,
                    severity,
                    "function declaration in nested block",
                    ctx.module.file_id,
                    ctx.tree.get_span(declaration_id),
                )
                .with_label("move declaration to a module, function, or static block root"),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_function_in_if() {
        let test = TestProgram::for_rule_without_prelude(NoInnerDeclarations);
        let result = test.lint_ast(
            "no_inner_declarations/test_detects_function_in_if.ts",
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
        let test = TestProgram::for_rule_without_prelude(NoInnerDeclarations);
        let result = test.lint_ast(
            "no_inner_declarations/test_detects_function_in_while.ts",
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
        let test = TestProgram::for_rule_without_prelude(NoInnerDeclarations);
        let result = test.lint_ast(
            "no_inner_declarations/test_allows_top_level_function.ts",
            "function foo() {}",
        );
        test.result(result).assert_no_lint("no-inner-declarations");
    }

    #[test]
    fn test_allows_function_inside_function() {
        let test = TestProgram::for_rule_without_prelude(NoInnerDeclarations);
        let result = test.lint_ast(
            "no_inner_declarations/test_allows_function_inside_function.ts",
            r#"
function outer() {
    function inner() {}
}
"#,
        );
        test.result(result).assert_no_lint("no-inner-declarations");
    }

    #[test]
    fn test_detects_function_in_nested_block_inside_function() {
        let test = TestProgram::for_rule_without_prelude(NoInnerDeclarations);
        let result = test.lint_ast(
            "no_inner_declarations/test_detects_function_in_nested_block_inside_function.ts",
            r#"
function outer() {
    {
        function inner() {}
    }
}
"#,
        );
        test.result(result).assert_lint("no-inner-declarations");
    }

    #[test]
    fn test_allows_function_in_static_block_root() {
        let test = TestProgram::for_rule_without_prelude(NoInnerDeclarations);
        let result = test.lint_ast(
            "no_inner_declarations/test_allows_function_in_static_block_root.ts",
            r#"
class Foo {
    static {
        function build() {}
    }
}
"#,
        );
        test.result(result).assert_no_lint("no-inner-declarations");
    }
}
