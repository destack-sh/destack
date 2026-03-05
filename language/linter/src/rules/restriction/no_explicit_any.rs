use destack_ast::{self as ast, TypeLiteral};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintFix, LintMeta, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow explicit `any` type annotations.
    ///
    /// The `any` type bypasses type checking and can lead to runtime errors.
    /// Prefer explicit types, `unknown`, or generics for better type safety.
    #[lint(
        id = "no-explicit-any",
        code = "LR013",
        category = Restriction,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = Always,
        recommended = Strict,
        stability = Stable
    )]
    pub NoExplicitAny,
    "Disallow explicit `any` type"
}

impl LintRule for NoExplicitAny {
    fn meta(&self) -> &'static LintMeta {
        NoExplicitAny::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        // inspect candidate expressions
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);
            if !matches!(expression, ast::Expression::TypeLiteral(TypeLiteral::Any)) {
                continue;
            }

            // resolve effective lint severity
            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }
            let span = ctx.tree.get_span(node_id);
            let mut diagnostic = LintDiagnostic::new(
                NO_EXPLICIT_ANY.id,
                NO_EXPLICIT_ANY.code,
                NO_EXPLICIT_ANY.category,
                severity,
                "`any` type is not allowed",
                ctx.module.file_id,
                span,
            )
            .with_label("use `unknown` or a specific type instead");

            // compute fixes only when requested by the runner
            if ctx.compute_fixes {
                let edits = ctx.edit_builder().replace(span, "unknown").into_edits();
                let fix = LintFix::safe("Replace `any` with `unknown`").with_edits(edits);
                diagnostic = diagnostic.with_fix(fix);
            }

            ctx.report(diagnostic);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_any_type_annotation() {
        let test = TestProgram::for_rule_without_prelude(NoExplicitAny);
        let result = test.lint_ast(
            "no_explicit_any/test_detects_any_type_annotation.ts",
            r#"
let x: any = 42;
"#,
        );
        test.result(result)
            .assert_lint("no-explicit-any")
            .assert_has_fix("no-explicit-any");
    }

    #[test]
    fn test_detects_any_parameter() {
        let test = TestProgram::for_rule_without_prelude(NoExplicitAny);
        let result = test.lint_ast(
            "no_explicit_any/test_detects_any_parameter.ts",
            r#"
function foo(x: any) {}
"#,
        );
        test.result(result).assert_lint("no-explicit-any");
    }

    #[test]
    fn test_detects_any_return_type() {
        let test = TestProgram::for_rule_without_prelude(NoExplicitAny);
        let result = test.lint_ast(
            "no_explicit_any/test_detects_any_return_type.ts",
            r#"
function foo(): any { return 42; }
"#,
        );
        test.result(result).assert_lint("no-explicit-any");
    }

    #[test]
    fn test_allows_unknown() {
        let test = TestProgram::for_rule_without_prelude(NoExplicitAny);
        let result = test.lint_ast(
            "no_explicit_any/test_allows_unknown.ts",
            r#"
let x: unknown = 42;
"#,
        );
        test.result(result).assert_no_lint("no-explicit-any");
    }

    #[test]
    fn test_allows_specific_types() {
        let test = TestProgram::for_rule_without_prelude(NoExplicitAny);
        let result = test.lint_ast(
            "no_explicit_any/test_allows_specific_types.ts",
            r#"
let x: number = 42;
let y: string = "hello";
"#,
        );
        test.result(result).assert_no_lint("no-explicit-any");
    }

    #[test]
    fn test_fix_any_type_annotation() {
        let test = TestProgram::for_rule_without_prelude(NoExplicitAny);
        let result = test.lint_ast(
            "no_explicit_any/test_fix_any_type_annotation.ts",
            r#"
let x: any = 42
"#,
        );
        test.result(result)
            .assert_lint("no-explicit-any")
            .assert_safe_fixed(
                r#"
let x: unknown = 42;
"#,
            );
    }

    #[test]
    fn test_fix_any_parameter_and_return_type() {
        let test = TestProgram::for_rule_without_prelude(NoExplicitAny);
        let result = test.lint_ast(
            "no_explicit_any/test_fix_any_parameter_and_return_type.ts",
            r#"
function foo(value: any): any {
    return value
}
"#,
        );
        test.result(result)
            .assert_lint_count("no-explicit-any", 2)
            .assert_safe_fixed(
                r#"
function foo(value: unknown): unknown {
    return value;
}
"#,
            );
    }

    #[test]
    fn test_mutation_flags_nested_any_occurrences() {
        let test = TestProgram::for_rule_without_prelude(NoExplicitAny);
        let result = test.lint_ast(
            "no_explicit_any/test_mutation_flags_nested_any_occurrences.ts",
            r#"
type Payload = { value: any, items: any[] }
"#,
        );
        test.result(result)
            .assert_lint_count("no-explicit-any", 2)
            .assert_safe_fixed(
                r#"
type Payload = { value: unknown, items: unknown[] };
"#,
            );
    }
}
