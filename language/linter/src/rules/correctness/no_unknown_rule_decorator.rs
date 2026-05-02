use std::sync::LazyLock;

use destack_ast as ast;
use destack_workspace::LintSeverity;

use crate::rules::all_rules;
use crate::{LintAstContext, LintMeta, LintReport, LintRule, declare_lint};

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
        code = "LC029",
        category = Correctness,
        level = Ast,
        requires_all = [],
        requires_any = [],
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

/// Return true when one decorator specifier is a known lint id or code.
fn is_valid_lint_specifier(specifier: &str) -> bool {
    LINT_SPECIFIERS.binary_search(&specifier).is_ok()
}

/// Split one decorator string argument into lint specifiers.
fn decorator_lint_specifiers(argument_text: &str) -> Vec<&str> {
    // support either one literal specifier or comma separated specifiers
    if !argument_text.contains(',') {
        let specifier = argument_text.trim();
        if specifier.is_empty() {
            return Vec::new();
        }
        return vec![specifier];
    }

    argument_text
        .split(',')
        .map(str::trim)
        .filter(|specifier| !specifier.is_empty())
        .collect()
}

/// Decorator names that expect a lint rule specifier as their first argument.
const RULE_DECORATORS: &[&str] = &["allow", "deny", "forbid", "warn"];

impl LintRule for NoUnknownRuleDecorator {
    /// Return lint metadata.
    fn meta(&self) -> &'static LintMeta {
        NoUnknownRuleDecorator::meta()
    }

    /// Check module AST nodes for unknown lint decorators.
    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintAstContext<'a>) {
        let meta = self.meta();

        // inspect decorator side nodes
        for node_id in ctx.tree.iter_nodes::<ast::Decorator>() {
            // check decorator name (must be single segment: allow, warn, deny, forbid)
            let Some(path) = ctx.decorator_path(node_id) else {
                continue;
            };
            let Some(last_segment) = path.last() else {
                continue;
            };

            // resolve name
            let name = ctx.strings.get(*last_segment);
            if !RULE_DECORATORS.contains(&name.as_ref()) {
                continue;
            }

            // inspect all positional string arguments (lint IDs or codes)
            let Some(arguments) = ctx.decorator_call(node_id).arguments else {
                continue;
            };
            let argument_ids = arguments.to_vec();

            // inspect candidate nodes
            for argument_id in argument_ids {
                let argument = ctx.tree.get(argument_id);
                let ast::Argument::Positional { value, .. } = argument else {
                    continue;
                };
                let argument_expression = ctx.tree.get(*value);
                let ast::Expression::ScalarLiteral(ast::ScalarLiteral::String(string_id)) =
                    argument_expression
                else {
                    continue;
                };
                let argument_text = ctx.strings.get(*string_id).to_string();
                let specifiers = decorator_lint_specifiers(argument_text.as_ref());

                // inspect candidate nodes
                for specifier in specifiers {
                    // check if the specifier is a valid lint rule ID or code
                    if is_valid_lint_specifier(specifier) {
                        continue;
                    }
                    let severity = ctx.get_effective_severity(meta, node_id);
                    if !severity.is_enabled() {
                        continue;
                    }

                    ctx.report(
                        LintReport::new(
                            NO_UNKNOWN_RULE_DECORATOR.id,
                            NO_UNKNOWN_RULE_DECORATOR.code,
                            NO_UNKNOWN_RULE_DECORATOR.category,
                            severity,
                            format!("unknown lint rule '{specifier}'"),
                            ctx.tree.get_span(*value),
                        )
                        .label("this lint rule does not exist"),
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_unknown_rule_id() {
        let test = TestProgram::for_rule_without_prelude(NoUnknownRuleDecorator);
        let result = test.lint_ast(
            "no_unknown_rule_decorator/test_detects_unknown_rule_id.ds",
            r#"
@allow("not-a-real-rule")
function foo() {}
"#,
        );
        test.result(result).assert_lint("no-unknown-rule-decorator");
    }

    #[test]
    fn test_detects_unknown_rule_code() {
        let test = TestProgram::for_rule_without_prelude(NoUnknownRuleDecorator);
        let result = test.lint_ast(
            "no_unknown_rule_decorator/test_detects_unknown_rule_code.ds",
            r#"
@deny("ZZ999")
function foo() {}
"#,
        );
        test.result(result).assert_lint("no-unknown-rule-decorator");
    }

    #[test]
    fn test_allows_valid_rule_id() {
        let test = TestProgram::for_rule_without_prelude(NoUnknownRuleDecorator);
        let result = test.lint_ast(
            "no_unknown_rule_decorator/test_allows_valid_rule_id.ds",
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
        let test = TestProgram::for_rule_without_prelude(NoUnknownRuleDecorator);
        let result = test.lint_ast(
            "no_unknown_rule_decorator/test_allows_valid_rule_code.ds",
            r#"
@warn("LU014")
function foo() {}
"#,
        );
        test.result(result)
            .assert_no_lint("no-unknown-rule-decorator");
    }

    #[test]
    fn test_checks_all_decorator_types() {
        let test = TestProgram::for_rule_without_prelude(NoUnknownRuleDecorator);
        let result = test.lint_ast(
            "no_unknown_rule_decorator/test_checks_all_decorator_types.ds",
            r#"
@forbid("fake-rule")
function foo() {}
"#,
        );
        test.result(result).assert_lint("no-unknown-rule-decorator");
    }

    #[test]
    fn test_ignores_other_decorators() {
        let test = TestProgram::for_rule_without_prelude(NoUnknownRuleDecorator);
        let result = test.lint_ast(
            "no_unknown_rule_decorator/test_ignores_other_decorators.ds",
            r#"
@deprecated("use bar instead")
function foo() {}
"#,
        );
        test.result(result)
            .assert_no_lint("no-unknown-rule-decorator");
    }

    #[test]
    fn test_checks_all_string_arguments() {
        let test = TestProgram::for_rule_without_prelude(NoUnknownRuleDecorator);
        let result = test.lint_ast(
            "no_unknown_rule_decorator/test_checks_all_string_arguments.ds",
            r#"
@allow("no-empty", "made-up-rule", "LC003")
function foo() {}
"#,
        );
        test.result(result).assert_lint("no-unknown-rule-decorator");
    }

    #[test]
    fn test_checks_comma_separated_lint_specifiers() {
        let test = TestProgram::for_rule_without_prelude(NoUnknownRuleDecorator);
        let result = test.lint_ast(
            "no_unknown_rule_decorator/test_checks_comma_separated_lint_specifiers.ds",
            r#"
@allow("no-empty, made-up-rule, LC003")
function foo() {}
"#,
        );
        test.result(result).assert_lint("no-unknown-rule-decorator");
    }
}
