use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Limit the number of function parameters.
    ///
    /// Functions with many parameters are harder to use and understand.
    /// Consider using an options object or breaking the function into smaller pieces.
    #[lint(
        id = "max-params",
        code = "LX001",
        category = Complexity,
        level = Ast
    )]
    pub MaxParams,
    "Limit function parameters"
}

/// Default maximum number of parameters.
const DEFAULT_MAX_PARAMS: usize = 4;

impl LintRule for MaxParams {
    fn meta(&self) -> &'static crate::LintMeta {
        MaxParams::meta()
    }

    fn check_module_ast<'a>(&self, severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let ast::Expression::Declaration(decl_id) = ctx.tree.get(node_id) else {
                continue;
            };

            let ast::Declaration::Function { signature, .. } = ctx.tree.get(*decl_id) else {
                continue;
            };

            let param_count = signature.dynamic_parameters.len();
            if param_count > DEFAULT_MAX_PARAMS {
                ctx.report(
                    LintDiagnostic::new(
                        MAX_PARAMS.id,
                        MAX_PARAMS.code,
                        MAX_PARAMS.category,
                        severity,
                        format!("function has {param_count} parameters (max {DEFAULT_MAX_PARAMS})"),
                        ctx.module.file_id,
                        ctx.tree.get_span(node_id),
                    )
                    .with_label("consider using an options object"),
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
    fn test_detects_too_many_params() {
        let test = TestProgram::for_rule(MaxParams);
        let result = test.lint_ast(
            "test.ds",
            r#"
function tooMany(a: int32, b: int32, c: int32, d: int32, e: int32) {
    return a + b + c + d + e;
}
"#,
        );
        test.result(result).assert_lint("max-params");
    }

    #[test]
    fn test_detects_exactly_over_limit() {
        let test = TestProgram::for_rule(MaxParams);
        let result = test.lint_ast(
            "test.ds",
            r#"
function fiveParams(a: int32, b: int32, c: int32, d: int32, e: int32) {}
"#,
        );
        test.result(result).assert_lint("max-params");
    }

    #[test]
    fn test_allows_four_params() {
        let test = TestProgram::for_rule(MaxParams);
        let result = test.lint_ast(
            "test.ds",
            r#"
function fourParams(a: int32, b: int32, c: int32, d: int32) {
    return a + b + c + d;
}
"#,
        );
        test.result(result).assert_no_lint("max-params");
    }

    #[test]
    fn test_allows_few_params() {
        let test = TestProgram::for_rule(MaxParams);
        let result = test.lint_ast(
            "test.ds",
            r#"
function add(a: int32, b: int32) {
    return a + b;
}
"#,
        );
        test.result(result).assert_no_lint("max-params");
    }

    #[test]
    fn test_allows_no_params() {
        let test = TestProgram::for_rule(MaxParams);
        let result = test.lint_ast(
            "test.ds",
            r#"
function noParams() {
    return 42;
}
"#,
        );
        test.result(result).assert_no_lint("max-params");
    }

    #[test]
    fn test_detects_lambda_too_many_params() {
        let test = TestProgram::for_rule(MaxParams);
        let result = test.lint_ast(
            "test.ds",
            r#"
const fn = (a: int32, b: int32, c: int32, d: int32, e: int32) => a + b + c + d + e;
"#,
        );
        test.result(result).assert_lint("max-params");
    }
}
