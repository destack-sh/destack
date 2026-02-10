use destack_ast::{self as ast, Expression, ScalarLiteral, TypeBinaryOperator};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintFix, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Prefer `as const` over literal type assertions.
    ///
    /// Use `x as const` instead of `x as "literal"` for better type inference.
    #[lint(
        id = "prefer-as-const",
        code = "LY034",
        category = Style,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Sometimes,
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

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expr = ctx.tree.get(node_id);

            // look for type cast expressions
            let Expression::TypeBinary {
                left,
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
                let severity = ctx.get_effective_severity(meta, node_id);
                if !severity.is_enabled() {
                    continue;
                }

                // attach a safe fix for exact literal self casts
                let mut diagnostic = LintDiagnostic::new(
                    PREFER_AS_CONST.id,
                    PREFER_AS_CONST.code,
                    PREFER_AS_CONST.category,
                    severity,
                    "use `as const` instead of literal type assertion",
                    ctx.module.file_id,
                    ctx.tree.get_span(node_id),
                )
                .with_label("prefer `as const`");

                if is_exact_literal_self_cast(ctx.tree, *left, *right) {
                    let right_span = ctx.tree.get_span(*right);
                    let edits = ctx.edit_builder().replace(right_span, "const").into_edits();
                    let fix = LintFix::safe("Replace literal type assertion with `as const`")
                        .with_edits(edits);
                    diagnostic = diagnostic.with_fix(fix);
                }

                ctx.report(diagnostic);
            }
        }
    }
}

/// Return true when a cast has the same literal value on both sides.
fn is_exact_literal_self_cast(
    tree: &ast::NodeTree,
    left_id: ast::LocalNodeId<Expression>,
    right_id: ast::LocalNodeId<Expression>,
) -> bool {
    let left = tree.get(left_id);
    let right = tree.get(right_id);
    match (left, right) {
        (
            Expression::ScalarLiteral(ScalarLiteral::String(left)),
            Expression::ScalarLiteral(ScalarLiteral::String(right)),
        ) => left == right,
        (
            Expression::ScalarLiteral(ScalarLiteral::Integer(left)),
            Expression::ScalarLiteral(ScalarLiteral::Integer(right)),
        ) => left == right,
        (
            Expression::ScalarLiteral(ScalarLiteral::Boolean(left)),
            Expression::ScalarLiteral(ScalarLiteral::Boolean(right)),
        ) => left == right,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_string_literal_cast() {
        let test = TestProgram::for_rule_without_prelude(PreferAsConst);
        let result = test.lint_ast(
            "prefer_as_const/test_detects_string_literal_cast.ds",
            r#"
const x = "hello" as "hello"
"#,
        );
        test.result(result).assert_lint("prefer-as-const");
    }

    #[test]
    fn test_fix_string_literal_self_cast() {
        let test = TestProgram::for_rule_without_prelude(PreferAsConst);
        let result = test.lint_ast(
            "prefer_as_const/test_fix_string_literal_self_cast.ds",
            r#"
const x = "hello" as "hello"
"#,
        );
        test.result(result)
            .assert_lint("prefer-as-const")
            .assert_has_fix("prefer-as-const")
            .assert_safe_fixed(
                r#"
const x = "hello" as const;
"#,
            );
    }

    #[test]
    fn test_no_fix_for_non_literal_self_cast() {
        let test = TestProgram::for_rule_without_prelude(PreferAsConst);
        let result = test.lint_ast(
            "prefer_as_const/test_no_fix_for_non_literal_self_cast.ds",
            r#"
const x = value as "hello"
"#,
        );
        test.result(result)
            .assert_lint("prefer-as-const")
            .assert_has_no_fix("prefer-as-const");
    }

    #[test]
    fn test_detects_number_literal_cast() {
        let test = TestProgram::for_rule_without_prelude(PreferAsConst);
        let result = test.lint_ast(
            "prefer_as_const/test_detects_number_literal_cast.ds",
            r#"
const x = 42 as 42
"#,
        );
        test.result(result).assert_lint("prefer-as-const");
    }

    #[test]
    fn test_fix_number_literal_self_cast() {
        let test = TestProgram::for_rule_without_prelude(PreferAsConst);
        let result = test.lint_ast(
            "prefer_as_const/test_fix_number_literal_self_cast.ds",
            r#"
const x = 42 as 42
"#,
        );
        test.result(result)
            .assert_lint("prefer-as-const")
            .assert_has_fix("prefer-as-const")
            .assert_safe_fixed(
                r#"
const x = 42 as const;
"#,
            );
    }

    #[test]
    fn test_fix_boolean_literal_self_cast() {
        let test = TestProgram::for_rule_without_prelude(PreferAsConst);
        let result = test.lint_ast(
            "prefer_as_const/test_fix_boolean_literal_self_cast.ds",
            r#"
const x = true as true
"#,
        );
        test.result(result)
            .assert_lint("prefer-as-const")
            .assert_has_fix("prefer-as-const")
            .assert_safe_fixed(
                r#"
const x = true as const;
"#,
            );
    }

    #[test]
    fn test_no_fix_for_string_literal_mismatch() {
        let test = TestProgram::for_rule_without_prelude(PreferAsConst);
        let result = test.lint_ast(
            "prefer_as_const/test_no_fix_for_string_literal_mismatch.ds",
            r#"
const x = "hello" as "world"
"#,
        );
        test.result(result)
            .assert_lint("prefer-as-const")
            .assert_has_no_fix("prefer-as-const");
    }

    #[test]
    fn test_no_fix_for_number_literal_mismatch() {
        let test = TestProgram::for_rule_without_prelude(PreferAsConst);
        let result = test.lint_ast(
            "prefer_as_const/test_no_fix_for_number_literal_mismatch.ds",
            r#"
const x = 41 as 42
"#,
        );
        test.result(result)
            .assert_lint("prefer-as-const")
            .assert_has_no_fix("prefer-as-const");
    }

    #[test]
    fn test_no_fix_for_boolean_literal_mismatch() {
        let test = TestProgram::for_rule_without_prelude(PreferAsConst);
        let result = test.lint_ast(
            "prefer_as_const/test_no_fix_for_boolean_literal_mismatch.ds",
            r#"
const x = false as true
"#,
        );
        test.result(result)
            .assert_lint("prefer-as-const")
            .assert_has_no_fix("prefer-as-const");
    }

    #[test]
    fn test_allows_as_const() {
        let test = TestProgram::for_rule_without_prelude(PreferAsConst);
        let result = test.lint_ast(
            "prefer_as_const/test_allows_as_const.ds",
            r#"
const x = "hello" as const
"#,
        );
        test.result(result).assert_no_lint("prefer-as-const");
    }

    #[test]
    fn test_allows_type_cast() {
        let test = TestProgram::for_rule_without_prelude(PreferAsConst);
        let result = test.lint_ast(
            "prefer_as_const/test_allows_type_cast.ds",
            r#"
const x = value as string
"#,
        );
        test.result(result).assert_no_lint("prefer-as-const");
    }

    #[test]
    fn test_allows_object_cast() {
        let test = TestProgram::for_rule_without_prelude(PreferAsConst);
        let result = test.lint_ast(
            "prefer_as_const/test_allows_object_cast.ds",
            r#"
const x = obj as { foo: string }
"#,
        );
        test.result(result).assert_no_lint("prefer-as-const");
    }
}
