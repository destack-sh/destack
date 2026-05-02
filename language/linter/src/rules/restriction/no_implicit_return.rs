use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintAstContext, LintFix, LintMeta, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Require explicit return statements.
    ///
    /// Implicit returns (expression bodies without `return`) can be confusing
    /// in larger functions. Use explicit `return` for clarity.
    #[lint(
        id = "no-implicit-return",
        code = "LR014",
        category = Restriction,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Always,
        recommended = Off,
        stability = Stable
    )]
    pub NoImplicitReturn,
    "Require explicit return statements"
}

impl LintRule for NoImplicitReturn {
    fn meta(&self) -> &'static LintMeta {
        NoImplicitReturn::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        // inspect candidate declarations
        for node_id in ctx.tree.iter_nodes::<ast::Declaration>() {
            let declaration = ctx.tree.get(node_id);
            let ast::Declaration::Function(declaration) = declaration else {
                continue;
            };
            let Some(body_id) = declaration.body else {
                continue;
            };

            // resolve body
            let body = ctx.tree.get(body_id);

            // flag functions with expression bodies (implicit return)
            if !matches!(body, ast::Expression::Block(_)) {
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }

                // resolve diagnostic span
                let body_span = ctx.tree.get_span(body_id);
                let mut diagnostic = LintReport::new(
                    NO_IMPLICIT_RETURN.id,
                    NO_IMPLICIT_RETURN.code,
                    NO_IMPLICIT_RETURN.category,
                    severity,
                    "implicit return in function",
                    body_span,
                )
                .label("use explicit `return` statement");

                // compute fixes only when requested by the runner
                if ctx.compute_fixes {
                    let body_text = ctx.get_span_text(body_span);
                    let replacement = format!("{{ return {body_text}; }}");
                    let edits = ctx
                        .edit_builder()
                        .replace(body_span, replacement)
                        .into_edits();
                    let fix = LintFix::safe("Wrap implicit return in an explicit block")
                        .with_edits(edits);
                    diagnostic = diagnostic.fix(fix);
                }

                ctx.report(diagnostic);
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
        let test = TestProgram::for_rule_without_prelude(NoImplicitReturn);
        let result = test.lint_ast(
            "no_implicit_return/test_detects_arrow_expression_body.ts",
            "const foo = () => 1;",
        );
        test.result(result)
            .assert_lint("no-implicit-return")
            .assert_has_fix("no-implicit-return");
    }

    #[test]
    fn test_fix_arrow_expression_body() {
        let test = TestProgram::for_rule_without_prelude(NoImplicitReturn);
        let result = test.lint_ast(
            "no_implicit_return/test_fix_arrow_expression_body.ts",
            "const foo = () => 1;",
        );
        test.result(result)
            .assert_lint("no-implicit-return")
            .assert_safe_fixed(
                r#"
const foo = () => {
    return 1;
};
"#,
            );
    }

    #[test]
    fn test_fix_arrow_expression_body_object_literal() {
        let test = TestProgram::for_rule_without_prelude(NoImplicitReturn);
        let result = test.lint_ast(
            "no_implicit_return/test_fix_arrow_expression_body_object_literal.ts",
            "const foo = () => ({ value: 1 });",
        );
        test.result(result)
            .assert_lint("no-implicit-return")
            .assert_safe_fixed(
                r#"
const foo = () => {
    return ({ value: 1 });
};
"#,
            );
    }

    #[test]
    fn test_fix_arrow_expression_body_parenthesized_expression() {
        let test = TestProgram::for_rule_without_prelude(NoImplicitReturn);
        let result = test.lint_ast(
            "no_implicit_return/test_fix_arrow_expression_body_parenthesized_expression.ts",
            "const foo = () => (value + 1);",
        );
        test.result(result)
            .assert_lint("no-implicit-return")
            .assert_has_fix("no-implicit-return")
            .assert_safe_fixed(
                r#"
const foo = () => {
    return (value + 1);
};
"#,
            );
    }

    #[test]
    fn test_fix_async_arrow_expression_body() {
        let test = TestProgram::for_rule_without_prelude(NoImplicitReturn);
        let result = test.lint_ast(
            "no_implicit_return/test_fix_async_arrow_expression_body.ts",
            "const foo = async () => await fetchValue();",
        );
        test.result(result)
            .assert_lint("no-implicit-return")
            .assert_has_fix("no-implicit-return")
            .assert_safe_fixed(
                r#"
const foo = async () => {
    return await fetchValue();
};
"#,
            );
    }

    #[test]
    fn test_detects_arrow_expression_body_complex() {
        let test = TestProgram::for_rule_without_prelude(NoImplicitReturn);
        let result = test.lint_ast(
            "no_implicit_return/test_detects_arrow_expression_body_complex.ts",
            "const add = (a: number, b: number) => a + b;",
        );
        test.result(result)
            .assert_lint("no-implicit-return")
            .assert_has_fix("no-implicit-return");
    }

    #[test]
    fn test_allows_arrow_block_body() {
        let test = TestProgram::for_rule_without_prelude(NoImplicitReturn);
        let result = test.lint_ast(
            "no_implicit_return/test_allows_arrow_block_body.ts",
            "const foo = () => { return 1; };",
        );
        test.result(result).assert_no_lint("no-implicit-return");
    }

    #[test]
    fn test_allows_function_with_block() {
        let test = TestProgram::for_rule_without_prelude(NoImplicitReturn);
        let result = test.lint_ast(
            "no_implicit_return/test_allows_function_with_block.ts",
            "function foo() { return 1; }",
        );
        test.result(result).assert_no_lint("no-implicit-return");
    }

    #[test]
    fn test_allows_void_function() {
        let test = TestProgram::for_rule_without_prelude(NoImplicitReturn);
        let result = test.lint_ast(
            "no_implicit_return/test_allows_void_function.ts",
            "function foo() { console.log('hi'); }",
        );
        test.result(result).assert_no_lint("no-implicit-return");
    }
}
