use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

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
    fn meta(&self) -> &'static crate::LintMeta {
        MaxStaticParams::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();
        let max_static_params = ctx.options.max_static_params;

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let ast::Expression::Declaration(decl_id) = ctx.tree.get(node_id) else {
                continue;
            };

            let declaration = ctx.tree.get(*decl_id);

            // count static parameters
            let Some(static_params) = declaration.static_parameters() else {
                continue;
            };

            let param_count = static_params.len();
            if param_count > max_static_params {
                let severity = ctx.get_effective_severity(meta, node_id);
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
                        ctx.tree.get_span(node_id),
                    )
                    .with_label("consider splitting into smaller components"),
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
}
