use destack as rust;
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::PyModule;

use crate::{
    ArtifactKey, ArtifactRecord, ArtifactSidecar, ArtifactVersion, BuildOutput, BuildRequest,
    Change, CheckOutput, Commit, Content, ContentId, Diagnostic, DirResolved, Edit, FormatOutput,
    FormatRequest, LintOutput, LintRequest, Module, PackageId, ParseOutput, ProfileId, Revision,
    SessionFile, Source, TargetId, TraceReport, artifact, diagnostic, dir, repository, session,
    source,
};

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Python language repository.
#[pyclass(name = "Repository", module = "destack._native", skip_from_py_object)]
#[derive(Debug, Clone)]
pub struct Repository {
    /// Rust bridge repository.
    repository: rust::Repository,
}

#[pymethods]
impl Repository {
    /// Open one repository from one source input.
    #[staticmethod]
    pub fn open(source: Source) -> PyResult<Self> {
        let repository = rust::Repository::open(source.into_bridge()).map_err(to_error)?;

        Ok(Self { repository })
    }

    /// Return this repository root path.
    pub fn root(&self) -> String {
        self.repository.root()
    }

    /// Open one workspace over this repository.
    pub fn workspace(&self) -> PyResult<Workspace> {
        let workspace =
            rust::Workspace::from_repository(self.repository.clone()).map_err(to_error)?;

        Ok(Workspace { workspace })
    }
}

/// Python language workspace.
#[pyclass(name = "Workspace", module = "destack._native", skip_from_py_object)]
#[derive(Debug)]
pub struct Workspace {
    /// Rust bridge workspace.
    workspace: rust::Workspace,
}

#[pymethods]
impl Workspace {
    /// Open one workspace from one source input.
    #[staticmethod]
    pub fn open(source: Source) -> PyResult<Self> {
        let workspace = rust::Workspace::open(source.into_bridge()).map_err(to_error)?;

        Ok(Self { workspace })
    }

    /// Return this workspace root path.
    pub fn root(&self) -> String {
        self.workspace.root()
    }

    /// Return the current workspace revision.
    pub fn revision(&self) -> PyResult<Revision> {
        let value = self.workspace.revision().map_err(to_error)?;

        Ok(Revision::from_bridge(
            rust::language::Revision::from_repository(value),
        ))
    }

    /// Return editable repository file paths.
    pub fn files(&self) -> PyResult<Vec<SessionFile>> {
        let files = self.workspace.files().map_err(to_error)?;
        let files = files.into_iter().map(SessionFile::from_bridge).collect();

        Ok(files)
    }

    /// Edit files through the current workspace revision.
    pub fn edit(&self, edits: Vec<Edit>) -> PyResult<Commit> {
        let edits = edits.into_iter().map(Edit::into_bridge).collect();
        let value = self.workspace.edit(edits).map_err(to_error)?;

        Ok(Commit::from_bridge(value))
    }

    /// Edit files when the current workspace revision still matches.
    pub fn edit_if_current(&self, revision: Revision, edits: Vec<Edit>) -> PyResult<Commit> {
        let revision = repository_revision(revision)?;
        let edits = edits.into_iter().map(Edit::into_bridge).collect();
        let value = self
            .workspace
            .edit_if_current(revision, edits)
            .map_err(to_error)?;

        Ok(Commit::from_bridge(value))
    }

    /// Reload tracked files from this workspace backing source.
    pub fn reload(&self) -> PyResult<Vec<Change>> {
        let updates = self.workspace.reload().map_err(to_error)?;
        let updates = updates.into_iter().map(Change::from_bridge).collect();

        Ok(updates)
    }

    /// Return one module path in the workspace.
    pub fn module(&self, path: String) -> PyResult<Module> {
        let value = self.workspace.module(path).map_err(to_error)?;

        Ok(Module::from_bridge(value.into_bridge()))
    }

    /// Return one named target in one package.
    pub fn target(
        &self,
        revision: Revision,
        package: PackageId,
        name: String,
    ) -> PyResult<TargetId> {
        let revision = repository_revision(revision)?;
        let package = package_id(package)?;
        let target = self
            .workspace
            .target(revision, package, name)
            .map_err(to_error)?;

        Ok(TargetId::from_bridge(target.into()))
    }

    /// Return the semantic profile selected by one module target name.
    pub fn profile(&self, revision: Revision, module: Module, name: String) -> PyResult<ProfileId> {
        let revision = repository_revision(revision)?;
        let module = bridge_module(module)?;
        let profile = self
            .workspace
            .profile(revision, module, name)
            .map_err(to_error)?;

        Ok(ProfileId::from_bridge(profile.into()))
    }

    /// Provide root artifacts for one immutable revision.
    pub fn provide(&self, revision: Revision, keys: Vec<ArtifactKey>) -> PyResult<()> {
        let revision = repository_revision(revision)?;
        let keys = keys
            .into_iter()
            .map(artifact_key)
            .collect::<PyResult<Vec<_>>>()?;

        self.workspace.provide(revision, keys).map_err(to_error)
    }

    /// Require one root artifact for one immutable revision.
    pub fn require(&self, revision: Revision, key: ArtifactKey) -> PyResult<ArtifactVersion> {
        let revision = repository_revision(revision)?;
        let key = artifact_key(key)?;
        let version = self.workspace.require(revision, key).map_err(to_error)?;

        Ok(ArtifactVersion::from_bridge(version.into()))
    }

    /// Return one raw artifact record for one immutable revision.
    pub fn artifact_record(
        &self,
        revision: Revision,
        key: ArtifactKey,
    ) -> PyResult<ArtifactRecord> {
        let revision = repository_revision(revision)?;
        let key = artifact_key(key)?;
        let record = self
            .workspace
            .artifact_record(revision, key)
            .map_err(to_error)?;

        Ok(ArtifactRecord::from_bridge(record))
    }

    /// Return the trace report for the latest completed artifact run.
    pub fn trace(&self, revision: Revision, detailed: bool) -> PyResult<Option<TraceReport>> {
        let revision = repository_revision(revision)?;
        let trace = self
            .workspace
            .trace(revision, detailed)
            .map_err(to_error)?
            .map(TraceReport::from_bridge);

        Ok(trace)
    }

    /// Build one typed language output for one immutable revision.
    pub fn build(&self, revision: Revision, request: BuildRequest) -> PyResult<BuildOutput> {
        let revision = repository_revision(revision)?;
        let request = build_request(request)?;
        let output = self.workspace.build(revision, request).map_err(to_error)?;

        Ok(BuildOutput::from_bridge(output))
    }

    /// Return one shared content payload by exact content id.
    pub fn content(&self, id: ContentId) -> PyResult<Content> {
        let id = content_id(id)?;
        let content = self.workspace.content(id).map_err(to_error)?;

        Ok(Content::from_bridge(content))
    }

    /// Return one text content payload by exact content id.
    pub fn text(&self, id: ContentId) -> PyResult<String> {
        let id = content_id(id)?;

        self.workspace.text(id).map_err(to_error)
    }

    /// Return one binary content payload by exact content id.
    pub fn bytes(&self, id: ContentId) -> PyResult<Vec<u8>> {
        let id = content_id(id)?;

        self.workspace.bytes(id).map_err(to_error)
    }

    /// Parse one loaded module.
    pub fn parse(&self, revision: Revision, module: Module) -> PyResult<ParseOutput> {
        let revision = repository_revision(revision)?;
        let module = bridge_module(module)?;
        let output = self.workspace.parse(revision, module).map_err(to_error)?;

        Ok(ParseOutput::from_bridge(output))
    }

    /// Return the resolved DIR artifact for one loaded module profile.
    pub fn resolve(
        &self,
        revision: Revision,
        module: Module,
        profile: ProfileId,
    ) -> PyResult<DirResolved> {
        let revision = repository_revision(revision)?;
        let module = bridge_module(module)?;
        let profile = profile_id(profile)?;
        let resolved = self
            .workspace
            .resolve(revision, module, profile)
            .map_err(to_error)?;

        Ok(DirResolved::from_bridge(resolved))
    }

    /// Check one loaded module profile.
    pub fn check(
        &self,
        revision: Revision,
        module: Module,
        profile: ProfileId,
    ) -> PyResult<CheckOutput> {
        let revision = repository_revision(revision)?;
        let module = bridge_module(module)?;
        let profile = profile_id(profile)?;
        let output = self
            .workspace
            .check(revision, module, profile)
            .map_err(to_error)?;

        Ok(CheckOutput::from_bridge(output))
    }

    /// Format one document for one immutable revision.
    pub fn format(&self, revision: Revision, request: FormatRequest) -> PyResult<FormatOutput> {
        let revision = repository_revision(revision)?;
        let request = format_request(request)?;
        let output = self.workspace.format(revision, request).map_err(to_error)?;

        Ok(FormatOutput::from_bridge(output))
    }

    /// Lint one scope for one immutable revision.
    pub fn lint(&self, revision: Revision, request: LintRequest) -> PyResult<LintOutput> {
        let revision = repository_revision(revision)?;
        let request = lint_request(request)?;
        let output = self.workspace.lint(revision, request).map_err(to_error)?;

        Ok(LintOutput::from_bridge(output))
    }

    /// Return diagnostics for one immutable revision.
    pub fn diagnostics(
        &self,
        revision: Revision,
        key: Option<ArtifactKey>,
    ) -> PyResult<Vec<Diagnostic>> {
        let revision = repository_revision(revision)?;
        let key = key.map(artifact_key).transpose()?;
        let diagnostics = self
            .workspace
            .diagnostics(revision, key)
            .map_err(to_error)?;
        let diagnostics = diagnostics
            .into_iter()
            .map(Diagnostic::from_bridge)
            .collect();

        Ok(diagnostics)
    }

    /// Return sidecars for one artifact key in one immutable revision.
    pub fn sidecars(&self, revision: Revision, key: ArtifactKey) -> PyResult<Vec<ArtifactSidecar>> {
        let revision = repository_revision(revision)?;
        let key = artifact_key(key)?;
        let sidecars = self.workspace.sidecars(revision, key).map_err(to_error)?;
        let sidecars = sidecars
            .into_iter()
            .map(ArtifactSidecar::from_bridge)
            .collect();

        Ok(sidecars)
    }
}

/// Python language session.
#[pyclass(name = "Session", module = "destack._native")]
#[derive(Debug)]
pub struct Session {
    /// Rust bridge session.
    session: rust::Session,
}

#[pymethods]
impl Session {
    /// Open one session from one source input.
    #[staticmethod]
    pub fn open(source: Source) -> PyResult<Self> {
        let session = rust::Session::open(source.into_bridge()).map_err(to_error)?;

        Ok(Self { session })
    }

    /// Return the current session revision.
    pub fn revision(&self) -> PyResult<Revision> {
        let value = self.session.revision().map_err(to_error)?;

        Ok(Revision::from_bridge(
            rust::language::Revision::from_repository(value),
        ))
    }

    /// Return editable repository file paths.
    pub fn files(&self) -> PyResult<Vec<SessionFile>> {
        let files = self.session.files().map_err(to_error)?;
        let files = files.into_iter().map(SessionFile::from_bridge).collect();

        Ok(files)
    }

    /// Edit files through the current session revision.
    pub fn edit(&self, edits: Vec<Edit>) -> PyResult<Commit> {
        let edits = edits.into_iter().map(Edit::into_bridge).collect();
        let value = self.session.edit(edits).map_err(to_error)?;

        Ok(Commit::from_bridge(value))
    }

    /// Edit files when the current revision still matches.
    pub fn edit_if_current(&self, revision: Revision, edits: Vec<Edit>) -> PyResult<Commit> {
        let revision = repository_revision(revision)?;
        let edits = edits.into_iter().map(Edit::into_bridge).collect();
        let value = self
            .session
            .edit_if_current(revision, edits)
            .map_err(to_error)?;

        Ok(Commit::from_bridge(value))
    }

    /// Reload tracked files from this session backing source.
    pub fn reload(&self) -> PyResult<Vec<Change>> {
        let updates = self.session.reload().map_err(to_error)?;
        let updates = updates.into_iter().map(Change::from_bridge).collect();

        Ok(updates)
    }

    /// Return one module path in the current session.
    pub fn module(&self, path: String) -> PyResult<Module> {
        let value = self.session.module(path).map_err(to_error)?;

        Ok(Module::from_bridge(value.into_bridge()))
    }

    /// Return one named target in one package.
    pub fn target(
        &self,
        revision: Revision,
        package: PackageId,
        name: String,
    ) -> PyResult<TargetId> {
        let revision = repository_revision(revision)?;
        let package = package_id(package)?;
        let target = self
            .session
            .target(revision, package, name)
            .map_err(to_error)?;

        Ok(TargetId::from_bridge(target.into()))
    }

    /// Return the semantic profile selected by one module target name.
    pub fn profile(&self, revision: Revision, module: Module, name: String) -> PyResult<ProfileId> {
        let revision = repository_revision(revision)?;
        let module = bridge_module(module)?;
        let profile = self
            .session
            .profile(revision, module, name)
            .map_err(to_error)?;

        Ok(ProfileId::from_bridge(profile.into()))
    }

    /// Provide root artifacts for one immutable revision.
    pub fn provide(&self, revision: Revision, keys: Vec<ArtifactKey>) -> PyResult<()> {
        let revision = repository_revision(revision)?;
        let keys = keys
            .into_iter()
            .map(artifact_key)
            .collect::<PyResult<Vec<_>>>()?;

        self.session.provide(revision, keys).map_err(to_error)
    }

    /// Require one root artifact for one immutable revision.
    pub fn require(&self, revision: Revision, key: ArtifactKey) -> PyResult<ArtifactVersion> {
        let revision = repository_revision(revision)?;
        let key = artifact_key(key)?;
        let version = self.session.require(revision, key).map_err(to_error)?;

        Ok(ArtifactVersion::from_bridge(version.into()))
    }

    /// Return one raw artifact record for one immutable revision.
    pub fn artifact_record(
        &self,
        revision: Revision,
        key: ArtifactKey,
    ) -> PyResult<ArtifactRecord> {
        let revision = repository_revision(revision)?;
        let key = artifact_key(key)?;
        let record = self
            .session
            .artifact_record(revision, key)
            .map_err(to_error)?;

        Ok(ArtifactRecord::from_bridge(record))
    }

    /// Return the trace report for the latest completed artifact run.
    pub fn trace(&self, revision: Revision, detailed: bool) -> PyResult<Option<TraceReport>> {
        let revision = repository_revision(revision)?;
        let trace = self
            .session
            .trace(revision, detailed)
            .map_err(to_error)?
            .map(TraceReport::from_bridge);

        Ok(trace)
    }

    /// Build one typed language output for one immutable revision.
    pub fn build(&self, revision: Revision, request: BuildRequest) -> PyResult<BuildOutput> {
        let revision = repository_revision(revision)?;
        let request = build_request(request)?;
        let output = self.session.build(revision, request).map_err(to_error)?;

        Ok(BuildOutput::from_bridge(output))
    }

    /// Return one shared content payload by exact content id.
    pub fn content(&self, id: ContentId) -> PyResult<Content> {
        let id = content_id(id)?;
        let content = self.session.content(id).map_err(to_error)?;

        Ok(Content::from_bridge(content))
    }

    /// Return one text content payload by exact content id.
    pub fn text(&self, id: ContentId) -> PyResult<String> {
        let id = content_id(id)?;

        self.session.text(id).map_err(to_error)
    }

    /// Return one binary content payload by exact content id.
    pub fn bytes(&self, id: ContentId) -> PyResult<Vec<u8>> {
        let id = content_id(id)?;

        self.session.bytes(id).map_err(to_error)
    }

    /// Parse one loaded module.
    pub fn parse(&self, revision: Revision, module: Module) -> PyResult<ParseOutput> {
        let revision = repository_revision(revision)?;
        let module = bridge_module(module)?;
        let output = self.session.parse(revision, module).map_err(to_error)?;

        Ok(ParseOutput::from_bridge(output))
    }

    /// Return the resolved DIR artifact for one loaded module profile.
    pub fn resolve(
        &self,
        revision: Revision,
        module: Module,
        profile: ProfileId,
    ) -> PyResult<DirResolved> {
        let revision = repository_revision(revision)?;
        let module = bridge_module(module)?;
        let profile = profile_id(profile)?;
        let resolved = self
            .session
            .resolve(revision, module, profile)
            .map_err(to_error)?;

        Ok(DirResolved::from_bridge(resolved))
    }

    /// Check one loaded module profile.
    pub fn check(
        &self,
        revision: Revision,
        module: Module,
        profile: ProfileId,
    ) -> PyResult<CheckOutput> {
        let revision = repository_revision(revision)?;
        let module = bridge_module(module)?;
        let profile = profile_id(profile)?;
        let output = self
            .session
            .check(revision, module, profile)
            .map_err(to_error)?;

        Ok(CheckOutput::from_bridge(output))
    }

    /// Format one document for one immutable revision.
    pub fn format(&self, revision: Revision, request: FormatRequest) -> PyResult<FormatOutput> {
        let revision = repository_revision(revision)?;
        let request = format_request(request)?;
        let output = self.session.format(revision, request).map_err(to_error)?;

        Ok(FormatOutput::from_bridge(output))
    }

    /// Lint one scope for one immutable revision.
    pub fn lint(&self, revision: Revision, request: LintRequest) -> PyResult<LintOutput> {
        let revision = repository_revision(revision)?;
        let request = lint_request(request)?;
        let output = self.session.lint(revision, request).map_err(to_error)?;

        Ok(LintOutput::from_bridge(output))
    }

    /// Return diagnostics for one immutable revision.
    pub fn diagnostics(
        &self,
        revision: Revision,
        key: Option<ArtifactKey>,
    ) -> PyResult<Vec<Diagnostic>> {
        let revision = repository_revision(revision)?;
        let key = key.map(artifact_key).transpose()?;
        let diagnostics = self.session.diagnostics(revision, key).map_err(to_error)?;
        let diagnostics = diagnostics
            .into_iter()
            .map(Diagnostic::from_bridge)
            .collect();

        Ok(diagnostics)
    }

    /// Return sidecars for one artifact key in one immutable revision.
    pub fn sidecars(&self, revision: Revision, key: ArtifactKey) -> PyResult<Vec<ArtifactSidecar>> {
        let revision = repository_revision(revision)?;
        let key = artifact_key(key)?;
        let sidecars = self.session.sidecars(revision, key).map_err(to_error)?;
        let sidecars = sidecars
            .into_iter()
            .map(ArtifactSidecar::from_bridge)
            .collect();

        Ok(sidecars)
    }
}

/// Return the package version.
#[pyfunction]
pub fn version() -> &'static str {
    VERSION
}

/// Load the native Python bridge module.
#[pymodule]
fn _native(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add("VERSION", VERSION)?;
    module.add_class::<Repository>()?;
    module.add_class::<Session>()?;
    module.add_class::<Workspace>()?;
    artifact::register(module)?;
    diagnostic::register(module)?;
    dir::register(module)?;
    repository::register(module)?;
    session::register(module)?;
    source::register(module)?;
    module.add_function(wrap_pyfunction!(version, module)?)?;

    Ok(())
}

/// Convert one generated revision into one repository revision.
fn repository_revision(revision: Revision) -> PyResult<rust::Revision> {
    revision.into_bridge().into_repository().map_err(to_error)
}

/// Convert one generated module into one Rust bridge module.
fn bridge_module(module: Module) -> PyResult<rust::Module> {
    let module = module.into_bridge();
    let module = rust::Module::try_from(module).map_err(to_error)?;

    Ok(module)
}

/// Convert one generated package id into one source package id.
fn package_id(package: PackageId) -> PyResult<rust::PackageId> {
    package.into_bridge().into_source().map_err(to_error)
}

/// Convert one generated profile id into one source profile id.
fn profile_id(profile: ProfileId) -> PyResult<rust::ProfileId> {
    profile.into_bridge().into_source().map_err(to_error)
}

/// Convert one generated content id into one source content id.
fn content_id(content: ContentId) -> PyResult<rust::ContentId> {
    content.into_bridge().into_source().map_err(to_error)
}

/// Convert one generated artifact key into one artifact key.
fn artifact_key(key: ArtifactKey) -> PyResult<rust::ArtifactKey> {
    key.into_bridge().into_artifact().map_err(to_error)
}

/// Convert one generated build request into one Rust build request.
fn build_request(request: BuildRequest) -> PyResult<rust::BuildRequest> {
    let request = request.into_bridge();
    let request = rust::BuildRequest::try_from(request).map_err(to_error)?;

    Ok(request)
}

/// Convert one generated format request into one Rust format request.
fn format_request(request: FormatRequest) -> PyResult<rust::FormatRequest> {
    let request = request.into_bridge();
    let request = rust::FormatRequest::try_from(request).map_err(to_error)?;

    Ok(request)
}

/// Convert one generated lint request into one Rust lint request.
fn lint_request(request: LintRequest) -> PyResult<rust::LintRequest> {
    let request = request.into_bridge();
    let request = rust::LintRequest::try_from(request).map_err(to_error)?;

    Ok(request)
}

/// Convert one Rust bridge error into one Python error.
fn to_error(error: impl ToString) -> PyErr {
    PyRuntimeError::new_err(error.to_string())
}
