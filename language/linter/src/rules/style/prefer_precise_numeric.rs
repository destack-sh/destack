use destack_ast::{self as ast, TypeLiteral};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Prefer precise numeric types over `number`.
    ///
    /// The `number` type is imprecise (JavaScript's float64). Prefer explicit
    /// types like `int32`, `int64`, or `float64` for clearer intent.
    #[lint(
        id = "prefer-precise-numeric",
        code = "LY042",
        category = Style,
        level = Ast,
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferPreciseNumeric,
    "Prefer precise numeric types"
}

impl LintRule for PreferPreciseNumeric {
    fn meta(&self) -> &'static crate::LintMeta {
        PreferPreciseNumeric::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);

            // look for TypeLiteral::Number expressions
            let ast::Expression::TypeLiteral(TypeLiteral::Number) = expression else {
                continue;
            };

            ctx.report(
                LintDiagnostic::new(
                    PREFER_PRECISE_NUMERIC.id,
                    PREFER_PRECISE_NUMERIC.code,
                    PREFER_PRECISE_NUMERIC.category,
                    severity,
                    "prefer precise numeric type over `number`",
                    ctx.module.file_id,
                    ctx.tree.get_span(node_id),
                )
                .with_label("use int32, int64, float32, or float64"),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_number_type_detected() {
        let test = TestProgram::for_rule(PreferPreciseNumeric);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(x: number): number {
    return x
}
"#,
        );
        test.result(result).assert_lint("prefer-precise-numeric");
    }

    #[test]
    fn test_int32_type_allowed() {
        let test = TestProgram::for_rule(PreferPreciseNumeric);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(x: int32): int32 {
    return x
}
"#,
        );
        test.result(result).assert_no_lint("prefer-precise-numeric");
    }

    #[test]
    fn test_float64_type_allowed() {
        let test = TestProgram::for_rule(PreferPreciseNumeric);
        let result = test.lint_ast(
            "test.ds",
            r#"
function foo(x: float64): float64 {
    return x
}
"#,
        );
        test.result(result).assert_no_lint("prefer-precise-numeric");
    }
}
