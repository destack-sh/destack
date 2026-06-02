use crate::LintMeta;
use std::sync::LazyLock;

use destack_dir as dir;
use destack_dir::LanguageItem;
use destack_source::Span;
use destack_workspace::LintSeverity;

use crate::rules::common::{
    expression_is_unqualified_path_name, is_simple_identifier, subtree_mentions_identifier_name,
};
use crate::{LintFix, LintModuleContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow shadowing of restricted or builtin names.
    ///
    /// Shadowing names like `NaN`, `Infinity`, `eval`, or `arguments` can lead
    /// to confusing behavior and potential bugs. Also warns when shadowing
    /// Destack language items like `Add` or `Type`.
    #[lint(
        id = "no-shadow-restricted-names",
        code = "LU029",
        category = Suspicious,
        level = Dir,
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
    "undefined",
];

/// Sorted list of all restricted names (JS globals + language items).
static RESTRICTED_NAMES: LazyLock<Vec<&'static str>> = LazyLock::new(|| {
    let mut names: Vec<&'static str> = JS_GLOBALS.to_vec();
    names.extend(LanguageItem::all().map(|item| item.export_name()));
    names.sort_unstable();
    names.dedup();
    names
});

/// Check if a name is a restricted name.
fn is_restricted_name(name: &str) -> bool {
    RESTRICTED_NAMES.binary_search(&name).is_ok()
}

impl LintRule for NoShadowRestrictedNames {
    fn meta(&self) -> &'static LintMeta {
        NoShadowRestrictedNames::meta()
    }

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();

        // check patterns (let/const bindings, function params, etc.)
        for node_id in ctx.dir.iter_nodes::<dir::Pattern>() {
            let pattern = ctx.dir.get(node_id);

            let name = match pattern {
                dir::Pattern::Binding { name, .. } => *name,
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

            let mut diagnostic = LintReport::new(
                NO_SHADOW_RESTRICTED_NAMES.id,
                NO_SHADOW_RESTRICTED_NAMES.code,
                NO_SHADOW_RESTRICTED_NAMES.category,
                severity,
                format!("shadowing of restricted name '{name_str}'"),
                ctx.dir.get_span(node_id),
            )
            .label(format!("'{name_str}' is a restricted name"));
            if ctx.compute_fixes
                && let Some(fix) = no_shadow_restricted_names_fix_for_pattern(ctx, node_id, name)
            {
                diagnostic = diagnostic.fix(fix);
            }

            ctx.report(diagnostic);
        }

        // check function/class/struct declarations
        for node_id in ctx.dir.iter_nodes::<dir::Declaration>() {
            let declaration = ctx.dir.get(node_id);
            let name = declaration.name();
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
                LintReport::new(
                    NO_SHADOW_RESTRICTED_NAMES.id,
                    NO_SHADOW_RESTRICTED_NAMES.code,
                    NO_SHADOW_RESTRICTED_NAMES.category,
                    severity,
                    format!("shadowing of restricted name '{name_str}'"),
                    ctx.dir.get_span(node_id),
                )
                .label(format!("'{name_str}' is a restricted name")),
            );
        }

        // check function parameters
        for node_id in ctx.dir.iter_nodes::<dir::Parameter>() {
            let param = ctx.dir.get(node_id);

            let name = match param {
                dir::Parameter::Named { name, .. } => *name,
                dir::Parameter::VariadicNamed { name, .. } => *name,
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

            let mut diagnostic = LintReport::new(
                NO_SHADOW_RESTRICTED_NAMES.id,
                NO_SHADOW_RESTRICTED_NAMES.code,
                NO_SHADOW_RESTRICTED_NAMES.category,
                severity,
                format!("shadowing of restricted name '{name_str}'"),
                ctx.dir.get_span(node_id),
            )
            .label(format!("'{name_str}' is a restricted name"));
            if ctx.compute_fixes
                && let Some(fix) = no_shadow_restricted_names_fix_for_parameter(ctx, node_id, name)
            {
                diagnostic = diagnostic.fix(fix);
            }

            ctx.report(diagnostic);
        }
    }
}

/// Build one suggestion rename fix for one pattern binding.
fn no_shadow_restricted_names_fix_for_pattern(
    ctx: &LintModuleContext<'_>,
    pattern_id: dir::LocalNodeId<dir::Pattern>,
    name: dir::StringId,
) -> Option<LintFix> {
    no_shadow_restricted_names_fix(ctx, pattern_id.id, ctx.dir.get_span(pattern_id), name)
}

/// Build one suggestion rename fix for one parameter binding.
fn no_shadow_restricted_names_fix_for_parameter(
    ctx: &LintModuleContext<'_>,
    parameter_id: dir::LocalNodeId<dir::Parameter>,
    name: dir::StringId,
) -> Option<LintFix> {
    no_shadow_restricted_names_fix(ctx, parameter_id.id, ctx.dir.get_span(parameter_id), name)
}

/// Build one suggestion rename fix when declaration is scope-local and unreferenced.
fn no_shadow_restricted_names_fix(
    ctx: &LintModuleContext<'_>,
    node_id: u32,
    declaration_span: Span,
    name: dir::StringId,
) -> Option<LintFix> {
    let name_text = ctx.strings.get(name).to_string();
    if !is_simple_identifier(&name_text) {
        return None;
    }

    // resolve the relevant declaration scope once
    let scope_root = identifier_scope_root(ctx, node_id);

    // keep declaration-only bindings to avoid unresolved references
    if scope_has_unqualified_path_reference(ctx, scope_root, name) {
        return None;
    }

    let name_span = exact_identifier_span(ctx, declaration_span, &name_text)?;
    let replacement_name = restricted_name_replacement(ctx, scope_root, &name_text)?;
    let edits = ctx
        .edit_builder()
        .replace(name_span, replacement_name.clone())
        .into_edits();
    Some(
        LintFix::suggestion(format!("Rename restricted binding to `{replacement_name}`"))
            .with_edits(edits),
    )
}

/// Build one replacement name for restricted identifiers in the local declaration scope.
fn restricted_name_replacement(
    ctx: &LintModuleContext<'_>,
    scope_root: IdentifierScopeRoot,
    name: &str,
) -> Option<String> {
    let base_name = format!("{name}Local");
    let base_name_id = ctx.string_id(&base_name);
    if !scope_mentions_identifier_name(ctx, scope_root, base_name_id) {
        return Some(base_name);
    }

    let mut suffix = 2_u32;
    loop {
        let candidate = format!("{name}Local{suffix}");
        let candidate_id = ctx.string_id(&candidate);
        if !scope_mentions_identifier_name(ctx, scope_root, candidate_id) {
            return Some(candidate);
        }
        suffix += 1;
        if suffix > 1024 {
            return Some(base_name);
        }
    }
}

/// One declaration scope root used for local rename suggestions.
#[derive(Clone, Copy, Debug)]
enum IdentifierScopeRoot {
    /// The whole module scope.
    Module,
    /// One source subtree root that establishes a declaration scope.
    Node(dir::NodeType, u32),
}

/// Return the nearest declaration-scope subtree root for one identifier node.
fn identifier_scope_root(ctx: &LintModuleContext<'_>, node_id: u32) -> IdentifierScopeRoot {
    let mut current_id = node_id;

    loop {
        let Some(parent_id) = ctx.dir.get_parent_id(current_id) else {
            return IdentifierScopeRoot::Module;
        };
        let parent_type = ctx.dir.get_node_type(parent_id);

        // block bodies establish the local declaration scope
        if parent_type == dir::NodeType::Block {
            return IdentifierScopeRoot::Node(parent_type, parent_id);
        }

        // callable owners establish parameter and method scopes
        if parent_type == dir::NodeType::Declaration {
            let declaration = ctx
                .dir
                .tree()
                .get(dir::LocalNodeId::<dir::Declaration>::new(parent_id));
            if matches!(declaration, dir::Declaration::Function(_)) {
                return IdentifierScopeRoot::Node(parent_type, parent_id);
            }
        }
        if parent_type == dir::NodeType::Member {
            let member = ctx
                .dir
                .tree()
                .get(dir::LocalNodeId::<dir::Member>::new(parent_id));
            if matches!(member, dir::Member::Method { .. }) {
                return IdentifierScopeRoot::Node(parent_type, parent_id);
            }
        }
        if parent_type == dir::NodeType::Property {
            let property = ctx
                .dir
                .tree()
                .get(dir::LocalNodeId::<dir::Property>::new(parent_id));
            if matches!(property, dir::Property::Method { .. }) {
                return IdentifierScopeRoot::Node(parent_type, parent_id);
            }
        }

        current_id = parent_id;
    }
}

/// Return true when one scope subtree mentions one identifier name.
fn scope_mentions_identifier_name(
    ctx: &LintModuleContext<'_>,
    scope_root: IdentifierScopeRoot,
    name: dir::StringId,
) -> bool {
    match scope_root {
        IdentifierScopeRoot::Module => ctx.roots.iter().copied().any(|root_id| {
            subtree_mentions_identifier_name(
                ctx.dir.tree(),
                dir::NodeType::Expression,
                root_id.id,
                name,
            )
        }),
        IdentifierScopeRoot::Node(node_type, node_id) => {
            subtree_mentions_identifier_name(ctx.dir.tree(), node_type, node_id, name)
        }
    }
}

/// Return true when one scope subtree contains one unqualified path reference.
fn scope_has_unqualified_path_reference(
    ctx: &LintModuleContext<'_>,
    scope_root: IdentifierScopeRoot,
    name: dir::StringId,
) -> bool {
    let mut visitor = ScopeReferenceSearchVisitor {
        options: dir::NodeVisitorOptions::default(),
        name,
        found_reference: false,
    };

    match scope_root {
        IdentifierScopeRoot::Module => {
            for root_id in ctx.roots.iter().copied() {
                if visitor.found_reference {
                    break;
                }

                dir::walk_any(
                    &mut visitor,
                    ctx.dir.tree(),
                    dir::NodeType::Expression,
                    root_id.id,
                );
            }
        }
        IdentifierScopeRoot::Node(node_type, node_id) => {
            dir::walk_any(&mut visitor, ctx.dir.tree(), node_type, node_id);
        }
    }

    visitor.found_reference
}

/// Visitor that finds one unqualified path reference in a scope subtree.
struct ScopeReferenceSearchVisitor {
    /// Visitor options.
    options: dir::NodeVisitorOptions,
    /// The target identifier name.
    name: dir::StringId,
    /// Whether a matching reference has been found.
    found_reference: bool,
}

impl dir::NodeVisitor for ScopeReferenceSearchVisitor {
    fn options(&self) -> &dir::NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        expression_id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // stop once one reference has been found
        if self.found_reference {
            return;
        }

        // match direct unqualified path references only
        if expression_is_unqualified_path_name(tree, expression_id, self.name) {
            self.found_reference = true;
            return;
        }

        dir::walk_expression(self, tree, expression_id, expression);
    }
}

/// Return the declaration span when it is exactly the identifier token.
fn exact_identifier_span(
    ctx: &LintModuleContext<'_>,
    search_span: Span,
    identifier: &str,
) -> Option<Span> {
    let source_text = ctx.get_span_text(search_span);
    if source_text == identifier {
        return Some(search_span);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    #[test]
    fn test_detects_nan_variable() {
        let test = TestProgram::for_rule_without_prelude(NoShadowRestrictedNames);
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
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
    fn test_detects_language_item_type() {
        let test = TestProgram::for_rule_without_prelude(NoShadowRestrictedNames);
        let result = test.lint(
            "no_shadow_restricted_names/test_detects_language_item_type.ds",
            r#"
let Type = 42
"#,
        );
        test.result(result)
            .assert_lint("no-shadow-restricted-names");
    }

    #[test]
    fn test_detects_language_item_add() {
        let test = TestProgram::for_rule_without_prelude(NoShadowRestrictedNames);
        let result = test.lint(
            "no_shadow_restricted_names/test_detects_language_item_add.ds",
            r#"
struct Add {}
"#,
        );
        test.result(result)
            .assert_lint("no-shadow-restricted-names");
    }

    #[test]
    fn test_allows_removed_language_item_range() {
        let test = TestProgram::for_rule_without_prelude(NoShadowRestrictedNames);
        let result = test.lint(
            "no_shadow_restricted_names/test_allows_removed_language_item_range.ds",
            r#"
function Range() {}
"#,
        );
        test.result(result)
            .assert_no_lint("no-shadow-restricted-names");
    }

    #[test]
    fn test_fix_renames_unreferenced_restricted_binding() {
        let test = TestProgram::for_rule_without_prelude(NoShadowRestrictedNames);
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
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
        let result = test.lint(
            "no_shadow_restricted_names/test_fix_ignores_comment_occurrences_of_restricted_name.ds",
            r#"
// nan appears in a comment but should not block the rename fix
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
// nan appears in a comment but should not block the rename fix
let NaNLocal = 0;
"#
            .trim()
        );
    }
}
