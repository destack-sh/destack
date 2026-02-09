use std::collections::{HashMap, HashSet};

use destack_source::{FileType, ModuleId, Span};
use destack_workspace::ModuleGraphKey;

use crate::rules::common::{find_cycle_path, strongly_connected_components};
use crate::{LintDiagnostic, LintProgramDirContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow circular module dependencies.
    ///
    /// Import cycles make code harder to reason about and can hide order dependent behavior.
    #[lint(
        id = "no-circular-dependency",
        code = "LR006",
        category = Restriction,
        level = Dir,
        scope = Program,
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
    fn meta(&self) -> &'static crate::LintMeta {
        NoCircularDependency::meta()
    }

    fn check_program_dir(&self, ctx: &mut LintProgramDirContext) {
        // resolve lint metadata
        let meta = self.meta();
        let severity = ctx.get_severity(meta);
        if !severity.is_enabled() {
            return;
        }

        // resolve module graph for this profile
        let graph_key = ModuleGraphKey::new(ctx.profile_id);
        let Some(graph) = ctx.program.index.module_graphs.get(&graph_key) else {
            return;
        };

        // collect modules eligible for this rule
        let eligible_modules = collect_eligible_modules(ctx);
        if eligible_modules.is_empty() {
            return;
        }
        let module_names = collect_module_display_names(ctx, &eligible_modules);

        // build the filtered dependency adjacency
        let adjacency = build_adjacency(&graph, &eligible_modules);
        drop(graph);

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
    severity: destack_workspace::LintSeverity,
    rule_id: &str,
    ctx: &LintProgramDirContext,
) -> LintDiagnostic {
    let module_ref = ctx.program.modules.get(module_id);
    let module = module_ref.read();

    let mut diagnostic = LintDiagnostic::new(
        NO_CIRCULAR_DEPENDENCY.id,
        NO_CIRCULAR_DEPENDENCY.code,
        NO_CIRCULAR_DEPENDENCY.category,
        severity,
        "circular module dependency",
        module.file_id,
        Span::empty(module.file_id),
    )
    .with_label("this module participates in an import cycle")
    .with_note(format!("cycle members: {members_note}"))
    .with_note(format!("rule: {rule_id}"));
    if let Some(cycle_path_note) = cycle_path_note {
        diagnostic = diagnostic.with_note(cycle_path_note.to_string());
    }

    diagnostic
}

/// Return eligible module ids for cycle checks.
fn collect_eligible_modules(ctx: &LintProgramDirContext) -> HashSet<ModuleId> {
    let mut modules = HashSet::new();

    for module_ref in ctx.program.modules.iter() {
        let module = module_ref.read();
        if !module.is_user() {
            continue;
        }

        let file = ctx.program.files.get(module.file_id);
        if !file.ty.is_code() || is_declaration_file(file.ty, ctx) {
            continue;
        }

        modules.insert(module.id);
    }

    modules
}

/// Collect display names for eligible modules.
fn collect_module_display_names(
    ctx: &LintProgramDirContext,
    module_ids: &HashSet<ModuleId>,
) -> HashMap<ModuleId, String> {
    let mut names = HashMap::new();

    for module_id in module_ids {
        let module_ref = ctx.program.modules.get(*module_id);
        let module = module_ref.read();
        let file = ctx.program.files.get(module.file_id);
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
fn is_declaration_file(file_type: FileType, ctx: &LintProgramDirContext) -> bool {
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
    graph: &destack_workspace::ModuleGraph,
    eligible_modules: &HashSet<ModuleId>,
) -> HashMap<ModuleId, Vec<ModuleId>> {
    let mut adjacency = HashMap::new();

    let mut module_ids = eligible_modules.iter().copied().collect::<Vec<_>>();
    module_ids.sort_unstable();

    for module_id in module_ids {
        let mut dependencies = graph
            .dependencies_for(module_id)
            .into_iter()
            .filter(|dependency| eligible_modules.contains(dependency))
            .collect::<Vec<_>>();
        dependencies.sort_unstable();
        adjacency.insert(module_id, dependencies);
    }

    adjacency
}

/// Return cycle components using strongly connected components.
fn cycle_components(adjacency: &HashMap<ModuleId, Vec<ModuleId>>) -> Vec<Vec<ModuleId>> {
    let components = strongly_connected_components(adjacency);

    components
        .into_iter()
        .filter(|component| {
            if component.len() > 1 {
                return true;
            }

            let module_id = component[0];
            adjacency
                .get(&module_id)
                .is_some_and(|dependencies| dependencies.contains(&module_id))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Add modules, run analysis, and lint the program at DIR level.
    fn lint_program_with_modules(
        modules: &[(&str, &str)],
        configure: impl FnOnce(&mut destack_workspace::LinterOptions),
    ) -> (TestProgram, Vec<LintDiagnostic>) {
        let test = TestProgram::new_without_prelude(vec![crate::boxed(NoCircularDependency)])
            .with_options(configure);
        let diagnostics = test.lint_program_dir_with_modules(modules);
        (test, diagnostics)
    }

    /// Collect diagnostics for this rule only.
    fn circular_diagnostics(diagnostics: &[LintDiagnostic]) -> Vec<&LintDiagnostic> {
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.rule_id == "no-circular-dependency")
            .collect()
    }

    /// Report import cycles across modules.
    #[test]
    fn test_reports_circular_dependency() {
        let (test, result) = lint_program_with_modules(
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
        let (test, result) = lint_program_with_modules(
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
        let (test, result) = lint_program_with_modules(
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
        let (test, result) = lint_program_with_modules(
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

    /// Report a module importing itself as a cycle.
    #[test]
    fn test_reports_self_cycle() {
        let (test, result) = lint_program_with_modules(
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
        let (test, result) = lint_program_with_modules(
            &[
                (
                    "no_circular_dependency/decl_a.d.ds",
                    r#"
import { B } from "./decl_b.d.ds";
export let A = B;
"#,
                ),
                (
                    "no_circular_dependency/decl_b.d.ds",
                    r#"
import { A } from "./decl_a.d.ds";
export let B = A;
"#,
                ),
            ],
            |_| {},
        );

        test.result(result).assert_no_lint("no-circular-dependency");
    }

    /// Include declaration files when configured.
    #[test]
    fn test_includes_declaration_files_when_enabled() {
        let (test, result) = lint_program_with_modules(
            &[
                (
                    "no_circular_dependency/decl_enabled_a.d.ds",
                    r#"
import { B } from "./decl_enabled_b.d.ds";
export let A = B;
"#,
                ),
                (
                    "no_circular_dependency/decl_enabled_b.d.ds",
                    r#"
import { A } from "./decl_enabled_a.d.ds";
export let B = A;
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
        let (test, result) = lint_program_with_modules(
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
            .map(|diagnostic| test.program.files.get(diagnostic.file_id).name.clone())
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
        let (_test, result) = lint_program_with_modules(
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
            assert!(
                diagnostic
                    .notes
                    .iter()
                    .any(|note| { note == "cycle members: note_a.ds, note_b.ds" }),
                "expected sorted cycle member note, notes={:?}",
                diagnostic.notes
            );
        }
    }

    /// Skip mixed declaration cycles by default.
    #[test]
    fn test_skips_mixed_declaration_cycle_by_default() {
        let (test, result) = lint_program_with_modules(
            &[
                (
                    "no_circular_dependency/mixed_decl_a.ds",
                    r#"
import { B } from "./mixed_decl_b.d.ds";
export let A = B;
"#,
                ),
                (
                    "no_circular_dependency/mixed_decl_b.d.ds",
                    r#"
import { A } from "./mixed_decl_a.ds";
export let B = A;
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
        let (test, result) = lint_program_with_modules(
            &[
                (
                    "no_circular_dependency/mixed_decl_enabled_a.ds",
                    r#"
import { B } from "./mixed_decl_enabled_b.d.ds";
export let A = B;
"#,
                ),
                (
                    "no_circular_dependency/mixed_decl_enabled_b.d.ds",
                    r#"
import { A } from "./mixed_decl_enabled_a.ds";
export let B = A;
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
