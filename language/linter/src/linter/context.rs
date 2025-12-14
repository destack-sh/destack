use std::sync::Arc;

use parking_lot::RwLock;

use destack_source::{FileId, ModuleId, Span};
use destack_workspace::{LinterRules, Module, Program, LintSeverity};

use crate::{LintCategory, LintDiagnostic};

/// Context provided to lint rules during checking.
pub struct LintContext<'a> {
    /// The program being linted.
    pub program: &'a Program,
    /// The module being checked.
    module: Option<Arc<RwLock<Module>>>,
    /// Linter configuration.
    rules: &'a LinterRules,
    /// Collected diagnostics.
    diagnostics: Vec<LintDiagnostic>,
}

impl std::fmt::Debug for LintContext<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LintContext").finish()
    }
}

impl<'a> LintContext<'a> {
    /// Create a new lint context for checking a module.
    pub fn for_module(
        program: &'a Program,
        module: Arc<RwLock<Module>>,
        rules: &'a LinterRules,
    ) -> Self {
        Self {
            program,
            module: Some(module),
            rules,
            diagnostics: Vec::new(),
        }
    }

    /// Create a new lint context for program-level checking.
    pub fn for_program(program: &'a Program, rules: &'a LinterRules) -> Self {
        Self {
            program,
            module: None,
            rules,
            diagnostics: Vec::new(),
        }
    }

    /// Get the module being checked, if any.
    pub fn module(&self) -> Option<&Arc<RwLock<Module>>> {
        self.module.as_ref()
    }

    /// Get the module id being checked.
    pub fn module_id(&self) -> Option<ModuleId> {
        self.module.as_ref().map(|m| m.read().id)
    }

    /// Get the file id of the current module.
    pub fn file_id(&self) -> Option<FileId> {
        self.module.as_ref().map(|m| m.read().file_id)
    }

    /// Get the configured severity for a rule, or None if not overridden.
    pub fn configured_severity(&self, rule_id: &str) -> Option<LintSeverity> {
        self.rules.get_severity(rule_id)
    }

    /// Check if a rule is enabled (not off).
    pub fn is_rule_enabled(&self, rule_id: &str, default: LintSeverity) -> bool {
        let severity = self.configured_severity(rule_id).unwrap_or(default);
        severity != LintSeverity::Off
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
