use std::sync::Arc;
use std::time::{Duration, Instant};

use destack_source::{ModuleId, PackageId};
use destack_workspace::{
    LintCategory, LintPreset, LinterOptions, Module, Package, Profile, ProfileId, Repository,
    Revision, Workspace,
};

use crate::linter::artifact::{read_ast, read_dir_checked, read_dir_declared, read_dir_exported};
use crate::{
    BoxedLintRule, LintAstContext, LintLevel, LintModuleDirContext, LintPackageAstContext,
    LintPackageDirContext, LintReport, LintScope, LintWorkspaceAstContext, LintWorkspaceDirContext,
    all_rules, recommended_rules, strict_rules,
};

/// Per lint rule performance metrics.
#[derive(Debug, Clone)]
pub struct LintRulePerformance {
    /// The lint rule id.
    pub id: &'static str,
    /// The lint diagnostic code.
    pub code: &'static str,
    /// The lint type name.
    pub name: &'static str,
    /// The lint category.
    pub category: LintCategory,
    /// The lint IR level.
    pub level: LintLevel,
    /// How many times this rule ran.
    pub runs: usize,
    /// How many diagnostics this rule produced.
    pub diagnostics: usize,
    /// The accumulated execution time for this rule.
    pub total_duration: Duration,
}

impl LintRulePerformance {
    /// Return the average execution time per run.
    pub fn average_duration(&self) -> Duration {
        if self.runs == 0 {
            return Duration::ZERO;
        }

        let nanos = self.total_duration.as_nanos() / self.runs as u128;
        let capped_nanos = nanos.min(u64::MAX as u128) as u64;
        Duration::from_nanos(capped_nanos)
    }
}

/// Aggregate lint performance metrics for a run.
#[derive(Debug, Clone, Default)]
pub struct LintPerformanceReport {
    /// Total time spent executing lint rules.
    pub total_duration: Duration,
    /// Total number of rule executions.
    pub total_rule_runs: usize,
    /// Total number of diagnostics emitted.
    pub total_diagnostics: usize,
    /// Per rule timing metrics.
    pub rules: Vec<LintRulePerformance>,
}

#[allow(clippy::too_many_arguments)]
impl LintPerformanceReport {
    /// Record one rule execution sample.
    pub fn record(
        &mut self,
        id: &'static str,
        code: &'static str,
        name: &'static str,
        category: LintCategory,
        level: LintLevel,
        duration: Duration,
        diagnostics: usize,
    ) {
        self.total_duration += duration;
        self.total_rule_runs += 1;
        self.total_diagnostics += diagnostics;

        if let Some(existing) = self.rules.iter_mut().find(|rule| rule.id == id) {
            existing.runs += 1;
            existing.diagnostics += diagnostics;
            existing.total_duration += duration;
            return;
        }

        self.rules.push(LintRulePerformance {
            id,
            code,
            name,
            category,
            level,
            runs: 1,
            diagnostics,
            total_duration: duration,
        });
    }

    /// Merge another performance report into this one.
    pub fn merge(&mut self, other: &Self) {
        self.total_duration += other.total_duration;
        self.total_rule_runs += other.total_rule_runs;
        self.total_diagnostics += other.total_diagnostics;

        for rule in &other.rules {
            if let Some(existing) = self.rules.iter_mut().find(|entry| entry.id == rule.id) {
                existing.runs += rule.runs;
                existing.diagnostics += rule.diagnostics;
                existing.total_duration += rule.total_duration;
            } else {
                self.rules.push(rule.clone());
            }
        }
    }

    /// Return rules sorted by descending total duration.
    pub fn sorted_by_total_duration(&self) -> Vec<LintRulePerformance> {
        let mut rules = self.rules.clone();
        rules.sort_by_key(|rule| std::cmp::Reverse(rule.total_duration));
        rules
    }
}

/// Lint result for one module with diagnostics and performance metrics.
#[derive(Debug, Clone, Default)]
pub struct LintModuleReport {
    /// The emitted diagnostics.
    pub diagnostics: Vec<LintReport>,
    /// Rule execution performance metrics.
    pub performance: LintPerformanceReport,
}

/// Lint result for a full multi module run.
#[derive(Debug, Clone, Default)]
pub struct LintRunReport {
    /// The emitted diagnostics.
    pub diagnostics: Vec<LintReport>,
    /// Rule execution performance metrics.
    pub performance: LintPerformanceReport,
}

/// Runs lint rules against modules, packages, and workspaces.
pub struct LintRunner {
    rules: Vec<BoxedLintRule>,
    /// Whether to compute fixes for diagnostics.
    compute_fixes: bool,
}

impl std::fmt::Debug for LintRunner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LintRunner")
            .field("rules", &self.rules.len())
            .finish()
    }
}

impl LintRunner {
    /// Create a runner with the given rules.
    pub fn new(rules: Vec<BoxedLintRule>) -> Self {
        Self {
            rules,
            compute_fixes: true,
        }
    }

    /// Enable or disable fix computation (builder pattern).
    pub fn with_fixes(mut self, compute: bool) -> Self {
        self.compute_fixes = compute;
        self
    }

    /// Check if fix computation is enabled.
    pub fn compute_fixes(&self) -> bool {
        self.compute_fixes
    }

    /// Create a runner with rules based on preset.
    pub fn from_preset(preset: LintPreset) -> Self {
        let rules = match preset {
            LintPreset::None => Vec::new(),
            LintPreset::Recommended => recommended_rules(),
            LintPreset::Strict => strict_rules(),
            LintPreset::All => all_rules(),
        };
        Self {
            rules,
            compute_fixes: true,
        }
    }

    /// Create a runner from linter options.
    pub fn from_options(options: &LinterOptions) -> Self {
        if options.categories.is_empty() && options.overrides.is_empty() {
            return Self::from_preset(options.preset);
        }

        // explicit category or rule overrides can enable any rule
        Self::all()
    }

    /// Create a runner with all rules.
    pub fn all() -> Self {
        Self::new(all_rules())
    }

    /// Create a runner with recommended rules.
    pub fn recommended() -> Self {
        Self::new(recommended_rules())
    }

    /// Create an empty runner.
    pub fn empty() -> Self {
        Self {
            rules: Vec::new(),
            compute_fixes: true,
        }
    }

    /// Get the rules.
    pub fn rules(&self) -> &[BoxedLintRule] {
        &self.rules
    }

    /// Return one module for one revision when present.
    fn repository_module(
        repository: &Repository,
        revision: Revision,
        module_id: ModuleId,
    ) -> Option<Arc<Module>> {
        repository.module(revision, module_id).ok().flatten()
    }

    /// Return one source file for one revision when present.
    fn repository_file(
        repository: &Repository,
        revision: Revision,
        file_id: destack_source::FileId,
    ) -> Option<Arc<destack_source::File>> {
        repository.file(revision, file_id).ok().flatten()
    }

    /// Return one package for one revision when present.
    fn repository_package(
        repository: &Repository,
        revision: Revision,
        package_id: PackageId,
    ) -> Option<Arc<Package>> {
        repository.package(revision, package_id).ok().flatten()
    }

    /// Return workspace metadata for one revision when present.
    fn repository_workspace(repository: &Repository, revision: Revision) -> Option<Arc<Workspace>> {
        repository.workspace(revision).ok()
    }

    /// Lint a module at a specific IR level.
    pub fn lint_module(
        &self,
        repository: Arc<Repository>,
        revision: Revision,
        module: Arc<Module>,
        profile: Profile,
        options: &LinterOptions,
        level: LintLevel,
    ) -> Vec<LintReport> {
        self.lint_module_profiled(repository, revision, module, profile, options, level)
            .diagnostics
    }

    /// Lint a module at a specific IR level and collect performance metrics.
    pub fn lint_module_profiled(
        &self,
        repository: Arc<Repository>,
        revision: Revision,
        module: Arc<Module>,
        profile: Profile,
        options: &LinterOptions,
        level: LintLevel,
    ) -> LintModuleReport {
        if !options.enabled {
            return LintModuleReport::default();
        }
        let mut performance = LintPerformanceReport::default();

        let diagnostics = match level {
            LintLevel::Ast => self.lint_module_ast(
                repository,
                revision,
                module,
                profile,
                options,
                Some(&mut performance),
            ),
            LintLevel::Dir => self.lint_module_dir(
                repository,
                revision,
                module,
                profile,
                options,
                Some(&mut performance),
            ),
            LintLevel::Mir => todo!("MIR rules not yet supported"),
        };

        LintModuleReport {
            diagnostics,
            performance,
        }
    }

    /// Lint a module at a specific IR level.
    fn lint_module_ast(
        &self,
        repository: Arc<Repository>,
        revision: Revision,
        module: Arc<Module>,
        _profile: Profile,
        options: &LinterOptions,
        mut performance: Option<&mut LintPerformanceReport>,
    ) -> Vec<LintReport> {
        let module = module.as_ref();
        let ast = read_ast(&repository, revision, module.id)
            .expect("lint AST pass requires committed AST artifact");
        let Some(file) = Self::repository_file(repository.as_ref(), revision, module.file_id)
        else {
            return Vec::new();
        };
        let mut ctx = LintAstContext::new(
            repository,
            module,
            revision,
            file,
            &ast.tree,
            &ast.parents,
            &ast.roots,
            &ast.strings,
            options,
            self.compute_fixes,
        );

        // check ast rules
        for rule in &self.rules {
            let meta = rule.meta();
            if meta.level == LintLevel::Ast
                && meta.scope == LintScope::Module
                && ctx.is_rule_supported(meta)
                && ctx.is_rule_enabled(meta)
            {
                let severity = ctx.get_severity(meta);
                if let Some(performance) = performance.as_deref_mut() {
                    let diagnostics_before = ctx.diagnostics().len();
                    let start = Instant::now();
                    rule.check_module_ast(severity, &mut ctx);
                    let duration = start.elapsed();
                    let diagnostics_after = ctx.diagnostics().len();
                    let diagnostics_added = diagnostics_after.saturating_sub(diagnostics_before);

                    performance.record(
                        meta.id,
                        meta.code,
                        meta.name,
                        meta.category,
                        meta.level,
                        duration,
                        diagnostics_added,
                    );
                } else {
                    rule.check_module_ast(severity, &mut ctx);
                }
            }
        }

        ctx.take_diagnostics()
    }

    /// Lint a module at DIR level.
    fn lint_module_dir(
        &self,
        repository: Arc<Repository>,
        revision: Revision,
        module: Arc<Module>,
        profile: Profile,
        options: &LinterOptions,
        mut performance: Option<&mut LintPerformanceReport>,
    ) -> Vec<LintReport> {
        // context
        let module = module.as_ref();
        let ast = read_ast(&repository, revision, module.id)
            .expect("lint DIR pass requires committed AST artifact");
        let Some(file) = Self::repository_file(repository.as_ref(), revision, module.file_id)
        else {
            return Vec::new();
        };
        let declared = read_dir_declared(&repository, revision, module.id, profile.id())
            .expect("lint DIR pass requires committed declared DIR artifact");
        let checked = read_dir_checked(&repository, revision, module.id, profile.id())
            .expect("lint DIR pass requires committed checked DIR artifact");
        let resolved = read_dir_exported(&repository, revision, module.id, profile.id())
            .expect("lint DIR pass requires committed exported DIR artifact");

        let mut ctx = LintModuleDirContext::new(
            repository,
            module,
            revision,
            profile,
            file,
            &ast.tree,
            &declared.tree,
            &declared.strings,
            &declared.symbols,
            &checked.types,
            declared.roots.clone(),
            declared.namespace_symbol,
            declared.namespace_scope,
            declared.default_symbol,
            resolved
                .namespace_exports
                .iter()
                .map(|export| export.module_id)
                .collect(),
            resolved.import_resolutions.clone(),
            resolved.export_by_symbol_key.clone(),
            options,
            self.compute_fixes,
        );

        // check dir rules
        for rule in &self.rules {
            let meta = rule.meta();
            if meta.level == LintLevel::Dir
                && meta.scope == LintScope::Module
                && ctx.is_rule_supported(meta)
                && ctx.is_rule_enabled(meta)
            {
                let severity = ctx.get_severity(meta);
                if let Some(performance) = performance.as_deref_mut() {
                    let diagnostics_before = ctx.diagnostics().len();
                    let start = Instant::now();
                    rule.check_module_dir(severity, &mut ctx);
                    let duration = start.elapsed();
                    let diagnostics_after = ctx.diagnostics().len();
                    let diagnostics_added = diagnostics_after.saturating_sub(diagnostics_before);

                    performance.record(
                        meta.id,
                        meta.code,
                        meta.name,
                        meta.category,
                        meta.level,
                        duration,
                        diagnostics_added,
                    );
                } else {
                    rule.check_module_dir(severity, &mut ctx);
                }
            }
        }

        ctx.take_diagnostics()
    }

    /// Lint a module by id.
    pub fn lint_module_by_id(
        &self,
        repository: Arc<Repository>,
        revision: Revision,
        module_id: ModuleId,
        profile: Profile,
        options: &LinterOptions,
        level: LintLevel,
    ) -> Vec<LintReport> {
        let Some(module) = Self::repository_module(repository.as_ref(), revision, module_id) else {
            return Vec::new();
        };
        self.lint_module(repository, revision, module, profile, options, level)
    }

    /// Lint all modules at a specific level.
    pub fn lint_all_modules(
        &self,
        repository: Arc<Repository>,
        revision: Revision,
        options: &LinterOptions,
        level: LintLevel,
    ) -> Vec<LintReport> {
        self.lint_all_modules_profiled(repository, revision, options, level)
            .diagnostics
    }

    /// Lint all modules at a specific level and collect performance metrics.
    pub fn lint_all_modules_profiled(
        &self,
        repository: Arc<Repository>,
        revision: Revision,
        options: &LinterOptions,
        level: LintLevel,
    ) -> LintRunReport {
        if !options.enabled {
            return LintRunReport::default();
        }

        let mut diagnostics = Vec::new();
        let mut performance = LintPerformanceReport::default();
        for module_id in repository
            .module_ids(revision)
            .expect("workspace module ids should load")
        {
            let Some(module) = Self::repository_module(repository.as_ref(), revision, module_id)
            else {
                continue;
            };
            let Ok(profile) = repository.module_profile(revision, module.id) else {
                continue;
            };
            let report = self.lint_module_profiled(
                repository.clone(),
                revision,
                module,
                profile.as_ref().clone(),
                options,
                level,
            );
            diagnostics.extend(report.diagnostics);
            performance.merge(&report.performance);
        }

        LintRunReport {
            diagnostics,
            performance,
        }
    }

    /// Lint the active workspace at AST level using workspace-scope rules.
    pub fn lint_workspace_ast(
        &self,
        repository: Arc<Repository>,
        revision: Revision,
        options: &LinterOptions,
    ) -> Vec<LintReport> {
        self.lint_workspace_ast_profiled(repository, revision, options)
            .diagnostics
    }

    /// Lint the active workspace at AST level and collect performance metrics.
    pub fn lint_workspace_ast_profiled(
        &self,
        repository: Arc<Repository>,
        revision: Revision,
        options: &LinterOptions,
    ) -> LintRunReport {
        if !options.enabled {
            return LintRunReport::default();
        }

        let mut performance = LintPerformanceReport::default();
        let Some(workspace) = Self::repository_workspace(repository.as_ref(), revision) else {
            return LintRunReport::default();
        };
        let mut ctx =
            LintWorkspaceAstContext::new(repository, workspace, revision, options.clone());

        for rule in &self.rules {
            let meta = rule.meta();
            if meta.scope != LintScope::Workspace
                || meta.level != LintLevel::Ast
                || !ctx.is_rule_supported(meta)
                || !ctx.is_rule_enabled(meta)
            {
                continue;
            }

            let diagnostics_before = ctx.diagnostics().len();
            let start = Instant::now();
            rule.check_workspace_ast(&mut ctx);
            let duration = start.elapsed();
            let diagnostics_after = ctx.diagnostics().len();
            let diagnostics_added = diagnostics_after.saturating_sub(diagnostics_before);

            performance.record(
                meta.id,
                meta.code,
                meta.name,
                meta.category,
                meta.level,
                duration,
                diagnostics_added,
            );
        }

        LintRunReport {
            diagnostics: ctx.take_diagnostics(),
            performance,
        }
    }

    /// Lint one package at AST level using package-scope rules.
    pub fn lint_package_ast(
        &self,
        repository: Arc<Repository>,
        revision: Revision,
        package_id: PackageId,
        options: &LinterOptions,
    ) -> Vec<LintReport> {
        self.lint_package_ast_profiled(repository, revision, package_id, options)
            .diagnostics
    }

    /// Lint one package at AST level and collect performance metrics.
    pub fn lint_package_ast_profiled(
        &self,
        repository: Arc<Repository>,
        revision: Revision,
        package_id: PackageId,
        options: &LinterOptions,
    ) -> LintRunReport {
        if !options.enabled {
            return LintRunReport::default();
        }

        let mut performance = LintPerformanceReport::default();
        let Some(package) = Self::repository_package(repository.as_ref(), revision, package_id)
        else {
            return LintRunReport::default();
        };
        let mut ctx = LintPackageAstContext::new(repository, package, revision, options.clone());

        for rule in &self.rules {
            let meta = rule.meta();
            if meta.scope != LintScope::Package
                || meta.level != LintLevel::Ast
                || !ctx.is_rule_supported(meta)
                || !ctx.is_rule_enabled(meta)
            {
                continue;
            }

            let diagnostics_before = ctx.diagnostics().len();
            let start = Instant::now();
            rule.check_package_ast(&mut ctx);
            let duration = start.elapsed();
            let diagnostics_after = ctx.diagnostics().len();
            let diagnostics_added = diagnostics_after.saturating_sub(diagnostics_before);

            performance.record(
                meta.id,
                meta.code,
                meta.name,
                meta.category,
                meta.level,
                duration,
                diagnostics_added,
            );
        }

        LintRunReport {
            diagnostics: ctx.take_diagnostics(),
            performance,
        }
    }

    /// Lint the active workspace at DIR level using workspace-scope rules.
    pub fn lint_workspace_dir(
        &self,
        repository: Arc<Repository>,
        revision: Revision,
        profile: ProfileId,
        options: &LinterOptions,
    ) -> Vec<LintReport> {
        self.lint_workspace_dir_profiled(repository, revision, profile, options)
            .diagnostics
    }

    /// Lint the active workspace at DIR level and collect performance metrics.
    pub fn lint_workspace_dir_profiled(
        &self,
        repository: Arc<Repository>,
        revision: Revision,
        profile: ProfileId,
        options: &LinterOptions,
    ) -> LintRunReport {
        if !options.enabled {
            return LintRunReport::default();
        }

        let mut performance = LintPerformanceReport::default();
        let Some(workspace) = Self::repository_workspace(repository.as_ref(), revision) else {
            return LintRunReport::default();
        };
        let mut ctx =
            LintWorkspaceDirContext::new(repository, workspace, revision, profile, options.clone());

        for rule in &self.rules {
            let meta = rule.meta();
            if meta.scope != LintScope::Workspace
                || meta.level != LintLevel::Dir
                || !ctx.is_rule_supported(meta)
                || !ctx.is_rule_enabled(meta)
            {
                continue;
            }

            let diagnostics_before = ctx.diagnostics().len();
            let start = Instant::now();
            rule.check_workspace_dir(&mut ctx);
            let duration = start.elapsed();
            let diagnostics_after = ctx.diagnostics().len();
            let diagnostics_added = diagnostics_after.saturating_sub(diagnostics_before);

            performance.record(
                meta.id,
                meta.code,
                meta.name,
                meta.category,
                meta.level,
                duration,
                diagnostics_added,
            );
        }

        LintRunReport {
            diagnostics: ctx.take_diagnostics(),
            performance,
        }
    }

    /// Lint one package at DIR level using package-scope rules.
    pub fn lint_package_dir(
        &self,
        repository: Arc<Repository>,
        revision: Revision,
        package_id: PackageId,
        profile: ProfileId,
        options: &LinterOptions,
    ) -> Vec<LintReport> {
        self.lint_package_dir_profiled(repository, revision, package_id, profile, options)
            .diagnostics
    }

    /// Lint one package at DIR level and collect performance metrics.
    pub fn lint_package_dir_profiled(
        &self,
        repository: Arc<Repository>,
        revision: Revision,
        package_id: PackageId,
        profile: ProfileId,
        options: &LinterOptions,
    ) -> LintRunReport {
        if !options.enabled {
            return LintRunReport::default();
        }

        let mut performance = LintPerformanceReport::default();
        let Some(package) = Self::repository_package(repository.as_ref(), revision, package_id)
        else {
            return LintRunReport::default();
        };
        let mut ctx =
            LintPackageDirContext::new(repository, package, revision, profile, options.clone());

        for rule in &self.rules {
            let meta = rule.meta();
            if meta.scope != LintScope::Package
                || meta.level != LintLevel::Dir
                || !ctx.is_rule_supported(meta)
                || !ctx.is_rule_enabled(meta)
            {
                continue;
            }

            let diagnostics_before = ctx.diagnostics().len();
            let start = Instant::now();
            rule.check_package_dir(&mut ctx);
            let duration = start.elapsed();
            let diagnostics_after = ctx.diagnostics().len();
            let diagnostics_added = diagnostics_after.saturating_sub(diagnostics_before);

            performance.record(
                meta.id,
                meta.code,
                meta.name,
                meta.category,
                meta.level,
                duration,
                diagnostics_added,
            );
        }

        LintRunReport {
            diagnostics: ctx.take_diagnostics(),
            performance,
        }
    }
}

impl Default for LintRunner {
    fn default() -> Self {
        Self::recommended()
    }
}
