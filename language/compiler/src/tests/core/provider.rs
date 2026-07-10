use std::cell::{Cell, RefCell};
use std::sync::Arc;

use destack_artifact::{
    ArtifactDependency, ArtifactDependencySet, ArtifactFailure, ArtifactKey, ArtifactPayload,
    ArtifactProvider, ArtifactSidecar, ArtifactVersion, DiagnosticAnchor, DiagnosticContext,
    DiagnosticDisplay, DiagnosticError, DiagnosticLike,
};
use destack_repository::{
    ArtifactAttemptOutcome, ArtifactAttemptRecorder, ArtifactBase, Clock, DependencySetResolution,
    ProviderContext, ProviderError, ProviderResult, Repository, Revision, Trace, TraceSnapshot,
    TraceView,
};
use destack_source::{
    Content, ContentId, DiagnosticCollection, DiagnosticLabel, FileId, ModuleId, Span,
};

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
    /// The artifact trace for this test provider.
    trace: Arc<Trace>,
    /// Whether attempts retain event traces, set by event assertions.
    emit_events: Cell<bool>,
}

impl TestProvider {
    /// Retain event traces for following attempts.
    pub(crate) fn retain_events(&self) {
        self.emit_events.set(true);
    }

    /// Create one test provider.
    pub(crate) fn new(repository: Arc<Repository>, revision: Revision) -> Self {
        let compiler = Compiler::new(repository.clone());

        Self {
            repository,
            revision,
            compiler,
            trace: Trace::new(Clock::default()),
            emit_events: Cell::new(false),
        }
    }

    /// Return a detailed snapshot of this provider's artifact trace.
    pub(crate) fn trace(&self) -> TraceSnapshot {
        self.trace.finish();

        self.trace.snapshot(TraceView::Detailed, |_| None, |_| None)
    }

    /// Require one artifact, building its closure depth first.
    pub(crate) fn require(&self, key: ArtifactKey) -> Result<ArtifactVersion, ProviderError> {
        self.resolve(key).map_err(|error| *error)
    }

    /// Resolve one artifact through the repository dependency-set model.
    fn resolve(&self, key: ArtifactKey) -> ProviderResult<ArtifactVersion> {
        let recorder = Arc::new(self.trace.begin(key, 0));

        if let Some(version) = recorder.span("terminal", || self.terminal_version(key))? {
            recorder.finish(ArtifactAttemptOutcome::MemoryCached);

            return Ok(version);
        }

        let base = recorder
            .span("base", || self.repository.artifact_base(self.revision, key))
            .map_err(|error| ProviderError::internal(error.to_string()))?;
        let mut set = recorder.span("collect", || self.collect(key, base))?;
        loop {
            match recorder.span("dependencies", || {
                self.repository.resolve_dependency_set(self.revision, set)
            })? {
                DependencySetResolution::Incomplete => {
                    set = recorder.span("collect", || self.collect(key, base))?;
                }
                DependencySetResolution::Pending {
                    frontier,
                    pending_set,
                } => {
                    recorder.record_counter("frontier", frontier.len() as u64);
                    for dependency in frontier {
                        self.resolve(dependency)?;
                    }
                    set = if let Some(pending_set) = pending_set {
                        pending_set
                    } else {
                        recorder.span("collect", || self.collect(key, base))?
                    };
                }
                DependencySetResolution::Resolved {
                    base: base_version,
                    dependencies,
                    failed,
                } => {
                    return self.commit(key, base, base_version, dependencies, failed, recorder);
                }
            }
        }
    }

    /// Return one terminal artifact binding.
    fn terminal_version(&self, key: ArtifactKey) -> ProviderResult<Option<ArtifactVersion>> {
        let version = self
            .repository
            .artifact_binding(self.revision, &key)
            .map_err(|error| ProviderError::internal(error.to_string()))?;
        let Some(version) = version else {
            return Ok(None);
        };

        Ok(self
            .repository
            .artifact_table()
            .outcome(&version)
            .map(|_| version))
    }

    /// Commit one frozen artifact dependency set.
    fn commit(
        &self,
        key: ArtifactKey,
        artifact_base: Option<ArtifactBase>,
        base: Option<ArtifactVersion>,
        dependencies: Vec<ArtifactDependency>,
        failed: Option<ArtifactKey>,
        recorder: Arc<ArtifactAttemptRecorder>,
    ) -> ProviderResult<ArtifactVersion> {
        let version = recorder.span("version", || {
            ArtifactVersion::new(
                key,
                self.repository.build_fingerprint(),
                base,
                dependencies.iter().cloned(),
            )
        });

        // record poisoned dependencies without running the provider
        if let Some(failed) = failed {
            recorder.span("complete", || {
                self.fail(
                    version,
                    base,
                    dependencies,
                    DiagnosticCollection::new(),
                    Vec::new(),
                    ArtifactFailure::requirement(failed),
                )
            })?;
            recorder.finish(ArtifactAttemptOutcome::Failed);

            return Ok(version);
        }

        // reuse committed memory or disk records before running the provider
        if recorder.span("memory_cache", || {
            self.repository.artifact_table().outcome(&version).is_some()
        }) {
            recorder.span("bind", || {
                self.repository
                    .bind_artifact(self.revision, version)
                    .map_err(|error| ProviderError::internal(error.to_string()))
            })?;
            recorder.finish(ArtifactAttemptOutcome::MemoryCached);

            return Ok(version);
        }
        if recorder.span("store_cache", || {
            self.repository
                .load_artifact(self.revision, version)
                .map_err(|error| ProviderError::internal(error.to_string()))
        })? {
            recorder.finish(ArtifactAttemptOutcome::StoreCached);

            return Ok(version);
        }

        // run the local test provider and publish its terminal outcome
        let context =
            TestProviderContext::new(self, key, artifact_base).with_recorder(recorder.clone());
        match recorder.span("provider", || self.provide(key, &context)) {
            Ok(payload) => {
                recorder.span("publish", || {
                    self.repository
                        .complete_artifact(
                            self.revision,
                            version,
                            base,
                            payload,
                            dependencies,
                            context.diagnostics(),
                            context.sidecars(),
                        )
                        .map_err(|error| ProviderError::internal(error.to_string()))
                })?;
                recorder.finish(ArtifactAttemptOutcome::Built);
            }
            Err(error) => match *error {
                ProviderError::Failed { failure } => {
                    recorder.span("complete", || {
                        self.fail(
                            version,
                            base,
                            dependencies,
                            context.diagnostics(),
                            context.sidecars(),
                            failure,
                        )
                    })?;
                    recorder.finish(ArtifactAttemptOutcome::Failed);
                }
                ProviderError::RequirementFailed { key } => {
                    recorder.span("complete", || {
                        self.fail(
                            version,
                            base,
                            dependencies,
                            context.diagnostics(),
                            context.sidecars(),
                            ArtifactFailure::requirement(key),
                        )
                    })?;
                    recorder.finish(ArtifactAttemptOutcome::Failed);
                }
                error => {
                    recorder.finish(ArtifactAttemptOutcome::Failed);

                    return Err(context.internal_error(error).into());
                }
            },
        }

        Ok(version)
    }

    /// Fail one artifact in the test repository.
    fn fail(
        &self,
        version: ArtifactVersion,
        base: Option<ArtifactVersion>,
        dependencies: Vec<ArtifactDependency>,
        diagnostics: DiagnosticCollection,
        sidecars: Vec<ArtifactSidecar>,
        failure: ArtifactFailure,
    ) -> ProviderResult<()> {
        self.repository
            .fail_artifact(
                self.revision,
                version,
                base,
                dependencies,
                diagnostics,
                sidecars,
                failure,
            )
            .map_err(|error| ProviderError::internal(error.to_string()).into())
    }

    /// Collect the dependency set for one artifact key.
    fn collect(
        &self,
        key: ArtifactKey,
        base: Option<ArtifactBase>,
    ) -> ProviderResult<ArtifactDependencySet> {
        match key.provider() {
            ArtifactProvider::Loader => self.collect_loader(key),
            ArtifactProvider::Compiler => {
                let context = TestProviderContext::new(self, key, base);

                self.compiler.collect(&context)
            }
            ArtifactProvider::Linter | ArtifactProvider::Index => Err(ProviderError::internal(
                format!("unsupported test artifact key: {key:?}"),
            )
            .into()),
        }
    }

    /// Collect the source dependency set for one loader-owned artifact.
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
                dependencies.observe(source);
            }
        }

        Ok(dependencies)
    }

    /// Provide one artifact payload.
    fn provide(
        &self,
        key: ArtifactKey,
        context: &TestProviderContext<'_>,
    ) -> ProviderResult<ArtifactPayload> {
        match key.provider() {
            ArtifactProvider::Loader => match key {
                ArtifactKey::DirParsed { module } => self.provide_dir_parsed(module),
                _ => Err(ProviderError::internal(format!(
                    "unsupported test loader artifact key: {key:?}"
                ))
                .into()),
            },
            ArtifactProvider::Compiler => self.compiler.provide(context),
            ArtifactProvider::Linter | ArtifactProvider::Index => Err(ProviderError::internal(
                format!("unsupported test artifact key: {key:?}"),
            )
            .into()),
        }
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
}

/// Provider attempt context used by compiler tests.
#[derive(Debug)]
struct TestProviderContext<'a> {
    /// The provider handling this attempt.
    provider: &'a TestProvider,
    /// The artifact key being built.
    key: ArtifactKey,
    /// The predecessor artifact selected for this attempt.
    base: Option<ArtifactBase>,
    /// The diagnostics recorded by this attempt.
    diagnostics: RefCell<DiagnosticCollection>,
    /// The sidecars recorded by this attempt.
    sidecars: RefCell<Vec<ArtifactSidecar>>,
    /// The artifact trace recorder for this attempt.
    recorder: Option<Arc<ArtifactAttemptRecorder>>,
}

impl<'a> TestProviderContext<'a> {
    /// Create one test provider context.
    fn new(provider: &'a TestProvider, key: ArtifactKey, base: Option<ArtifactBase>) -> Self {
        Self {
            provider,
            key,
            base,
            diagnostics: RefCell::new(DiagnosticCollection::new()),
            sidecars: RefCell::new(Vec::new()),
            recorder: None,
        }
    }

    /// Attach one artifact trace recorder.
    fn with_recorder(mut self, recorder: Arc<ArtifactAttemptRecorder>) -> Self {
        self.recorder = Some(recorder);

        self
    }

    /// Return diagnostics produced by this attempt.
    fn diagnostics(&self) -> DiagnosticCollection {
        self.diagnostics.borrow().clone()
    }

    /// Return sidecars produced by this attempt.
    fn sidecars(&self) -> Vec<ArtifactSidecar> {
        self.sidecars.borrow().clone()
    }

    /// Return one internal provider error with any retained trace sidecar.
    fn internal_error(&self, error: ProviderError) -> ProviderError {
        let mut message = error.to_string();

        if let Some(events) = self.check_event_sidecar() {
            message.push_str("\n\n=== check.events ===\n");
            message.push_str(&events);
        }

        ProviderError::internal(message)
    }

    /// Return the retained check event sidecar for this attempt.
    fn check_event_sidecar(&self) -> Option<String> {
        self.sidecars.borrow().iter().find_map(|sidecar| {
            let is_check_events = sidecar.name == "events"
                && sidecar
                    .labels
                    .get("phase")
                    .is_some_and(|phase| phase == "check");
            if !is_check_events {
                return None;
            }

            match &sidecar.content {
                Content::Text { content } => Some(content.clone()),
                Content::Binary { .. } => None,
            }
        })
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

    /// Return the predecessor artifact selected for this attempt.
    fn artifact_base(&self) -> Option<ArtifactBase> {
        self.base
    }

    /// Emit event traces when one event assertion requested them.
    fn emit_events(&self) -> bool {
        self.provider.emit_events.get()
    }

    /// Return the recorder for this artifact attempt.
    fn recorder(&self) -> Option<&ArtifactAttemptRecorder> {
        self.recorder.as_deref()
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
