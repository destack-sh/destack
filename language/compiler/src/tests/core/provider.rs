use std::cell::RefCell;
use std::sync::Arc;

use destack_artifact::{
    ArtifactDependency, ArtifactDependencySet, ArtifactFailure, ArtifactKey, ArtifactPayload,
    ArtifactProvider, ArtifactSidecar, ArtifactVersion, DiagnosticAnchor, DiagnosticContext,
    DiagnosticDisplay, DiagnosticError, DiagnosticLike,
};
use destack_repository::{
    ArtifactProduct, Collector, Provider, ProviderContext, ProviderError, ProviderOutput,
    ProviderResult, Repository, Revision,
};
use destack_source::{ContentId, DiagnosticCollection, DiagnosticLabel, FileId, ModuleId, Span};

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
    /// Whether provider attempts should emit event traces.
    emit_events: bool,
}

impl TestProvider {
    /// Create one test provider.
    pub(crate) fn new(repository: Arc<Repository>, revision: Revision, emit_events: bool) -> Self {
        let compiler = Compiler::new(repository.clone());

        Self {
            repository,
            revision,
            compiler,
            emit_events,
        }
    }

    /// Require one artifact, building its closure depth first.
    pub(crate) fn require(&self, key: ArtifactKey) -> Result<ArtifactVersion, ProviderError> {
        self.repository
            .resolve(self, self.revision, key)
            .map_err(|error| *error)
    }

    /// Collect the source closure for one loader-owned artifact.
    fn collect_loader(&self, key: ArtifactKey) -> ProviderResult<ArtifactDependencySet> {
        let ArtifactKey::DirParsed { module } = key else {
            return Err(ProviderError::internal(format!(
                "unsupported test loader artifact key: {key:?}"
            ))
            .into());
        };

        let module = self
            .repository
            .module(self.revision, module)
            .map_err(|error| ProviderError::internal(error.to_string()))?
            .ok_or_else(|| ProviderError::internal(format!("missing module: {module:?}")))?;

        // observe each contributing source file
        let mut dependencies = ArtifactDependencySet::default();
        for dependency in
            parsed_dependencies(self.repository.as_ref(), self.revision, module.as_ref())
        {
            if let ArtifactDependency::Source(source) = dependency {
                dependencies.observe_source(source);
            }
        }

        Ok(dependencies)
    }

    /// Provide one parsed DIR artifact from source.
    fn provide_dir_parsed(&self, module: ModuleId) -> ProviderResult<ArtifactPayload> {
        let module = self
            .repository
            .module(self.revision, module)
            .map_err(|error| ProviderError::internal(error.to_string()))?
            .ok_or_else(|| ProviderError::internal(format!("missing module: {module:?}")))?;

        // parse code modules only
        if module.is_code() {
            let dir = parse_module(module.as_ref(), self.repository.as_ref(), self.revision);

            return Ok(ArtifactPayload::DirParsed(Arc::new(dir)));
        }

        Err(
            ProviderError::internal(format!("module has no parsed DIR payload: {:?}", module.id))
                .into(),
        )
    }

    /// Harvest one provider attempt into its terminal output.
    fn harvest(
        context: &TestProviderContext<'_>,
        result: ProviderResult<ArtifactPayload>,
    ) -> ProviderResult<ProviderOutput> {
        let product = match result {
            Ok(payload) => ArtifactProduct::Ready(payload),
            Err(error) => match *error {
                ProviderError::Failed { failure } => ArtifactProduct::Failed(failure),
                ProviderError::RequirementFailed { key } => {
                    ArtifactProduct::Failed(ArtifactFailure::requirement(key))
                }
                error => return Err(error.into()),
            },
        };

        Ok(ProviderOutput {
            product,
            diagnostics: context.diagnostics(),
            sidecars: context.sidecars(),
        })
    }
}

impl Collector for TestProvider {
    /// Collect the dependency closure for one artifact key.
    fn collect(
        &self,
        _revision: Revision,
        key: ArtifactKey,
    ) -> ProviderResult<ArtifactDependencySet> {
        match key.provider() {
            ArtifactProvider::Loader => self.collect_loader(key),
            ArtifactProvider::Compiler => {
                let context = TestProviderContext::new(self, key);

                self.compiler.collect(&context)
            }
            ArtifactProvider::Linter | ArtifactProvider::Query => Err(ProviderError::internal(
                format!("unsupported test artifact key: {key:?}"),
            )
            .into()),
        }
    }
}

impl Provider for TestProvider {
    /// Build one artifact over its frozen dependency closure.
    fn provide(&self, _revision: Revision, key: ArtifactKey) -> ProviderResult<ProviderOutput> {
        let context = TestProviderContext::new(self, key);
        let result = match key.provider() {
            ArtifactProvider::Loader => match key {
                ArtifactKey::DirParsed { module } => self.provide_dir_parsed(module),
                _ => Err(ProviderError::internal(format!(
                    "unsupported test loader artifact key: {key:?}"
                ))
                .into()),
            },
            ArtifactProvider::Compiler => self.compiler.provide(&context),
            ArtifactProvider::Linter | ArtifactProvider::Query => Err(ProviderError::internal(
                format!("unsupported test artifact key: {key:?}"),
            )
            .into()),
        };

        Self::harvest(&context, result)
    }
}

/// Provider attempt context used by compiler tests.
#[derive(Debug)]
struct TestProviderContext<'a> {
    /// The provider handling this attempt.
    provider: &'a TestProvider,
    /// The artifact key being built.
    key: ArtifactKey,
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
            diagnostics: RefCell::new(DiagnosticCollection::new()),
            sidecars: RefCell::new(Vec::new()),
        }
    }

    /// Return diagnostics produced by this attempt.
    fn diagnostics(&self) -> DiagnosticCollection {
        self.diagnostics.borrow().clone()
    }

    /// Return sidecars produced by this attempt.
    fn sidecars(&self) -> Vec<ArtifactSidecar> {
        self.sidecars.borrow().clone()
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
    fn file_content_id(&self, file: FileId) -> Result<ContentId, DiagnosticError> {
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

    /// Emit event traces from compiler test attempts.
    fn emit_events(&self) -> bool {
        self.provider.emit_events
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
