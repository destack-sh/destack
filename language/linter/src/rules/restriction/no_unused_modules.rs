use std::collections::HashSet;

use destack_source::{FileType, ModuleId, Span};
use destack_workspace::{ModuleGraphKey, TargetDiscovery, discover_entry_modules_relaxed};

use crate::{LintDiagnostic, LintProgramDirContext, LintRule, declare_lint};

declare_lint! {
    /// Disallow exported modules that are never imported by another module.
    ///
    /// Unused exported modules increase maintenance surface and can indicate
    /// dead or misplaced API boundaries.
    #[lint(
        id = "no-unused-modules",
        code = "LR029",
        category = Restriction,
        level = Dir,
        scope = Program,
        requires_all = [],
        requires_any = [],
        fixable = No,
        recommended = Off,
        stability = Stable,
        declarations = Exclude
    )]
    pub NoUnusedModules,
    "Disallow exported modules that are never imported"
}

impl LintRule for NoUnusedModules {
    fn meta(&self) -> &'static crate::LintMeta {
        NoUnusedModules::meta()
    }

    fn check_program_dir(&self, ctx: &mut LintProgramDirContext) {
        let meta = self.meta();
        let severity = ctx.get_severity(meta);
        if !severity.is_enabled() {
            return;
        }

        // resolve the module graph for the active profile
        let graph_key = ModuleGraphKey::new(ctx.profile_id);
        let Some(graph) = ctx.program.index.module_graphs.get(&graph_key) else {
            return;
        };

        // collect user code modules eligible for this rule
        let eligible_modules = collect_eligible_modules(ctx);
        if eligible_modules.is_empty() {
            return;
        }
        let entry_modules = collect_profile_target_entry_modules(ctx, &eligible_modules);

        // collect exported modules with no incoming user import edges
        let mut unused_module_ids = Vec::new();
        for module_id in eligible_modules.iter().copied() {
            if !module_has_exports(ctx, module_id) {
                continue;
            }
            if entry_modules.contains(&module_id) {
                continue;
            }

            let has_user_dependents = graph
                .dependents_for(module_id)
                .into_iter()
                .filter(|dependent| eligible_modules.contains(dependent))
                .any(|dependent| dependent != module_id);
            if has_user_dependents {
                continue;
            }

            unused_module_ids.push(module_id);
        }
        drop(graph);

        if unused_module_ids.is_empty() {
            return;
        }

        // emit deterministic diagnostics sorted by file name
        unused_module_ids.sort_by(|left, right| {
            let left_name = module_file_name(ctx, *left);
            let right_name = module_file_name(ctx, *right);
            left_name.cmp(&right_name).then_with(|| left.cmp(right))
        });

        for module_id in unused_module_ids {
            let module_ref = ctx.program.modules.get(module_id);
            let module = module_ref.read();
            let file = ctx.program.files.get(module.file_id);

            ctx.report(
                LintDiagnostic::new(
                    NO_UNUSED_MODULES.id,
                    NO_UNUSED_MODULES.code,
                    NO_UNUSED_MODULES.category,
                    severity,
                    "unused exported module",
                    module.file_id,
                    Span::empty(module.file_id),
                )
                .with_label("this module exports symbols but is never imported")
                .with_note(format!("module: {}", file.name)),
            );
        }
    }
}

/// Collect user code modules eligible for this rule.
fn collect_eligible_modules(ctx: &LintProgramDirContext) -> HashSet<ModuleId> {
    let mut modules = HashSet::new();

    // inspect all modules and keep user code modules only
    for module_ref in ctx.program.modules.iter() {
        let module = module_ref.read();
        if module.id == ctx.program.root_module_id {
            continue;
        }
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

/// Collect target entry modules from configured package targets.
fn collect_profile_target_entry_modules(
    ctx: &LintProgramDirContext,
    eligible_modules: &HashSet<ModuleId>,
) -> HashSet<ModuleId> {
    let mut entry_modules = HashSet::new();

    // inspect package targets for entry roots
    for package_ref in ctx.program.packages.iter() {
        let package = package_ref.read();
        let package_path = package.path.clone();

        for (target_id, target) in &package.targets {
            if target.synthetic || target.discovery != TargetDiscovery::Entry {
                continue;
            }

            let discovered_modules = discover_entry_modules_relaxed(
                &ctx.program.modules,
                package.id,
                &package_path,
                target,
                target_id,
            );
            let Ok(discovered_modules) = discovered_modules else {
                continue;
            };

            // include only modules that this lint can report
            for module_id in discovered_modules {
                if eligible_modules.contains(&module_id) {
                    entry_modules.insert(module_id);
                }
            }
        }
    }

    entry_modules
}

/// Return true when a file should be treated as declaration-only.
fn is_declaration_file(file_type: FileType, ctx: &LintProgramDirContext) -> bool {
    if ctx.options().include_declaration_files {
        return false;
    }

    matches!(
        file_type,
        FileType::TypeScriptDeclaration | FileType::DestackDeclaration
    )
}

/// Return true when the module has exports in the active profile DIR.
fn module_has_exports(ctx: &LintProgramDirContext, module_id: ModuleId) -> bool {
    let module_ref = ctx.program.modules.get(module_id);
    let module = module_ref.read();
    let Some(dir) = module.dir_maybe(ctx.profile_id) else {
        return false;
    };

    // keep any module with named exports, export assignment, or namespace exports
    !dir.exported_symbols.read().is_empty()
        || dir.export_assignment.read().is_some()
        || !dir.namespace_exports.read().is_empty()
}

/// Return the file name for deterministic sorting.
fn module_file_name(ctx: &LintProgramDirContext, module_id: ModuleId) -> String {
    let module_ref = ctx.program.modules.get(module_id);
    let module = module_ref.read();
    let file = ctx.program.files.get(module.file_id);
    file.name.clone()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use destack_workspace::TargetDiscovery;

    use super::*;
    use crate::linter::TestProgram;

    /// Add modules, run DIR analysis, and lint program scope rules.
    fn lint_program_with_modules(
        modules: &[(&str, &str)],
        entry_paths: &[&str],
        configure: impl FnOnce(&mut destack_workspace::LinterOptions),
    ) -> (TestProgram, Vec<LintDiagnostic>) {
        let test = TestProgram::new_without_prelude(vec![crate::boxed(NoUnusedModules)])
            .with_options(configure);

        let mut module_ids = Vec::new();

        // add all modules to the test program
        for (path, source) in modules {
            let module_id = test.add_module(path, source);
            module_ids.push(module_id);
        }

        // run import and analysis for all modules
        for module_id in &module_ids {
            test.import_module(*module_id);
        }
        test.enqueue_profile_resolution_once();
        for module_id in &module_ids {
            test.analyze_module(*module_id);
        }
        test.compile();

        // configure one explicit target entry root when requested
        if !entry_paths.is_empty() && !module_ids.is_empty() {
            let first_module = test.program.modules.get(module_ids[0]);
            let first_module = first_module.read();
            let package_id = first_module.package_id;
            drop(first_module);

            let package_ref = test.program.packages.get(package_id);
            let mut package = package_ref.write();
            let target_id = destack_workspace::TargetId::new(package_id, "lint-entry");
            let target = destack_workspace::Target::js("lint-entry")
                .with_discovery(TargetDiscovery::Entry)
                .with_runtime(destack_workspace::Runtime::Browser)
                .with_platform(destack_workspace::Platform::Web)
                .with_lib(vec!["es2024".to_string()])
                .with_entry(entry_paths.iter().map(PathBuf::from).collect());
            package.targets.insert(target_id, target);
        }

        let diagnostics = test.lint_program_dir();
        (test, diagnostics)
    }

    /// Report exported modules that are never imported.
    #[test]
    fn test_flags_unused_exported_module() {
        let (test, diagnostics) = lint_program_with_modules(
            &[(
                "no_unused_modules/unused.ds",
                r#"
export const value = 1;
"#,
            )],
            &[],
            |_| {},
        );

        test.result(diagnostics)
            .assert_lint("no-unused-modules")
            .assert_lint_count("no-unused-modules", 1);
    }

    /// Allow exported modules when another module imports them.
    #[test]
    fn test_allows_imported_exported_module() {
        let (test, diagnostics) = lint_program_with_modules(
            &[
                (
                    "no_unused_modules/source.ds",
                    r#"
export const value = 1;
"#,
                ),
                (
                    "no_unused_modules/consumer.ds",
                    r#"
import { value } from "./source.ds";
const copy = value;
"#,
                ),
            ],
            &[],
            |_| {},
        );

        test.result(diagnostics).assert_no_lint("no-unused-modules");
    }

    /// Ignore modules that do not export anything.
    #[test]
    fn test_ignores_modules_without_exports() {
        let (test, diagnostics) = lint_program_with_modules(
            &[(
                "no_unused_modules/no_exports.ds",
                r#"
const value = 1;
"#,
            )],
            &[],
            |_| {},
        );

        test.result(diagnostics).assert_no_lint("no-unused-modules");
    }

    /// Report each unused exported module independently.
    #[test]
    fn test_reports_multiple_unused_modules() {
        let (test, diagnostics) = lint_program_with_modules(
            &[
                (
                    "no_unused_modules/unused_first.ds",
                    r#"
export const first = 1;
"#,
                ),
                (
                    "no_unused_modules/unused_second.ds",
                    r#"
export const second = 2;
"#,
                ),
            ],
            &[],
            |_| {},
        );

        test.result(diagnostics)
            .assert_lint("no-unused-modules")
            .assert_lint_count("no-unused-modules", 2);
    }

    /// Skip declaration files by default.
    #[test]
    fn test_skips_declaration_files_by_default() {
        let (test, diagnostics) = lint_program_with_modules(
            &[(
                "no_unused_modules/unused_decl.d.ds",
                r#"
export const value: int32;
"#,
            )],
            &[],
            |_| {},
        );

        test.result(diagnostics).assert_no_lint("no-unused-modules");
    }

    /// Include declaration files when requested.
    #[test]
    fn test_includes_declaration_files_when_enabled() {
        let (test, diagnostics) = lint_program_with_modules(
            &[(
                "no_unused_modules/unused_decl_enabled.d.ds",
                r#"
export const value: int32;
"#,
            )],
            &[],
            |options| {
                options.include_declaration_files = true;
            },
        );

        test.result(diagnostics).assert_lint("no-unused-modules");
    }

    /// Keep entry modules for the active profile out of unused-module diagnostics.
    #[test]
    fn test_skips_active_profile_entry_modules() {
        let (test, diagnostics) = lint_program_with_modules(
            &[
                (
                    "no_unused_modules/entry.ds",
                    r#"
export const run = 1;
"#,
                ),
                (
                    "no_unused_modules/unused.ds",
                    r#"
export const dead = 1;
"#,
                ),
            ],
            &["no_unused_modules/entry.ds"],
            |_| {},
        );

        let lint_file_names = diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.rule_id == "no-unused-modules")
            .map(|diagnostic| test.program.files.get(diagnostic.file_id).name.clone())
            .collect::<Vec<_>>();

        assert!(
            lint_file_names
                .iter()
                .any(|name| name.ends_with("unused.ds"))
        );
        assert!(
            !lint_file_names
                .iter()
                .any(|name| name.ends_with("entry.ds"))
        );
        test.result(diagnostics)
            .assert_lint_count("no-unused-modules", 1);
    }
}
