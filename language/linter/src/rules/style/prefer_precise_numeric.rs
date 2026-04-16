use crate::LintMeta;
use destack_ast::{self as ast, TypeLiteral};
use destack_workspace::LintSeverity;

use crate::{LintAstContext, LintDiagnostic, LintFix, LintRule, declare_lint};

declare_lint! {
    /// Prefer precise numeric types over `number`.
    ///
    /// The `number` type is imprecise (JavaScript's float64). Prefer explicit
    /// types like `int32`, `int64`, or `float64` for clearer intent.
    #[lint(
        id = "prefer-precise-numeric",
        code = "LY050",
        category = Style,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Always,
        recommended = Strict,
        stability = Stable
    )]
    pub PreferPreciseNumeric,
    "Prefer precise numeric types"
}

impl LintRule for PreferPreciseNumeric {
    fn meta(&self) -> &'static LintMeta {
        PreferPreciseNumeric::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::TypeExpression>() {
            let expression = ctx.tree.get(node_id);

            // look for TypeLiteral::Number expressions
            let ast::TypeExpression::Literal {
                value: TypeLiteral::Number,
            } = expression
            else {
                continue;
            };

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            let span = ctx.tree.get_span(node_id);
            let edits = ctx.edit_builder().replace(span, "float64").into_edits();
            let fix = LintFix::safe("Replace `number` with `float64`").with_edits(edits);

            ctx.report(
                LintDiagnostic::new(
                    PREFER_PRECISE_NUMERIC.id,
                    PREFER_PRECISE_NUMERIC.code,
                    PREFER_PRECISE_NUMERIC.category,
                    severity,
                    "prefer precise numeric type over `number`",
                    ctx.module.file_id,
                    span,
                )
                .with_label("use int32, int64, float32, or float64")
                .with_fix(fix),
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
        let test = TestProgram::for_rule_without_prelude(PreferPreciseNumeric);
        let result = test.lint_ast(
            "prefer_precise_numeric/test_number_type_detected.ds",
            r#"
function foo(x: number): number {
    return x
}
"#,
        );
        test.result(result)
            .assert_lint("prefer-precise-numeric")
            .assert_has_fix("prefer-precise-numeric");
    }

    #[test]
    fn test_int32_type_allowed() {
        let test = TestProgram::for_rule_without_prelude(PreferPreciseNumeric);
        let result = test.lint_ast(
            "prefer_precise_numeric/test_int32_type_allowed.ds",
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
        let test = TestProgram::for_rule_without_prelude(PreferPreciseNumeric);
        let result = test.lint_ast(
            "prefer_precise_numeric/test_float64_type_allowed.ds",
            r#"
function foo(x: float64): float64 {
    return x
}
"#,
        );
        test.result(result).assert_no_lint("prefer-precise-numeric");
    }

    #[test]
    fn test_fix_number_parameter_and_return() {
        let test = TestProgram::for_rule_without_prelude(PreferPreciseNumeric);
        let result = test.lint_ast(
            "prefer_precise_numeric/test_fix_number_parameter_and_return.ds",
            r#"
function foo(x: number): number {
    return x
}
"#,
        );
        test.result(result)
            .assert_lint_count("prefer-precise-numeric", 2)
            .assert_safe_fixed(
                r#"
function foo(x: float64): float64 {
    return x;
}
"#,
            );
    }

    #[test]
    fn test_fix_number_in_type_alias() {
        let test = TestProgram::for_rule_without_prelude(PreferPreciseNumeric);
        let result = test.lint_ast(
            "prefer_precise_numeric/test_fix_number_in_type_alias.ds",
            r#"
type Box = { value: number, items: number[] }
"#,
        );
        test.result(result)
            .assert_lint_count("prefer-precise-numeric", 2)
            .assert_safe_fixed(
                r#"
type Box = { value: float64, items: float64[] };
"#,
            );
    }

    #[test]
    fn test_mutation_flags_number_in_union_members() {
        let test = TestProgram::for_rule_without_prelude(PreferPreciseNumeric);
        let result = test.lint_ast(
            "prefer_precise_numeric/test_mutation_flags_number_in_union_members.ds",
            r#"
type Metric = number | "auto"
"#,
        );
        test.result(result)
            .assert_lint("prefer-precise-numeric")
            .assert_safe_fixed(
                r#"
type Metric = float64 | "auto";
"#,
            );
    }
}
