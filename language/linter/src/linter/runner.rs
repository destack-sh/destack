use std::sync::Arc;

use parking_lot::RwLock;

use destack_source::{LinterOptions, ModuleId};
use destack_workspace::{Module, Program};

use super::{BoxedLintRule, LintContext, LintDiagnostic, LintLevel};

/// Runs lint rules against modules and programs.
pub struct LintRunner {
    rules: Vec<BoxedLintRule>,
}

impl std::fmt::Debug for LintRunner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LintRunner")
            .field("rule_count", &self.rules.len())
            .finish()
    }
}

impl LintRunner {
    /// Create a new lint runner with the given rules.
    pub fn new(rules: Vec<BoxedLintRule>) -> Self {
        Self { rules }
    }

    /// Create an empty lint runner.
    pub fn empty() -> Self {
        Self { rules: Vec::new() }
    }

    /// Create a lint runner with all recommended rules.
    pub fn with_recommended_rules() -> Self {
        Self::new(crate::rules::recommended_rules())
    }

    /// Create a lint runner with all built-in rules.
    pub fn with_all_rules() -> Self {
        Self::new(crate::rules::all_rules())
    }

    /// Add a rule to the runner.
    pub fn add_rule(&mut self, rule: BoxedLintRule) {
        self.rules.push(rule);
    }

    /// Get the number of rules.
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// Lint a single module at a specific IR level.
    pub fn lint_module(
        &self,
        program: &Program,
        module: Arc<RwLock<Module>>,
        options: &LinterOptions,
        level: LintLevel,
    ) -> Vec<LintDiagnostic> {
        if !options.enabled {
            return Vec::new();
        }

        let mut ctx = LintContext::for_module(program, module, &options.rules);

        for rule in &self.rules {
            // skip rules that don't run at this level
            if !rule.runs_at_level(level) {
                continue;
            }

            // skip disabled rules
            let severity = rule.effective_severity(
                options.rules.get_severity(rule.meta().id),
            );
            if !severity.is_enabled() {
                continue;
            }

            rule.check_module(&mut ctx);
        }

        ctx.take_diagnostics()
    }

    /// Lint the entire program (cross-module analysis).
    pub fn lint_program(&self, program: &Program, options: &LinterOptions) -> Vec<LintDiagnostic> {
        if !options.enabled {
            return Vec::new();
        }

        let mut ctx = LintContext::for_program(program, &options.rules);

        for rule in &self.rules {
            // skip disabled rules
            let severity = rule.effective_severity(
                options.rules.get_severity(rule.meta().id),
            );
            if !severity.is_enabled() {
                continue;
            }

            rule.check_program(&mut ctx);
        }

        ctx.take_diagnostics()
    }

    /// Lint all modules in the program at a specific level.
    pub fn lint_all_modules(
        &self,
        program: &Program,
        options: &LinterOptions,
        level: LintLevel,
    ) -> Vec<LintDiagnostic> {
        if !options.enabled {
            return Vec::new();
        }

        let mut all_diagnostics = Vec::new();

        for module in program.modules.iter() {
            let diagnostics = self.lint_module(program, module, options, level);
            all_diagnostics.extend(diagnostics);
        }

        all_diagnostics
    }

    /// Get all rules for iteration.
    pub fn rules(&self) -> &[BoxedLintRule] {
        &self.rules
    }
}

impl Default for LintRunner {
    fn default() -> Self {
        Self::empty()
    }
}

/// Lint a single module by id.
pub fn lint_module_by_id(
    runner: &LintRunner,
    program: &Program,
    module_id: ModuleId,
    options: &LinterOptions,
    level: LintLevel,
) -> Vec<LintDiagnostic> {
    let module = program.modules.get(module_id);
    runner.lint_module(program, module, options, level)
}
