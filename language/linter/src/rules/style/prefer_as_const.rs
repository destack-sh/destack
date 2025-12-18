use destack_ast::{self as ast, Expression, ScalarLiteral, TypeBinaryOperator};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Prefer `as const` over literal type assertions.
    ///
    /// Use `x as const` instead of `x as "literal"` for better type inference.
    #[lint(
        id = "prefer-as-const",
        code = "LY027",
        category = Style,
        level = Ast,
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferAsConst,
    "Prefer `as const` over literal type assertions"
}

impl LintRule for PreferAsConst {
    fn meta(&self) -> &'static crate::LintMeta {
        PreferAsConst::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expr = ctx.tree.get(node_id);

            // look for type cast expressions
            let Expression::TypeBinary {
                left: _,
                operator: TypeBinaryOperator::Cast,
                right,
            } = expr
            else {
                continue;
            };

            let right_expr = ctx.tree.get(*right);

            // check if the cast type is a literal
            let is_literal_cast = matches!(
                right_expr,
                Expression::ScalarLiteral(
                    ScalarLiteral::String(_)
                        | ScalarLiteral::Integer(_)
                        | ScalarLiteral::Boolean(_)
                )
            );

            if is_literal_cast {
                ctx.report(
                    LintDiagnostic::new(
                        PREFER_AS_CONST.id,
                        PREFER_AS_CONST.code,
                        PREFER_AS_CONST.category,
                        severity,
                        "use `as const` instead of literal type assertion",
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label("prefer `as const`"),
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
    fn test_detects_string_literal_cast() {
        let test = TestProgram::for_rule(PreferAsConst);
        let result = test.lint_ast(
            "test.ds",
            r#"
const x = "hello" as "hello"
"#,
        );
        test.result(result).assert_lint("prefer-as-const");
    }

    #[test]
    fn test_detects_number_literal_cast() {
        let test = TestProgram::for_rule(PreferAsConst);
        let result = test.lint_ast(
            "test.ds",
            r#"
const x = 42 as 42
"#,
        );
        test.result(result).assert_lint("prefer-as-const");
    }

    #[test]
    fn test_allows_as_const() {
        let test = TestProgram::for_rule(PreferAsConst);
        let result = test.lint_ast(
            "test.ds",
            r#"
const x = "hello" as const
"#,
        );
        test.result(result).assert_no_lint("prefer-as-const");
    }

    #[test]
    fn test_allows_type_cast() {
        let test = TestProgram::for_rule(PreferAsConst);
        let result = test.lint_ast(
            "test.ds",
            r#"
const x = value as string
"#,
        );
        test.result(result).assert_no_lint("prefer-as-const");
    }

    #[test]
    fn test_allows_object_cast() {
        let test = TestProgram::for_rule(PreferAsConst);
        let result = test.lint_ast(
            "test.ds",
            r#"
const x = obj as { foo: string }
"#,
        );
        test.result(result).assert_no_lint("prefer-as-const");
    }
}
