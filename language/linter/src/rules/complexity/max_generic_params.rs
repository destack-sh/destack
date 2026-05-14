use crate::LintMeta;
use destack_dir as dir;
use destack_workspace::LintSeverity;

use crate::rules::common::{
    CallableOwnerId, for_each_callable_signature, function_signature_generic_parameter_count,
};
use crate::{LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Limit the number of generic parameters on a declaration.
    ///
    /// Declarations with many generic parameters are harder to use and understand.
    /// Consider splitting into smaller components or using associated types.
    #[lint(
        id = "max-generic-params",
        code = "LX011",
        category = Complexity,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub MaxGenericParams,
    "Limit generic parameters"
}

impl LintRule for MaxGenericParams {
    fn meta(&self) -> &'static LintMeta {
        MaxGenericParams::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        // resolve lint metadata and threshold
        let meta = self.meta();
        let max_generic_params = ctx.options.complexity.max_generic_params;

        // check declaration-level generic parameters
        for declaration_id in ctx.dir.iter_nodes::<dir::Declaration>() {
            let declaration = ctx.dir.get(declaration_id);

            // count generic parameters
            let Some(generic_params) = declaration.generic_parameters() else {
                continue;
            };

            let param_count = generic_params.len();
            if param_count > max_generic_params {
                let severity = ctx.get_effective_severity(meta, declaration_id);
                if !severity.is_enabled() {
                    continue;
                }

                ctx.report(
                    LintReport::new(
                        MAX_GENERIC_PARAMS.id,
                        MAX_GENERIC_PARAMS.code,
                        MAX_GENERIC_PARAMS.category,
                        severity,
                        format!(
                            "declaration has {param_count} generic parameters (max {max_generic_params})"
                        ),
                        ctx.dir.get_span(declaration_id))
                    .label("consider splitting into smaller components"),
                );
            }
        }

        // check method-level generic parameters on class and object methods
        for_each_callable_signature(ctx.dir.tree(), |owner_id, signature, _body| {
            // declaration functions are already covered in declaration pass
            if let CallableOwnerId::Declaration(_) = owner_id {
                return;
            }

            // count generic parameters on this method signature
            let param_count = function_signature_generic_parameter_count(signature);
            if param_count <= max_generic_params {
                return;
            }

            // report class and interface methods
            if let CallableOwnerId::Member(member_id) = owner_id {
                report_method_generic_params(ctx, meta, member_id, param_count, max_generic_params);
                return;
            }

            // report object and type-literal methods
            if let CallableOwnerId::Property(property_id) = owner_id {
                report_method_generic_params(
                    ctx,
                    meta,
                    property_id,
                    param_count,
                    max_generic_params,
                );
            }
        });
    }
}

/// Report generic-parameter overflow for one callable method owner.
fn report_method_generic_params<T: dir::Node + Clone>(
    ctx: &mut LintModuleContext<'_>,
    meta: &'static LintMeta,
    owner_id: dir::LocalNodeId<T>,
    param_count: usize,
    max_generic_params: usize,
) where
    dir::Tree: dir::TreeStore<T>,
{
    // resolve owner span before moving owner id
    let owner_span = ctx.dir.get_span(owner_id);

    // resolve effective severity for this method owner
    let severity = ctx.get_effective_severity(meta, owner_id);
    if !severity.is_enabled() {
        return;
    }

    // report one method generic-parameter overflow
    ctx.report(
        LintReport::new(
            MAX_GENERIC_PARAMS.id,
            MAX_GENERIC_PARAMS.code,
            MAX_GENERIC_PARAMS.category,
            severity,
            format!("method has {param_count} generic parameters (max {max_generic_params})"),
            owner_span,
        )
        .label("consider splitting into smaller components"),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_flags_many_generic_params_on_function() {
        let test = TestProgram::for_rule_without_prelude(MaxGenericParams);
        let result = test.lint(
            "max_generic_params/test_flags_many_generic_params_on_function.ds",
            r#"
function combine<A, B, C, D, E>(a: A, b: B, c: C, d: D, e: E) {
    return [a, b, c, d, e];
}
"#,
        );
        test.result(result).assert_lint("max-generic-params");
    }

    #[test]
    fn test_flags_many_generic_params_on_class() {
        let test = TestProgram::for_rule_without_prelude(MaxGenericParams);
        let result = test.lint(
            "max_generic_params/test_flags_many_generic_params_on_class.ds",
            r#"
class Container<A, B, C, D, E> {
    a: A;
    b: B;
    c: C;
    d: D;
    e: E;
}
"#,
        );
        test.result(result).assert_lint("max-generic-params");
    }

    #[test]
    fn test_flags_many_generic_params_on_interface() {
        let test = TestProgram::for_rule_without_prelude(MaxGenericParams);
        let result = test.lint(
            "max_generic_params/test_flags_many_generic_params_on_interface.ds",
            r#"
interface Handler<A, B, C, D, E> {
    handle(a: A, b: B, c: C, d: D, e: E): void;
}
"#,
        );
        test.result(result).assert_lint("max-generic-params");
    }

    #[test]
    fn test_allows_few_generic_params() {
        let test = TestProgram::for_rule_without_prelude(MaxGenericParams);
        let result = test.lint(
            "max_generic_params/test_allows_few_generic_params.ds",
            r#"
function pair<A, B>(a: A, b: B) {
    return [a, b];
}
"#,
        );
        test.result(result).assert_no_lint("max-generic-params");
    }

    #[test]
    fn test_allows_exactly_at_limit() {
        let test = TestProgram::for_rule_without_prelude(MaxGenericParams);
        let result = test.lint(
            "max_generic_params/test_allows_exactly_at_limit.ds",
            r#"
function quad<A, B, C, D>(a: A, b: B, c: C, d: D) {
    return [a, b, c, d];
}
"#,
        );
        test.result(result).assert_no_lint("max-generic-params");
    }

    #[test]
    fn test_allows_no_generic_params() {
        let test = TestProgram::for_rule_without_prelude(MaxGenericParams);
        let result = test.lint(
            "max_generic_params/test_allows_no_generic_params.ds",
            r#"
function identity(x: int32) {
    return x;
}
"#,
        );
        test.result(result).assert_no_lint("max-generic-params");
    }

    #[test]
    fn test_flags_many_generic_params_on_class_method() {
        let test = TestProgram::for_rule_without_prelude(MaxGenericParams);
        let result = test.lint(
            "max_generic_params/test_flags_many_generic_params_on_class_method.ds",
            r#"
class Container {
    transform<A, B, C, D, E>(value: A): A {
        return value;
    }
}
"#,
        );
        test.result(result).assert_lint("max-generic-params");
    }

    #[test]
    fn test_flags_many_generic_params_on_object_method() {
        let test = TestProgram::for_rule_without_prelude(MaxGenericParams);
        let result = test.lint(
            "max_generic_params/test_flags_many_generic_params_on_object_method.ds",
            r#"
const container = {
    transform<A, B, C, D, E>(value: A): A {
        return value;
    }
}
"#,
        );
        test.result(result).assert_lint("max-generic-params");
    }
}
