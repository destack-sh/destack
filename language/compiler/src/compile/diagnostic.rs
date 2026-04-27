use crate::{
    CompileError, CompileWarning, Compiler, DiagnosticAnchor, ResolveError, ResolveWarning,
};
use destack_artifact::ArtifactStore;
use destack_source::{Diagnostic, DiagnosticSeverity, LabeledSpan, ModuleId, Span};
use destack_workspace::{Repository, Revision};

/// Diagnostic encountered during compilation.
#[derive(Debug, Clone)]
pub enum CompileDiagnostic {
    /// Error.
    Error(CompileError),
    /// Warning.
    Warning(CompileWarning),
}

impl From<CompileError> for CompileDiagnostic {
    fn from(error: CompileError) -> Self {
        Self::Error(error)
    }
}

impl From<CompileWarning> for CompileDiagnostic {
    fn from(warning: CompileWarning) -> Self {
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
    pub fn message(
        &self,
        revision: Revision,
        repository: &Repository,
        artifacts: &ArtifactStore,
    ) -> String {
        match self {
            Self::Error(error) => error.message(revision, repository, artifacts),
            Self::Warning(warning) => warning.message(revision, repository, artifacts),
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
    pub fn to_diagnostic(
        &self,
        revision: Revision,
        repository: &Repository,
        artifacts: &ArtifactStore,
    ) -> Diagnostic {
        let anchor = self.anchor();
        let severity = self.severity();
        let message = self.message(revision, repository, artifacts);
        let code = self.full_code();

        // get file and span from anchor, falling back to the repository root file
        let (file_id, span) = anchor
            .to_file_span(revision, repository, artifacts)
            .unwrap_or_else(|| {
                let fallback = repository.root_file_id();
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
    /// Whether declaration diagnostics should be skipped.
    skip_lib_check: bool,
}

impl Compiler {
    /// Check whether an error should be emitted.
    pub(super) fn should_emit_error(&self, revision: Revision, error: &CompileError) -> bool {
        match error {
            CompileError::Resolve(error) => self.should_emit_resolve_error(revision, error),
            _ => true,
        }
    }

    /// Check whether a warning should be emitted.
    pub(super) fn should_emit_warning(&self, revision: Revision, warning: &CompileWarning) -> bool {
        match warning {
            CompileWarning::Resolve(warning) => self.should_emit_resolve_warning(revision, warning),
            _ => true,
        }
    }

    /// Resolve profile-level skip-lib-check state for one anchored diagnostic.
    fn profile_skip_lib_check_for_anchor(
        &self,
        revision: Revision,
        anchor: &DiagnosticAnchor,
    ) -> bool {
        // skip when the diagnostic is not anchored to a profile scoped dir node
        let DiagnosticAnchor::DirNode(anchored) = anchor else {
            return false;
        };

        // skip when the dir node has no profile provenance
        let Some(profile_id) = anchored.profile_id else {
            return false;
        };

        // resolve skip-lib-check from the diagnostic revision
        let profile = self.profile_for_revision(revision, profile_id);
        profile.key.skip_lib_check
    }

    /// Resolve skip-lib-check state for one anchored module.
    fn skip_lib_check_for_anchor(
        &self,
        revision: Revision,
        anchor: &DiagnosticAnchor,
        module_id: ModuleId,
    ) -> bool {
        // allow profile-level suppression first
        if self.profile_skip_lib_check_for_anchor(revision, anchor) {
            return true;
        }

        // fall back to module-local compatibility options
        let context = self.context(revision).ok();
        let Some(context) = context else {
            return false;
        };
        let module_options = context.module_check_options_for_module(module_id);
        module_options.skip_lib_check
    }

    /// Build module-level diagnostic emission policy for one anchor.
    fn module_diagnostic_policy_for_anchor(
        &self,
        revision: Revision,
        anchor: &DiagnosticAnchor,
    ) -> Option<ModuleDiagnosticPolicy> {
        // allow non-module anchors to bypass module-gated suppression
        let module_id = anchor.module_id()?;

        // load module and compatibility options
        let context = self.context(revision).ok()?;
        let module = context.module(module_id);
        let module = module.as_ref();
        let skip_lib_check = self.skip_lib_check_for_anchor(revision, anchor, module_id);

        Some(ModuleDiagnosticPolicy {
            is_declaration: module.language_type.is_declaration(),
            skip_lib_check,
        })
    }

    /// Check whether a resolve diagnostic should be emitted.
    fn should_emit_resolve_for_anchor(
        &self,
        revision: Revision,
        anchor: &DiagnosticAnchor,
    ) -> bool {
        // allow diagnostics without a module anchor
        let Some(policy) = self.module_diagnostic_policy_for_anchor(revision, anchor) else {
            return true;
        };

        // skip lib checks for declaration modules
        if policy.is_declaration && policy.skip_lib_check {
            return false;
        }

        true
    }

    /// Check whether a resolve error should be emitted.
    fn should_emit_resolve_error(&self, revision: Revision, error: &ResolveError) -> bool {
        let anchor = error.anchor();

        self.should_emit_resolve_for_anchor(revision, &anchor)
    }

    /// Check whether a resolve warning should be emitted.
    fn should_emit_resolve_warning(&self, revision: Revision, warning: &ResolveWarning) -> bool {
        let anchor = warning.anchor();

        self.should_emit_resolve_for_anchor(revision, &anchor)
    }
}
