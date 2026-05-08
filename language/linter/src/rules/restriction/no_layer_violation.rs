use crate::LintMeta;
use std::collections::HashMap;

use destack_artifact::{DirExported, DirImported};
use destack_source::{FileId, FileType, ModuleId, Span};
use destack_workspace::{DiagnosticPolicy, LintModuleBoundariesOptions, LintSeverity};

use crate::rules::common::glob_matches;
use crate::{LintReport, LintRule, LintWorkspaceDirContext, declare_lint};

declare_lint! {
    /// Disallow imports that violate configured module boundary constraints.
    ///
    /// This rule enforces dependency direction between configured components
    /// in `linter.moduleBoundaries`.
    #[lint(
        id = "no-layer-violation",
        code = "LR036",
        category = Restriction,
        level = Dir,
        scope = Workspace,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Off,
        stability = Stable,
        declarations = Exclude
    )]
    pub NoLayerViolation,
    "Disallow imports that cross forbidden module boundaries"
}

impl LintRule for NoLayerViolation {
    fn meta(&self) -> &'static LintMeta {
        NoLayerViolation::meta()
    }

    fn check_workspace_dir(&self, ctx: &mut LintWorkspaceDirContext) {
        // resolve lint metadata and base severity
        let meta = self.meta();
        let rule_severity = ctx.get_severity(meta);
        if !rule_severity.is_enabled() {
            return;
        }

        // resolve module boundary options
        let module_boundaries = ctx.options().restriction.module_boundaries.clone();
        if module_boundaries.components.is_empty() {
            return;
        }

        // collect eligible user modules and their component assignments
        let descriptors = collect_module_descriptors(ctx, &module_boundaries);
        if descriptors.is_empty() {
            return;
        }
        let descriptor_index = build_descriptor_index(&descriptors);

        // collect forbidden dependency diagnostics while graph borrow is active
        let forbidden_dependency_diagnostics = collect_forbidden_dependency_diagnostics(
            &module_boundaries,
            rule_severity,
            &descriptors,
            &descriptor_index,
            ctx,
        );

        // report forbidden dependency edges
        for diagnostic in forbidden_dependency_diagnostics {
            ctx.report(diagnostic);
        }

        // report modules not mapped to any component when configured
        report_unknown_component_modules(ctx, &module_boundaries, &descriptors);
    }
}

/// One eligible module and derived boundary metadata.
#[derive(Debug, Clone)]
struct ModuleDescriptor {
    /// The module id.
    module_id: ModuleId,
    /// The file id for diagnostics.
    file_id: FileId,
    /// The stable display name for diagnostics and sorting.
    file_name: String,
    /// Candidate path strings for glob matching.
    path_candidates: Vec<String>,
    /// The matched component name, if any.
    component_name: Option<String>,
}

/// Collect eligible module descriptors with component assignments.
fn collect_module_descriptors(
    ctx: &LintWorkspaceDirContext,
    module_boundaries: &LintModuleBoundariesOptions,
) -> Vec<ModuleDescriptor> {
    let mut descriptors = Vec::new();

    // collect user modules and assign configured components
    for module_id in ctx.workspace_module_ids() {
        let Some(module) = ctx.repository_module(module_id) else {
            continue;
        };
        let Some(file) = ctx.repository_file(module.file_id) else {
            continue;
        };
        if !file.ty.is_code() || is_declaration_file(file.ty, ctx) {
            continue;
        }

        let path_candidates = module_path_candidates(module.path.as_deref(), &file.name);
        let component_name = resolve_module_component(module_boundaries, &path_candidates);

        descriptors.push(ModuleDescriptor {
            module_id: module.id,
            file_id: module.file_id,
            file_name: file.name.clone(),
            path_candidates,
            component_name,
        });
    }

    // keep deterministic diagnostic ordering by source file name
    descriptors.sort_by(|left, right| {
        left.file_name
            .cmp(&right.file_name)
            .then_with(|| left.module_id.cmp(&right.module_id))
    });

    descriptors
}

/// Build a descriptor index map by module id.
fn build_descriptor_index(descriptors: &[ModuleDescriptor]) -> HashMap<ModuleId, usize> {
    let mut index = HashMap::new();

    // map each module id to its descriptor index
    for (position, descriptor) in descriptors.iter().enumerate() {
        index.insert(descriptor.module_id, position);
    }

    index
}

/// Collect diagnostics for forbidden dependency edges between configured components.
fn collect_forbidden_dependency_diagnostics(
    module_boundaries: &LintModuleBoundariesOptions,
    rule_severity: LintSeverity,
    descriptors: &[ModuleDescriptor],
    descriptor_index: &HashMap<ModuleId, usize>,
    ctx: &LintWorkspaceDirContext,
) -> Vec<LintReport> {
    let mut diagnostics = Vec::new();

    // inspect dependency edges from each eligible source module
    for source in descriptors {
        let Some(from_component) = source.component_name.as_deref() else {
            continue;
        };

        let mut dependencies = module_dependencies(ctx, source.module_id)
            .into_iter()
            .filter_map(|dependency_id| descriptor_index.get(&dependency_id).copied())
            .collect::<Vec<_>>();
        dependencies.sort_unstable();
        dependencies.dedup();

        // keep deterministic ordering by target file name
        dependencies.sort_by(|left, right| {
            let left_file = &descriptors[*left].file_name;
            let right_file = &descriptors[*right].file_name;
            left_file.cmp(right_file).then_with(|| {
                descriptors[*left]
                    .module_id
                    .cmp(&descriptors[*right].module_id)
            })
        });

        // report each disallowed dependency edge
        for dependency_index in dependencies {
            let target = &descriptors[dependency_index];
            let Some(to_component) = target.component_name.as_deref() else {
                continue;
            };

            if dependency_is_allowed(
                module_boundaries,
                from_component,
                to_component,
                &source.path_candidates,
            ) {
                continue;
            }

            diagnostics.push(
                LintReport::new(
                    NO_LAYER_VIOLATION.id,
                    NO_LAYER_VIOLATION.code,
                    NO_LAYER_VIOLATION.category,
                    rule_severity,
                    format!(
                        "forbidden module boundary dependency from `{from_component}` to `{to_component}`"
                    ),
                    Span::empty(source.file_id))
                .label("this module imports across a forbidden boundary")
                .note(format!("source module: {}", source.file_name))
                .note(format!("target module: {}", target.file_name))
                .note(format!(
                    "allowed dependencies for `{from_component}` are configured by linter.moduleBoundaries.rules"
                )),
            );
        }
    }

    diagnostics
}

/// Return direct module dependencies for one module.
fn module_dependencies(ctx: &LintWorkspaceDirContext, module_id: ModuleId) -> Vec<ModuleId> {
    let mut dependencies = Vec::new();

    if let Some(imported) = ctx.imported_dir(module_id) {
        collect_imported_module_dependencies(&imported, &mut dependencies);
    }

    if let Some(exported) = ctx.exported_dir(module_id) {
        collect_exported_module_dependencies(&exported, &mut dependencies);
    }

    dependencies
}

/// Extend one dependency list with direct import edges.
fn collect_imported_module_dependencies(imported: &DirImported, dependencies: &mut Vec<ModuleId>) {
    // collect import edges for both value and type space
    for dependency in &imported.dependencies {
        let resolution = dependency.resolution;
        if let Some(module_id) = resolution.value.and_then(|target| target.module_id()) {
            dependencies.push(module_id);
        }

        if let Some(module_id) = resolution.type_target.and_then(|target| target.module_id()) {
            dependencies.push(module_id);
        }
    }
}

/// Extend one dependency list with direct export edges.
fn collect_exported_module_dependencies(exported: &DirExported, dependencies: &mut Vec<ModuleId>) {
    // collect namespace re export edges
    for export in exported.exports.namespace_exports.iter() {
        if let Some(module_id) = export.module_id.module_id() {
            dependencies.push(module_id);
        }
    }
}

/// Report modules that do not match any configured component.
fn report_unknown_component_modules(
    ctx: &mut LintWorkspaceDirContext,
    module_boundaries: &LintModuleBoundariesOptions,
    descriptors: &[ModuleDescriptor],
) {
    // map unknown component policy to lint severity
    let Some(severity) = lint_severity_for_policy(module_boundaries.unknown_component_policy)
    else {
        return;
    };

    // report unmatched modules
    for descriptor in descriptors {
        if descriptor.component_name.is_some() {
            continue;
        }

        ctx.report(
            LintReport::new(
                NO_LAYER_VIOLATION.id,
                NO_LAYER_VIOLATION.code,
                NO_LAYER_VIOLATION.category,
                severity,
                "module is not assigned to any configured component",
                Span::empty(descriptor.file_id),
            )
            .label("this module does not match any linter.moduleBoundaries.components pattern")
            .note(format!("module: {}", descriptor.file_name))
            .note("add a matching component pattern or relax unknownComponentPolicy"),
        );
    }
}

/// Return true when one dependency edge is allowed by module boundary options.
fn dependency_is_allowed(
    module_boundaries: &LintModuleBoundariesOptions,
    from_component: &str,
    to_component: &str,
    source_path_candidates: &[String],
) -> bool {
    // always allow dependencies inside the same component
    if from_component == to_component {
        return true;
    }

    // allow explicit per path exceptions first
    if dependency_matches_exception(
        module_boundaries,
        from_component,
        to_component,
        source_path_candidates,
    ) {
        return true;
    }

    // enforce configured dependency rule for the source component when present
    let Some(rule) = module_boundaries
        .dependency_rules
        .iter()
        .find(|rule| rule.from == from_component)
    else {
        return true;
    };

    rule.allow.iter().any(|allowed| allowed == to_component)
}

/// Return true when one dependency edge matches an explicit exception.
fn dependency_matches_exception(
    module_boundaries: &LintModuleBoundariesOptions,
    from_component: &str,
    to_component: &str,
    source_path_candidates: &[String],
) -> bool {
    // evaluate all exceptions for this component pair
    for exception in &module_boundaries.exceptions {
        if exception.from != from_component || exception.to != to_component {
            continue;
        }

        // allow broad pair exception without path filters
        if exception.path_patterns.is_empty() {
            return true;
        }

        // allow when any exception pattern matches source module path candidates
        for pattern in &exception.path_patterns {
            for path_candidate in source_path_candidates {
                if glob_matches(pattern, path_candidate) {
                    return true;
                }
            }
        }
    }

    false
}

/// Resolve one module component from configured component patterns.
fn resolve_module_component(
    module_boundaries: &LintModuleBoundariesOptions,
    path_candidates: &[String],
) -> Option<String> {
    // match components in declaration order
    for component in &module_boundaries.components {
        for pattern in &component.path_patterns {
            for path_candidate in path_candidates {
                if glob_matches(pattern, path_candidate) {
                    return Some(component.name.clone());
                }
            }
        }
    }

    None
}

/// Build path candidates for glob matching from module and file metadata.
fn module_path_candidates(module_path: Option<&std::path::Path>, file_name: &str) -> Vec<String> {
    let mut candidates = Vec::new();

    // prefer stable workspace style file names for configuration matching
    let file_name = normalize_glob_text(file_name);
    candidates.push(file_name);

    // include exported module path as an additional matching surface
    if let Some(module_path) = module_path {
        let module_path = normalize_glob_text(&module_path.to_string_lossy());
        if !candidates.contains(&module_path) {
            candidates.push(module_path);
        }
    }

    candidates
}

/// Normalize path text for glob matching across platforms.
fn normalize_glob_text(text: &str) -> String {
    text.replace('\\', "/")
}

/// Return lint severity for one diagnostic policy.
fn lint_severity_for_policy(policy: DiagnosticPolicy) -> Option<LintSeverity> {
    match policy {
        DiagnosticPolicy::Allow => None,
        DiagnosticPolicy::Warn => Some(LintSeverity::Warning),
        DiagnosticPolicy::Deny => Some(LintSeverity::Error),
    }
}

/// Return true when one file should be excluded by declaration filtering.
fn is_declaration_file(file_type: FileType, ctx: &LintWorkspaceDirContext) -> bool {
    if ctx.options().include_declaration_files {
        return false;
    }

    matches!(
        file_type,
        FileType::TypeScriptDeclaration | FileType::DestackDeclaration
    )
}

#[cfg(test)]
mod tests {
    use destack_workspace::{
        LintModuleBoundariesOptions, LintModuleComponent, LintModuleDependencyException,
        LintModuleDependencyRule,
    };

    use super::*;
    use crate::linter::TestProgram;
    use crate::test_modules;

    /// Add modules, run analysis, and lint the workspace at DIR level.
    fn lint_workspace_with_modules(
        modules: &[(&str, &str)],
        configure: impl FnOnce(&mut destack_workspace::LinterOptions),
    ) -> (TestProgram, Vec<LintReport>) {
        let test = TestProgram::new_without_prelude(vec![crate::boxed(NoLayerViolation)])
            .with_options(configure);
        let diagnostics = test.lint_workspace_dir_with_modules(modules);
        (test, diagnostics)
    }

    /// Return a baseline module boundary configuration for tests.
    fn test_module_boundaries() -> LintModuleBoundariesOptions {
        LintModuleBoundariesOptions {
            components: vec![
                LintModuleComponent {
                    name: "app".to_string(),
                    path_patterns: vec!["no_layer_violation/app/*".to_string()],
                },
                LintModuleComponent {
                    name: "domain".to_string(),
                    path_patterns: vec!["no_layer_violation/domain/*".to_string()],
                },
                LintModuleComponent {
                    name: "infra".to_string(),
                    path_patterns: vec!["no_layer_violation/infra/*".to_string()],
                },
            ],
            dependency_rules: vec![
                LintModuleDependencyRule {
                    from: "app".to_string(),
                    allow: vec!["domain".to_string()],
                },
                LintModuleDependencyRule {
                    from: "domain".to_string(),
                    allow: Vec::new(),
                },
            ],
            ..LintModuleBoundariesOptions::default()
        }
    }

    /// Return diagnostics emitted by this rule only.
    fn no_layer_violation_diagnostics(diagnostics: &[LintReport]) -> Vec<&LintReport> {
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.rule_id == "no-layer-violation")
            .collect()
    }

    /// Return diagnostics emitted for forbidden dependency edges only.
    fn forbidden_dependency_diagnostics(diagnostics: &[LintReport]) -> Vec<&LintReport> {
        no_layer_violation_diagnostics(diagnostics)
            .into_iter()
            .filter(|diagnostic| {
                diagnostic
                    .message
                    .starts_with("forbidden module boundary dependency")
            })
            .collect()
    }

    /// Return diagnostics emitted for unknown component modules only.
    fn unknown_component_diagnostics(diagnostics: &[LintReport]) -> Vec<&LintReport> {
        no_layer_violation_diagnostics(diagnostics)
            .into_iter()
            .filter(|diagnostic| {
                diagnostic.message() == "module is not assigned to any configured component"
            })
            .collect()
    }

    /// Report dependencies that violate configured component direction.
    #[test]
    fn test_flags_forbidden_component_dependency() {
        let (test, diagnostics) = lint_workspace_with_modules(
            test_modules! {
                "no_layer_violation/app/service.ds" => r#"
import { query } from "../infra/database.ds";

export const value = query;
"#,
                "no_layer_violation/infra/database.ds" => r#"
export const query = 1;
"#,
            },
            |options| {
                options.restriction.module_boundaries = test_module_boundaries();
            },
        );

        test.result(diagnostics)
            .assert_lint("no-layer-violation")
            .assert_lint_count("no-layer-violation", 1);
    }

    /// Report one diagnostic per forbidden dependency target.
    #[test]
    fn test_flags_multiple_forbidden_component_dependencies() {
        let (test, diagnostics) = lint_workspace_with_modules(
            test_modules! {
                "no_layer_violation/app/service.ds" => r#"
import { queryOne } from "../infra/database_one.ds";
import { queryTwo } from "../infra/database_two.ds";

export const value = [queryOne, queryTwo];
"#,
                "no_layer_violation/infra/database_one.ds" => r#"
export const queryOne = 1;
"#,
                "no_layer_violation/infra/database_two.ds" => r#"
export const queryTwo = 2;
"#,
            },
            |options| {
                options.restriction.module_boundaries = test_module_boundaries();
            },
        );

        test.result(diagnostics)
            .assert_lint("no-layer-violation")
            .assert_lint_count("no-layer-violation", 2);
    }

    /// Report one diagnostic when repeated imports hit one forbidden target.
    #[test]
    fn test_deduplicates_forbidden_dependency_edge() {
        let (test, diagnostics) = lint_workspace_with_modules(
            test_modules! {
                "no_layer_violation/app/service.ds" => r#"
import { query } from "../infra/database.ds";
import { query as queryAlias } from "../infra/database.ds";

export const value = [query, queryAlias];
"#,
                "no_layer_violation/infra/database.ds" => r#"
export const query = 1;
"#,
            },
            |options| {
                options.restriction.module_boundaries = test_module_boundaries();
            },
        );

        test.result(diagnostics)
            .assert_lint("no-layer-violation")
            .assert_lint_count("no-layer-violation", 1);
    }

    /// Allow dependencies that match configured component direction.
    #[test]
    fn test_allows_allowed_component_dependency() {
        let (test, diagnostics) = lint_workspace_with_modules(
            test_modules! {
                "no_layer_violation/app/service.ds" => r#"
import { rule } from "../domain/rule.ds";

export const value = rule;
"#,
                "no_layer_violation/domain/rule.ds" => r#"
export const rule = 1;
"#,
            },
            |options| {
                options.restriction.module_boundaries = test_module_boundaries();
            },
        );

        test.result(diagnostics)
            .assert_no_lint("no-layer-violation");
    }

    /// Allow dependencies inside one component.
    #[test]
    fn test_allows_same_component_dependency() {
        let (test, diagnostics) = lint_workspace_with_modules(
            test_modules! {
                "no_layer_violation/app/main.ds" => r#"
import { helper } from "./helper.ds";

export const value = helper;
"#,
                "no_layer_violation/app/helper.ds" => r#"
export const helper = 1;
"#,
            },
            |options| {
                options.restriction.module_boundaries = test_module_boundaries();
            },
        );

        test.result(diagnostics)
            .assert_no_lint("no-layer-violation");
    }

    /// Allow dependencies when a matching exception is configured.
    #[test]
    fn test_allows_configured_exception() {
        let (test, diagnostics) = lint_workspace_with_modules(
            test_modules! {
                "no_layer_violation/app/bootstrap.ds" => r#"
import { query } from "../infra/database.ds";

export const value = query;
"#,
                "no_layer_violation/infra/database.ds" => r#"
export const query = 1;
"#,
            },
            |options| {
                let mut module_boundaries = test_module_boundaries();
                module_boundaries
                    .exceptions
                    .push(LintModuleDependencyException {
                        from: "app".to_string(),
                        to: "infra".to_string(),
                        path_patterns: vec!["no_layer_violation/app/bootstrap.ds".to_string()],
                        reason: Some("composition root".to_string()),
                    });
                options.restriction.module_boundaries = module_boundaries;
            },
        );

        test.result(diagnostics)
            .assert_no_lint("no-layer-violation");
    }

    /// Allow dependencies when the component pair is globally excepted.
    #[test]
    fn test_allows_global_component_pair_exception() {
        let (test, diagnostics) = lint_workspace_with_modules(
            test_modules! {
                "no_layer_violation/app/bootstrap.ds" => r#"
import { query } from "../infra/database.ds";

export const value = query;
"#,
                "no_layer_violation/infra/database.ds" => r#"
export const query = 1;
"#,
            },
            |options| {
                let mut module_boundaries = test_module_boundaries();
                module_boundaries
                    .exceptions
                    .push(LintModuleDependencyException {
                        from: "app".to_string(),
                        to: "infra".to_string(),
                        path_patterns: Vec::new(),
                        reason: Some("global pair exception".to_string()),
                    });
                options.restriction.module_boundaries = module_boundaries;
            },
        );

        test.result(diagnostics)
            .assert_no_lint("no-layer-violation");
    }

    /// Report dependencies when exception path patterns do not match.
    #[test]
    fn test_reports_when_exception_pattern_does_not_match() {
        let (test, diagnostics) = lint_workspace_with_modules(
            test_modules! {
                "no_layer_violation/app/service.ds" => r#"
import { query } from "../infra/database.ds";

export const value = query;
"#,
                "no_layer_violation/infra/database.ds" => r#"
export const query = 1;
"#,
            },
            |options| {
                let mut module_boundaries = test_module_boundaries();
                module_boundaries
                    .exceptions
                    .push(LintModuleDependencyException {
                        from: "app".to_string(),
                        to: "infra".to_string(),
                        path_patterns: vec!["no_layer_violation/app/bootstrap.ds".to_string()],
                        reason: Some("not matching".to_string()),
                    });
                options.restriction.module_boundaries = module_boundaries;
            },
        );

        test.result(diagnostics)
            .assert_lint("no-layer-violation")
            .assert_lint_count("no-layer-violation", 1);
    }

    /// Allow dependencies from components with no explicit dependency rule.
    #[test]
    fn test_allows_component_without_explicit_rule() {
        let (test, diagnostics) = lint_workspace_with_modules(
            test_modules! {
                "no_layer_violation/infra/worker.ds" => r#"
import { value } from "../app/service.ds";

export const result = value;
"#,
                "no_layer_violation/app/service.ds" => r#"
export const value = 1;
"#,
            },
            |options| {
                options.restriction.module_boundaries = test_module_boundaries();
            },
        );

        test.result(diagnostics)
            .assert_no_lint("no-layer-violation");
    }

    /// Warn on modules not assigned to any component when configured.
    #[test]
    fn test_warns_on_unknown_component_module() {
        let (_test, diagnostics) = lint_workspace_with_modules(
            test_modules! {
                "no_layer_violation/misc/tool.ds" => r#"
export const tool = 1;
"#,
            },
            |options| {
                let mut module_boundaries = test_module_boundaries();
                module_boundaries.unknown_component_policy = DiagnosticPolicy::Warn;
                options.restriction.module_boundaries = module_boundaries;
            },
        );

        let unknown_diagnostics = unknown_component_diagnostics(&diagnostics);
        assert_eq!(unknown_diagnostics.len(), 1);
        assert_eq!(unknown_diagnostics[0].severity, LintSeverity::Warning);
    }

    /// Error on modules not assigned to any component when configured.
    #[test]
    fn test_errors_on_unknown_component_module() {
        let (_test, diagnostics) = lint_workspace_with_modules(
            test_modules! {
                "no_layer_violation/misc/tool.ds" => r#"
export const tool = 1;
"#,
            },
            |options| {
                let mut module_boundaries = test_module_boundaries();
                module_boundaries.unknown_component_policy = DiagnosticPolicy::Deny;
                options.restriction.module_boundaries = module_boundaries;
            },
        );

        let unknown_diagnostics = unknown_component_diagnostics(&diagnostics);
        assert_eq!(unknown_diagnostics.len(), 1);
        assert_eq!(unknown_diagnostics[0].severity, LintSeverity::Error);
    }

    /// Skip unknown component diagnostics when policy allows it.
    #[test]
    fn test_ignores_unknown_component_module_when_allowed() {
        let (test, diagnostics) = lint_workspace_with_modules(
            test_modules! {
                "no_layer_violation/misc/tool.ds" => r#"
export const tool = 1;
"#,
            },
            |options| {
                options.restriction.module_boundaries = test_module_boundaries();
            },
        );

        test.result(diagnostics)
            .assert_no_lint("no-layer-violation");
    }

    /// Skip unknown component diagnostics for declaration files by default.
    #[test]
    fn test_skips_declaration_file_unknown_component_by_default() {
        let (test, diagnostics) = lint_workspace_with_modules(
            test_modules! {
                "no_layer_violation/misc/tool.d.ds" => r#"
export declare const tool: number;
"#,
            },
            |options| {
                let mut module_boundaries = test_module_boundaries();
                module_boundaries.unknown_component_policy = DiagnosticPolicy::Warn;
                options.restriction.module_boundaries = module_boundaries;
            },
        );

        test.result(diagnostics)
            .assert_no_lint("no-layer-violation");
    }

    /// Include declaration file diagnostics when declaration files are enabled.
    #[test]
    fn test_includes_declaration_file_unknown_component_when_enabled() {
        let (_test, diagnostics) = lint_workspace_with_modules(
            test_modules! {
                "no_layer_violation/misc/tool.d.ds" => r#"
export declare const tool: number;
"#,
            },
            |options| {
                let mut module_boundaries = test_module_boundaries();
                module_boundaries.unknown_component_policy = DiagnosticPolicy::Warn;
                options.restriction.module_boundaries = module_boundaries;
                options.include_declaration_files = true;
            },
        );

        let unknown_diagnostics = unknown_component_diagnostics(&diagnostics);
        assert_eq!(unknown_diagnostics.len(), 1);
        assert_eq!(unknown_diagnostics[0].severity, LintSeverity::Warning);
    }

    /// Allow all dependencies when no components are configured.
    #[test]
    fn test_allows_when_no_components_configured() {
        let (test, diagnostics) = lint_workspace_with_modules(
            test_modules! {
                "no_layer_violation/app/service.ds" => r#"
import { query } from "../infra/database.ds";

export const value = query;
"#,
                "no_layer_violation/infra/database.ds" => r#"
export const query = 1;
"#,
            },
            |options| {
                options.restriction.module_boundaries = LintModuleBoundariesOptions::default();
            },
        );

        test.result(diagnostics)
            .assert_no_lint("no-layer-violation");
    }

    /// Match overlapping component patterns in declaration order.
    #[test]
    fn test_prefers_first_matching_component_pattern() {
        let (test, diagnostics) = lint_workspace_with_modules(
            test_modules! {
                "no_layer_violation/app/service.ds" => r#"
import { query } from "../infra/database.ds";

export const value = query;
"#,
                "no_layer_violation/infra/database.ds" => r#"
export const query = 1;
"#,
            },
            |options| {
                options.restriction.module_boundaries = LintModuleBoundariesOptions {
                    components: vec![
                        LintModuleComponent {
                            name: "app".to_string(),
                            path_patterns: vec!["no_layer_violation/*".to_string()],
                        },
                        LintModuleComponent {
                            name: "infra".to_string(),
                            path_patterns: vec!["no_layer_violation/infra/*".to_string()],
                        },
                    ],
                    dependency_rules: vec![LintModuleDependencyRule {
                        from: "app".to_string(),
                        allow: Vec::new(),
                    }],
                    ..LintModuleBoundariesOptions::default()
                };
            },
        );

        // with first match semantics: both modules map to app, so no violation
        test.result(diagnostics)
            .assert_no_lint("no-layer-violation");
    }

    /// Skip forbidden dependency diagnostics to unknown component targets.
    #[test]
    fn test_does_not_report_forbidden_dependency_to_unknown_target_component() {
        let (_test, diagnostics) = lint_workspace_with_modules(
            test_modules! {
                "no_layer_violation/app/service.ds" => r#"
import { value } from "../misc/tool.ds";

export const result = value;
"#,
                "no_layer_violation/misc/tool.ds" => r#"
export const value = 1;
"#,
            },
            |options| {
                let mut module_boundaries = test_module_boundaries();
                module_boundaries.unknown_component_policy = DiagnosticPolicy::Warn;
                options.restriction.module_boundaries = module_boundaries;
            },
        );

        let forbidden = forbidden_dependency_diagnostics(&diagnostics);
        let unknown = unknown_component_diagnostics(&diagnostics);
        assert_eq!(forbidden.len(), 0);
        assert_eq!(unknown.len(), 1);
    }

    /// Skip declaration file dependency diagnostics by default.
    #[test]
    fn test_skips_declaration_file_dependency_by_default() {
        let (test, diagnostics) = lint_workspace_with_modules(
            test_modules! {
                "no_layer_violation/app/service.ds" => r#"
import { query } from "../infra/database.d.ds";

export const value = query;
"#,
                "no_layer_violation/infra/database.d.ds" => r#"
export declare const query: number;
"#,
            },
            |options| {
                options.restriction.module_boundaries = test_module_boundaries();
            },
        );

        test.result(diagnostics)
            .assert_no_lint("no-layer-violation");
    }

    /// Include declaration file dependencies when enabled.
    #[test]
    fn test_reports_declaration_file_dependency_when_enabled() {
        let (test, diagnostics) = lint_workspace_with_modules(
            test_modules! {
                "no_layer_violation/app/service.ds" => r#"
import { query } from "../infra/database.d.ds";

export const value = query;
"#,
                "no_layer_violation/infra/database.d.ds" => r#"
export declare const query: number;
"#,
            },
            |options| {
                options.restriction.module_boundaries = test_module_boundaries();
                options.include_declaration_files = true;
            },
        );

        test.result(diagnostics)
            .assert_lint("no-layer-violation")
            .assert_lint_count("no-layer-violation", 1);
    }
}
