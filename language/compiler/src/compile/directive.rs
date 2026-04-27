use destack_source::DiagnosticSeverity;
use destack_workspace::{DiagnosticPolicy, LintSeverity};

use crate::{CompileError, CompileWarning, Compiler, DiagnosticAnchor, ImportError, ResolveError};

/// Severity override derived from a diagnostic directive decorator.
#[derive(Debug, Clone, Copy)]
struct DiagnosticDirectiveOverride {
    /// The override severity.
    severity: LintSeverity,
    /// Whether this override forbids inner changes.
    is_forbidden: bool,
}

impl Compiler {
    /// Resolve the effective error severity after diagnostic overrides.
    pub(crate) fn error_effective_severity(
        &self,
        error: &CompileError,
    ) -> Option<DiagnosticSeverity> {
        if let CompileError::Import(error) = error
            && let Some(severity) = self.import_policy_severity(error)
        {
            return Some(severity);
        }

        if let CompileError::Resolve(error) = error
            && let Some(severity) = self.resolve_policy_severity(error)
        {
            return Some(severity);
        }

        let CompileError::Optimize(error) = error else {
            return Some(DiagnosticSeverity::Error);
        };

        if !error.is_directive() {
            return Some(DiagnosticSeverity::Error);
        }

        self.diagnostic_effective_severity(error.anchor(), error.code(), LintSeverity::Error)
    }

    /// Resolve the effective warning severity after diagnostic overrides.
    pub(crate) fn warning_effective_severity(
        &self,
        warning: &CompileWarning,
    ) -> Option<DiagnosticSeverity> {
        self.diagnostic_effective_severity(
            warning.anchor(),
            &warning.full_code(),
            LintSeverity::Warning,
        )
    }

    /// Resolve the effective diagnostic severity after directive overrides.
    fn diagnostic_effective_severity(
        &self,
        anchor: DiagnosticAnchor,
        diagnostic_code: &str,
        base: LintSeverity,
    ) -> Option<DiagnosticSeverity> {
        // collect decorator overrides for the anchor
        let overrides = self.diagnostic_overrides_for_anchor(anchor, diagnostic_code);

        // apply overrides from outermost to innermost
        let mut effective = base;
        let mut is_forbidden = false;
        for override_info in overrides.into_iter().rev() {
            // honor forbid boundaries
            if is_forbidden {
                continue;
            }
            effective = override_info.severity;
            is_forbidden = override_info.is_forbidden;
        }

        // map to diagnostic severity
        match effective {
            LintSeverity::Off => None,
            LintSeverity::Note => Some(DiagnosticSeverity::Note),
            LintSeverity::Warning => Some(DiagnosticSeverity::Warning),
            LintSeverity::Error => Some(DiagnosticSeverity::Error),
        }
    }

    /// Resolve import error severity from policy settings.
    fn import_policy_severity(&self, error: &ImportError) -> Option<DiagnosticSeverity> {
        let context = self.current_context_maybe()?;
        let module_id = error.anchor().module_id()?;
        let module = context.module(module_id);
        let module = module.as_ref();
        let policy = match error {
            ImportError::ConflictingBinding { is_local, .. } => {
                if *is_local {
                    context
                        .compiler_options_for_module(module)
                        .map(|options| options.no_redeclared_locals)
                } else {
                    None
                }
            }
            _ => None,
        };

        match policy.unwrap_or(DiagnosticPolicy::Allow) {
            DiagnosticPolicy::Allow => None,
            DiagnosticPolicy::Warn => Some(DiagnosticSeverity::Warning),
            DiagnosticPolicy::Deny => Some(DiagnosticSeverity::Error),
        }
    }

    /// Resolve resolve-phase error severity from policy settings.
    fn resolve_policy_severity(&self, error: &ResolveError) -> Option<DiagnosticSeverity> {
        let context = self.current_context_maybe()?;
        let module_id = error.anchor().module_id()?;
        let module = context.module(module_id);
        let module = module.as_ref();
        let policy = match error {
            ResolveError::UnsupportedInternalModule { .. } => context
                .compiler_options_for_module(module)
                .map(|options| options.no_internal_import),
            _ => None,
        };

        match policy.unwrap_or(DiagnosticPolicy::Allow) {
            DiagnosticPolicy::Allow => None,
            DiagnosticPolicy::Warn => Some(DiagnosticSeverity::Warning),
            DiagnosticPolicy::Deny => Some(DiagnosticSeverity::Error),
        }
    }

    /// Collect diagnostic directive overrides for an anchor.
    fn diagnostic_overrides_for_anchor(
        &self,
        anchor: DiagnosticAnchor,
        diagnostic_code: &str,
    ) -> Vec<DiagnosticDirectiveOverride> {
        let _ = (anchor, diagnostic_code);

        Vec::new()
    }
}
