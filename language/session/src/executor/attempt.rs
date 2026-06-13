use std::sync::Arc;

use destack_artifact::{
    ArtifactKey, ArtifactSidecar, DiagnosticAnchor, DiagnosticContext, DiagnosticDisplay,
    DiagnosticError, DiagnosticLike,
};
use destack_repository::{ArtifactTracer, ProviderContext, Repository, Revision};
use destack_source::{
    DiagnosticCollection, DiagnosticLabel, FileContentId, FileId, ModuleId, PackageId, Span,
};
use parking_lot::Mutex;

/// One artifact provider attempt owned by a session worker.
#[derive(Debug)]
pub(crate) struct ProviderAttempt {
    /// The repository that owns the pinned revision.
    pub(super) repository: Arc<Repository>,
    /// The pinned revision for this attempt.
    pub(super) revision: Revision,
    /// The artifact key being built.
    key: ArtifactKey,
    /// The diagnostics produced by this attempt.
    diagnostics: Mutex<DiagnosticCollection>,
    /// The sidecars produced by this attempt.
    sidecars: Mutex<Vec<ArtifactSidecar>>,
    /// The tracer for this attempt, when the run is traced.
    tracer: Option<Arc<ArtifactTracer>>,
}

impl ProviderAttempt {
    /// Create one provider attempt.
    pub(crate) fn new(repository: Arc<Repository>, revision: Revision, key: ArtifactKey) -> Self {
        Self {
            repository,
            revision,
            key,
            diagnostics: Mutex::new(DiagnosticCollection::new()),
            sidecars: Mutex::new(Vec::new()),
            tracer: None,
        }
    }

    /// Attach one tracer to this attempt.
    pub(crate) fn with_tracer(mut self, tracer: Arc<ArtifactTracer>) -> Self {
        self.tracer = Some(tracer);

        self
    }

    /// Return the pinned repository revision for this attempt.
    pub(crate) fn revision(&self) -> Revision {
        self.revision
    }

    /// Return the artifact key being built.
    pub(crate) fn key(&self) -> ArtifactKey {
        self.key
    }

    /// Return diagnostics produced by this attempt.
    pub(super) fn diagnostics(&self) -> DiagnosticCollection {
        self.diagnostics.lock().clone()
    }

    /// Return sidecars produced by this attempt.
    pub(super) fn sidecars(&self) -> Vec<ArtifactSidecar> {
        self.sidecars.lock().clone()
    }

    /// Build one invalid diagnostic anchor error.
    fn invalid_anchor(message: impl Into<String>) -> DiagnosticError {
        DiagnosticError::InvalidAnchor {
            message: message.into(),
        }
    }

    /// Resolve one provider diagnostic anchor into a final source label.
    fn resolve_label(
        &self,
        anchor: &DiagnosticAnchor,
        message: Option<String>,
    ) -> Result<DiagnosticLabel, DiagnosticError> {
        let span = self.anchor_span(anchor)?;
        let content = self.file_content_id(span.file)?;

        Ok(DiagnosticLabel {
            content,
            span,
            message,
        })
    }

    /// Resolve one provider diagnostic anchor into a source span.
    fn anchor_span(&self, anchor: &DiagnosticAnchor) -> Result<Span, DiagnosticError> {
        let span = match anchor {
            DiagnosticAnchor::Span(span) => Ok(*span),
            DiagnosticAnchor::File(file) => Ok(Span::empty(*file)),
            DiagnosticAnchor::Module(module) => self.module_span(*module),
            DiagnosticAnchor::Package(package) => self.package_span(*package),
        }?;

        Ok(span)
    }

    /// Return the span covering one module file.
    fn module_span(&self, module_id: ModuleId) -> Result<Span, DiagnosticError> {
        let file = self.module_file_id(module_id)?;

        Ok(Span::empty(file))
    }

    /// Return the span covering one package manifest file.
    fn package_span(&self, package_id: PackageId) -> Result<Span, DiagnosticError> {
        let package = self
            .repository
            .package(self.revision, package_id)
            .map_err(|error| {
                Self::invalid_anchor(format!("failed to read diagnostic package: {error}"))
            })?;
        let Some(package) = package else {
            return Err(Self::invalid_anchor(format!(
                "diagnostic package is not tracked in revision: {package_id:?}"
            )));
        };
        let Some(file) = package.destack_file_id else {
            return Err(Self::invalid_anchor(format!(
                "diagnostic package has no destack config file: {package_id:?}"
            )));
        };

        Ok(Span::empty(file))
    }

    /// Return one file content id in this revision.
    fn file_content_id(&self, file: FileId) -> Result<FileContentId, DiagnosticError> {
        let content = self
            .repository
            .file_content_id(self.revision, file)
            .map_err(|error| {
                Self::invalid_anchor(format!(
                    "failed to read diagnostic file content id: {error}"
                ))
            })?;
        let Some(content) = content else {
            return Err(Self::invalid_anchor(format!(
                "diagnostic file is not tracked in revision: {file:?}"
            )));
        };

        Ok(content)
    }

    /// Return one module file id.
    fn module_file_id(&self, module_id: ModuleId) -> Result<FileId, DiagnosticError> {
        let module = self
            .repository
            .module(self.revision, module_id)
            .map_err(|error| {
                Self::invalid_anchor(format!("failed to read diagnostic module: {error}"))
            })?;
        let Some(module) = module else {
            return Err(Self::invalid_anchor(format!(
                "diagnostic module is not tracked in revision: {module_id:?}"
            )));
        };

        Ok(module.file_id)
    }
}

impl DiagnosticContext for ProviderAttempt {
    /// Resolve one provider diagnostic anchor into a final source label.
    fn label(
        &self,
        anchor: &DiagnosticAnchor,
        message: Option<String>,
    ) -> Result<DiagnosticLabel, DiagnosticError> {
        self.resolve_label(anchor, message)
    }

    /// Display one repository-backed value when the context can resolve it.
    fn display(&self, display: DiagnosticDisplay) -> Result<String, DiagnosticError> {
        match display {
            DiagnosticDisplay::Module(module) => {
                let display = self
                    .repository
                    .module_display(self.revision, module)
                    .map_err(|error| {
                        Self::invalid_anchor(format!("failed to format diagnostic module: {error}"))
                    })?;
                let Some(display) = display else {
                    return Err(Self::invalid_anchor(format!(
                        "diagnostic module is not tracked in revision: {module:?}"
                    )));
                };

                Ok(display)
            }
            DiagnosticDisplay::Package(package) => {
                let display = self
                    .repository
                    .package_display(self.revision, package)
                    .map_err(|error| {
                        Self::invalid_anchor(format!(
                            "failed to format diagnostic package: {error}"
                        ))
                    })?;
                let Some(display) = display else {
                    return Err(Self::invalid_anchor(format!(
                        "diagnostic package has no display name: {package:?}"
                    )));
                };

                Ok(display)
            }
            DiagnosticDisplay::Target(target) => {
                let display = self
                    .repository
                    .target_display(self.revision, target)
                    .map_err(|error| {
                        Self::invalid_anchor(format!("failed to format diagnostic target: {error}"))
                    })?;
                let Some(display) = display else {
                    return Err(Self::invalid_anchor(format!(
                        "diagnostic target is not tracked in revision: {target:?}"
                    )));
                };

                Ok(display)
            }
        }
    }
}

impl ProviderContext for ProviderAttempt {
    /// Return the pinned repository revision for this attempt.
    fn revision(&self) -> Revision {
        self.revision()
    }

    /// Return the artifact key being built.
    fn artifact_key(&self) -> ArtifactKey {
        self.key
    }

    /// Add an already-final diagnostic collection produced by this attempt.
    fn emit_diagnostics(&self, diagnostics: DiagnosticCollection) {
        if diagnostics.is_empty() {
            return;
        }

        self.diagnostics.lock().merge_from(&diagnostics);
    }

    /// Add one sidecar produced by this attempt.
    fn emit_sidecar(&self, sidecar: ArtifactSidecar) {
        self.sidecars.lock().push(sidecar);
    }

    /// Add one diagnostic produced by this attempt.
    fn emit(&self, diagnostic: &dyn DiagnosticLike) -> Result<(), DiagnosticError> {
        let diagnostic = diagnostic.to_diagnostic(self)?;
        let diagnostics = DiagnosticCollection::from_diagnostics(vec![diagnostic]);

        self.emit_diagnostics(diagnostics);

        Ok(())
    }

    /// Return the tracer recording this attempt, when the run is traced.
    fn tracer(&self) -> Option<&ArtifactTracer> {
        self.tracer.as_deref()
    }
}
