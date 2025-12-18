use destack_ast::{self as ast, Declaration, FunctionKind};
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Require explicit return type annotations on functions.
    ///
    /// Explicit return types improve code readability and catch errors early.
    /// They also provide better IDE support and documentation.
    #[lint(
        id = "explicit-function-return-type",
        code = "LD001",
        category = Pedantic,
        level = Ast,
        fixable = No,
        recommended = Off,
        stability = Stable
    )]
    pub ExplicitFunctionReturnType,
    "Require explicit function return types"
}

impl LintRule for ExplicitFunctionReturnType {
    fn meta(&self) -> &'static crate::LintMeta {
        ExplicitFunctionReturnType::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        for node_id in ctx.tree.iter_nodes::<ast::Declaration>() {
            let declaration = ctx.tree.get(node_id);
            let Declaration::Function { signature, .. } = declaration else {
                continue;
            };

            // skip lambda functions, they may have inferred types
            if signature.kind == FunctionKind::Lambda {
                continue;
            }

            // has return type
            if signature.return_type.is_some() {
                continue;
            }

            let span = ctx.tree.get_span(node_id);
            ctx.report(
                LintDiagnostic::new(
                    EXPLICIT_FUNCTION_RETURN_TYPE.id,
                    EXPLICIT_FUNCTION_RETURN_TYPE.code,
                    EXPLICIT_FUNCTION_RETURN_TYPE.category,
                    severity,
                    "function is missing explicit return type",
                    ctx.module.file_id,
                    span,
                )
                .with_label("add return type annotation"),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_missing_return_type() {
        let test = TestProgram::for_rule(ExplicitFunctionReturnType);
        let result = test.lint_ast(
            "test.ts",
            r#"
function foo() {
    return 42;
}
"#,
        );
        test.result(result)
            .assert_lint("explicit-function-return-type");
    }

    #[test]
    fn test_allows_explicit_return_type() {
        let test = TestProgram::for_rule(ExplicitFunctionReturnType);
        let result = test.lint_ast(
            "test.ts",
            r#"
function foo(): number {
    return 42;
}
"#,
        );
        test.result(result)
            .assert_no_lint("explicit-function-return-type");
    }

    #[test]
    fn test_allows_void_return_type() {
        let test = TestProgram::for_rule(ExplicitFunctionReturnType);
        let result = test.lint_ast(
            "test.ts",
            r#"
function foo(): void {
    console.log("hello");
}
"#,
        );
        test.result(result)
            .assert_no_lint("explicit-function-return-type");
    }

    #[test]
    fn test_allows_arrow_functions() {
        let test = TestProgram::for_rule(ExplicitFunctionReturnType);
        let result = test.lint_ast(
            "test.ts",
            r#"
const foo = () => 42;
"#,
        );
        test.result(result)
            .assert_no_lint("explicit-function-return-type");
    }
}
