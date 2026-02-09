use std::sync::LazyLock;

use destack_ast as ast;
use destack_builtin::LanguageSymbol;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintModuleAstContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow shadowing of restricted or builtin names.
    ///
    /// Shadowing names like `NaN`, `Infinity`, `eval`, or `arguments` can lead
    /// to confusing behavior and potential bugs. Also warns when shadowing
    /// Destack language items like `Add`, `Type`, or `Range`.
    #[lint(
        id = "no-shadow-restricted-names",
        code = "LU029",
        category = Suspicious,
        level = Ast,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Always,
        stability = Stable
    )]
    pub NoShadowRestrictedNames,
    "Disallow shadowing of restricted or builtin names"
}

/// JavaScript global names that should not be shadowed (for interop).
const JS_GLOBALS: &[&str] = &[
    "Array",
    "Boolean",
    "Error",
    "Function",
    "Infinity",
    "JSON",
    "Math",
    "NaN",
    "Number",
    "Object",
    "Promise",
    "String",
    "Symbol",
    "arguments",
    "console",
    "eval",
    "globalThis",
];

/// Sorted list of all restricted names (JS globals + language items).
static RESTRICTED_NAMES: LazyLock<Vec<&'static str>> = LazyLock::new(|| {
    let mut names: Vec<&'static str> = JS_GLOBALS.to_vec();
    names.extend(LanguageSymbol::all().map(|item| item.export_name()));
    names.sort_unstable();
    names.dedup();
    names
});

/// Check if a name is a restricted name.
fn is_restricted_name(name: &str) -> bool {
    RESTRICTED_NAMES.binary_search(&name).is_ok()
}

impl LintRule for NoShadowRestrictedNames {
    fn meta(&self) -> &'static crate::LintMeta {
        NoShadowRestrictedNames::meta()
    }

    fn check_module_ast<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleAstContext<'a>) {
        let meta = self.meta();

        // check patterns (let/const bindings, function params, etc.)
        for node_id in ctx.tree.iter_nodes::<ast::Pattern>() {
            let pattern = ctx.tree.get(node_id);

            let name = match pattern {
                ast::Pattern::Binding { name, .. } => *name,
                _ => continue,
            };

            let name_str = ctx.strings.get(name);
            if !is_restricted_name(name_str.as_ref()) {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            ctx.report(
                LintDiagnostic::new(
                    NO_SHADOW_RESTRICTED_NAMES.id,
                    NO_SHADOW_RESTRICTED_NAMES.code,
                    NO_SHADOW_RESTRICTED_NAMES.category,
                    severity,
                    format!("shadowing of restricted name '{}'", name_str.as_ref()),
                    ctx.module.file_id,
                    ctx.tree.get_span(node_id),
                )
                .with_label(format!("'{}' is a restricted name", name_str.as_ref())),
            );
        }

        // check function/class/struct declarations
        for node_id in ctx.tree.iter_nodes::<ast::Declaration>() {
            let declaration = ctx.tree.get(node_id);
            let name = declaration.descriptor().name;
            let Some(name) = name else {
                continue;
            };

            let name_str = ctx.strings.get(name.string());
            if !is_restricted_name(name_str.as_ref()) {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            ctx.report(
                LintDiagnostic::new(
                    NO_SHADOW_RESTRICTED_NAMES.id,
                    NO_SHADOW_RESTRICTED_NAMES.code,
                    NO_SHADOW_RESTRICTED_NAMES.category,
                    severity,
                    format!("shadowing of restricted name '{}'", name_str.as_ref()),
                    ctx.module.file_id,
                    ctx.tree.get_span(node_id),
                )
                .with_label(format!("'{}' is a restricted name", name_str.as_ref())),
            );
        }

        // check function parameters
        for node_id in ctx.tree.iter_nodes::<ast::Parameter>() {
            let param = ctx.tree.get(node_id);

            let name = match param {
                ast::Parameter::Named { name, .. } => *name,
                ast::Parameter::VariadicNamed { name, .. } => *name,
                _ => continue,
            };

            let name_str = ctx.strings.get(name);
            if !is_restricted_name(name_str.as_ref()) {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            ctx.report(
                LintDiagnostic::new(
                    NO_SHADOW_RESTRICTED_NAMES.id,
                    NO_SHADOW_RESTRICTED_NAMES.code,
                    NO_SHADOW_RESTRICTED_NAMES.category,
                    severity,
                    format!("shadowing of restricted name '{}'", name_str.as_ref()),
                    ctx.module.file_id,
                    ctx.tree.get_span(node_id),
                )
                .with_label(format!("'{}' is a restricted name", name_str.as_ref())),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_nan_variable() {
        let test = TestProgram::for_rule_without_prelude(NoShadowRestrictedNames);
        let result = test.lint_ast(
            "no_shadow_restricted_names/test_detects_nan_variable.ds",
            r#"
let NaN = 0
"#,
        );
        test.result(result)
            .assert_lint("no-shadow-restricted-names");
    }

    #[test]
    fn test_detects_infinity_variable() {
        let test = TestProgram::for_rule_without_prelude(NoShadowRestrictedNames);
        let result = test.lint_ast(
            "no_shadow_restricted_names/test_detects_infinity_variable.ds",
            r#"
let Infinity = 100
"#,
        );
        test.result(result)
            .assert_lint("no-shadow-restricted-names");
    }

    #[test]
    fn test_detects_arguments_param() {
        let test = TestProgram::for_rule_without_prelude(NoShadowRestrictedNames);
        let result = test.lint_ast(
            "no_shadow_restricted_names/test_detects_arguments_param.ds",
            r#"
function foo(arguments: int) {}
"#,
        );
        test.result(result)
            .assert_lint("no-shadow-restricted-names");
    }

    #[test]
    fn test_detects_object_class() {
        let test = TestProgram::for_rule_without_prelude(NoShadowRestrictedNames);
        let result = test.lint_ast(
            "no_shadow_restricted_names/test_detects_object_class.ds",
            r#"
class Object {}
"#,
        );
        test.result(result)
            .assert_lint("no-shadow-restricted-names");
    }

    #[test]
    fn test_detects_array_function() {
        let test = TestProgram::for_rule_without_prelude(NoShadowRestrictedNames);
        let result = test.lint_ast(
            "no_shadow_restricted_names/test_detects_array_function.ds",
            r#"
function Array() {}
"#,
        );
        test.result(result)
            .assert_lint("no-shadow-restricted-names");
    }

    #[test]
    fn test_allows_normal_names() {
        let test = TestProgram::for_rule_without_prelude(NoShadowRestrictedNames);
        let result = test.lint_ast(
            "no_shadow_restricted_names/test_allows_normal_names.ds",
            r#"
const x = 42
let myVar = "hello"
function foo() {}
"#,
        );
        test.result(result)
            .assert_no_lint("no-shadow-restricted-names");
    }

    #[test]
    fn test_allows_similar_names() {
        let test = TestProgram::for_rule_without_prelude(NoShadowRestrictedNames);
        let result = test.lint_ast(
            "no_shadow_restricted_names/test_allows_similar_names.ds",
            r#"
const undefinedValue = 42
let isNaN = true
"#,
        );
        test.result(result)
            .assert_no_lint("no-shadow-restricted-names");
    }

    #[test]
    fn test_detects_language_symbol_type() {
        let test = TestProgram::for_rule_without_prelude(NoShadowRestrictedNames);
        let result = test.lint_ast(
            "no_shadow_restricted_names/test_detects_language_symbol_type.ds",
            r#"
let Type = 42
"#,
        );
        test.result(result)
            .assert_lint("no-shadow-restricted-names");
    }

    #[test]
    fn test_detects_language_symbol_add() {
        let test = TestProgram::for_rule_without_prelude(NoShadowRestrictedNames);
        let result = test.lint_ast(
            "no_shadow_restricted_names/test_detects_language_symbol_add.ds",
            r#"
struct Add {}
"#,
        );
        test.result(result)
            .assert_lint("no-shadow-restricted-names");
    }

    #[test]
    fn test_detects_language_symbol_range() {
        let test = TestProgram::for_rule_without_prelude(NoShadowRestrictedNames);
        let result = test.lint_ast(
            "no_shadow_restricted_names/test_detects_language_symbol_range.ds",
            r#"
function Range() {}
"#,
        );
        test.result(result)
            .assert_lint("no-shadow-restricted-names");
    }
}
