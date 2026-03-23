use crate::{
    AnalyzeError, AnalyzeWarning, Compiler, DiagnosticAnchor, ResolveError, ResolveWarning,
    TaskError, TaskWarning,
};
use destack_artifact::ArtifactStore;
use destack_source::{Diagnostic, DiagnosticSeverity, LabeledSpan, ModuleId, Span};

use destack_workspace::Program;

/// Diagnostic encountered during compilation.
#[derive(Debug, Clone)]
pub enum CompileDiagnostic {
    /// Error.
    Error(TaskError),
    /// Warning.
    Warning(TaskWarning),
}

impl From<TaskError> for CompileDiagnostic {
    fn from(error: TaskError) -> Self {
        Self::Error(error)
    }
}

impl From<TaskWarning> for CompileDiagnostic {
    fn from(warning: TaskWarning) -> Self {
        Self::Warning(warning)
    }
}

impl CompileDiagnostic {
    /// Get the severity of the diagnostic.
    pub fn severity(&self) -> DiagnosticSeverity {
        match self {
            Self::Error(_) => DiagnosticSeverity::Error,
            Self::Warning(_) => DiagnosticSeverity::Warning,
        }
    }

    /// Get the message of the diagnostic.
    pub fn message(&self, program: &Program, artifacts: &ArtifactStore) -> String {
        match self {
            Self::Error(error) => error.message(program, artifacts),
            Self::Warning(warning) => warning.message(program, artifacts),
        }
    }

    /// Get the anchor of the diagnostic.
    pub fn anchor(&self) -> DiagnosticAnchor {
        match self {
            Self::Error(error) => error.anchor(),
            Self::Warning(warning) => warning.anchor(),
        }
    }

    /// Get the full code of the diagnostic.
    pub fn full_code(&self) -> String {
        match self {
            Self::Error(error) => error.full_code(),
            Self::Warning(warning) => warning.full_code(),
        }
    }

    /// Turn the diagnostic into a full Destack diagnostic.
    pub fn to_diagnostic(&self, program: &Program, artifacts: &ArtifactStore) -> Diagnostic {
        let anchor = self.anchor();
        let severity = self.severity();
        let message = self.message(program, artifacts);
        let code = self.full_code();

        // get file and span from anchor, falling back to program's fallback file
        let (file_id, span) = anchor.to_file_span(program, artifacts).unwrap_or_else(|| {
            let fallback = program.fallback_file_id;
            (fallback, Span::empty(fallback))
        });

        let primary_span = LabeledSpan {
            span,
            label: message.clone(),
        };

        Diagnostic {
            code,
            original_code: None,
            severity,
            original_severity: None,
            message,
            file_id,
            primary_span,
            primary_highlight_spans: None,
            secondary_spans: None,
            suggestions: None,
        }
    }
}

/// Module-level diagnostic emission inputs for one anchored diagnostic.
#[derive(Debug, Clone, Copy)]
struct ModuleDiagnosticPolicy {
    /// Whether the anchored module is a declaration module.
    is_declaration: bool,
    /// Whether the anchored module is a TypeScript module.
    is_typescript: bool,
    /// Whether the anchored module is a JavaScript module.
    is_javascript: bool,
    /// Whether declaration diagnostics should be skipped.
    skip_lib_check: bool,
    /// Whether TypeScript compatibility diagnostics are enabled.
    check_ts: bool,
    /// Whether JavaScript compatibility diagnostics are enabled.
    check_js: bool,
}

impl Compiler {
    /// Check whether an error should be emitted.
    pub(super) fn should_emit_error(&self, error: &TaskError) -> bool {
        match error {
            TaskError::Resolve(error) => self.should_emit_resolve_error(error),
            TaskError::Analyze(error) => self.should_emit_analyze_error(error),
            _ => true,
        }
    }

    /// Check whether a warning should be emitted.
    pub(super) fn should_emit_warning(&self, warning: &TaskWarning) -> bool {
        match warning {
            TaskWarning::Resolve(warning) => self.should_emit_resolve_warning(warning),
            TaskWarning::Analyze(warning) => self.should_emit_analyze_warning(warning),
            _ => true,
        }
    }

    /// Resolve profile-level skip-lib-check state for one anchored diagnostic.
    fn profile_skip_lib_check_for_anchor(&self, anchor: &DiagnosticAnchor) -> bool {
        // skip when the diagnostic is not anchored to a profile scoped dir node
        let DiagnosticAnchor::DirNode(anchored) = anchor else {
            return false;
        };

        // skip when the dir node has no profile provenance
        let Some(profile_id) = anchored.profile_id else {
            return false;
        };

        // resolve skip-lib-check from the active profile key
        let profile = self.program.profile(profile_id);
        profile.key.skip_lib_check
    }

    /// Resolve skip-lib-check state for one anchored module.
    fn skip_lib_check_for_anchor(&self, anchor: &DiagnosticAnchor, module_id: ModuleId) -> bool {
        // allow profile-level suppression first
        if self.profile_skip_lib_check_for_anchor(anchor) {
            return true;
        }

        // fall back to module-local compatibility options
        let module_options = self.module_check_options_for_module(module_id);
        module_options.skip_lib_check
    }

    /// Build module-level diagnostic emission policy for one anchor.
    fn module_diagnostic_policy_for_anchor(
        &self,
        anchor: &DiagnosticAnchor,
    ) -> Option<ModuleDiagnosticPolicy> {
        // allow non-module anchors to bypass module-gated suppression
        let module_id = anchor.module_id()?;

        // load module and compatibility options
        let module = self.program.modules.get(module_id);
        let module = module.as_ref();
        let options = self.module_check_options_for_module(module_id);
        let skip_lib_check = self.skip_lib_check_for_anchor(anchor, module_id);

        Some(ModuleDiagnosticPolicy {
            is_declaration: module.language_type.is_declaration(),
            is_typescript: module.language_type.is_typescript(),
            is_javascript: module.language_type.is_javascript(),
            skip_lib_check,
            check_ts: options.check_ts,
            check_js: options.check_js,
        })
    }

    /// Check whether a resolve diagnostic should be emitted.
    fn should_emit_resolve_for_anchor(&self, anchor: &DiagnosticAnchor) -> bool {
        // allow diagnostics without a module anchor
        let Some(policy) = self.module_diagnostic_policy_for_anchor(anchor) else {
            return true;
        };

        // skip lib checks for declaration modules
        if policy.is_declaration && policy.skip_lib_check {
            return false;
        }

        true
    }

    /// Check whether an analyze diagnostic should be emitted.
    fn should_emit_analyze_for_anchor(&self, anchor: &DiagnosticAnchor) -> bool {
        // allow diagnostics without a module anchor
        let Some(policy) = self.module_diagnostic_policy_for_anchor(anchor) else {
            return true;
        };

        // skip lib checks for declaration modules
        if policy.is_declaration && policy.skip_lib_check {
            return false;
        }

        // skip checks for unchecked compatibility modules
        if policy.is_typescript && !policy.is_declaration && !policy.check_ts {
            return false;
        }
        if policy.is_javascript && !policy.check_js {
            return false;
        }

        true
    }

    /// Check whether a resolve error should be emitted.
    fn should_emit_resolve_error(&self, error: &ResolveError) -> bool {
        let anchor = error.anchor();

        self.should_emit_resolve_for_anchor(&anchor)
    }

    /// Check whether a resolve warning should be emitted.
    fn should_emit_resolve_warning(&self, warning: &ResolveWarning) -> bool {
        let anchor = warning.anchor();

        self.should_emit_resolve_for_anchor(&anchor)
    }

    /// Check whether an analyze error should be emitted.
    fn should_emit_analyze_error(&self, error: &AnalyzeError) -> bool {
        // always emit language gating errors
        if matches!(
            error,
            AnalyzeError::TypeScriptDisabled { .. } | AnalyzeError::JavaScriptDisabled { .. }
        ) {
            return true;
        }

        let anchor = error.anchor();

        self.should_emit_analyze_for_anchor(&anchor)
    }

    /// Check whether an analyze warning should be emitted.
    fn should_emit_analyze_warning(&self, warning: &AnalyzeWarning) -> bool {
        let anchor = warning.anchor();

        self.should_emit_analyze_for_anchor(&anchor)
    }
}
