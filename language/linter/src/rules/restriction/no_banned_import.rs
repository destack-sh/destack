use destack_dir::{self as dir, NodeVisitor, NodeVisitorOptions, walk_expression};
use destack_source::ModuleId;
use destack_workspace::LintSeverity;

use crate::rules::common::{expression_import_target_specifier, glob_matches};
use crate::{LintMeta, LintModuleContext, LintReport, LintRule, declare_lint};

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

    fn check_module<'a>(&self, _severity: LintSeverity, ctx: &mut LintModuleContext<'a>) {
        let meta = self.meta();

        // walk the module expression tree
        let mut visitor = NoBannedImportVisitor::new(ctx, meta);
        visitor.run();
    }
}

/// Node visitor that reports banned import targets.
struct NoBannedImportVisitor<'a, 'b> {
    /// The lint context.
    ctx: &'a mut LintModuleContext<'b>,
    /// The lint metadata.
    meta: &'a LintMeta,
    /// The restricted import patterns for this run.
    restricted_patterns: Vec<String>,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl<'a, 'b> NoBannedImportVisitor<'a, 'b> {
    /// Build a visitor for no-banned-import checks.
    fn new(ctx: &'a mut LintModuleContext<'b>, meta: &'a LintMeta) -> Self {
        let restricted_patterns = ctx
            .options
            .restriction
            .restricted_imports
            .iter()
            .map(|pattern| pattern.trim())
            .filter(|pattern| !pattern.is_empty())
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();

        Self {
            ctx,
            meta,
            restricted_patterns,
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
        let tree = self.ctx.dir.tree();

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
        let specifier_id = expression_import_target_specifier(self.ctx.dir.tree(), expression);
        let Some(specifier_id) = specifier_id else {
            return;
        };
        let specifier_text = self.ctx.strings.get(specifier_id).to_string();

        // match specifier or resolved target identity against restricted patterns
        let Some(matched_target) = matching_target(
            self.ctx,
            expression_id,
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
}

impl TargetSurface {
    /// Return a human readable label for diagnostics.
    fn label(&self) -> &'static str {
        match self {
            Self::Specifier => "import specifier",
            Self::ResolvedModulePath => "resolved module path",
            Self::ResolvedModuleName => "resolved module file name",
        }
    }
}

/// Return the resolved module target for an import like expression.
fn expression_target_module(
    ctx: &LintModuleContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
    expression: &dir::Expression,
) -> Option<ModuleId> {
    if !matches!(
        expression,
        dir::Expression::Import { .. }
            | dir::Expression::Export {
                target: Some(_),
                ..
            }
    ) {
        return None;
    }

    let relation = match expression {
        dir::Expression::Import { .. } => dir::ModuleRelation::Import,
        dir::Expression::Export { .. } => dir::ModuleRelation::ReExport,
        _ => unreachable!(),
    };

    ctx.modules
        .target_for_source(expression_id.into_global_any(ctx.module_id()), relation)
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
    ctx: &LintModuleContext<'_>,
    expression_id: dir::LocalNodeId<dir::Expression>,
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
    let target_module = expression_target_module(ctx, expression_id, expression)?;
    let module = ctx.repository_module(target_module)?;
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

    None
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
}
