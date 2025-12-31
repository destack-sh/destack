use std::sync::LazyLock;

use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::rules::all_rules;
use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow unknown lint rule IDs in `@allow`/`@warn`/`@deny`/`@forbid` decorators.
    ///
    /// Using an unknown lint ID is likely a typo or outdated code. This rule ensures
    /// that all lint suppressions and configurations refer to existing lint rules.
    ///
    /// Note: This rule only validates lint rule IDs and codes (e.g., `no-empty`, `LC003`).
    /// Compiler warning codes are not checked as they reside in a separate crate.
    #[lint(
        id = "no-unknown-rule-decorator",
        code = "LC049",
        category = Correctness,
        level = Ast,
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoUnknownRuleDecorator,
    "Disallow unknown lint rule IDs in decorators"
}

/// Sorted list of all valid lint IDs and codes.
static LINT_SPECIFIERS: LazyLock<Vec<&'static str>> = LazyLock::new(|| {
    let mut specifiers = Vec::new();
    for rule in all_rules() {
        let meta = rule.meta();
        specifiers.push(meta.id);
        specifiers.push(meta.code);
    }
    specifiers.sort_unstable();
    specifiers.dedup();
    specifiers
});

fn is_valid_lint_specifier(specifier: &str) -> bool {
    LINT_SPECIFIERS.binary_search(&specifier).is_ok()
}

/// Decorator names that expect a lint rule specifier as their first argument.
const RULE_DECORATORS: &[&str] = &["allow", "deny", "forbid", "warn"];

impl LintRule for NoUnknownRuleDecorator {
    fn meta(&self) -> &'static crate::LintMeta {
        NoUnknownRuleDecorator::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        for node_id in ctx.tree.iter_nodes::<ast::Annotation>() {
            let annotation = ctx.tree.get(node_id);
            let ast::Annotation::Decorator { node, .. } = annotation else {
                continue;
            };

            let decorator = ctx.tree.get(*node);

            // check decorator name (must be single segment: allow, warn, deny, forbid)
            if decorator.left.segments.len() != 1 {
                continue;
            }

            let name = ctx.strings.get(decorator.left.segments[0]);
            if !RULE_DECORATORS.contains(&name.as_ref()) {
                continue;
            }

            // extract the string argument (lint ID or code)
            let Some(arguments) = &decorator.arguments else {
                continue;
            };
            let Some(first_argument_id) = arguments.first() else {
                continue;
            };
            let first_argument = ctx.tree.get(*first_argument_id);
            let ast::Argument::Positional { value, .. } = first_argument else {
                continue;
            };
            let argument_expression = ctx.tree.get(*value);
            let ast::Expression::ScalarLiteral(ast::ScalarLiteral::String(string_id)) =
                argument_expression
            else {
                continue;
            };
            let specifier = ctx.strings.get(*string_id);

            // check if the specifier is a valid lint rule ID or code
            if is_valid_lint_specifier(specifier.as_ref()) {
                continue;
            }
            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            ctx.report(
                LintDiagnostic::new(
                    NO_UNKNOWN_RULE_DECORATOR.id,
                    NO_UNKNOWN_RULE_DECORATOR.code,
                    NO_UNKNOWN_RULE_DECORATOR.category,
                    severity,
                    format!("unknown lint rule '{}'", specifier.as_ref()),
                    ctx.module.file_id,
                    ctx.tree.get_span(*value),
                )
                .with_label("this lint rule does not exist"),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_unknown_rule_id() {
        let test = TestProgram::for_rule_without_builtins(NoUnknownRuleDecorator);
        let result = test.lint_ast(
            "test.ds",
            r#"
@allow("not-a-real-rule")
function foo() {}
"#,
        );
        test.result(result).assert_lint("no-unknown-rule-decorator");
    }

    #[test]
    fn test_detects_unknown_rule_code() {
        let test = TestProgram::for_rule_without_builtins(NoUnknownRuleDecorator);
        let result = test.lint_ast(
            "test.ds",
            r#"
@deny("ZZ999")
function foo() {}
"#,
        );
        test.result(result).assert_lint("no-unknown-rule-decorator");
    }

    #[test]
    fn test_allows_valid_rule_id() {
        let test = TestProgram::for_rule_without_builtins(NoUnknownRuleDecorator);
        let result = test.lint_ast(
            "test.ds",
            r#"
@allow("no-empty")
function foo() {}
"#,
        );
        test.result(result)
            .assert_no_lint("no-unknown-rule-decorator");
    }

    #[test]
    fn test_allows_valid_rule_code() {
        let test = TestProgram::for_rule_without_builtins(NoUnknownRuleDecorator);
        let result = test.lint_ast(
            "test.ds",
            r#"
@warn("LU002")
function foo() {}
"#,
        );
        test.result(result)
            .assert_no_lint("no-unknown-rule-decorator");
    }

    #[test]
    fn test_checks_all_decorator_types() {
        let test = TestProgram::for_rule_without_builtins(NoUnknownRuleDecorator);
        let result = test.lint_ast(
            "test.ds",
            r#"
@forbid("fake-rule")
function foo() {}
"#,
        );
        test.result(result).assert_lint("no-unknown-rule-decorator");
    }

    #[test]
    fn test_ignores_other_decorators() {
        let test = TestProgram::for_rule_without_builtins(NoUnknownRuleDecorator);
        let result = test.lint_ast(
            "test.ds",
            r#"
@deprecated("use bar instead")
function foo() {}
"#,
        );
        test.result(result)
            .assert_no_lint("no-unknown-rule-decorator");
    }
}
