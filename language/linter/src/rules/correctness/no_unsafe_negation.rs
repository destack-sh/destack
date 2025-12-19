use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintFix, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow negation of the left operand of relational operators.
    ///
    /// Expressions like `!a in b` are often a mistake, as they parse as `(!a) in b`
    /// instead of the intended `!(a in b)`. The same applies to `instanceof`.
    #[lint(
        id = "no-unsafe-negation",
        code = "LC012",
        category = Correctness,
        level = Ast,
        fixable = Always,
        recommended = Always,
        stability = Stable
    )]
    pub NoUnsafeNegation,
    "Disallow negation of left operand in relational operators"
}

impl LintRule for NoUnsafeNegation {
    fn meta(&self) -> &'static crate::LintMeta {
        NoUnsafeNegation::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            // check for Binary expressions with in/instanceof
            if let ast::Expression::Binary {
                left,
                operator,
                right,
            } = ctx.tree.get(node_id)
            {
                // only check relational operators
                if !matches!(
                    operator,
                    ast::BinaryOperator::In | ast::BinaryOperator::InstanceOf
                ) {
                    continue;
                }

                // check if left side is a logical not and get the inner expression
                if let Some(inner_id) = get_negated_inner(ctx, *left) {
                    let severity = ctx.get_effective_severity(meta, node_id);
                    if !severity.is_enabled() {
                        continue;
                    }

                    let operator_name = match operator {
                        ast::BinaryOperator::In => "in",
                        ast::BinaryOperator::InstanceOf => "instanceof",
                        _ => "operator",
                    };

                    // make fix: convert `!a in b` to `!(a in b)`
                    let expression_span = ctx.tree.get_span(node_id);
                    let inner_span = ctx.tree.get_span(inner_id);
                    let inner_text = ctx.get_span_text(inner_span);
                    let right_span = ctx.tree.get_span(*right);
                    let right_text = ctx.get_span_text(right_span);
                    let replacement = format!("!({inner_text} {operator_name} {right_text})");
                    let edits = ctx
                        .edit_builder()
                        .replace(expression_span, replacement)
                        .into_edits();
                    let fix = LintFix::safe("Wrap in parentheses").with_edits(edits);

                    ctx.report(
                        LintDiagnostic::new(
                            NO_UNSAFE_NEGATION.id,
                            NO_UNSAFE_NEGATION.code,
                            NO_UNSAFE_NEGATION.category,
                            severity,
                            format!("negation of left operand of `{operator_name}`"),
                            ctx.module.file_id,
                            expression_span,
                        )
                        .with_label(format!(
                            "this parses as `(!a) {operator_name} b`, use `!(a {operator_name} b)` instead"
                        ))
                        .with_fix(fix),
                    );
                }
            }
        }
    }
}

/// Get the inner expression if this is a logical not operation.
/// Returns the inner expression ID, or None if not a negation.
fn get_negated_inner(
    ctx: &LintModuleAstContext<'_>,
    expression_id: ast::LocalNodeId<ast::Expression>,
) -> Option<ast::LocalNodeId<ast::Expression>> {
    let expression = ctx.tree.get(expression_id);
    match expression {
        ast::Expression::Unary { operator, right } if *operator == ast::UnaryOperator::Not => {
            Some(*right)
        }
        ast::Expression::Parenthesized { expression } => get_negated_inner(ctx, *expression),
        _ => None,
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

    #[test]
    fn test_fix_in_operator() {
        let test = TestProgram::for_rule(NoUnsafeNegation);
        let result = test.lint_ast(
            "test.ds",
            r#"
let obj = { a: 1 }
let result = !key in obj
"#,
        );
        test.result(result)
            .assert_lint("no-unsafe-negation")
            .assert_safe_fixed(
                r#"
let obj = { a: 1 };
let result = !(key in obj);
"#,
            );
    }

    #[test]
    fn test_fix_instanceof_operator() {
        let test = TestProgram::for_rule(NoUnsafeNegation);
        let result = test.lint_ast(
            "test.ds",
            r#"
class Foo {}
let x = new Foo()
let result = !x instanceof Foo
"#,
        );
        test.result(result)
            .assert_lint("no-unsafe-negation")
            .assert_safe_fixed(
                r#"
class Foo { }
let x = new Foo();
let result = !(x instanceof Foo);
"#,
            );
    }
}
