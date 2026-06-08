use crate::LintMeta;
use destack_dir as dir;
use destack_repository::{LintSeverity, MaxParamsCountThis};

use crate::rules::common::{
    CallableOwnerId, ThisParameterCount, for_each_callable_signature,
    function_signature_parameter_count,
};
use crate::{LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Limit the number of function parameters.
    ///
    /// Functions with many parameters are harder to use and understand.
    /// Consider using an options object or breaking the function into smaller pieces.
    #[lint(
        id = "max-params",
        code = "LX008",
        category = Complexity,
        level = Dir,
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
    fn meta(&self) -> &'static LintMeta {
        MaxParams::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        // resolve lint metadata and threshold
        let meta = self.meta();
        let max_params = ctx.options().complexity.max_params;
        let this_parameter_count = match ctx.options().complexity.max_params_count_this {
            MaxParamsCountThis::Never => ThisParameterCount::Never,
            MaxParamsCountThis::ExceptVoid => ThisParameterCount::ExceptVoid,
            MaxParamsCountThis::Always => ThisParameterCount::Always,
        };

        // check all callable signatures in declarations, methods, and properties
        for_each_callable_signature(ctx.dir.tree(), |owner_id, signature, _body| {
            // keep this-parameter counting aligned with the configured option
            let parameter_count =
                function_signature_parameter_count(ctx.dir.tree(), signature, this_parameter_count);
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
fn report_excessive_parameter_count<T: dir::Node + Clone>(
    ctx: &mut LintModuleContext<'_>,
    meta: &'static LintMeta,
    owner_id: dir::LocalNodeId<T>,
    parameter_count: usize,
    max_params: usize,
) where
    dir::Tree: dir::TreeStore<T>,
{
    // resolve owner span before moving owner id into severity lookup
    let owner_span = ctx.dir.get_span(owner_id);

    // resolve effective severity for this callable owner
    let severity = ctx.get_effective_severity(meta, owner_id);
    if !severity.is_enabled() {
        return;
    }

    // report one over-limit callable diagnostic
    ctx.report(
        LintReport::new(
            MAX_PARAMS.id,
            MAX_PARAMS.code,
            MAX_PARAMS.category,
            severity,
            format!("function has {parameter_count} parameters (max {max_params})"),
            owner_span,
        )
        .label("consider using an options object"),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_too_many_params() {
        let test = TestProgram::for_rule_without_prelude(MaxParams);
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
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
            .with_options(|options| options.complexity.max_params = 1);
        let result = test.lint(
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
            .with_options(|options| options.complexity.max_params = 1);
        let result = test.lint(
            "max_params/test_counts_non_void_this_parameter.ds",
            r#"
function withTypedThis(this: Example, a: int32) {}
"#,
        );
        test.result(result).assert_lint("max-params");
    }

    #[test]
    fn test_counts_void_this_when_enabled() {
        let test = TestProgram::for_rule_without_prelude(MaxParams).with_options(|options| {
            options.complexity.max_params = 1;
            options.complexity.max_params_count_this = MaxParamsCountThis::Always;
        });
        let result = test.lint(
            "max_params/test_counts_void_this_when_enabled.ds",
            r#"
function withVoidThis(this: void, a: int32) {}
"#,
        );
        test.result(result).assert_lint("max-params");
    }

    #[test]
    fn test_ignores_this_when_count_this_is_never() {
        let test = TestProgram::for_rule_without_prelude(MaxParams).with_options(|options| {
            options.complexity.max_params = 1;
            options.complexity.max_params_count_this = MaxParamsCountThis::Never;
        });
        let result = test.lint(
            "max_params/test_ignores_this_when_count_this_is_never.ds",
            r#"
function withTypedThis(this: Example, a: int32) {}
"#,
        );
        test.result(result).assert_no_lint("max-params");
    }
}
