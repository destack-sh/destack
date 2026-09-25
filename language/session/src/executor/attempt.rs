use std::sync::Arc;

use parking_lot::Mutex;
use tspp_artifact::{
    ArtifactDependency, ArtifactKey, DiagnosticAnchor, DiagnosticContext, DiagnosticDisplay,
    DiagnosticError, DiagnosticLike, DiagnosticRecord, SourceDependency,
};
use tspp_core::Blob;
use tspp_repository::{
    ArtifactAttemptRecorder, ArtifactBase, ProviderContext, Repository, Revision,
};
use tspp_source::{DiagnosticLabel, DiagnosticTarget, FileId, ModuleId, PackageId, Span};

/// One artifact provider attempt owned by an executor worker.
#[derive(Debug)]
pub(crate) struct ProviderAttempt {
    /// The repository that owns the pinned revision.
    pub(super) repository: Arc<Repository>,
    /// The pinned revision for this attempt.
    pub(super) revision: Revision,
    /// The artifact key being built.
    key: ArtifactKey,
    /// The predecessor artifact selected for this attempt.
    base: Option<Arc<ArtifactBase>>,
    /// The frozen dependency observations for provider execution.
    dependencies: Option<Arc<[ArtifactDependency]>>,
    /// The diagnostics produced by this attempt.
    diagnostics: Mutex<Vec<DiagnosticRecord>>,
    /// The dependencies read during provider execution.
    reads: Mutex<Vec<ArtifactDependency>>,
    /// The artifact reads blocked during dependency collection.
    blocked: Mutex<Vec<ArtifactKey>>,
    /// The recorder for this artifact attempt, when the run is timed.
    recorder: Option<Arc<ArtifactAttemptRecorder>>,
}

impl ProviderAttempt {
    /// Create one provider attempt.
    pub(crate) fn new(repository: Arc<Repository>, revision: Revision, key: ArtifactKey) -> Self {
        Self {
            repository,
            revision,
            key,
            base: None,
            dependencies: None,
            diagnostics: Mutex::new(Vec::new()),
            reads: Mutex::new(Vec::new()),
            blocked: Mutex::new(Vec::new()),
            recorder: None,
        }
    }

    /// Take the dependencies read during provider execution.
    pub(crate) fn take_reads(&self) -> Vec<ArtifactDependency> {
        std::mem::take(&mut self.reads.lock())
    }

    /// Attach one recorder to this attempt.
    pub(crate) fn with_recorder(mut self, recorder: Arc<ArtifactAttemptRecorder>) -> Self {
        self.recorder = Some(recorder);

        self
    }

    /// Attach the predecessor artifact selected for this attempt.
    pub(crate) fn with_base(mut self, base: Option<Arc<ArtifactBase>>) -> Self {
        self.base = base;

        self
    }

    /// Attach the frozen dependency observations for provider execution.
    pub(crate) fn with_dependencies(mut self, dependencies: Arc<[ArtifactDependency]>) -> Self {
        self.dependencies = Some(dependencies);

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

    /// Return the predecessor artifact selected for this attempt.
    pub(crate) fn base(&self) -> Option<&ArtifactBase> {
        self.base.as_deref()
    }

    /// Return diagnostics produced by this attempt.
    pub(super) fn diagnostics(&self) -> Vec<DiagnosticRecord> {
        self.diagnostics.lock().clone()
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
        let target = self.anchor_target(anchor)?;
        let blob = self.file_blob(target.file())?;

        Ok(DiagnosticLabel {
            blob,
            target,
            message,
        })
    }

    /// Resolve one provider diagnostic anchor into a source target.
    fn anchor_target(
        &self,
        anchor: &DiagnosticAnchor,
    ) -> Result<DiagnosticTarget, DiagnosticError> {
        let target = match anchor {
            DiagnosticAnchor::Span(span) => DiagnosticTarget::Span(*span),
            DiagnosticAnchor::File(file) => DiagnosticTarget::File(*file),
            DiagnosticAnchor::Module(module) => {
                DiagnosticTarget::File(self.module_file_id(*module)?)
            }
            DiagnosticAnchor::Package(package) => {
                DiagnosticTarget::File(self.package_span(*package)?.file)
            }
        };

        Ok(target)
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
        let Some(configuration) = package.configuration.as_ref() else {
            return Err(Self::invalid_anchor(format!(
                "diagnostic package has no manifest file: {package_id:?}"
            )));
        };

        Ok(Span::empty(configuration.file_id))
    }

    /// Return one file's blob in this revision.
    fn file_blob(&self, file: FileId) -> Result<Blob, DiagnosticError> {
        let blob = self
            .repository
            .file_blob(self.revision, file)
            .map_err(|error| {
                Self::invalid_anchor(format!("failed to read diagnostic File Blob: {error}"))
            })?;
        let Some(blob) = blob else {
            return Err(Self::invalid_anchor(format!(
                "diagnostic file is not tracked in revision: {file:?}"
            )));
        };

        // retain the exact source selected by this label
        let source = SourceDependency::file(file, blob.id);
        self.observe(ArtifactDependency::Source(source));

        Ok(blob)
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

impl ProviderAttempt {
    /// Drain the artifact reads blocked during dependency collection.
    pub(super) fn take_blocked(&self) -> Vec<ArtifactKey> {
        std::mem::take(&mut self.blocked.lock())
    }
}

impl ProviderContext for ProviderAttempt {
    /// Record one dependency read during provider execution.
    fn observe(&self, dependency: ArtifactDependency) {
        self.reads.lock().push(dependency);
    }

    /// Record one blocked artifact read during dependency collection.
    fn record_blocked(&self, artifact_key: ArtifactKey) {
        self.blocked.lock().push(artifact_key);
    }

    /// Return the pinned repository revision for this attempt.
    fn revision(&self) -> Revision {
        self.revision()
    }

    /// Return the artifact key being built.
    fn artifact_key(&self) -> ArtifactKey {
        self.key
    }

    /// Return the predecessor artifact selected for this attempt.
    fn artifact_base(&self) -> Option<&ArtifactBase> {
        self.base()
    }

    /// Return the frozen dependency observations for provider execution.
    fn artifact_dependencies(&self) -> Option<&[ArtifactDependency]> {
        self.dependencies.as_deref()
    }

    /// Add already-recorded diagnostics produced by this attempt.
    fn emit_diagnostics(&self, diagnostics: Vec<DiagnosticRecord>) {
        if diagnostics.is_empty() {
            return;
        }

        self.diagnostics.lock().extend(diagnostics);
    }

    /// Add one diagnostic produced by this attempt.
    fn emit(&self, diagnostic: &dyn DiagnosticLike) -> Result<(), DiagnosticError> {
        let record = DiagnosticRecord::new(diagnostic.to_diagnostic(self)?);
        self.emit_diagnostics(vec![record]);

        Ok(())
    }

    /// Return the recorder for this artifact attempt, when the run is timed.
    fn recorder(&self) -> Option<&ArtifactAttemptRecorder> {
        self.recorder.as_deref()
    }
}
