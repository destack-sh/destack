use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, walk_expression};
use destack_workspace::LintSeverity;

use crate::rules::common::{
    expression_import_target_specifier, expression_is_global_qualified_member,
    expression_static_string_literal, expression_unwrap_transparent, glob_matches,
};
use crate::{LintMeta, LintModuleDirContext, LintReport, LintRule, declare_lint};

declare_lint! {
    /// Disallow imports from configured banned module specifiers.
    ///
    /// This rule checks import and reexport targets against
    /// `linter.restrictedImports` patterns.
    #[lint(
        id = "no-banned-import",
        code = "LR003",
        category = Restriction,
        level = Dir,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Off,
        stability = Stable,
        declarations = Exclude
    )]
    pub NoBannedImport,
    "Disallow imports from configured module specifiers"
}

impl LintRule for NoBannedImport {
    fn meta(&self) -> &'static LintMeta {
        NoBannedImport::meta()
    }

    fn check_module_dir<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleDirContext<'a>) {
        let meta = self.meta();

        // walk the module expression tree
        let mut visitor = NoBannedImportVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Node visitor that reports banned import targets.
struct NoBannedImportVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleDirContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The restricted import patterns for this run.
    restricted_patterns: Vec<String>,
    /// The `require` identifier.
    require_name: destack_core::StringId,
    /// The global qualifier symbols.
    global_qualifiers: Vec<dir::GlobalSymbolId>,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> NoBannedImportVisitor<'a, 'b> {
    /// Build a visitor for no-banned-import checks.
    fn new(ctx: &'a mut LintModuleDirContext<'b>, meta: &'a LintMeta) -> Self {
        let restricted_patterns = ctx
            .options
            .restriction
            .restricted_imports
            .iter()
            .map(|pattern| pattern.trim())
            .filter(|pattern| !pattern.is_empty())
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        let require_name = ctx.string_id("require");
        let global_qualifiers = ctx.global_qualifier_symbols();

        Self {
            ctx,
            meta,
            restricted_patterns,
            require_name,
            global_qualifiers,
            options: NodeVisitorOptions::default(),
        }
    }

    /// Walk the DIR tree roots.
    fn run(&mut self) {
        // skip when no patterns are configured
        if self.restricted_patterns.is_empty() {
            return;
        }

        // capture roots and tree references
        let roots = self.ctx.roots.clone();
        let tree = self.ctx.tree;

        // walk the module expression tree
        for root_id in roots {
            let expression = tree.get(root_id);
            self.visit_expression(tree, root_id, expression);
        }
    }

    /// Check one expression for banned import targets.
    fn check_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // resolve the static module specifier
        let specifier_id =
            expression_import_target_specifier(self.ctx.tree, expression).or_else(|| {
                expression_require_target_specifier(
                    self.ctx.tree,
                    expression,
                    &self.global_qualifiers,
                    self.require_name,
                )
            });
        let Some(specifier_id) = specifier_id else {
            return;
        };
        let specifier_text = self.ctx.strings.get(specifier_id).to_string();

        // match specifier or resolved target identity against restricted patterns
        let Some(matched_target) = matching_target(
            self.ctx,
            expression,
            &specifier_text,
            &self.restricted_patterns,
        ) else {
            return;
        };

        // honor per node severity
        let severity = self.ctx.get_effective_severity(self.meta, expression_id);
        if !severity.is_enabled() {
            return;
        }

        // report one banned import target
        let span = self.ctx.get_span(expression_id);
        self.ctx.report(
            LintReport::new(
                NO_BANNED_IMPORT.id,
                NO_BANNED_IMPORT.code,
                NO_BANNED_IMPORT.category,
                severity,
                format!("banned import target `{specifier_text}`"),
                span,
            )
            .label("this import target is restricted by project configuration")
            .note(format!(
                "matched restricted pattern `{}` against {} `{}`",
                matched_target.pattern,
                matched_target.surface.label(),
                matched_target.target
            )),
        );
    }
}

impl NodeVisitor for NoBannedImportVisitor<'_, '_> {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // inspect this expression for import targets
        self.check_expression(id, expression);

        // walk expression children
        walk_expression(self, tree, id, expression);
    }
}

/// A matched import target and match source details.
struct MatchedTarget {
    /// The restricted pattern that matched.
    pattern: String,
    /// The text that matched the restricted pattern.
    target: String,
    /// The semantic surface used for matching.
    surface: TargetSurface,
}

/// The semantic surface used for restricted import matching.
enum TargetSurface {
    /// The original import specifier text.
    Specifier,
    /// The resolved module path.
    ResolvedModulePath,
    /// The resolved module file name.
    ResolvedModuleName,
    /// The resolved `declare module` binding specifier.
    ResolvedBindingSpecifier,
}

impl TargetSurface {
    /// Return a human readable label for diagnostics.
    fn label(&self) -> &'static str {
        match self {
            Self::Specifier => "import specifier",
            Self::ResolvedModulePath => "resolved module path",
            Self::ResolvedModuleName => "resolved module file name",
            Self::ResolvedBindingSpecifier => "resolved module binding",
        }
    }
}

/// Return the resolved module target for an import like expression.
fn expression_target_module(expression: &dir::Expression) -> Option<dir::ModuleTarget> {
    match expression {
        dir::Expression::Import { target_module, .. }
        | dir::Expression::ReExport { target_module, .. } => Some(*target_module),
        _ => None,
    }
}

/// Return the first restricted pattern that matches a target specifier.
fn matching_pattern(target: &str, patterns: &[String]) -> Option<String> {
    patterns
        .iter()
        .find(|pattern| glob_matches(pattern.as_str(), target))
        .cloned()
}

/// Return the first restricted target match for one import expression.
fn matching_target(
    ctx: &LintModuleDirContext<'_>,
    expression: &dir::Expression,
    specifier_text: &str,
    patterns: &[String],
) -> Option<MatchedTarget> {
    // check the explicit specifier first
    if let Some(pattern) = matching_pattern(specifier_text, patterns) {
        return Some(MatchedTarget {
            pattern,
            target: specifier_text.to_string(),
            surface: TargetSurface::Specifier,
        });
    }

    // resolve target module
    let target_module = expression_target_module(expression)?;
    match target_module {
        dir::ModuleTarget::Module(module_id) => {
            let module = ctx.repository_module(module_id)?;
            let module = module.as_ref();

            // check resolved module path next
            if let Some(path) = module.path.as_ref() {
                let path_text = path.to_string_lossy().to_string();
                if let Some(pattern) = matching_pattern(&path_text, patterns) {
                    return Some(MatchedTarget {
                        pattern,
                        target: path_text,
                        surface: TargetSurface::ResolvedModulePath,
                    });
                }
            }

            // check resolved file name as a fallback
            let file = ctx.repository_file(module.file_id)?;
            if let Some(pattern) = matching_pattern(&file.name, patterns) {
                return Some(MatchedTarget {
                    pattern,
                    target: file.name.clone(),
                    surface: TargetSurface::ResolvedModuleName,
                });
            }
        }
        dir::ModuleTarget::Binding(binding_specifier) => {
            let binding_text = ctx.strings.get(binding_specifier);
            if let Some(pattern) = matching_pattern(binding_text.as_ref(), patterns) {
                return Some(MatchedTarget {
                    pattern,
                    target: binding_text.to_string(),
                    surface: TargetSurface::ResolvedBindingSpecifier,
                });
            }
        }
        dir::ModuleTarget::External(specifier) => {
            let binding_text = ctx.strings.get(specifier);
            if let Some(pattern) = matching_pattern(binding_text.as_ref(), patterns) {
                return Some(MatchedTarget {
                    pattern,
                    target: binding_text.to_string(),
                    surface: TargetSurface::ResolvedBindingSpecifier,
                });
            }
        }
    }

    None
}

/// Return one static `require()` target specifier for require-like calls.
fn expression_require_target_specifier(
    tree: &dir::Tree,
    expression: &dir::Expression,
    global_qualifiers: &[dir::GlobalSymbolId],
    require_name: destack_core::StringId,
) -> Option<destack_core::StringId> {
    let dir::Expression::Call {
        left,
        generic_arguments,
        arguments,
    } = expression
    else {
        return None;
    };

    if !generic_arguments.is_empty() {
        return None;
    }
    if arguments.len() != 1 {
        return None;
    }

    let callee_id = expression_unwrap_transparent(tree, *left);
    let callee = tree.get(callee_id);

    let is_require = match callee {
        dir::Expression::UnresolvedPath { path, .. }
        | dir::Expression::GlobalReference { path, .. }
            if path.segments.len() == 1 && path.segments[0] == require_name =>
        {
            true
        }
        _ => {
            expression_is_global_qualified_member(tree, callee_id, global_qualifiers, require_name)
        }
    };
    if !is_require {
        return None;
    }

    let argument = tree.get(arguments[0]);
    let dir::Argument::Positional { value, .. } = argument else {
        return None;
    };

    expression_static_string_literal(tree, *value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::{TestProgram, test_modules};

    /// Report imports that match exact restricted module names.
    #[test]
    fn test_flags_exact_restricted_import() {
        let test = TestProgram::for_rule_without_prelude(NoBannedImport).with_options(|options| {
            options.restriction.restricted_imports = vec!["legacy/http".to_string()];
        });
        let diagnostics = test.lint_dir(
            "no_banned_import/test_flags_exact_restricted_import.ds",
            r#"
import { client } from "legacy/http";
"#,
        );

        test.result(diagnostics).assert_lint("no-banned-import");
    }

    /// Report imports that match wildcard restricted patterns.
    #[test]
    fn test_flags_wildcard_restricted_import() {
        let test = TestProgram::for_rule_without_prelude(NoBannedImport).with_options(|options| {
            options.restriction.restricted_imports = vec!["internal/*".to_string()];
        });
        let diagnostics = test.lint_dir(
            "no_banned_import/test_flags_wildcard_restricted_import.ds",
            r#"
import { cache } from "internal/cache";
"#,
        );

        test.result(diagnostics).assert_lint("no-banned-import");
    }

    /// Report re exports that match restricted patterns.
    #[test]
    fn test_flags_re_export_target() {
        let test = TestProgram::for_rule_without_prelude(NoBannedImport).with_options(|options| {
            options.restriction.restricted_imports = vec!["legacy/*".to_string()];
        });
        let diagnostics = test.lint_dir(
            "no_banned_import/test_flags_re_export_target.ds",
            r#"
export { tool } from "legacy/toolkit";
"#,
        );

        test.result(diagnostics).assert_lint("no-banned-import");
    }

    /// Report imports when the resolved module path matches a restricted pattern.
    #[test]
    fn test_flags_resolved_module_path_pattern() {
        let test = TestProgram::for_rule_without_prelude(NoBannedImport).with_options(|options| {
            options.restriction.restricted_imports = vec!["*/source.ds".to_string()];
        });
        let diagnostics = test.lint_module_dir_with_modules(
            test_modules! {
                "no_banned_import/source.ds" => r#"
export const value = 1;
"#,
                "no_banned_import/consumer.ds" => r#"
import { value } from "./source";

const copy = value;
"#,
            },
            "no_banned_import/consumer.ds",
        );

        test.result(diagnostics).assert_lint("no-banned-import");
    }

    /// Allow imports that do not match restricted patterns.
    #[test]
    fn test_allows_non_matching_import() {
        let test = TestProgram::for_rule_without_prelude(NoBannedImport).with_options(|options| {
            options.restriction.restricted_imports = vec!["internal/*".to_string()];
        });
        let diagnostics = test.lint_dir(
            "no_banned_import/test_allows_non_matching_import.ds",
            r#"
import { safe } from "public/safe";
"#,
        );

        test.result(diagnostics).assert_no_lint("no-banned-import");
    }

    /// Skip declarations by default.
    #[test]
    fn test_skips_declaration_files_by_default() {
        let test = TestProgram::for_rule_without_prelude(NoBannedImport).with_options(|options| {
            options.restriction.restricted_imports = vec!["internal/*".to_string()];
        });
        let diagnostics = test.lint_dir(
            "no_banned_import/test_skips_declaration_files_by_default.d.ds",
            r#"
import { service } from "internal/service";
"#,
        );

        test.result(diagnostics).assert_no_lint("no-banned-import");
    }

    /// Include declaration files when requested.
    #[test]
    fn test_includes_declaration_files_when_enabled() {
        let test = TestProgram::for_rule_without_prelude(NoBannedImport).with_options(|options| {
            options.restriction.restricted_imports = vec!["internal/*".to_string()];
            options.include_declaration_files = true;
        });
        let diagnostics = test.lint_dir(
            "no_banned_import/test_includes_declaration_files_when_enabled.d.ds",
            r#"
import { service } from "internal/service";
"#,
        );

        test.result(diagnostics).assert_lint("no-banned-import");
    }

    /// Report dynamic imports with static string targets.
    #[test]
    fn test_flags_dynamic_import_with_static_string_target() {
        let test = TestProgram::for_rule_without_prelude(NoBannedImport).with_options(|options| {
            options.restriction.restricted_imports = vec!["internal/*".to_string()];
        });
        let diagnostics = test.lint_dir(
            "no_banned_import/test_flags_dynamic_import_with_static_string_target.ds",
            r#"
await import("internal/cache");
"#,
        );

        test.result(diagnostics).assert_lint("no-banned-import");
    }

    /// Report require() imports with static string targets.
    #[test]
    fn test_flags_require_call_with_static_string_target() {
        let test = TestProgram::for_rule_with_prelude(NoBannedImport).with_options(|options| {
            options.restriction.restricted_imports = vec!["internal/*".to_string()];
        });
        let diagnostics = test.lint_dir(
            "no_banned_import/test_flags_require_call_with_static_string_target.ds",
            r#"
const cache = require("internal/cache");
cache;
"#,
        );

        test.result(diagnostics).assert_lint("no-banned-import");
    }

    /// Report global require() imports with static string targets.
    #[test]
    fn test_flags_global_require_call_with_static_string_target() {
        let test = TestProgram::for_rule_with_prelude(NoBannedImport).with_options(|options| {
            options.restriction.restricted_imports = vec!["internal/*".to_string()];
        });
        let diagnostics = test.lint_dir(
            "no_banned_import/test_flags_global_require_call_with_static_string_target.ds",
            r#"
const cache = globalThis.require("internal/cache");
cache;
"#,
        );

        test.result(diagnostics).assert_lint("no-banned-import");
    }

    /// Allow shadowed require bindings.
    #[test]
    fn test_allows_shadowed_require_call() {
        let test = TestProgram::for_rule_with_prelude(NoBannedImport).with_options(|options| {
            options.restriction.restricted_imports = vec!["internal/*".to_string()];
        });
        let diagnostics = test.lint_dir(
            "no_banned_import/test_allows_shadowed_require_call.ds",
            r#"
function require(name: string): string {
    return name;
}

const cache = require("internal/cache");
cache;
"#,
        );

        test.result(diagnostics).assert_no_lint("no-banned-import");
    }

    /// Allow dynamic imports with non-static targets.
    #[test]
    fn test_allows_dynamic_import_with_non_static_target() {
        let test = TestProgram::for_rule_without_prelude(NoBannedImport).with_options(|options| {
            options.restriction.restricted_imports = vec!["internal/*".to_string()];
        });
        let diagnostics = test.lint_dir(
            "no_banned_import/test_allows_dynamic_import_with_non_static_target.ds",
            r#"
const moduleName = "internal/cache";
await import(moduleName);
"#,
        );

        test.result(diagnostics).assert_no_lint("no-banned-import");
    }
}
