use std::sync::Arc;

use destack_artifact::{
    ArtifactDependency, ArtifactFailure, ArtifactKey, ArtifactOutcome, ArtifactPayload,
    ArtifactVersion, DiagnosticAnchor, DiagnosticContext, DiagnosticDisplay, DiagnosticError,
    DiagnosticLike,
};
use destack_source::{
    DiagnosticCollection, DiagnosticLabel, FileContentId, FileId, ModuleId, PackageId, Span,
};
use destack_workspace::{ProviderContext, ProviderError, Repository, Revision};
use parking_lot::Mutex;

use crate::SessionError;

/// One session-owned artifact provider attempt.
#[derive(Debug)]
pub(crate) struct SessionProviderContext {
    /// The repository that owns the pinned revision.
    pub(super) repository: Arc<Repository>,
    /// The pinned revision for this attempt.
    pub(super) revision: Revision,
    /// The artifact key being built.
    key: ArtifactKey,
    /// The exact dependencies read by this attempt.
    dependencies: Mutex<Vec<ArtifactDependency>>,
    /// The diagnostics produced by this attempt.
    diagnostics: Mutex<DiagnosticCollection>,
}

impl SessionProviderContext {
    /// Create one provider attempt context.
    pub(crate) fn new(repository: Arc<Repository>, revision: Revision, key: ArtifactKey) -> Self {
        Self {
            repository,
            revision,
            key,
            dependencies: Mutex::new(Vec::new()),
            diagnostics: Mutex::new(DiagnosticCollection::new()),
        }
    }

    /// Return the pinned repository revision for this attempt.
    pub(crate) fn revision(&self) -> Revision {
        self.revision
    }

    /// Return the artifact key being built.
    pub(crate) fn artifact_key(&self) -> ArtifactKey {
        self.key
    }

    /// Return the exact dependencies read by this attempt.
    pub(crate) fn dependencies(&self) -> Vec<ArtifactDependency> {
        self.dependencies.lock().clone()
    }

    /// Return diagnostics produced by this attempt.
    pub(crate) fn diagnostic_collection(&self) -> DiagnosticCollection {
        self.diagnostics.lock().clone()
    }

    /// Complete this attempt with its ready payload.
    pub(crate) fn complete_ready(
        &self,
        payload: ArtifactPayload,
    ) -> Result<ArtifactVersion, SessionError> {
        let dependencies = self.dependencies();
        let diagnostics = self.diagnostic_collection();
        let version = ArtifactVersion::new(self.key, dependencies.iter().cloned());

        self.repository.complete_artifact(
            self.revision,
            version,
            payload,
            dependencies,
            diagnostics,
        )?;

        Ok(version)
    }

    /// Complete this attempt with a provider failure.
    pub(crate) fn complete_failed(
        &self,
        failure: ArtifactFailure,
    ) -> Result<ArtifactVersion, SessionError> {
        let dependencies = self.dependencies();
        let diagnostics = self.diagnostic_collection();
        let version = ArtifactVersion::new(self.key, dependencies.iter().cloned());

        self.repository.fail_artifact(
            self.revision,
            version,
            dependencies,
            diagnostics,
            failure,
        )?;

        Ok(version)
    }

    /// Add one dependency if it has not already been added.
    fn add_dependency(&self, dependency: ArtifactDependency) {
        let mut dependencies = self.dependencies.lock();
        if !dependencies.contains(&dependency) {
            dependencies.push(dependency);
        }
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
        let Some(file) = package.destack_file_id.or(package.package_file_id) else {
            return Err(Self::invalid_anchor(format!(
                "diagnostic package has no manifest file: {package_id:?}"
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

impl DiagnosticContext for SessionProviderContext {
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

impl ProviderContext for SessionProviderContext {
    /// Return the pinned repository revision for this attempt.
    fn revision(&self) -> Revision {
        self.revision()
    }

    /// Return the artifact key being built.
    fn artifact_key(&self) -> ArtifactKey {
        self.key
    }

    /// Require one artifact and return its exact version when ready.
    fn require(&self, key: ArtifactKey) -> Result<ArtifactVersion, ProviderError> {
        if key == self.key {
            return Err(ProviderError::RequirementFailed { key });
        }

        let version = self
            .repository
            .artifact_version(self.revision, &key)
            .map_err(|error| ProviderError::Internal {
                message: format!("failed to read required artifact version: {error}"),
            })?;
        let Some(version) = version else {
            return Err(ProviderError::blocked(key));
        };

        match self.repository.artifact_store().outcome(&version) {
            Some(ArtifactOutcome::Ok) => {}
            Some(ArtifactOutcome::Failed(_)) => {
                self.add_dependency(ArtifactDependency::artifact(version));

                return Err(ProviderError::RequirementFailed { key });
            }
            None => return Err(ProviderError::blocked(key)),
        }

        self.add_dependency(ArtifactDependency::artifact(version));

        Ok(version)
    }

    /// Add one exact dependency read by this attempt.
    fn track(&self, dependency: ArtifactDependency) {
        self.add_dependency(dependency);
    }

    /// Add an already-final diagnostic collection produced by this attempt.
    fn emit_collection(&self, diagnostics: DiagnosticCollection) {
        if diagnostics.is_empty() {
            return;
        }

        self.diagnostics.lock().merge_from(&diagnostics);
    }

    /// Add one diagnostic produced by this attempt.
    fn emit(&self, diagnostic: &dyn DiagnosticLike) -> Result<(), DiagnosticError> {
        let diagnostic = diagnostic.to_diagnostic(self)?;
        let diagnostics = DiagnosticCollection::from_diagnostics(vec![diagnostic]);

        self.emit_collection(diagnostics);

        Ok(())
    }
}
