use destack_ast as ast;
use destack_source::Span;
use destack_workspace::LintSeverity;

use crate::rules::common::{declaration_at_allowed_root, declaration_expression};
use crate::{LintAstContext, LintDiagnostic, LintMeta, LintRule, declare_lint};

declare_lint! {
    /// Disallow function and `var` declarations in nested blocks.
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
    "Disallow function and `var` declarations in nested blocks"
}

impl LintRule for NoInnerDeclarations {
    fn meta(&self) -> &'static LintMeta {
        NoInnerDeclarations::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        // nested functions
        for declaration_id in ctx.tree.iter_nodes::<ast::Declaration>() {
            let declaration = ctx.tree.get(declaration_id);
            if !matches!(declaration, ast::Declaration::Function(_)) {
                continue;
            }

            // require one declaration wrapper expression
            let Some(declaration_expression_id) =
                declaration_expression(ctx.tree, ctx.parents, declaration_id)
            else {
                continue;
            };

            // allow declaration roots at module, function, and static block boundaries
            if declaration_at_allowed_root(ctx.tree, ctx.parents, declaration_expression_id) {
                continue;
            }

            report_nested_declaration(
                ctx,
                meta,
                declaration_id,
                ctx.tree.get_span(declaration_id),
                "function declaration in nested block",
            );
        }

        // nested vars
        if !ctx
            .options
            .correctness
            .no_inner_declarations_check_var_declarations
        {
            return;
        }
        for expression_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(expression_id);
            let ast::Expression::Let { mutability, .. } = expression else {
                continue;
            };
            if *mutability != ast::Mutability::Mutable {
                continue;
            }

            // allow declaration roots at module, function, and static block boundaries
            if declaration_at_allowed_root(ctx.tree, ctx.parents, expression_id) {
                continue;
            }

            report_nested_declaration(
                ctx,
                meta,
                expression_id,
                ctx.tree.get_span(expression_id),
                "var declaration in nested block",
            );
        }
    }
}

/// Report one nested inner declaration diagnostic when the rule is enabled.
fn report_nested_declaration<T: ast::Node>(
    ctx: &mut LintAstContext<'_>,
    meta: &LintMeta,
    node_id: ast::LocalNodeId<T>,
    span: Span,
    message: &str,
) {
    let severity = ctx.get_effective_severity(meta, node_id);
    if !severity.is_enabled() {
        return;
    }

    ctx.report(
        LintDiagnostic::new(
            NO_INNER_DECLARATIONS.id,
            NO_INNER_DECLARATIONS.code,
            NO_INNER_DECLARATIONS.category,
            severity,
            message,
            ctx.module.file_id,
            span,
        )
        .with_label("move declaration to a module, function, or static block root"),
    );
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

    #[test]
    fn test_detects_var_in_if() {
        let test = TestProgram::for_rule_without_prelude(NoInnerDeclarations);
        let result = test.lint_ast(
            "no_inner_declarations/test_detects_var_in_if.ts",
            r#"
if (true) {
    var foo = 1
}
"#,
        );
        test.result(result).assert_lint("no-inner-declarations");
    }

    #[test]
    fn test_allows_top_level_var() {
        let test = TestProgram::for_rule_without_prelude(NoInnerDeclarations);
        let result = test.lint_ast(
            "no_inner_declarations/test_allows_top_level_var.ts",
            "var foo = 1",
        );
        test.result(result).assert_no_lint("no-inner-declarations");
    }

    #[test]
    fn test_allows_var_at_function_root() {
        let test = TestProgram::for_rule_without_prelude(NoInnerDeclarations);
        let result = test.lint_ast(
            "no_inner_declarations/test_allows_var_at_function_root.ts",
            r#"
function outer() {
    var foo = 1
}
"#,
        );
        test.result(result).assert_no_lint("no-inner-declarations");
    }

    #[test]
    fn test_detects_var_in_nested_block_inside_function() {
        let test = TestProgram::for_rule_without_prelude(NoInnerDeclarations);
        let result = test.lint_ast(
            "no_inner_declarations/test_detects_var_in_nested_block_inside_function.ts",
            r#"
function outer() {
    {
        var foo = 1
    }
}
"#,
        );
        test.result(result).assert_lint("no-inner-declarations");
    }

    #[test]
    fn test_allows_nested_var_when_option_disabled() {
        let test =
            TestProgram::for_rule_without_prelude(NoInnerDeclarations).with_options(|options| {
                options
                    .correctness
                    .no_inner_declarations_check_var_declarations = false;
            });
        let result = test.lint_ast(
            "no_inner_declarations/test_allows_nested_var_when_option_disabled.ts",
            r#"
if (true) {
    var foo = 1
}
"#,
        );
        test.result(result).assert_no_lint("no-inner-declarations");
    }
}
