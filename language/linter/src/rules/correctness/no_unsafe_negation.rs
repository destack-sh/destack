use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow negation of the left operand of relational operators.
    ///
    /// Expressions like `!a in b` are often a mistake, as they parse as `(!a) in b`
    /// instead of the intended `!(a in b)`. The same applies to `instanceof`.
    #[lint(
        id = "no-unsafe-negation",
        code = "LC012",
        category = Correctness,
        level = Ast
    )]
    pub NoUnsafeNegation,
    "Disallow negation of left operand in relational operators"
}

impl LintRule for NoUnsafeNegation {
    fn meta(&self) -> &'static crate::LintMeta {
        NoUnsafeNegation::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            // check for Binary expressions with in/instanceof
            if let ast::Expression::Binary { left, operator, .. } = ctx.tree.get(node_id) {
                // only check relational operators
                if !matches!(
                    operator,
                    ast::BinaryOperator::In | ast::BinaryOperator::InstanceOf
                ) {
                    continue;
                }

                // check if left side is a logical not
                if is_logical_not(ctx, *left) {
                    let operator_name = match operator {
                        ast::BinaryOperator::In => "in",
                        ast::BinaryOperator::InstanceOf => "instanceof",
                        _ => "operator",
                    };
                    ctx.report(
                        LintDiagnostic::new(
                            NO_UNSAFE_NEGATION.id,
                            NO_UNSAFE_NEGATION.code,
                            NO_UNSAFE_NEGATION.category,
                            severity,
                            format!("negation of left operand of `{operator_name}`"),
                            ctx.module.file_id,
                            ctx.tree.get_span(node_id),
                        )
                        .with_label(format!(
                            "this parses as `(!a) {operator_name} b`, use `!(a {operator_name} b)` instead"
                        )),
                    );
                }
            }
        }
    }
}

/// Check if an expression is a logical not operation.
fn is_logical_not(
    ctx: &LintModuleAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> bool {
    let expression = ctx.tree.get(expression_id);
    match expression {
        ast::Expression::Unary { operator, .. } => *operator == ast::UnaryOperator::Not,
        ast::Expression::Parenthesized { expression } => is_logical_not(ctx, *expression),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_negation_in_in() {
        let test = TestProgram::for_rule(NoUnsafeNegation);
        let result = test.lint_ast(
            "test.ds",
            r#"
let obj = { a: 1 };
let key = "a";
!key in obj;
"#,
        );
        test.result(result).assert_lint("no-unsafe-negation");
    }

    #[test]
    fn test_detects_negation_in_instanceof() {
        let test = TestProgram::for_rule(NoUnsafeNegation);
        let result = test.lint_ast(
            "test.ds",
            r#"
class Foo {}
let x = new Foo();
!x instanceof Foo;
"#,
        );
        test.result(result).assert_lint("no-unsafe-negation");
    }

    #[test]
    fn test_allows_negation_outside_parens() {
        let test = TestProgram::for_rule(NoUnsafeNegation);
        let result = test.lint_ast(
            "test.ds",
            r#"
let obj = { a: 1 };
let key = "a";
!(key in obj);
"#,
        );
        // ! is outside the parens, so left operand of `in` is `key`, not `!key`
        test.result(result).assert_no_lint("no-unsafe-negation");
    }

    #[test]
    fn test_allows_non_negated_in() {
        let test = TestProgram::for_rule(NoUnsafeNegation);
        let result = test.lint_ast(
            "test.ds",
            r#"
let obj = { a: 1 };
let key = "a";
key in obj;
"#,
        );
        test.result(result).assert_no_lint("no-unsafe-negation");
    }

    #[test]
    fn test_allows_non_negated_instanceof() {
        let test = TestProgram::for_rule(NoUnsafeNegation);
        let result = test.lint_ast(
            "test.ds",
            r#"
class Foo {}
let x = new Foo();
x instanceof Foo;
"#,
        );
        test.result(result).assert_no_lint("no-unsafe-negation");
    }
}
