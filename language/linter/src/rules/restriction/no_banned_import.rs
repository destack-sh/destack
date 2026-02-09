use destack_base::StringId;
use destack_dir as dir;
use destack_workspace::LintSeverity;

use crate::rules::common::glob_matches;
use crate::{LintDiagnostic, LintMeta, LintModuleDirContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow imports from configured banned module specifiers.
    ///
    /// This rule checks import and re-export targets against
    /// `linter.restrictedImports` patterns.
    #[lint(
        id = "no-banned-import",
        code = "LR004",
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

        // collect configured restricted patterns
        let restricted_patterns = ctx
            .options
            .restricted_imports
            .iter()
            .map(|pattern| pattern.trim())
            .filter(|pattern| !pattern.is_empty())
            .collect::<Vec<_>>();
        if restricted_patterns.is_empty() {
            return;
        }

        // inspect module import and re export expressions
        for expression_id in ctx.tree.iter_node_ids_of_type::<dir::Expression>() {
            let expression = ctx.tree.get(expression_id);
            let Some(specifier_id) = expression_target_specifier(expression) else {
                continue;
            };
            let specifier_text = ctx.program.strings.get(specifier_id).to_string();

            // match specifier or resolved target identity against restricted patterns
            let Some(matched_target) =
                matching_target(ctx, expression, &specifier_text, &restricted_patterns)
            else {
                continue;
            };

            let severity = ctx.get_effective_severity(meta, expression_id);
            if !severity.is_enabled() {
                continue;
            }

            // report one banned import target
            let span = ctx.get_span(expression_id);
            ctx.report(
                LintDiagnostic::new(
                    NO_BANNED_IMPORT.id,
                    NO_BANNED_IMPORT.code,
                    NO_BANNED_IMPORT.category,
                    severity,
                    format!("banned import target `{specifier_text}`"),
                    ctx.module.file_id,
                    span,
                )
                .with_label("this import target is restricted by project configuration")
                .with_note(format!(
                    "matched restricted pattern `{}` against {} `{}`",
                    matched_target.pattern,
                    matched_target.surface.label(),
                    matched_target.target
                )),
            );
        }
    }
}

/// A matched import target and match source details.
struct MatchedTarget<'a> {
    /// The restricted pattern that matched.
    pattern: &'a str,
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

/// Return the target module specifier text for an import like expression.
fn expression_target_specifier(expression: &dir::Expression) -> Option<StringId> {
    let target = match expression {
        dir::Expression::Import { target, .. }
        | dir::Expression::ReExport { target, .. }
        | dir::Expression::UnresolvedImport { target, .. }
        | dir::Expression::UnresolvedReExport { target, .. } => *target,
        _ => return None,
    };

    Some(target)
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
fn matching_pattern<'a>(target: &str, patterns: &'a [&str]) -> Option<&'a str> {
    patterns
        .iter()
        .copied()
        .find(|pattern| glob_matches(pattern, target))
}

/// Return the first restricted target match for one import expression.
fn matching_target<'a>(
    ctx: &LintModuleDirContext<'_>,
    expression: &dir::Expression,
    specifier_text: &str,
    patterns: &'a [&str],
) -> Option<MatchedTarget<'a>> {
    // check the explicit specifier first
    if let Some(pattern) = matching_pattern(specifier_text, patterns) {
        return Some(MatchedTarget {
            pattern,
            target: specifier_text.to_string(),
            surface: TargetSurface::Specifier,
        });
    }

    let target_module = expression_target_module(expression)?;
    match target_module {
        dir::ModuleTarget::Module(module_id) => {
            let module_ref = ctx.program.modules.get(module_id);
            let module = module_ref.read();

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
            let file = ctx.program.files.get(module.file_id);
            if let Some(pattern) = matching_pattern(&file.name, patterns) {
                return Some(MatchedTarget {
                    pattern,
                    target: file.name.clone(),
                    surface: TargetSurface::ResolvedModuleName,
                });
            }
        }
        dir::ModuleTarget::Binding(binding_specifier) => {
            let binding_text = ctx.program.strings.get(binding_specifier);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::{TestProgram, test_modules};

    /// Report imports that match exact restricted module names.
    #[test]
    fn test_flags_exact_restricted_import() {
        let test = TestProgram::for_rule_without_prelude(NoBannedImport).with_options(|options| {
            options.restricted_imports = vec!["legacy/http".to_string()];
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
            options.restricted_imports = vec!["internal/*".to_string()];
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
            options.restricted_imports = vec!["legacy/*".to_string()];
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
            options.restricted_imports = vec!["*/source.ds".to_string()];
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
            options.restricted_imports = vec!["internal/*".to_string()];
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
            options.restricted_imports = vec!["internal/*".to_string()];
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
            options.restricted_imports = vec!["internal/*".to_string()];
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
