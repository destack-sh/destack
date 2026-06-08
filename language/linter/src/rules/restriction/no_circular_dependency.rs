use crate::LintMeta;
use std::collections::{HashMap, HashSet};

use crate::rules::common::{find_cycle_path, strongly_connected_components};
use crate::{LintPackageContext, LintReport, LintRule, declare_lint};
use destack_artifact::DirExported;
use destack_dir as dir;
use destack_source::{FileType, ModuleId, Span};

declare_lint! {
    /// Disallow circular module dependencies.
    ///
    /// Import cycles make code harder to reason about and can hide order dependent behavior.
    #[lint(
        id = "no-circular-dependency",
        code = "LR005",
        category = Restriction,
        level = Dir,
        scope = Package,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Off,
        stability = Stable
    )]
    pub NoCircularDependency,
    "Disallow circular module dependencies"
}

impl LintRule for NoCircularDependency {
    fn meta(&self) -> &'static LintMeta {
        NoCircularDependency::meta()
    }

    fn check_package(&self, ctx: &mut LintPackageContext) {
        // resolve lint metadata
        let meta = self.meta();
        let severity = ctx.get_severity(meta);
        if !severity.is_enabled() {
            return;
        }

        // collect modules eligible for this rule
        let eligible_modules = collect_eligible_modules(ctx);
        if eligible_modules.is_empty() {
            return;
        }
        let module_names = collect_module_display_names(ctx, &eligible_modules);

        // build the filtered dependency adjacency
        let adjacency = build_adjacency(ctx, &eligible_modules);

        // find cycle components
        let mut cycle_components = cycle_components(&adjacency);
        if cycle_components.is_empty() {
            return;
        }

        // emit one diagnostic per member in each cycle component
        for component in &mut cycle_components {
            component.sort_by(|left, right| {
                let left_name = module_names
                    .get(left)
                    .map(String::as_str)
                    .unwrap_or_default();
                let right_name = module_names
                    .get(right)
                    .map(String::as_str)
                    .unwrap_or_default();
                left_name.cmp(right_name).then_with(|| left.cmp(right))
            });
            let member_names = component
                .iter()
                .map(|module_id| format_module_name(&module_names, *module_id))
                .collect::<Vec<_>>();
            let members_note = member_names.join(", ");
            let cycle_path_note = format_cycle_path_note(&adjacency, component, &module_names);

            for module_id in component.iter().copied() {
                let diagnostic = build_cycle_diagnostic(
                    module_id,
                    &members_note,
                    cycle_path_note.as_deref(),
                    severity,
                    meta.id,
                    ctx,
                );
                ctx.report(diagnostic);
            }
        }
    }
}

/// Build a cycle diagnostic for one module.
fn build_cycle_diagnostic(
    module_id: ModuleId,
    members_note: &str,
    cycle_path_note: Option<&str>,
    severity: destack_repository::LintSeverity,
    rule_id: &str,
    ctx: &LintPackageContext,
) -> LintReport {
    let module = ctx
        .session
        .repository_module(module_id)
        .unwrap_or_else(|| panic!("missing module for {module_id:?}"));
    let module = module.as_ref();

    let mut diagnostic = LintReport::new(
        NO_CIRCULAR_DEPENDENCY.id,
        NO_CIRCULAR_DEPENDENCY.code,
        NO_CIRCULAR_DEPENDENCY.category,
        severity,
        "circular module dependency",
        Span::empty(module.file_id),
    )
    .label("this module participates in an import cycle")
    .note(format!("cycle members: {members_note}"))
    .note(format!("rule: {rule_id}"));
    if let Some(cycle_path_note) = cycle_path_note {
        diagnostic = diagnostic.note(cycle_path_note.to_string());
    }

    diagnostic
}

/// Return eligible module ids for cycle checks.
fn collect_eligible_modules(ctx: &LintPackageContext) -> HashSet<ModuleId> {
    let mut modules = HashSet::new();

    for module_id in ctx.package_module_ids() {
        let Some(module) = ctx.session.repository_module(module_id) else {
            continue;
        };
        let Some(file) = ctx.session.repository_file(module.file_id) else {
            continue;
        };
        if !file.ty.is_code() || is_declaration_file(file.ty, ctx) {
            continue;
        }

        modules.insert(module.id);
    }

    modules
}

/// Collect display names for eligible modules.
fn collect_module_display_names(
    ctx: &LintPackageContext,
    module_ids: &HashSet<ModuleId>,
) -> HashMap<ModuleId, String> {
    let mut names = HashMap::new();

    for module_id in module_ids {
        let Some(module) = ctx.session.repository_module(*module_id) else {
            continue;
        };
        let module = module.as_ref();
        let Some(file) = ctx.session.repository_file(module.file_id) else {
            continue;
        };
        names.insert(*module_id, file.name.clone());
    }

    names
}

/// Format one module name for diagnostics.
fn format_module_name(module_names: &HashMap<ModuleId, String>, module_id: ModuleId) -> String {
    module_names
        .get(&module_id)
        .cloned()
        .unwrap_or_else(|| module_id.to_string())
}

/// Build one cycle path note for a component.
fn format_cycle_path_note(
    adjacency: &HashMap<ModuleId, Vec<ModuleId>>,
    component: &[ModuleId],
    module_names: &HashMap<ModuleId, String>,
) -> Option<String> {
    let cycle = find_cycle_path(adjacency, component)?;
    if cycle.len() < 2 {
        return None;
    }

    let cycle_names = cycle
        .iter()
        .map(|module_id| format_module_name(module_names, *module_id))
        .collect::<Vec<_>>();

    Some(format!("example cycle: {}", cycle_names.join(" -> ")))
}

/// Return true when the file should be skipped for declaration filtering.
fn is_declaration_file(file_type: FileType, ctx: &LintPackageContext) -> bool {
    if ctx.options().include_declaration_files {
        return false;
    }

    matches!(
        file_type,
        FileType::TypeScriptDeclaration | FileType::DestackDeclaration
    )
}

/// Build a dependency adjacency filtered to eligible modules.
fn build_adjacency(
    ctx: &LintPackageContext,
    eligible_modules: &HashSet<ModuleId>,
) -> HashMap<ModuleId, Vec<ModuleId>> {
    let mut adjacency = HashMap::new();
    let mut module_ids = eligible_modules.iter().copied().collect::<Vec<_>>();
    module_ids.sort_unstable();

    for module_id in module_ids {
        let mut dependencies = Vec::new();

        // collect post expansion import edges
        if let (Some(imported), Some(expanded)) = (
            ctx.session.dir_imported(module_id),
            ctx.session.dir_expanded(module_id),
        ) {
            let modules = expanded.module_table(&imported);
            collect_module_table_dependencies(&modules, &mut dependencies);
        }

        // collect export edges
        if let Some(exported) = ctx.session.dir_exported(module_id) {
            collect_exported_module_dependencies(&exported, &mut dependencies);
        }

        // filter to eligible modules
        dependencies.retain(|dependency| eligible_modules.contains(dependency));
        dependencies.sort_unstable();
        dependencies.dedup();
        adjacency.insert(module_id, dependencies);
    }

    adjacency
}

/// Extend one dependency list with direct import edges.
fn collect_module_table_dependencies(
    modules: &dir::ModuleTable<'_>,
    dependencies: &mut Vec<ModuleId>,
) {
    // collect import edges for both value and type space
    for dependency in modules.iter() {
        // keep resolved module edges
        if let Some(module_id) = dependency.target {
            dependencies.push(module_id);
        }
    }
}

/// Extend one dependency list with direct export edges.
fn collect_exported_module_dependencies(exported: &DirExported, dependencies: &mut Vec<ModuleId>) {
    // collect star export edges
    for export in exported.exports.star_exports() {
        if let Some(module_id) = export.target {
            dependencies.push(module_id);
        }
    }
}

/// Return cycle components using strongly connected components.
fn cycle_components(adjacency: &HashMap<ModuleId, Vec<ModuleId>>) -> Vec<Vec<ModuleId>> {
    let components = strongly_connected_components(adjacency);

    components
        .into_iter()
        .filter(|component| component_is_cycle(adjacency, component))
        .collect()
}

/// Return whether one strongly connected component represents a cycle.
fn component_is_cycle(
    adjacency: &HashMap<ModuleId, Vec<ModuleId>>,
    component: &[ModuleId],
) -> bool {
    if component.len() > 1 {
        return true;
    }

    let Some(module_id) = component.first().copied() else {
        return false;
    };

    adjacency
        .get(&module_id)
        .is_some_and(|dependencies| dependencies.contains(&module_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Add modules, run analysis, and lint the package at DIR level.
    fn lint_package_with_modules(
        modules: &[(&str, &str)],
        configure: impl FnOnce(&mut destack_repository::LinterOptions),
    ) -> (TestProgram, Vec<LintReport>) {
        let test = TestProgram::new_without_prelude(vec![crate::boxed(NoCircularDependency)])
            .with_options(configure);
        let diagnostics = test.lint_package_dir_with_modules(modules);
        (test, diagnostics)
    }

    /// Collect diagnostics for this rule only.
    fn circular_diagnostics(diagnostics: &[LintReport]) -> Vec<&LintReport> {
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.rule_id == "no-circular-dependency")
            .collect()
    }

    /// Report import cycles across modules.
    #[test]
    fn test_reports_circular_dependency() {
        let (test, result) = lint_package_with_modules(
            &[
                (
                    "no_circular_dependency/a.ds",
                    r#"
import { B } from "./b.ds";
export let A = B;
"#,
                ),
                (
                    "no_circular_dependency/b.ds",
                    r#"
import { A } from "./a.ds";
export let B = A;
"#,
                ),
            ],
            |_| {},
        );

        test.result(result)
            .assert_lint("no-circular-dependency")
            .assert_lint_count("no-circular-dependency", 2);
    }

    /// Allow modules without import cycles.
    #[test]
    fn test_allows_acyclic_dependencies() {
        let (test, result) = lint_package_with_modules(
            &[
                (
                    "no_circular_dependency/acyclic_a.ds",
                    r#"
import { B } from "./acyclic_b.ds";
export let A = B;
"#,
                ),
                (
                    "no_circular_dependency/acyclic_b.ds",
                    r#"
export let B = 1;
"#,
                ),
            ],
            |_| {},
        );

        test.result(result).assert_no_lint("no-circular-dependency");
    }

    /// Report cycles with three modules.
    #[test]
    fn test_reports_three_module_cycle() {
        let (test, result) = lint_package_with_modules(
            &[
                (
                    "no_circular_dependency/three_a.ds",
                    r#"
import { B } from "./three_b.ds";
export let A = B;
"#,
                ),
                (
                    "no_circular_dependency/three_b.ds",
                    r#"
import { C } from "./three_c.ds";
export let B = C;
"#,
                ),
                (
                    "no_circular_dependency/three_c.ds",
                    r#"
import { A } from "./three_a.ds";
export let C = A;
"#,
                ),
            ],
            |_| {},
        );

        test.result(result)
            .assert_lint("no-circular-dependency")
            .assert_lint_count("no-circular-dependency", 3);
    }

    /// Report disjoint cycle components independently.
    #[test]
    fn test_reports_disjoint_cycles() {
        let (test, result) = lint_package_with_modules(
            &[
                (
                    "no_circular_dependency/disjoint_a.ds",
                    r#"
import { B } from "./disjoint_b.ds";
export let A = B;
"#,
                ),
                (
                    "no_circular_dependency/disjoint_b.ds",
                    r#"
import { A } from "./disjoint_a.ds";
export let B = A;
"#,
                ),
                (
                    "no_circular_dependency/disjoint_c.ds",
                    r#"
import { D } from "./disjoint_d.ds";
export let C = D;
"#,
                ),
                (
                    "no_circular_dependency/disjoint_d.ds",
                    r#"
import { C } from "./disjoint_c.ds";
export let D = C;
"#,
                ),
            ],
            |_| {},
        );

        test.result(result)
            .assert_lint("no-circular-dependency")
            .assert_lint_count("no-circular-dependency", 4);
    }

    /// Report direct self import cycles.
    #[test]
    fn test_reports_self_import_cycle() {
        let (test, result) = lint_package_with_modules(
            &[(
                "no_circular_dependency/self_cycle.ds",
                r#"
import { value } from "./self_cycle.ds";
export let value = 1;
"#,
            )],
            |_| {},
        );

        test.result(result)
            .assert_lint("no-circular-dependency")
            .assert_lint_count("no-circular-dependency", 1);
    }

    /// Skip declaration files by default.
    #[test]
    fn test_skips_declaration_files_by_default() {
        let (test, result) = lint_package_with_modules(
            &[
                (
                    "no_circular_dependency/decl_a.d.ds",
                    r#"
import type { B } from "./decl_b.d.ds";

export type A = { b: B };
"#,
                ),
                (
                    "no_circular_dependency/decl_b.d.ds",
                    r#"
import type { A } from "./decl_a.d.ds";

export type B = { a: A };
"#,
                ),
            ],
            |_| {},
        );

        test.result(result).assert_no_lint("no-circular-dependency");
    }

    /// Report pure declaration cycles when declarations are included.
    #[test]
    #[ignore = "compiler artifacts do not materialize declaration-only type import edges yet"]
    fn test_reports_pure_declaration_cycles_when_enabled() {
        let (test, result) = lint_package_with_modules(
            &[
                (
                    "no_circular_dependency/decl_enabled_a.d.ds",
                    r#"
import type { B } from "./decl_enabled_b.d.ds";

export type A = { b: B };
"#,
                ),
                (
                    "no_circular_dependency/decl_enabled_b.d.ds",
                    r#"
import type { A } from "./decl_enabled_a.d.ds";

export type B = { a: A };
"#,
                ),
            ],
            |options| {
                options.include_declaration_files = true;
            },
        );

        test.result(result)
            .assert_lint("no-circular-dependency")
            .assert_lint_count("no-circular-dependency", 2);
    }

    /// Report only modules that are members of a cycle.
    #[test]
    fn test_reports_only_cycle_members_in_mixed_graph() {
        let (test, result) = lint_package_with_modules(
            &[
                (
                    "no_circular_dependency/mixed_a.ds",
                    r#"
import { B } from "./mixed_b.ds";
export let A = B;
"#,
                ),
                (
                    "no_circular_dependency/mixed_b.ds",
                    r#"
import { C } from "./mixed_c.ds";
export let B = C;
"#,
                ),
                (
                    "no_circular_dependency/mixed_c.ds",
                    r#"
import { A } from "./mixed_a.ds";
export let C = A;
"#,
                ),
                (
                    "no_circular_dependency/mixed_d.ds",
                    r#"
import { A } from "./mixed_a.ds";
export let D = A;
"#,
                ),
                (
                    "no_circular_dependency/mixed_e.ds",
                    r#"
import { D } from "./mixed_d.ds";
export let E = D;
"#,
                ),
            ],
            |_| {},
        );

        let circular = circular_diagnostics(&result);
        assert_eq!(circular.len(), 3);

        let mut file_names = circular
            .iter()
            .map(|diagnostic| test.repository_file(diagnostic.primary.file).name.clone())
            .collect::<Vec<_>>();
        file_names.sort();
        assert_eq!(
            file_names,
            vec![
                "mixed_a.ds".to_string(),
                "mixed_b.ds".to_string(),
                "mixed_c.ds".to_string(),
            ]
        );
    }

    /// Emit deterministic sorted cycle member notes.
    #[test]
    fn test_emits_sorted_cycle_member_note() {
        let (_test, result) = lint_package_with_modules(
            &[
                (
                    "no_circular_dependency/note_b.ds",
                    r#"
import { A } from "./note_a.ds";
export let B = A;
"#,
                ),
                (
                    "no_circular_dependency/note_a.ds",
                    r#"
import { B } from "./note_b.ds";
export let A = B;
"#,
                ),
            ],
            |_| {},
        );

        let circular = circular_diagnostics(&result);
        assert_eq!(circular.len(), 2);
        for diagnostic in circular {
            let notes = diagnostic.notes().collect::<Vec<_>>();
            assert!(
                notes
                    .iter()
                    .any(|note| { *note == "cycle members: note_a.ds, note_b.ds" }),
                "expected sorted cycle member note, notes={:?}",
                notes
            );
        }
    }

    /// Skip mixed declaration cycles by default.
    #[test]
    fn test_skips_mixed_declaration_cycle_by_default() {
        let (test, result) = lint_package_with_modules(
            &[
                (
                    "no_circular_dependency/mixed_decl_a.ds",
                    r#"
import type { B } from "./mixed_decl_b.d.ds";

export type A = { b: B };
"#,
                ),
                (
                    "no_circular_dependency/mixed_decl_b.d.ds",
                    r#"
import type { A } from "./mixed_decl_a.ds";

export type B = { a: A };
"#,
                ),
            ],
            |_| {},
        );

        test.result(result).assert_no_lint("no-circular-dependency");
    }

    /// Report mixed declaration cycles when declarations are included.
    #[test]
    fn test_reports_mixed_declaration_cycle_when_enabled() {
        let (test, result) = lint_package_with_modules(
            &[
                (
                    "no_circular_dependency/mixed_decl_enabled_a.ds",
                    r#"
import type { B } from "./mixed_decl_enabled_b.d.ds";

export type A = { b: B };
"#,
                ),
                (
                    "no_circular_dependency/mixed_decl_enabled_b.d.ds",
                    r#"
import type { A } from "./mixed_decl_enabled_a.ds";

export type B = { a: A };
"#,
                ),
            ],
            |options| {
                options.include_declaration_files = true;
            },
        );

        test.result(result)
            .assert_lint("no-circular-dependency")
            .assert_lint_count("no-circular-dependency", 2);
    }
}
