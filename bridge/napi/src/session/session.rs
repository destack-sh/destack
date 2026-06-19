use destack as rust;
use napi::Result;
use napi_derive::napi;

use crate::{
    ArtifactKey, ArtifactRecord, ArtifactSidecar, ArtifactVersion, BuildOutput, BuildRequest,
    Change, CheckOutput, Commit, Content, ContentId, Diagnostic, DirResolved, Edit, FormatOutput,
    FormatRequest, LintOutput, LintRequest, Module, PackageId, ParseOutput, ProfileId, Revision,
    SessionFile, Source, TargetId, TraceReport,
};

/// Durable language repository exposed to Node API bindings.
#[derive(Debug)]
#[napi]
pub struct Repository {
    /// Durable language repository.
    repository: rust::Repository,
}

#[napi]
impl Repository {
    /// Open one repository from one source input.
    #[napi(factory)]
    pub fn open(source: Source) -> Result<Self> {
        let source = source.into_bridge()?;
        let repository = rust::Repository::open(source).map_err(to_error)?;

        Ok(Self { repository })
    }

    /// Return this repository root path.
    #[napi]
    pub fn root(&self) -> String {
        self.repository.root()
    }

    /// Open one workspace over this repository.
    #[napi]
    pub fn workspace(&self) -> Result<Workspace> {
        let workspace =
            rust::Workspace::from_repository(self.repository.clone()).map_err(to_error)?;

        Ok(Workspace { workspace })
    }
}

/// Tooling workspace exposed to Node API bindings.
#[derive(Debug)]
#[napi]
pub struct Workspace {
    /// Tooling workspace.
    workspace: rust::Workspace,
}

#[napi]
impl Workspace {
    /// Open one workspace from one source input.
    #[napi(factory)]
    pub fn open(source: Source) -> Result<Self> {
        let source = source.into_bridge()?;
        let workspace = rust::Workspace::open(source).map_err(to_error)?;

        Ok(Self { workspace })
    }

    /// Return this workspace root path.
    #[napi]
    pub fn root(&self) -> String {
        self.workspace.root()
    }

    /// Return the current workspace revision.
    #[napi]
    pub fn revision(&self) -> Result<Revision> {
        let revision = self.workspace.revision().map_err(to_error)?;

        Ok(Revision::from_bridge(
            rust::language::Revision::from_repository(revision),
        ))
    }

    /// Return editable repository file paths at the current revision.
    #[napi]
    pub fn files(&self) -> Result<Vec<SessionFile>> {
        let files = self.workspace.files().map_err(to_error)?;
        let files = files.into_iter().map(SessionFile::from_bridge).collect();

        Ok(files)
    }

    /// Edit files through the current workspace revision.
    #[napi]
    pub fn edit(&self, edits: Vec<Edit>) -> Result<Commit> {
        let edits = edits
            .into_iter()
            .map(Edit::into_bridge)
            .collect::<Result<Vec<_>>>()?;
        let result = self.workspace.edit(edits).map_err(to_error)?;

        Ok(Commit::from_bridge(result))
    }

    /// Edit files when the current workspace revision still matches.
    #[napi(js_name = "editIfCurrent")]
    pub fn edit_if_current(&self, revision: Revision, edits: Vec<Edit>) -> Result<Commit> {
        let revision = repository_revision(revision)?;
        let edits = edits
            .into_iter()
            .map(Edit::into_bridge)
            .collect::<Result<Vec<_>>>()?;
        let result = self
            .workspace
            .edit_if_current(revision, edits)
            .map_err(to_error)?;

        Ok(Commit::from_bridge(result))
    }

    /// Reload tracked files from this workspace backing source.
    #[napi]
    pub fn reload(&self) -> Result<Vec<Change>> {
        let changes = self.workspace.reload().map_err(to_error)?;
        let changes = changes.into_iter().map(Change::from_bridge).collect();

        Ok(changes)
    }

    /// Return one module path in the workspace root.
    #[napi]
    pub fn module(&self, path: String) -> Result<Module> {
        let module = self.workspace.module(path).map_err(to_error)?;

        Ok(Module::from_bridge(module.into_bridge()))
    }

    /// Return one named target in one package.
    #[napi]
    pub fn target(&self, revision: Revision, package: PackageId, name: String) -> Result<TargetId> {
        let revision = repository_revision(revision)?;
        let package = package_id(package)?;
        let target = self
            .workspace
            .target(revision, package, name)
            .map_err(to_error)?;

        Ok(TargetId::from_bridge(target.into()))
    }

    /// Return the semantic profile selected by one module target name.
    #[napi]
    pub fn profile(&self, revision: Revision, module: Module, name: String) -> Result<ProfileId> {
        let revision = repository_revision(revision)?;
        let module = bridge_module(module)?;
        let profile = self
            .workspace
            .profile(revision, module, name)
            .map_err(to_error)?;

        Ok(ProfileId::from_bridge(profile.into()))
    }

    /// Provide root artifacts for one immutable revision.
    #[napi]
    pub fn provide(&self, revision: Revision, keys: Vec<ArtifactKey>) -> Result<()> {
        let revision = repository_revision(revision)?;
        let keys = keys
            .into_iter()
            .map(artifact_key)
            .collect::<Result<Vec<_>>>()?;

        self.workspace.provide(revision, keys).map_err(to_error)
    }

    /// Require one root artifact for one immutable revision.
    #[napi]
    pub fn require(&self, revision: Revision, key: ArtifactKey) -> Result<ArtifactVersion> {
        let revision = repository_revision(revision)?;
        let key = artifact_key(key)?;
        let version = self.workspace.require(revision, key).map_err(to_error)?;

        Ok(ArtifactVersion::from_bridge(version.into()))
    }

    /// Return one raw artifact record for one immutable revision.
    #[napi(js_name = "artifactRecord")]
    pub fn artifact_record(&self, revision: Revision, key: ArtifactKey) -> Result<ArtifactRecord> {
        let revision = repository_revision(revision)?;
        let key = artifact_key(key)?;
        let record = self
            .workspace
            .artifact_record(revision, key)
            .map_err(to_error)?;

        Ok(ArtifactRecord::from_bridge(record))
    }

    /// Return the trace report for the latest completed artifact run.
    #[napi]
    pub fn trace(&self, revision: Revision, detailed: bool) -> Result<Option<TraceReport>> {
        let revision = repository_revision(revision)?;
        let trace = self.workspace.trace(revision, detailed).map_err(to_error)?;
        let trace = trace.map(TraceReport::from_bridge);

        Ok(trace)
    }

    /// Build one typed language output for one immutable revision.
    #[napi]
    pub fn build(&self, revision: Revision, request: BuildRequest) -> Result<BuildOutput> {
        let revision = repository_revision(revision)?;
        let request = build_request(request)?;
        let output = self.workspace.build(revision, request).map_err(to_error)?;

        Ok(BuildOutput::from_bridge(output))
    }

    /// Return one shared content payload by exact content id.
    #[napi]
    pub fn content(&self, id: ContentId) -> Result<Content> {
        let id = content_id(id)?;
        let content = self.workspace.content(id).map_err(to_error)?;

        Ok(Content::from_bridge(content))
    }

    /// Return one text content payload by exact content id.
    #[napi]
    pub fn text(&self, id: ContentId) -> Result<String> {
        let id = content_id(id)?;

        self.workspace.text(id).map_err(to_error)
    }

    /// Return one binary content payload by exact content id.
    #[napi]
    pub fn bytes(&self, id: ContentId) -> Result<Vec<u8>> {
        let id = content_id(id)?;

        self.workspace.bytes(id).map_err(to_error)
    }

    /// Parse one loaded module.
    #[napi]
    pub fn parse(&self, revision: Revision, module: Module) -> Result<ParseOutput> {
        let revision = repository_revision(revision)?;
        let module = bridge_module(module)?;
        let output = self.workspace.parse(revision, module).map_err(to_error)?;

        Ok(ParseOutput::from_bridge(output))
    }

    /// Return the resolved DIR artifact for one loaded module profile.
    #[napi]
    pub fn resolve(
        &self,
        revision: Revision,
        module: Module,
        profile: ProfileId,
    ) -> Result<DirResolved> {
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
    #[napi]
    pub fn check(
        &self,
        revision: Revision,
        module: Module,
        profile: ProfileId,
    ) -> Result<CheckOutput> {
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
    #[napi]
    pub fn format(&self, revision: Revision, request: FormatRequest) -> Result<FormatOutput> {
        let revision = repository_revision(revision)?;
        let request = format_request(request)?;
        let output = self.workspace.format(revision, request).map_err(to_error)?;

        Ok(FormatOutput::from_bridge(output))
    }

    /// Lint one scope for one immutable revision.
    #[napi]
    pub fn lint(&self, revision: Revision, request: LintRequest) -> Result<LintOutput> {
        let revision = repository_revision(revision)?;
        let request = lint_request(request)?;
        let output = self.workspace.lint(revision, request).map_err(to_error)?;

        Ok(LintOutput::from_bridge(output))
    }

    /// Return diagnostics for one immutable revision.
    #[napi]
    pub fn diagnostics(
        &self,
        revision: Revision,
        key: Option<ArtifactKey>,
    ) -> Result<Vec<Diagnostic>> {
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
    #[napi]
    pub fn sidecars(&self, revision: Revision, key: ArtifactKey) -> Result<Vec<ArtifactSidecar>> {
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

/// Live language session exposed to Node API bindings.
#[derive(Debug)]
#[napi]
pub struct Session {
    /// Live language session.
    session: rust::Session,
}

#[napi]
impl Session {
    /// Open one session from one source input.
    #[napi(factory)]
    pub fn open(source: Source) -> Result<Self> {
        let source = source.into_bridge()?;
        let session = rust::Session::open(source).map_err(to_error)?;

        Ok(Self { session })
    }

    /// Return the current session revision.
    #[napi]
    pub fn revision(&self) -> Result<Revision> {
        let revision = self.session.revision().map_err(to_error)?;

        Ok(Revision::from_bridge(
            rust::language::Revision::from_repository(revision),
        ))
    }

    /// Return editable repository file paths at the current revision.
    #[napi]
    pub fn files(&self) -> Result<Vec<SessionFile>> {
        let files = self.session.files().map_err(to_error)?;
        let files = files.into_iter().map(SessionFile::from_bridge).collect();

        Ok(files)
    }

    /// Edit files through the default session ref.
    #[napi]
    pub fn edit(&self, edits: Vec<Edit>) -> Result<Commit> {
        let edits = edits
            .into_iter()
            .map(Edit::into_bridge)
            .collect::<Result<Vec<_>>>()?;
        let result = self.session.edit(edits).map_err(to_error)?;

        Ok(Commit::from_bridge(result))
    }

    /// Edit files when the current revision still matches.
    #[napi(js_name = "editIfCurrent")]
    pub fn edit_if_current(&self, revision: Revision, edits: Vec<Edit>) -> Result<Commit> {
        let revision = repository_revision(revision)?;
        let edits = edits
            .into_iter()
            .map(Edit::into_bridge)
            .collect::<Result<Vec<_>>>()?;
        let result = self
            .session
            .edit_if_current(revision, edits)
            .map_err(to_error)?;

        Ok(Commit::from_bridge(result))
    }

    /// Reload tracked files from this session backing source.
    #[napi]
    pub fn reload(&self) -> Result<Vec<Change>> {
        let changes = self.session.reload().map_err(to_error)?;
        let changes = changes.into_iter().map(Change::from_bridge).collect();

        Ok(changes)
    }

    /// Return one module path in the default session ref.
    #[napi]
    pub fn module(&self, path: String) -> Result<Module> {
        let module = self.session.module(path).map_err(to_error)?;

        Ok(Module::from_bridge(module.into_bridge()))
    }

    /// Return one named target in one package.
    #[napi]
    pub fn target(&self, revision: Revision, package: PackageId, name: String) -> Result<TargetId> {
        let revision = repository_revision(revision)?;
        let package = package_id(package)?;
        let target = self
            .session
            .target(revision, package, name)
            .map_err(to_error)?;

        Ok(TargetId::from_bridge(target.into()))
    }

    /// Return the semantic profile selected by one module target name.
    #[napi]
    pub fn profile(&self, revision: Revision, module: Module, name: String) -> Result<ProfileId> {
        let revision = repository_revision(revision)?;
        let module = bridge_module(module)?;
        let profile = self
            .session
            .profile(revision, module, name)
            .map_err(to_error)?;

        Ok(ProfileId::from_bridge(profile.into()))
    }

    /// Provide root artifacts for one immutable revision.
    #[napi]
    pub fn provide(&self, revision: Revision, keys: Vec<ArtifactKey>) -> Result<()> {
        let revision = repository_revision(revision)?;
        let keys = keys
            .into_iter()
            .map(artifact_key)
            .collect::<Result<Vec<_>>>()?;

        self.session.provide(revision, keys).map_err(to_error)
    }

    /// Require one root artifact for one immutable revision.
    #[napi]
    pub fn require(&self, revision: Revision, key: ArtifactKey) -> Result<ArtifactVersion> {
        let revision = repository_revision(revision)?;
        let key = artifact_key(key)?;
        let version = self.session.require(revision, key).map_err(to_error)?;

        Ok(ArtifactVersion::from_bridge(version.into()))
    }

    /// Return one raw artifact record for one immutable revision.
    #[napi(js_name = "artifactRecord")]
    pub fn artifact_record(&self, revision: Revision, key: ArtifactKey) -> Result<ArtifactRecord> {
        let revision = repository_revision(revision)?;
        let key = artifact_key(key)?;
        let record = self
            .session
            .artifact_record(revision, key)
            .map_err(to_error)?;

        Ok(ArtifactRecord::from_bridge(record))
    }

    /// Return the trace report for the latest completed artifact run.
    #[napi]
    pub fn trace(&self, revision: Revision, detailed: bool) -> Result<Option<TraceReport>> {
        let revision = repository_revision(revision)?;
        let trace = self
            .session
            .trace(revision, detailed)
            .map_err(to_error)?
            .map(TraceReport::from_bridge);

        Ok(trace)
    }

    /// Build one typed language output for one immutable revision.
    #[napi]
    pub fn build(&self, revision: Revision, request: BuildRequest) -> Result<BuildOutput> {
        let revision = repository_revision(revision)?;
        let request = build_request(request)?;
        let output = self.session.build(revision, request).map_err(to_error)?;

        Ok(BuildOutput::from_bridge(output))
    }

    /// Return one shared content payload by exact content id.
    #[napi]
    pub fn content(&self, id: ContentId) -> Result<Content> {
        let id = content_id(id)?;
        let content = self.session.content(id).map_err(to_error)?;

        Ok(Content::from_bridge(content))
    }

    /// Return one text content payload by exact content id.
    #[napi]
    pub fn text(&self, id: ContentId) -> Result<String> {
        let id = content_id(id)?;

        self.session.text(id).map_err(to_error)
    }

    /// Return one binary content payload by exact content id.
    #[napi]
    pub fn bytes(&self, id: ContentId) -> Result<Vec<u8>> {
        let id = content_id(id)?;

        self.session.bytes(id).map_err(to_error)
    }

    /// Parse one loaded module.
    #[napi]
    pub fn parse(&self, revision: Revision, module: Module) -> Result<ParseOutput> {
        let revision = repository_revision(revision)?;
        let module = bridge_module(module)?;
        let output = self.session.parse(revision, module).map_err(to_error)?;

        Ok(ParseOutput::from_bridge(output))
    }

    /// Return the resolved DIR artifact for one loaded module profile.
    #[napi]
    pub fn resolve(
        &self,
        revision: Revision,
        module: Module,
        profile: ProfileId,
    ) -> Result<DirResolved> {
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
    #[napi]
    pub fn check(
        &self,
        revision: Revision,
        module: Module,
        profile: ProfileId,
    ) -> Result<CheckOutput> {
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
    #[napi]
    pub fn format(&self, revision: Revision, request: FormatRequest) -> Result<FormatOutput> {
        let revision = repository_revision(revision)?;
        let request = format_request(request)?;
        let output = self.session.format(revision, request).map_err(to_error)?;

        Ok(FormatOutput::from_bridge(output))
    }

    /// Lint one scope for one immutable revision.
    #[napi]
    pub fn lint(&self, revision: Revision, request: LintRequest) -> Result<LintOutput> {
        let revision = repository_revision(revision)?;
        let request = lint_request(request)?;
        let output = self.session.lint(revision, request).map_err(to_error)?;

        Ok(LintOutput::from_bridge(output))
    }

    /// Return diagnostics for one immutable revision.
    #[napi]
    pub fn diagnostics(
        &self,
        revision: Revision,
        key: Option<ArtifactKey>,
    ) -> Result<Vec<Diagnostic>> {
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
    #[napi]
    pub fn sidecars(&self, revision: Revision, key: ArtifactKey) -> Result<Vec<ArtifactSidecar>> {
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

/// Convert one generated revision into one repository revision.
fn repository_revision(revision: Revision) -> Result<rust::Revision> {
    revision.into_bridge()?.into_repository().map_err(to_error)
}

/// Convert one generated module into one Rust bridge module.
fn bridge_module(module: Module) -> Result<rust::Module> {
    let module = module.into_bridge()?;
    let module = rust::Module::try_from(module).map_err(to_error)?;

    Ok(module)
}

/// Convert one generated package id into one source package id.
fn package_id(package: PackageId) -> Result<rust::PackageId> {
    package.into_bridge()?.into_source().map_err(to_error)
}

/// Convert one generated profile id into one source profile id.
fn profile_id(profile: ProfileId) -> Result<rust::ProfileId> {
    profile.into_bridge()?.into_source().map_err(to_error)
}

/// Convert one generated content id into one source content id.
fn content_id(content: ContentId) -> Result<rust::ContentId> {
    content.into_bridge()?.into_source().map_err(to_error)
}

/// Convert one generated artifact key into one artifact key.
fn artifact_key(key: ArtifactKey) -> Result<rust::ArtifactKey> {
    key.into_bridge()?.into_artifact().map_err(to_error)
}

/// Convert one generated build request into one Rust build request.
fn build_request(request: BuildRequest) -> Result<rust::BuildRequest> {
    let request = request.into_bridge()?;
    let request = rust::BuildRequest::try_from(request).map_err(to_error)?;

    Ok(request)
}

/// Convert one generated format request into one Rust format request.
fn format_request(request: FormatRequest) -> Result<rust::FormatRequest> {
    let request = request.into_bridge()?;
    let request = rust::FormatRequest::try_from(request).map_err(to_error)?;

    Ok(request)
}

/// Convert one generated lint request into one Rust lint request.
fn lint_request(request: LintRequest) -> Result<rust::LintRequest> {
    let request = request.into_bridge()?;
    let request = rust::LintRequest::try_from(request).map_err(to_error)?;

    Ok(request)
}

/// Convert one bridge error into one NAPI error.
fn to_error(error: impl ToString) -> napi::Error {
    napi::Error::from_reason(error.to_string())
}
