use std::sync::Arc;

use destack_workspace::{LintSeverity, LinterOptions, Program};

use crate::{LintDiagnostic, LintMeta};

/// Context for program-level (cross-module) linting.
pub struct LintProgramContext {
    /// The program being linted.
    pub program: Arc<Program>,
    /// Linter configuration.
    options: LinterOptions,
    /// Collected diagnostics.
    diagnostics: Vec<LintDiagnostic>,
}

impl std::fmt::Debug for LintProgramContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LintProgramContext").finish()
    }
}

impl LintProgramContext {
    /// Create a new program lint context.
    pub fn new(program: Arc<Program>, options: LinterOptions) -> Self {
        Self {
            program,
            options,
            diagnostics: Vec::new(),
        }
    }

    /// Get the linter options.
    pub fn options(&self) -> &LinterOptions {
        &self.options
    }

    /// Resolve severity for a rule.
    pub fn get_severity(&self, meta: &LintMeta) -> LintSeverity {
        self.options
            .resolve_severity(meta.id, meta.category, meta.category.default_severity())
    }

    /// Check if a rule is enabled.
    pub fn is_rule_enabled(&self, meta: &LintMeta) -> bool {
        self.get_severity(meta).is_enabled()
    }

    /// Report a lint diagnostic.
    pub fn report(&mut self, diagnostic: LintDiagnostic) {
        if diagnostic.is_enabled() {
            self.diagnostics.push(diagnostic);
        }
    }

    /// Take the collected diagnostics.
    pub fn take_diagnostics(&mut self) -> Vec<LintDiagnostic> {
        std::mem::take(&mut self.diagnostics)
    }

    /// Get reference to collected diagnostics.
    pub fn diagnostics(&self) -> &[LintDiagnostic] {
        &self.diagnostics
    }
}
