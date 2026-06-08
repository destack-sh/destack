use crate::LintMeta;
use destack_dir as dir;
use destack_repository::LintSeverity;

use crate::rules::common::{
    expression_has_side_effects, pattern_is_underscore_binding_or_wildcard,
};
use crate::{LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Warn on underscore bindings with no side effects.
    ///
    /// Binding to `_` or `_name` is useful when you want to ignore a value from an expression with side effects.
    /// If the expression has no side
    /// effects, the binding is useless and can be removed.
    #[lint(
        id = "no-useless-underscore-binding",
        code = "LX023",
        category = Complexity,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub NoUselessUnderscoreBinding,
    "Warn on useless underscore bindings"
}

impl LintRule for NoUselessUnderscoreBinding {
    fn meta(&self) -> &'static LintMeta {
        NoUselessUnderscoreBinding::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.dir.iter_nodes::<dir::Expression>() {
            let expression = ctx.dir.get(node_id);

            let (dir::Expression::Let { declarators, .. }
            | dir::Expression::Using { declarators, .. }) = expression
            else {
                continue;
            };

            for declarator_id in declarators {
                let declarator = ctx.dir.get(*declarator_id);

                // skip declarations without initializer values
                let Some(value_id) = declarator.value else {
                    continue;
                };

                // keep only wildcard or underscore style bindings
                if !pattern_is_underscore_binding_or_wildcard(ctx, declarator.pattern) {
                    continue;
                }

                // allow underscore bindings for side effect values
                if expression_has_side_effects(ctx, value_id) {
                    continue;
                }

                let severity = ctx.get_effective_severity(meta, *declarator_id);
                if !severity.is_enabled() {
                    continue;
                }

                ctx.report(
                    LintReport::new(
                        NO_USELESS_UNDERSCORE_BINDING.id,
                        NO_USELESS_UNDERSCORE_BINDING.code,
                        NO_USELESS_UNDERSCORE_BINDING.category,
                        severity,
                        "underscore binding with no side effects is useless",
                        ctx.dir.get_span(*declarator_id),
                    )
                    .label("remove this binding"),
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
    fn test_wildcard_with_call_allowed() {
        let test = TestProgram::for_rule_without_prelude(NoUselessUnderscoreBinding);
        let result = test.lint(
            "no_useless_underscore_binding/test_wildcard_with_call_allowed.ds",
            r#"
let _ = doSomething()
"#,
        );
        test.result(result)
            .assert_no_lint("no-useless-underscore-binding");
    }

    #[test]
    fn test_wildcard_with_literal_detected() {
        let test = TestProgram::for_rule_without_prelude(NoUselessUnderscoreBinding);
        let result = test.lint(
            "no_useless_underscore_binding/test_wildcard_with_literal_detected.ds",
            r#"
let _ = 42
"#,
        );
        test.result(result)
            .assert_lint("no-useless-underscore-binding");
    }

    #[test]
    fn test_underscore_name_with_literal_detected() {
        let test = TestProgram::for_rule_without_prelude(NoUselessUnderscoreBinding);
        let result = test.lint(
            "no_useless_underscore_binding/test_underscore_name_with_literal_detected.ds",
            r#"
let _unused = "hello"
"#,
        );
        test.result(result)
            .assert_lint("no-useless-underscore-binding");
    }

    #[test]
    fn test_underscore_name_with_call_allowed() {
        let test = TestProgram::for_rule_without_prelude(NoUselessUnderscoreBinding);
        let result = test.lint(
            "no_useless_underscore_binding/test_underscore_name_with_call_allowed.ds",
            r#"
let _result = fetchData()
"#,
        );
        test.result(result)
            .assert_no_lint("no-useless-underscore-binding");
    }

    #[test]
    fn test_normal_binding_with_literal_allowed() {
        let test = TestProgram::for_rule_without_prelude(NoUselessUnderscoreBinding);
        let result = test.lint(
            "no_useless_underscore_binding/test_normal_binding_with_literal_allowed.ds",
            r#"
let x = 42
"#,
        );
        test.result(result)
            .assert_no_lint("no-useless-underscore-binding");
    }

    #[test]
    fn test_wildcard_with_variable_detected() {
        let test = TestProgram::for_rule_without_prelude(NoUselessUnderscoreBinding);
        let result = test.lint(
            "no_useless_underscore_binding/test_wildcard_with_variable_detected.ds",
            r#"
let _ = someVariable
"#,
        );
        test.result(result)
            .assert_lint("no-useless-underscore-binding");
    }

    #[test]
    fn test_wildcard_with_await_allowed() {
        let test = TestProgram::for_rule_without_prelude(NoUselessUnderscoreBinding);
        let result = test.lint(
            "no_useless_underscore_binding/test_wildcard_with_await_allowed.ds",
            r#"
async function foo() {
    let _ = await promise
}
"#,
        );
        test.result(result)
            .assert_no_lint("no-useless-underscore-binding");
    }
}
