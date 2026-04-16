use crate::LintMeta;
use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::rules::common::{
    CallableOwnerId, for_each_callable_signature, function_signature_static_parameter_count,
};
use crate::{LintAstContext, LintDiagnostic, LintRule, declare_lint};

declare_lint! {
    /// Limit the number of static parameters on a declaration.
    ///
    /// Declarations with many static parameters are harder to use and understand.
    /// Consider splitting into smaller components or using associated types.
    #[lint(
        id = "max-static-params",
        code = "LX011",
        category = Complexity,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub MaxStaticParams,
    "Limit static parameters"
}

impl LintRule for MaxStaticParams {
    fn meta(&self) -> &'static LintMeta {
        MaxStaticParams::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        // resolve lint metadata and threshold
        let meta = self.meta();
        let max_static_params = ctx.options.complexity.max_static_params;

        // check declaration-level static parameters
        for declaration_id in ctx.tree.iter_nodes::<ast::Declaration>() {
            let declaration = ctx.tree.get(declaration_id);

            // count static parameters
            let Some(static_params) = declaration.generic_parameters() else {
                continue;
            };

            let param_count = static_params.len();
            if param_count > max_static_params {
                let severity = ctx.get_effective_severity(meta, declaration_id);
                if !severity.is_enabled() {
                    continue;
                }

                ctx.report(
                    LintDiagnostic::new(
                        MAX_STATIC_PARAMS.id,
                        MAX_STATIC_PARAMS.code,
                        MAX_STATIC_PARAMS.category,
                        severity,
                        format!(
                            "declaration has {param_count} static parameters (max {max_static_params})"
                        ),
                        ctx.module.file_id,
                        ctx.tree.get_span(declaration_id),
                    )
                    .with_label("consider splitting into smaller components"),
                );
            }
        }

        // check method-level static parameters on class and object methods
        for_each_callable_signature(ctx.tree, |owner_id, signature, _body| {
            // declaration functions are already covered in declaration pass
            if let CallableOwnerId::Declaration(_) = owner_id {
                return;
            }

            // count static parameters on this method signature
            let param_count = function_signature_static_parameter_count(signature);
            if param_count <= max_static_params {
                return;
            }

            // report class and interface methods
            if let CallableOwnerId::Member(member_id) = owner_id {
                report_method_static_params(ctx, meta, member_id, param_count, max_static_params);
                return;
            }

            // report object and type-literal methods
            if let CallableOwnerId::Property(property_id) = owner_id {
                report_method_static_params(ctx, meta, property_id, param_count, max_static_params);
            }
        });
    }
}

/// Report static-parameter overflow for one callable method owner.
fn report_method_static_params<T: ast::Node + Clone>(
    ctx: &mut LintAstContext<'_>,
    meta: &'static LintMeta,
    owner_id: ast::LocalNodeId<T>,
    param_count: usize,
    max_static_params: usize,
) {
    // resolve owner span before moving owner id
    let owner_span = ctx.tree.get_span(owner_id);

    // resolve effective severity for this method owner
    let severity = ctx.get_effective_severity(meta, owner_id);
    if !severity.is_enabled() {
        return;
    }

    // report one method static-parameter overflow
    ctx.report(
        LintDiagnostic::new(
            MAX_STATIC_PARAMS.id,
            MAX_STATIC_PARAMS.code,
            MAX_STATIC_PARAMS.category,
            severity,
            format!("method has {param_count} static parameters (max {max_static_params})"),
            ctx.module.file_id,
            owner_span,
        )
        .with_label("consider splitting into smaller components"),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_flags_many_static_params_on_function() {
        let test = TestProgram::for_rule_without_prelude(MaxStaticParams);
        let result = test.lint_ast(
            "max_static_params/test_flags_many_static_params_on_function.ds",
            r#"
function combine<A, B, C, D, E>(a: A, b: B, c: C, d: D, e: E) {
    return [a, b, c, d, e];
}
"#,
        );
        test.result(result).assert_lint("max-static-params");
    }

    #[test]
    fn test_flags_many_static_params_on_class() {
        let test = TestProgram::for_rule_without_prelude(MaxStaticParams);
        let result = test.lint_ast(
            "max_static_params/test_flags_many_static_params_on_class.ds",
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
        test.result(result).assert_lint("max-static-params");
    }

    #[test]
    fn test_flags_many_static_params_on_interface() {
        let test = TestProgram::for_rule_without_prelude(MaxStaticParams);
        let result = test.lint_ast(
            "max_static_params/test_flags_many_static_params_on_interface.ds",
            r#"
interface Handler<A, B, C, D, E> {
    handle(a: A, b: B, c: C, d: D, e: E): void;
}
"#,
        );
        test.result(result).assert_lint("max-static-params");
    }

    #[test]
    fn test_allows_few_static_params() {
        let test = TestProgram::for_rule_without_prelude(MaxStaticParams);
        let result = test.lint_ast(
            "max_static_params/test_allows_few_static_params.ds",
            r#"
function pair<A, B>(a: A, b: B) {
    return [a, b];
}
"#,
        );
        test.result(result).assert_no_lint("max-static-params");
    }

    #[test]
    fn test_allows_exactly_at_limit() {
        let test = TestProgram::for_rule_without_prelude(MaxStaticParams);
        let result = test.lint_ast(
            "max_static_params/test_allows_exactly_at_limit.ds",
            r#"
function quad<A, B, C, D>(a: A, b: B, c: C, d: D) {
    return [a, b, c, d];
}
"#,
        );
        test.result(result).assert_no_lint("max-static-params");
    }

    #[test]
    fn test_allows_no_static_params() {
        let test = TestProgram::for_rule_without_prelude(MaxStaticParams);
        let result = test.lint_ast(
            "max_static_params/test_allows_no_static_params.ds",
            r#"
function identity(x: int32) {
    return x;
}
"#,
        );
        test.result(result).assert_no_lint("max-static-params");
    }

    #[test]
    fn test_flags_many_static_params_on_class_method() {
        let test = TestProgram::for_rule_without_prelude(MaxStaticParams);
        let result = test.lint_ast(
            "max_static_params/test_flags_many_static_params_on_class_method.ds",
            r#"
class Container {
    transform<A, B, C, D, E>(value: A): A {
        return value;
    }
}
"#,
        );
        test.result(result).assert_lint("max-static-params");
    }

    #[test]
    fn test_flags_many_static_params_on_object_method() {
        let test = TestProgram::for_rule_without_prelude(MaxStaticParams);
        let result = test.lint_ast(
            "max_static_params/test_flags_many_static_params_on_object_method.ds",
            r#"
const container = {
    transform<A, B, C, D, E>(value: A): A {
        return value;
    }
}
"#,
        );
        test.result(result).assert_lint("max-static-params");
    }
}
