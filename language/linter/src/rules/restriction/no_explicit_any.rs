use destack_ast::{self as ast, TypeLiteral};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow explicit `any` type annotations.
    ///
    /// The `any` type bypasses type checking and can lead to runtime errors.
    /// Prefer explicit types, `unknown`, or generics for better type safety.
    #[lint(
        id = "no-explicit-any",
        code = "LR004",
        category = Restriction,
        level = Ast,
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub NoExplicitAny,
    "Disallow explicit `any` type"
}

impl LintRule for NoExplicitAny {
    fn meta(&self) -> &'static crate::LintMeta {
        NoExplicitAny::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);
            if !matches!(expression, ast::Expression::TypeLiteral(TypeLiteral::Any)) {
                continue;
            }

            let span = ctx.tree.get_span(node_id);
            ctx.report(
                LintDiagnostic::new(
                    NO_EXPLICIT_ANY.id,
                    NO_EXPLICIT_ANY.code,
                    NO_EXPLICIT_ANY.category,
                    severity,
                    "`any` type is not allowed",
                    ctx.module.file_id,
                    span,
                )
                .with_label("use `unknown` or a specific type instead"),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_any_type_annotation() {
        let test = TestProgram::for_rule(NoExplicitAny);
        let result = test.lint_ast(
            "test.ts",
            r#"
let x: any = 42;
"#,
        );
        test.result(result).assert_lint("no-explicit-any");
    }

    #[test]
    fn test_detects_any_parameter() {
        let test = TestProgram::for_rule(NoExplicitAny);
        let result = test.lint_ast(
            "test.ts",
            r#"
function foo(x: any) {}
"#,
        );
        test.result(result).assert_lint("no-explicit-any");
    }

    #[test]
    fn test_detects_any_return_type() {
        let test = TestProgram::for_rule(NoExplicitAny);
        let result = test.lint_ast(
            "test.ts",
            r#"
function foo(): any { return 42; }
"#,
        );
        test.result(result).assert_lint("no-explicit-any");
    }

    #[test]
    fn test_allows_unknown() {
        let test = TestProgram::for_rule(NoExplicitAny);
        let result = test.lint_ast(
            "test.ts",
            r#"
let x: unknown = 42;
"#,
        );
        test.result(result).assert_no_lint("no-explicit-any");
    }

    #[test]
    fn test_allows_specific_types() {
        let test = TestProgram::for_rule(NoExplicitAny);
        let result = test.lint_ast(
            "test.ts",
            r#"
let x: number = 42;
let y: string = "hello";
"#,
        );
        test.result(result).assert_no_lint("no-explicit-any");
    }
}
