use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::rules::common::{
    CallableOwnerId, ThisParameterCount, for_each_callable_signature,
    function_signature_parameter_count,
};
use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Limit the number of function parameters.
    ///
    /// Functions with many parameters are harder to use and understand.
    /// Consider using an options object or breaking the function into smaller pieces.
    #[lint(
        id = "max-params",
        code = "LX008",
        category = Complexity,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub MaxParams,
    "Limit function parameters"
}

impl LintRule for MaxParams {
    fn meta(&self) -> &'static crate::LintMeta {
        MaxParams::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        // resolve lint metadata and threshold
        let meta = self.meta();
        let max_params = ctx.options.max_params;

        // check all callable signatures in declarations, methods, and properties
        for_each_callable_signature(ctx.tree, |owner_id, signature, _body| {
            // match eslint max-params default this counting: except-void
            let parameter_count = function_signature_parameter_count(
                ctx.tree,
                signature,
                ThisParameterCount::ExceptVoid,
            );
            if parameter_count <= max_params {
                return;
            }

            // report for declaration owners
            if let CallableOwnerId::Declaration(declaration_id) = owner_id {
                report_excessive_parameter_count(
                    ctx,
                    meta,
                    declaration_id,
                    parameter_count,
                    max_params,
                );
                return;
            }

            // report for member owners
            if let CallableOwnerId::Member(member_id) = owner_id {
                report_excessive_parameter_count(ctx, meta, member_id, parameter_count, max_params);
                return;
            }

            // report for property owners
            if let CallableOwnerId::Property(property_id) = owner_id {
                report_excessive_parameter_count(
                    ctx,
                    meta,
                    property_id,
                    parameter_count,
                    max_params,
                );
            }
        });
    }
}

/// Report one max-params violation for a callable owner.
fn report_excessive_parameter_count<T: ast::Node + Clone>(
    ctx: &mut LintModuleAstContext<'_>,
    meta: &'static crate::LintMeta,
    owner_id: ast::LocalNodeId<T>,
    parameter_count: usize,
    max_params: usize,
) {
    // resolve owner span before moving owner id into severity lookup
    let owner_span = ctx.tree.get_span(owner_id.clone());

    // resolve effective severity for this callable owner
    let severity = ctx.get_effective_severity(meta, owner_id);
    if !severity.is_enabled() {
        return;
    }

    // report one over-limit callable diagnostic
    ctx.report(
        LintDiagnostic::new(
            MAX_PARAMS.id,
            MAX_PARAMS.code,
            MAX_PARAMS.category,
            severity,
            format!("function has {parameter_count} parameters (max {max_params})"),
            ctx.module.file_id,
            owner_span,
        )
        .with_label("consider using an options object"),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_too_many_params() {
        let test = TestProgram::for_rule_without_prelude(MaxParams);
        let result = test.lint_ast(
            "max_params/test_detects_too_many_params.ds",
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
        let test = TestProgram::for_rule_without_prelude(MaxParams);
        let result = test.lint_ast(
            "max_params/test_detects_exactly_over_limit.ds",
            r#"
function fiveParams(a: int32, b: int32, c: int32, d: int32, e: int32) {}
"#,
        );
        test.result(result).assert_lint("max-params");
    }

    #[test]
    fn test_allows_four_params() {
        let test = TestProgram::for_rule_without_prelude(MaxParams);
        let result = test.lint_ast(
            "max_params/test_allows_four_params.ds",
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
        let test = TestProgram::for_rule_without_prelude(MaxParams);
        let result = test.lint_ast(
            "max_params/test_allows_few_params.ds",
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
        let test = TestProgram::for_rule_without_prelude(MaxParams);
        let result = test.lint_ast(
            "max_params/test_allows_no_params.ds",
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
        let test = TestProgram::for_rule_without_prelude(MaxParams);
        let result = test.lint_ast(
            "max_params/test_detects_lambda_too_many_params.ds",
            r#"
const fn = (a: int32, b: int32, c: int32, d: int32, e: int32) => a + b + c + d + e;
"#,
        );
        test.result(result).assert_lint("max-params");
    }

    #[test]
    fn test_detects_method_too_many_params() {
        let test = TestProgram::for_rule_without_prelude(MaxParams);
        let result = test.lint_ast(
            "max_params/test_detects_method_too_many_params.ds",
            r#"
class Example {
    method(a: int32, b: int32, c: int32, d: int32, e: int32) {}
}
"#,
        );
        test.result(result).assert_lint("max-params");
    }

    #[test]
    fn test_detects_object_method_too_many_params() {
        let test = TestProgram::for_rule_without_prelude(MaxParams);
        let result = test.lint_ast(
            "max_params/test_detects_object_method_too_many_params.ds",
            r#"
const object = {
    method(a: int32, b: int32, c: int32, d: int32, e: int32) {}
}
"#,
        );
        test.result(result).assert_lint("max-params");
    }

    #[test]
    fn test_ignores_void_this_parameter() {
        let test = TestProgram::for_rule_without_prelude(MaxParams)
            .with_options(|options| options.max_params = 1);
        let result = test.lint_ast(
            "max_params/test_ignores_void_this_parameter.ds",
            r#"
function withVoidThis(this: void, a: int32) {}
"#,
        );
        test.result(result).assert_no_lint("max-params");
    }

    #[test]
    fn test_counts_non_void_this_parameter() {
        let test = TestProgram::for_rule_without_prelude(MaxParams)
            .with_options(|options| options.max_params = 1);
        let result = test.lint_ast(
            "max_params/test_counts_non_void_this_parameter.ds",
            r#"
function withTypedThis(this: Example, a: int32) {}
"#,
        );
        test.result(result).assert_lint("max-params");
    }
}
