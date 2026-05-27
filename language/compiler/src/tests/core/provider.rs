use std::cell::RefCell;
use std::sync::Arc;

use destack_artifact::{
    ArtifactDependency, ArtifactFailure, ArtifactKey, ArtifactPayload, ArtifactProvider,
    ArtifactSidecar, ArtifactVersion, DiagnosticAnchor, DiagnosticContext, DiagnosticDisplay,
    DiagnosticError, DiagnosticLike,
};
use destack_source::{
    DiagnosticCollection, DiagnosticLabel, FileContentId, FileId, ModuleId, Span,
};
use destack_workspace::{ProviderContext, ProviderError, Repository, Revision};

use super::module::{parse_module, parsed_dependencies};
use crate::Compiler;

/// Synchronous compiler artifact provider used by crate-local tests.
#[derive(Debug)]
pub(crate) struct TestProvider {
    /// The repository that owns the pinned revision.
    repository: Arc<Repository>,
    /// The pinned test revision.
    revision: Revision,
    /// The compiler under test.
    compiler: Compiler,
    /// The active requirement stack.
    active: RefCell<Vec<ArtifactKey>>,
}

impl TestProvider {
    /// Create one test provider.
    pub(crate) fn new(repository: Arc<Repository>, revision: Revision) -> Self {
        let compiler = Compiler::new(repository.clone());

        Self {
            repository,
            revision,
            compiler,
            active: RefCell::new(Vec::new()),
        }
    }

    /// Require one artifact.
    pub(crate) fn require(&self, key: ArtifactKey) -> Result<ArtifactVersion, ProviderError> {
        // reuse finished artifacts
        if let Some(version) = self.ready_artifact(key)? {
            return Ok(version);
        }

        // provide missing artifact synchronously
        self.push_active(key)?;
        let result = self.provide(key);
        self.pop_active(key);

        result
    }

    /// Provide one missing artifact.
    fn provide(&self, key: ArtifactKey) -> Result<ArtifactVersion, ProviderError> {
        // run provider against this attempt context
        let context = TestProviderContext::new(self, key);
        let result = match key.provider() {
            ArtifactProvider::Loader => self.provide_loader(key),
            ArtifactProvider::Compiler => self.compiler.provide(&context),
            ArtifactProvider::Linter | ArtifactProvider::Query => Err(Box::new(
                ProviderError::internal(format!("unsupported test artifact key: {key:?}")),
            )),
        };

        // publish the attempt outcome
        match result {
            Ok(payload) => self.complete_ready(key, &context, payload),
            Err(error) => self.complete_failed(key, &context, *error),
        }
    }

    /// Provide one loader-owned artifact.
    fn provide_loader(&self, key: ArtifactKey) -> Result<ArtifactPayload, Box<ProviderError>> {
        match key {
            ArtifactKey::DirParsed { module } => self.provide_dir_parsed(module),
            ArtifactKey::Data { .. } => Err(Box::new(ProviderError::internal(format!(
                "unsupported test loader artifact key: {key:?}"
            )))),
            _ => Err(Box::new(ProviderError::internal(format!(
                "non loader artifact key reached test loader: {key:?}"
            )))),
        }
    }

    /// Provide one parsed DIR artifact.
    fn provide_dir_parsed(&self, module: ModuleId) -> Result<ArtifactPayload, Box<ProviderError>> {
        // load repository module
        let module = self
            .repository
            .module(self.revision, module)
            .map_err(|error| ProviderError::internal(error.to_string()))?
            .ok_or_else(|| ProviderError::internal(format!("missing module: {module:?}")))?;

        // parse code modules
        if module.is_code() {
            let dir = parse_module(module.as_ref(), self.repository.as_ref(), self.revision);

            return Ok(ArtifactPayload::DirParsed(dir));
        }

        Err(Box::new(ProviderError::internal(format!(
            "module has no parsed DIR payload: {:?}",
            module.id
        ))))
    }

    /// Complete one ready provider attempt.
    fn complete_ready(
        &self,
        key: ArtifactKey,
        context: &TestProviderContext<'_>,
        payload: ArtifactPayload,
    ) -> Result<ArtifactVersion, ProviderError> {
        // collect attempt output
        let mut dependencies = context.dependencies();
        if let ArtifactPayload::DirParsed(_) = &payload {
            if let ArtifactKey::DirParsed { module } = key {
                let module = self
                    .repository
                    .module(self.revision, module)
                    .map_err(|error| ProviderError::internal(error.to_string()))?
                    .ok_or_else(|| {
                        ProviderError::internal(format!("missing module: {module:?}"))
                    })?;
                dependencies.extend(parsed_dependencies(
                    self.repository.as_ref(),
                    self.revision,
                    module.as_ref(),
                ));
            }
        }
        let diagnostics = context.diagnostics();
        let sidecars = context.sidecars();
        let version = ArtifactVersion::new(key, dependencies.iter().cloned());

        // publish ready artifact
        self.repository
            .complete_artifact(
                self.revision,
                version,
                payload,
                dependencies,
                diagnostics,
                sidecars,
            )
            .map_err(|error| ProviderError::internal(error.to_string()))?;

        Ok(version)
    }

    /// Complete one failed provider attempt.
    fn complete_failed(
        &self,
        key: ArtifactKey,
        context: &TestProviderContext<'_>,
        error: ProviderError,
    ) -> Result<ArtifactVersion, ProviderError> {
        // keep missing requirements out of the artifact store
        let failure = match &error {
            ProviderError::Blocked { .. } => return Err(error),
            ProviderError::RequirementFailed { key } => ArtifactFailure::requirement(*key),
            ProviderError::Corrupt { version } => ArtifactFailure::corrupt(*version),
            ProviderError::Failed { failure } => failure.clone(),
            ProviderError::Internal { message } => ArtifactFailure::internal(message.clone()),
        };

        // collect attempt output
        let dependencies = context.dependencies();
        let diagnostics = context.diagnostics();
        let sidecars = context.sidecars();
        let version = ArtifactVersion::new(key, dependencies.iter().cloned());

        // publish failed artifact
        self.repository
            .fail_artifact(
                self.revision,
                version,
                dependencies,
                diagnostics,
                sidecars,
                failure,
            )
            .map_err(|error| ProviderError::internal(error.to_string()))?;

        Err(error)
    }

    /// Return one ready artifact version.
    fn ready_artifact(&self, key: ArtifactKey) -> Result<Option<ArtifactVersion>, ProviderError> {
        // find current version
        let Some(version) = self.artifact_version(key)? else {
            return Ok(None);
        };

        // return only completed ready artifacts
        match self.repository.artifact_store().outcome(&version) {
            Some(destack_artifact::ArtifactOutcome::Ok) => Ok(Some(version)),
            Some(destack_artifact::ArtifactOutcome::Failed(_)) => {
                Err(ProviderError::RequirementFailed { key })
            }
            None => Ok(None),
        }
    }

    /// Return the current artifact version.
    fn artifact_version(&self, key: ArtifactKey) -> Result<Option<ArtifactVersion>, ProviderError> {
        self.repository
            .artifact_version(self.revision, &key)
            .map_err(|error| ProviderError::internal(error.to_string()))
    }

    /// Mark one artifact as active.
    fn push_active(&self, key: ArtifactKey) -> Result<(), ProviderError> {
        // reject dependency cycles
        let mut active = self.active.borrow_mut();
        if active.contains(&key) {
            return Err(ProviderError::RequirementFailed { key });
        }

        // push active requirement
        active.push(key);

        Ok(())
    }

    /// Mark one artifact as inactive.
    fn pop_active(&self, key: ArtifactKey) {
        // pop exact requirement
        let actual = self.active.borrow_mut().pop();

        assert_eq!(actual, Some(key));
    }
}

/// Provider attempt context used by compiler tests.
#[derive(Debug)]
struct TestProviderContext<'a> {
    /// The provider handling this attempt.
    provider: &'a TestProvider,
    /// The artifact key being built.
    key: ArtifactKey,
    /// The dependencies recorded by this attempt.
    dependencies: RefCell<Vec<ArtifactDependency>>,
    /// The diagnostics recorded by this attempt.
    diagnostics: RefCell<DiagnosticCollection>,
    /// The sidecars recorded by this attempt.
    sidecars: RefCell<Vec<ArtifactSidecar>>,
}

impl<'a> TestProviderContext<'a> {
    /// Create one test provider context.
    fn new(provider: &'a TestProvider, key: ArtifactKey) -> Self {
        Self {
            provider,
            key,
            dependencies: RefCell::new(Vec::new()),
            diagnostics: RefCell::new(DiagnosticCollection::new()),
            sidecars: RefCell::new(Vec::new()),
        }
    }

    /// Return the exact dependencies read by this attempt.
    fn dependencies(&self) -> Vec<ArtifactDependency> {
        self.dependencies.borrow().clone()
    }

    /// Return diagnostics produced by this attempt.
    fn diagnostics(&self) -> DiagnosticCollection {
        self.diagnostics.borrow().clone()
    }

    /// Return sidecars produced by this attempt.
    fn sidecars(&self) -> Vec<ArtifactSidecar> {
        self.sidecars.borrow().clone()
    }

    /// Add one exact dependency.
    fn add_dependency(&self, dependency: ArtifactDependency) {
        let mut dependencies = self.dependencies.borrow_mut();
        if !dependencies.contains(&dependency) {
            dependencies.push(dependency);
        }
    }

    /// Return one diagnostic label.
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

    /// Return the source span for one diagnostic anchor.
    fn anchor_span(&self, anchor: &DiagnosticAnchor) -> Result<Span, DiagnosticError> {
        match anchor {
            DiagnosticAnchor::Span(span) => Ok(*span),
            DiagnosticAnchor::File(file) => Ok(Span::empty(*file)),
            DiagnosticAnchor::Module(module) => self.module_file_id(*module).map(Span::empty),
            DiagnosticAnchor::Package(_) => Err(Self::invalid_anchor(
                "package diagnostics are not used in compiler tests",
            )),
        }
    }

    /// Return one file content id in this revision.
    fn file_content_id(&self, file: FileId) -> Result<FileContentId, DiagnosticError> {
        self.provider
            .repository
            .file_content_id(self.provider.revision, file)
            .map_err(|error| Self::invalid_anchor(error.to_string()))?
            .ok_or_else(|| Self::invalid_anchor(format!("missing diagnostic file: {file:?}")))
    }

    /// Return the file id for one module.
    fn module_file_id(&self, module: ModuleId) -> Result<FileId, DiagnosticError> {
        self.provider
            .repository
            .module(self.provider.revision, module)
            .map_err(|error| Self::invalid_anchor(error.to_string()))?
            .map(|module| module.file_id)
            .ok_or_else(|| Self::invalid_anchor(format!("missing diagnostic module: {module:?}")))
    }

    /// Build one invalid anchor error.
    fn invalid_anchor(message: impl Into<String>) -> DiagnosticError {
        DiagnosticError::InvalidAnchor {
            message: message.into(),
        }
    }
}

impl DiagnosticContext for TestProviderContext<'_> {
    /// Resolve one provider diagnostic anchor into a final source label.
    fn label(
        &self,
        anchor: &DiagnosticAnchor,
        message: Option<String>,
    ) -> Result<DiagnosticLabel, DiagnosticError> {
        self.resolve_label(anchor, message)
    }

    /// Display one repository-backed value.
    fn display(&self, display: DiagnosticDisplay) -> Result<String, DiagnosticError> {
        match display {
            DiagnosticDisplay::Module(module) => self
                .provider
                .repository
                .module_display(self.provider.revision, module)
                .map_err(|error| Self::invalid_anchor(error.to_string()))?
                .ok_or_else(|| Self::invalid_anchor(format!("missing module: {module:?}"))),
            DiagnosticDisplay::Package(package) => self
                .provider
                .repository
                .package_display(self.provider.revision, package)
                .map_err(|error| Self::invalid_anchor(error.to_string()))?
                .ok_or_else(|| Self::invalid_anchor(format!("missing package: {package:?}"))),
            DiagnosticDisplay::Target(target) => self
                .provider
                .repository
                .target_display(self.provider.revision, target)
                .map_err(|error| Self::invalid_anchor(error.to_string()))?
                .ok_or_else(|| Self::invalid_anchor(format!("missing target: {target:?}"))),
        }
    }
}

impl ProviderContext for TestProviderContext<'_> {
    /// Return the pinned repository revision for this attempt.
    fn revision(&self) -> Revision {
        self.provider.revision
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

        match self.provider.require(key) {
            Ok(version) => {
                self.add_dependency(ArtifactDependency::artifact(version));

                Ok(version)
            }
            Err(ProviderError::RequirementFailed { key }) => {
                if let Some(version) = self.provider.artifact_version(key)? {
                    self.add_dependency(ArtifactDependency::artifact(version));
                }

                Err(ProviderError::RequirementFailed { key })
            }
            Err(error) => Err(error),
        }
    }

    /// Require many artifacts and return their exact versions when ready.
    fn require_all(&self, keys: &[ArtifactKey]) -> Result<Vec<ArtifactVersion>, ProviderError> {
        let mut versions = Vec::with_capacity(keys.len());
        let mut blocked = Vec::new();

        for key in keys {
            match self.require(*key) {
                Ok(version) => versions.push(version),
                Err(ProviderError::Blocked { .. }) => blocked.push(*key),
                Err(error) => return Err(error),
            }
        }

        if blocked.is_empty() {
            Ok(versions)
        } else {
            Err(ProviderError::blocked_many(blocked))
        }
    }

    /// Add one exact dependency read by this attempt.
    fn track(&self, dependency: ArtifactDependency) {
        self.add_dependency(dependency);
    }

    /// Add an already-final diagnostic collection produced by this attempt.
    fn emit_diagnostics(&self, diagnostics: DiagnosticCollection) {
        self.diagnostics.borrow_mut().merge_from(&diagnostics);
    }

    /// Add one sidecar produced by this attempt.
    fn emit_sidecar(&self, sidecar: ArtifactSidecar) {
        self.sidecars.borrow_mut().push(sidecar);
    }

    /// Add one diagnostic produced by this attempt.
    fn emit(&self, diagnostic: &dyn DiagnosticLike) -> Result<(), DiagnosticError> {
        let diagnostic = diagnostic.to_diagnostic(self)?;

        self.diagnostics.borrow_mut().insert(diagnostic);

        Ok(())
    }
}
