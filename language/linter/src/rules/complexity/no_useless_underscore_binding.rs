use destack_ast::{self as ast, Pattern};
use destack_workspace::LintSeverity;

use crate::rules::common::has_side_effects;
use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Warn on underscore bindings with no side effects.
    ///
    /// Binding to `_` or `_name` is useful when you want to ignore a value
    /// from an expression with side effects. If the expression has no side
    /// effects, the binding is useless and can be removed.
    #[lint(
        id = "no-useless-underscore-binding",
        code = "LX024",
        category = Complexity,
        level = Ast,
        fixable = No,
        recommended = Strict,
        stability = Stable
    )]
    pub NoUselessUnderscoreBinding,
    "Warn on useless underscore bindings"
}

/// Check if a pattern is an underscore pattern (wildcard or binding starting with _).
fn is_underscore_pattern(
    ctx: &LintModuleAstContext<'_>,
    pattern_id: ast::LocalNodeId<Pattern>,
) -> bool {
    let pattern = ctx.tree.get(pattern_id);
    match pattern {
        Pattern::Wildcard => true,
        Pattern::Binding { name, .. } => {
            let name_str = ctx.strings.get(*name);
            name_str.as_ref().starts_with('_')
        }
        _ => false,
    }
}

impl LintRule for NoUselessUnderscoreBinding {
    fn meta(&self) -> &'static crate::LintMeta {
        NoUselessUnderscoreBinding::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Expression>() {
            let expression = ctx.tree.get(node_id);

            let (ast::Expression::Let { declarators, .. }
            | ast::Expression::Using { declarators, .. }) = expression
            else {
                continue;
            };

            for declarator_id in declarators {
                let declarator = ctx.tree.get(*declarator_id);

                // skip if no value (just declaration without initialization)
                let Some(value_id) = declarator.value else {
                    continue;
                };

                // check if pattern is underscore
                if !is_underscore_pattern(ctx, declarator.pattern) {
                    continue;
                }

                // check if value has side effects
                if has_side_effects(ctx, value_id) {
                    continue;
                }

                let severity = ctx.get_effective_severity(meta, *declarator_id);
                if !severity.is_enabled() {
                    continue;
                }

                ctx.report(
                    LintDiagnostic::new(
                        NO_USELESS_UNDERSCORE_BINDING.id,
                        NO_USELESS_UNDERSCORE_BINDING.code,
                        NO_USELESS_UNDERSCORE_BINDING.category,
                        severity,
                        "underscore binding with no side effects is useless",
                        ctx.module.file_id,
                        ctx.tree.get_span(*declarator_id),
                    )
                    .with_label("remove this binding"),
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
        let test = TestProgram::for_rule_without_builtins(NoUselessUnderscoreBinding);
        let result = test.lint_ast(
            "test.ds",
            r#"
let _ = doSomething()
"#,
        );
        test.result(result)
            .assert_no_lint("no-useless-underscore-binding");
    }

    #[test]
    fn test_wildcard_with_literal_detected() {
        let test = TestProgram::for_rule_without_builtins(NoUselessUnderscoreBinding);
        let result = test.lint_ast(
            "test.ds",
            r#"
let _ = 42
"#,
        );
        test.result(result)
            .assert_lint("no-useless-underscore-binding");
    }

    #[test]
    fn test_underscore_name_with_literal_detected() {
        let test = TestProgram::for_rule_without_builtins(NoUselessUnderscoreBinding);
        let result = test.lint_ast(
            "test.ds",
            r#"
let _unused = "hello"
"#,
        );
        test.result(result)
            .assert_lint("no-useless-underscore-binding");
    }

    #[test]
    fn test_underscore_name_with_call_allowed() {
        let test = TestProgram::for_rule_without_builtins(NoUselessUnderscoreBinding);
        let result = test.lint_ast(
            "test.ds",
            r#"
let _result = fetchData()
"#,
        );
        test.result(result)
            .assert_no_lint("no-useless-underscore-binding");
    }

    #[test]
    fn test_normal_binding_with_literal_allowed() {
        let test = TestProgram::for_rule_without_builtins(NoUselessUnderscoreBinding);
        let result = test.lint_ast(
            "test.ds",
            r#"
let x = 42
"#,
        );
        test.result(result)
            .assert_no_lint("no-useless-underscore-binding");
    }

    #[test]
    fn test_wildcard_with_variable_detected() {
        let test = TestProgram::for_rule_without_builtins(NoUselessUnderscoreBinding);
        let result = test.lint_ast(
            "test.ds",
            r#"
let _ = someVariable
"#,
        );
        test.result(result)
            .assert_lint("no-useless-underscore-binding");
    }

    #[test]
    fn test_wildcard_with_await_allowed() {
        let test = TestProgram::for_rule_without_builtins(NoUselessUnderscoreBinding);
        let result = test.lint_ast(
            "test.ds",
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
