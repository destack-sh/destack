use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::RwLock;

use destack_source::ModuleId;
use destack_workspace::{LintCategory, LintPreset, LinterOptions, Module, ProfileId, Program};

use crate::{
    BoxedLintRule, LintDiagnostic, LintLevel, LintModuleAstContext, LintModuleDirContext,
    all_rules, recommended_rules,
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
    pub diagnostics: Vec<LintDiagnostic>,
    /// Rule execution performance metrics.
    pub performance: LintPerformanceReport,
}

/// Lint result for a full multi module run.
#[derive(Debug, Clone, Default)]
pub struct LintRunReport {
    /// The emitted diagnostics.
    pub diagnostics: Vec<LintDiagnostic>,
    /// Rule execution performance metrics.
    pub performance: LintPerformanceReport,
}

/// Runs lint rules against modules and programs.
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
            LintPreset::All => all_rules(),
        };
        Self {
            rules,
            compute_fixes: true,
        }
    }

    /// Create a runner from linter options.
    pub fn from_options(options: &LinterOptions) -> Self {
        Self::from_preset(options.preset)
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

    /// Lint a module at a specific IR level.
    pub fn lint_module(
        &self,
        program: Arc<Program>,
        module: Arc<RwLock<Module>>,
        profile: ProfileId,
        options: &LinterOptions,
        level: LintLevel,
    ) -> Vec<LintDiagnostic> {
        self.lint_module_profiled(program, module, profile, options, level)
            .diagnostics
    }

    /// Lint a module at a specific IR level and collect performance metrics.
    pub fn lint_module_profiled(
        &self,
        program: Arc<Program>,
        module: Arc<RwLock<Module>>,
        profile: ProfileId,
        options: &LinterOptions,
        level: LintLevel,
    ) -> LintModuleReport {
        if !options.enabled {
            return LintModuleReport::default();
        }
        let mut performance = LintPerformanceReport::default();

        let diagnostics = match level {
            LintLevel::Ast => {
                self.lint_module_ast(program, module, profile, options, Some(&mut performance))
            }
            LintLevel::Dir => {
                self.lint_module_dir(program, module, profile, options, Some(&mut performance))
            }
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
        program: Arc<Program>,
        module: Arc<RwLock<Module>>,
        _profile: ProfileId,
        options: &LinterOptions,
        mut performance: Option<&mut LintPerformanceReport>,
    ) -> Vec<LintDiagnostic> {
        let module = module.read();
        let ast = &module.ast();
        let file = program.files.get(module.file_id);
        let mut ctx = LintModuleAstContext::new(
            program,
            &module,
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
        program: Arc<Program>,
        module: Arc<RwLock<Module>>,
        profile: ProfileId,
        options: &LinterOptions,
        mut performance: Option<&mut LintPerformanceReport>,
    ) -> Vec<LintDiagnostic> {
        // context
        let module = module.read();
        let ast = &module.ast();
        let file = program.files.get(module.file_id);
        let dir = module.dir(profile);
        let tree = dir.tree.read();
        let symbols = dir.symbols.read();
        let types = dir.types.read();
        let namespace_exports = dir.namespace_exports.read();
        let imported_modules = dir.imported_modules.read();
        let exported_symbols = dir.exported_symbols.read();

        let mut ctx = LintModuleDirContext::new(
            program,
            &module,
            profile,
            file,
            &ast.tree,
            &tree,
            &symbols,
            &types,
            dir.roots.clone(),
            dir.namespace_symbol,
            dir.namespace_scope,
            dir.default_symbol,
            namespace_exports
                .iter()
                .map(|export| export.module_id)
                .collect(),
            imported_modules.clone(),
            exported_symbols.clone(),
            options,
            self.compute_fixes,
        );

        // check dir rules
        for rule in &self.rules {
            let meta = rule.meta();
            if meta.level == LintLevel::Dir
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
        program: Arc<Program>,
        module_id: ModuleId,
        profile: ProfileId,
        options: &LinterOptions,
        level: LintLevel,
    ) -> Vec<LintDiagnostic> {
        let module = program.modules.get(module_id);
        self.lint_module(program, module, profile, options, level)
    }

    /// Lint all modules at a specific level.
    pub fn lint_all_modules(
        &self,
        program: Arc<Program>,
        options: &LinterOptions,
        level: LintLevel,
    ) -> Vec<LintDiagnostic> {
        self.lint_all_modules_profiled(program, options, level)
            .diagnostics
    }

    /// Lint all modules at a specific level and collect performance metrics.
    pub fn lint_all_modules_profiled(
        &self,
        program: Arc<Program>,
        options: &LinterOptions,
        level: LintLevel,
    ) -> LintRunReport {
        if !options.enabled {
            return LintRunReport::default();
        }

        let mut diagnostics = Vec::new();
        let mut performance = LintPerformanceReport::default();
        for module in program.modules.iter() {
            let profile = program.default_profile_id_for_module(module.read().id);
            let report =
                self.lint_module_profiled(program.clone(), module, profile, options, level);
            diagnostics.extend(report.diagnostics);
            performance.merge(&report.performance);
        }

        LintRunReport {
            diagnostics,
            performance,
        }
    }
}

impl Default for LintRunner {
    fn default() -> Self {
        Self::recommended()
    }
}
