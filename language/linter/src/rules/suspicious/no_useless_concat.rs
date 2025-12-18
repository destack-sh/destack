use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow unnecessary concatenation of string literals.
    ///
    /// Concatenating two string literals like `"a" + "b"` is unnecessary since
    /// they could be written as a single literal `"ab"`.
    #[lint(
        id = "no-useless-concat",
        code = "LU003",
        category = Suspicious,
        level = Ast,
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoUselessConcat,
    "Disallow useless string concatenation"
}

impl LintRule for NoUselessConcat {
    fn meta(&self) -> &'static crate::LintMeta {
        NoUselessConcat::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let ast::Expression::Binary {
                left,
                operator,
                right,
            } = ctx.tree.get(node_id)
            else {
                continue;
            };

            // only check addition
            if *operator != ast::BinaryOperator::Add {
                continue;
            }

            // check if both sides are string literals
            if is_string_literal(ctx, *left) && is_string_literal(ctx, *right) {
                ctx.report(
                    LintDiagnostic::new(
                        NO_USELESS_CONCAT.id,
                        NO_USELESS_CONCAT.code,
                        NO_USELESS_CONCAT.category,
                        severity,
                        "useless string concatenation",
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label("combine these into a single string literal"),
                );
            }
        }
    }
}

/// Check if an expression is a string literal.
fn is_string_literal(
    ctx: &LintModuleAstContext<'_>,
    expr_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    let expr = ctx.tree.get(expr_id);
    match expr {
        ast::Expression::ScalarLiteral(ast::ScalarLiteral::String(_)) => true,
        ast::Expression::Parenthesized { expression } => is_string_literal(ctx, *expression),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_string_concat() {
        let test = TestProgram::for_rule(NoUselessConcat);
        let result = test.lint_ast(
            "test.ds",
            r#"
const x = "hello" + "world";
"#,
        );
        test.result(result).assert_lint("no-useless-concat");
    }

    #[test]
    fn test_detects_string_concat_empty() {
        let test = TestProgram::for_rule(NoUselessConcat);
        let result = test.lint_ast(
            "test.ds",
            r#"
const x = "" + "hello";
"#,
        );
        test.result(result).assert_lint("no-useless-concat");
    }

    #[test]
    fn test_allows_variable_concat() {
        let test = TestProgram::for_rule(NoUselessConcat);
        let result = test.lint_ast(
            "test.ds",
            r#"
const a = "hello";
const x = a + "world";
"#,
        );
        test.result(result).assert_no_lint("no-useless-concat");
    }

    #[test]
    fn test_allows_number_addition() {
        let test = TestProgram::for_rule(NoUselessConcat);
        let result = test.lint_ast(
            "test.ds",
            r#"
const x = 1 + 2;
"#,
        );
        test.result(result).assert_no_lint("no-useless-concat");
    }

    #[test]
    fn test_allows_mixed_concat() {
        let test = TestProgram::for_rule(NoUselessConcat);
        let result = test.lint_ast(
            "test.ds",
            r#"
const x = "hello" + 42;
"#,
        );
        test.result(result).assert_no_lint("no-useless-concat");
    }
}
