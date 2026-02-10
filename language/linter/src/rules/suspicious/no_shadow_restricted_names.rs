use std::sync::LazyLock;

use destack_ast as ast;
use destack_builtin::LanguageSymbol;
use destack_source::Span;
use destack_workspace::LintSeverity;

use crate::{LintDiagnostic, LintFix, LintModuleAstContext, LintRule, declare_lint};

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
        fixable = Sometimes,
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

            let name_str = ctx.strings.get(name).to_string();
            if !is_restricted_name(&name_str) {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            let mut diagnostic = LintDiagnostic::new(
                NO_SHADOW_RESTRICTED_NAMES.id,
                NO_SHADOW_RESTRICTED_NAMES.code,
                NO_SHADOW_RESTRICTED_NAMES.category,
                severity,
                format!("shadowing of restricted name '{}'", name_str),
                ctx.module.file_id,
                ctx.tree.get_span(node_id),
            )
            .with_label(format!("'{}' is a restricted name", name_str));
            if ctx.compute_fixes
                && let Some(fix) =
                    no_shadow_restricted_names_fix(ctx, ctx.tree.get_span(node_id), name)
            {
                diagnostic = diagnostic.with_fix(fix);
            }

            ctx.report(diagnostic);
        }

        // check function/class/struct declarations
        for node_id in ctx.tree.iter_nodes::<ast::Declaration>() {
            let declaration = ctx.tree.get(node_id);
            let name = declaration.descriptor().name;
            let Some(name) = name else {
                continue;
            };

            let name_str = ctx.strings.get(name.string()).to_string();
            if !is_restricted_name(&name_str) {
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
                    format!("shadowing of restricted name '{}'", name_str),
                    ctx.module.file_id,
                    ctx.tree.get_span(node_id),
                )
                .with_label(format!("'{}' is a restricted name", name_str)),
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

            let name_str = ctx.strings.get(name).to_string();
            if !is_restricted_name(&name_str) {
                continue;
            }

            let severity = ctx.get_effective_severity(meta, node_id);
            if !severity.is_enabled() {
                continue;
            }

            let mut diagnostic = LintDiagnostic::new(
                NO_SHADOW_RESTRICTED_NAMES.id,
                NO_SHADOW_RESTRICTED_NAMES.code,
                NO_SHADOW_RESTRICTED_NAMES.category,
                severity,
                format!("shadowing of restricted name '{}'", name_str),
                ctx.module.file_id,
                ctx.tree.get_span(node_id),
            )
            .with_label(format!("'{}' is a restricted name", name_str));
            if ctx.compute_fixes
                && let Some(fix) =
                    no_shadow_restricted_names_fix(ctx, ctx.tree.get_span(node_id), name)
            {
                diagnostic = diagnostic.with_fix(fix);
            }

            ctx.report(diagnostic);
        }
    }
}

/// Build one suggestion rename fix when declaration is file-local and unreferenced.
fn no_shadow_restricted_names_fix(
    ctx: &LintModuleAstContext<'_>,
    declaration_span: Span,
    name: ast::StringId,
) -> Option<LintFix> {
    let name_text = ctx.strings.get(name).to_string();
    if !is_simple_identifier(&name_text) {
        return None;
    }

    // keep declaration-only bindings to avoid unresolved references
    if has_unqualified_path_reference(ctx, name) {
        return None;
    }

    let name_span = find_unique_identifier_span_in_span(ctx, declaration_span, &name_text)?;
    let replacement_name = restricted_name_replacement(ctx, &name_text);
    let edits = ctx
        .edit_builder()
        .replace(name_span, replacement_name.clone())
        .into_edits();
    Some(
        LintFix::suggestion(format!("Rename restricted binding to `{replacement_name}`"))
            .with_edits(edits),
    )
}

/// Build one replacement name for restricted identifiers.
fn restricted_name_replacement(ctx: &LintModuleAstContext<'_>, name: &str) -> String {
    let base_name = format!("{name}Local");
    let base_name_id = ctx.strings.intern(&base_name);
    if !identifier_name_exists_in_ast(ctx, base_name_id) {
        return base_name;
    }

    let mut suffix = 2_u32;
    loop {
        let candidate = format!("{name}Local{suffix}");
        let candidate_id = ctx.strings.intern(&candidate);
        if !identifier_name_exists_in_ast(ctx, candidate_id) {
            return candidate;
        }
        suffix += 1;
        if suffix > 1024 {
            return base_name;
        }
    }
}

/// Return true when one unqualified path references this identifier.
fn has_unqualified_path_reference(ctx: &LintModuleAstContext<'_>, name: ast::StringId) -> bool {
    for expression_id in ctx.tree.iter_nodes::<ast::Expression>() {
        let expression = ctx.tree.get(expression_id);
        if let ast::Expression::Path { path, .. } = expression
            && path.segments.len() == 1
            && path.segments[0] == name
        {
            return true;
        }
    }

    false
}

/// Return true when this identifier name is already present in declarations or paths.
fn identifier_name_exists_in_ast(ctx: &LintModuleAstContext<'_>, name: ast::StringId) -> bool {
    for pattern_id in ctx.tree.iter_nodes::<ast::Pattern>() {
        let pattern = ctx.tree.get(pattern_id);
        if let ast::Pattern::Binding {
            name: binding_name, ..
        } = pattern
            && *binding_name == name
        {
            return true;
        }
    }

    for parameter_id in ctx.tree.iter_nodes::<ast::Parameter>() {
        let parameter = ctx.tree.get(parameter_id);
        match parameter {
            ast::Parameter::Named {
                name: parameter_name,
                ..
            }
            | ast::Parameter::VariadicNamed {
                name: parameter_name,
                ..
            } if *parameter_name == name => {
                return true;
            }
            _ => {}
        }
    }

    for declaration_id in ctx.tree.iter_nodes::<ast::Declaration>() {
        let declaration = ctx.tree.get(declaration_id);
        if declaration
            .descriptor()
            .name
            .is_some_and(|declaration_name| declaration_name.string() == name)
        {
            return true;
        }
    }

    for expression_id in ctx.tree.iter_nodes::<ast::Expression>() {
        let expression = ctx.tree.get(expression_id);
        if let ast::Expression::Path { path, .. } = expression
            && path.segments.len() == 1
            && path.segments[0] == name
        {
            return true;
        }
    }

    false
}

/// Return one unique identifier span in a declaration span.
fn find_unique_identifier_span_in_span(
    ctx: &LintModuleAstContext<'_>,
    search_span: Span,
    identifier: &str,
) -> Option<Span> {
    let source_text = ctx.get_span_text(search_span);
    let mut matches = Vec::new();
    let mut offset = 0usize;
    while let Some(found) = source_text[offset..].find(identifier) {
        let start = offset + found;
        let end = start + identifier.len();
        if is_identifier_boundary(source_text, start, end) {
            matches.push((start, end));
        }
        offset = end;
    }

    if matches.len() != 1 {
        return None;
    }
    let (start, end) = matches[0];
    Some(Span::new(
        search_span.file,
        search_span.start + start as u32,
        search_span.start + end as u32,
    ))
}

/// Return true for identifier boundary positions.
fn is_identifier_boundary(source_text: &str, start: usize, end: usize) -> bool {
    let left = source_text[..start].chars().next_back();
    let right = source_text[end..].chars().next();
    !left.is_some_and(is_identifier_char) && !right.is_some_and(is_identifier_char)
}

/// Return true for identifier chars.
fn is_identifier_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '$'
}

/// Return true when one text is a simple identifier.
fn is_simple_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_' || first == '$') {
        return false;
    }
    chars.all(is_identifier_char)
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

    #[test]
    fn test_fix_renames_unreferenced_restricted_binding() {
        let test = TestProgram::for_rule_without_prelude(NoShadowRestrictedNames);
        let result = test.lint_ast(
            "no_shadow_restricted_names/test_fix_renames_unreferenced_restricted_binding.ds",
            r#"
let NaN = 0
"#,
        );
        test.result(result.clone())
            .assert_lint("no-shadow-restricted-names")
            .assert_has_fix("no-shadow-restricted-names");

        let fixed = test.result(result).apply_fixes(None);
        assert_eq!(
            fixed.trim(),
            r#"
let NaNLocal = 0;
"#
            .trim()
        );
    }

    #[test]
    fn test_no_fix_for_referenced_restricted_binding() {
        let test = TestProgram::for_rule_without_prelude(NoShadowRestrictedNames);
        let result = test.lint_ast(
            "no_shadow_restricted_names/test_no_fix_for_referenced_restricted_binding.ds",
            r#"
let NaN = 0
console.log(NaN)
"#,
        );
        test.result(result)
            .assert_lint("no-shadow-restricted-names")
            .assert_has_no_fix("no-shadow-restricted-names");
    }

    #[test]
    fn test_fix_uses_unique_suffix_when_local_name_already_exists() {
        let test = TestProgram::for_rule_without_prelude(NoShadowRestrictedNames);
        let result = test.lint_ast(
            "no_shadow_restricted_names/test_fix_uses_unique_suffix_when_local_name_already_exists.ds",
            r#"
let NaNLocal = 1
let NaN = 0
"#,
        );
        test.result(result.clone())
            .assert_lint("no-shadow-restricted-names")
            .assert_has_fix("no-shadow-restricted-names");

        let fixed = test.result(result).apply_fixes(None);
        assert_eq!(
            fixed.trim(),
            r#"
let NaNLocal = 1;
let NaNLocal2 = 0;
"#
            .trim()
        );
    }

    #[test]
    fn test_fix_ignores_comment_occurrences_of_restricted_name() {
        let test = TestProgram::for_rule_without_prelude(NoShadowRestrictedNames);
        let result = test.lint_ast(
            "no_shadow_restricted_names/test_fix_ignores_comment_occurrences_of_restricted_name.ds",
            r#"
// NaN appears in a comment but should not block the rename fix
let NaN = 0
"#,
        );
        test.result(result.clone())
            .assert_lint("no-shadow-restricted-names")
            .assert_has_fix("no-shadow-restricted-names");

        let fixed = test.result(result).apply_fixes(None);
        assert_eq!(
            fixed.trim(),
            r#"
// NaN appears in a comment but should not block the rename fix
let NaNLocal = 0;
"#
            .trim()
        );
    }
}
