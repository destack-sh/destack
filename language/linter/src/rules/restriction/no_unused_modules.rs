use crate::LintMeta;
use std::collections::HashSet;

use destack_artifact::{DirExported, DirImported};
use destack_source::{FileType, ModuleId, Span};
use destack_workspace::TargetDiscovery;

use crate::{LintReport, LintRule, LintWorkspaceDirContext, declare_lint};

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
        scope = Workspace,
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
    fn meta(&self) -> &'static LintMeta {
        NoUnusedModules::meta()
    }

    fn check_workspace_dir(&self, ctx: &mut LintWorkspaceDirContext) {
        let meta = self.meta();
        let severity = ctx.get_severity(meta);
        if !severity.is_enabled() {
            return;
        }

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

            let has_user_dependents = eligible_modules.iter().copied().any(|dependent| {
                dependent != module_id
                    && module_dependencies(ctx, dependent)
                        .into_iter()
                        .any(|dependency| dependency == module_id)
            });
            if has_user_dependents {
                continue;
            }

            unused_module_ids.push(module_id);
        }

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
            let Some(module) = ctx.repository_module(module_id) else {
                continue;
            };
            let module = module.as_ref();
            let Some(file) = ctx.repository_file(module.file_id) else {
                continue;
            };

            ctx.report(
                LintReport::new(
                    NO_UNUSED_MODULES.id,
                    NO_UNUSED_MODULES.code,
                    NO_UNUSED_MODULES.category,
                    severity,
                    "unused exported module",
                    Span::empty(module.file_id),
                )
                .label("this module exports symbols but is never imported")
                .note(format!("module: {}", file.name)),
            );
        }
    }
}

impl NoUnusedModules {
    /// Collect eligible modules reachable from one target entry root.
    fn collect_reachable_entry_modules(
        &self,
        ctx: &LintWorkspaceDirContext,
        entry_module_id: ModuleId,
        eligible_modules: &HashSet<ModuleId>,
        entry_modules: &mut HashSet<ModuleId>,
    ) {
        let mut pending = vec![entry_module_id];
        let mut visited = HashSet::new();

        // walk exported dependencies from the target entry
        while let Some(module_id) = pending.pop() {
            if !visited.insert(module_id) {
                continue;
            }

            if eligible_modules.contains(&module_id) {
                entry_modules.insert(module_id);
            }

            for dependency in module_dependencies(ctx, module_id) {
                pending.push(dependency);
            }
        }
    }
}

/// Collect user code modules eligible for this rule.
fn collect_eligible_modules(ctx: &LintWorkspaceDirContext) -> HashSet<ModuleId> {
    let mut modules = HashSet::new();

    // inspect all modules and keep user code modules only
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

        modules.insert(module.id);
    }

    modules
}

/// Collect target entry modules from configured package targets.
fn collect_profile_target_entry_modules(
    ctx: &LintWorkspaceDirContext,
    eligible_modules: &HashSet<ModuleId>,
) -> HashSet<ModuleId> {
    let mut entry_modules = HashSet::new();

    // inspect package targets for entry roots
    for package_id in ctx.workspace_package_ids() {
        let Some(package) = ctx.repository_package(package_id) else {
            continue;
        };
        for (target_id, target) in &package.targets {
            if target.discovery != TargetDiscovery::Entry {
                continue;
            }

            let discovered_modules = ctx.repository.target_module_ids(ctx.revision, *target_id);
            let Ok(discovered_modules) = discovered_modules else {
                continue;
            };

            // walk from every target entry so document roots keep reachable code alive
            for module_id in discovered_modules {
                NoUnusedModules.collect_reachable_entry_modules(
                    ctx,
                    module_id,
                    eligible_modules,
                    &mut entry_modules,
                );
            }
        }
    }

    entry_modules
}

/// Return true when a file should be treated as declaration-only.
fn is_declaration_file(file_type: FileType, ctx: &LintWorkspaceDirContext) -> bool {
    if ctx.options().include_declaration_files {
        return false;
    }

    matches!(
        file_type,
        FileType::TypeScriptDeclaration | FileType::DestackDeclaration
    )
}

/// Return true when the module has exports in the active profile DIR.
fn module_has_exports(ctx: &LintWorkspaceDirContext, module_id: ModuleId) -> bool {
    let Some(dir) = ctx.exported_dir(module_id) else {
        return false;
    };

    // keep any module with named exports, export assignment, or namespace exports
    !dir.exports.is_empty()
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

/// Return the file name for deterministic sorting.
fn module_file_name(ctx: &LintWorkspaceDirContext, module_id: ModuleId) -> String {
    let Some(module) = ctx.repository_module(module_id) else {
        return module_id.to_string();
    };
    let module = module.as_ref();
    let Some(file) = ctx.repository_file(module.file_id) else {
        return module_id.to_string();
    };
    file.name.clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linter::TestProgram;

    /// Add modules, run DIR analysis, and lint workspace scope rules.
    fn lint_workspace_with_modules(
        modules: &[(&str, &str)],
        entry_paths: &[&str],
        configure: impl FnOnce(&mut destack_workspace::LinterOptions),
    ) -> (TestProgram, Vec<LintReport>) {
        let test = TestProgram::new_without_prelude(vec![crate::boxed(NoUnusedModules)])
            .with_options(configure);

        let mut module_ids = Vec::new();

        // add all modules to the test repository
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
            test.set_root_target_entries("lint-entry", entry_paths);
        }

        let diagnostics = test.lint_workspace_dir();
        (test, diagnostics)
    }

    /// Report exported modules that are never imported.
    #[test]
    fn test_flags_unused_exported_module() {
        let (test, diagnostics) = lint_workspace_with_modules(
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
        let (test, diagnostics) = lint_workspace_with_modules(
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
        let (test, diagnostics) = lint_workspace_with_modules(
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
        let (test, diagnostics) = lint_workspace_with_modules(
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
        let (test, diagnostics) = lint_workspace_with_modules(
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
        let (test, diagnostics) = lint_workspace_with_modules(
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
        let (test, diagnostics) = lint_workspace_with_modules(
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
            .map(|diagnostic| test.repository_file(diagnostic.primary.file).name.clone())
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
