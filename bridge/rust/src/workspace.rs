use std::path::{Path, PathBuf};

use destack_artifact as artifact;
use destack_bridge_language as bridge;
use destack_repository as repository;
use destack_source::{ContentId, PackageId, ProfileId, TargetId};

use crate::{BuildRequest, FormatRequest, LintRequest, Module, Repository, Result, Session};

/// Tooling workspace exposed to bridge clients.
#[derive(Debug)]
pub struct Workspace {
    /// Workspace repository.
    repository: Repository,
    /// Primary workspace root.
    root: PathBuf,
    /// Real tooling workspace.
    workspace: destack_workspace::Workspace,
}

impl Workspace {
    /// Open one workspace from one source input.
    pub fn open(source: bridge::Source) -> Result<Self> {
        let repository = Repository::open(source)?;

        Self::from_repository(repository)
    }

    /// Open one workspace from one repository.
    pub fn from_repository(repository: Repository) -> Result<Self> {
        let root = repository.repository.path().to_path_buf();
        let workspace = destack_workspace::Workspace::new(
            repository.repository.clone(),
            None,
            vec![root.clone()],
            destack_session::Session::default_worker_count(),
            None,
        )
        .map_err(crate::Error::new)?;

        Ok(Self {
            repository,
            root,
            workspace,
        })
    }

    /// Return this workspace repository.
    pub fn repository(&self) -> &Repository {
        &self.repository
    }

    /// Return this workspace root path.
    pub fn root(&self) -> String {
        self.root.to_string_lossy().to_string()
    }

    /// Return the current workspace revision.
    pub fn revision(&self) -> Result<repository::Revision> {
        self.root_session()?.revision()
    }

    /// Return editable repository file paths at the current revision.
    pub fn files(&self) -> Result<Vec<bridge::SessionFile>> {
        self.root_session()?.files()
    }

    /// Edit files through the current workspace revision.
    pub fn edit(&self, edits: Vec<bridge::Edit>) -> Result<bridge::Commit> {
        self.root_session()?.edit(edits)
    }

    /// Edit files when the current workspace revision still matches.
    pub fn edit_if_current(
        &self,
        revision: repository::Revision,
        edits: Vec<bridge::Edit>,
    ) -> Result<bridge::Commit> {
        self.root_session()?.edit_if_current(revision, edits)
    }

    /// Reload tracked files from this workspace backing source.
    pub fn reload(&self) -> Result<Vec<bridge::Change>> {
        self.root_session()?.reload()
    }

    /// Return one module path in the workspace root.
    pub fn module(&self, path: impl AsRef<Path>) -> Result<Module> {
        self.root_session()?.module(path)
    }

    /// Return one named target in one package.
    pub fn target(
        &self,
        revision: repository::Revision,
        package: PackageId,
        name: impl AsRef<str>,
    ) -> Result<TargetId> {
        self.root_session()?.target(revision, package, name)
    }

    /// Return the semantic profile selected by one module target name.
    pub fn profile(
        &self,
        revision: repository::Revision,
        module: Module,
        name: impl AsRef<str>,
    ) -> Result<ProfileId> {
        self.root_session()?.profile(revision, module, name)
    }

    /// Provide root artifacts for one immutable revision.
    pub fn provide(
        &self,
        revision: repository::Revision,
        keys: Vec<artifact::ArtifactKey>,
    ) -> Result<()> {
        self.root_session()?.provide(revision, keys)
    }

    /// Require one root artifact for one immutable revision.
    pub fn require(
        &self,
        revision: repository::Revision,
        key: artifact::ArtifactKey,
    ) -> Result<artifact::ArtifactVersion> {
        self.root_session()?.require(revision, key)
    }

    /// Return one raw artifact record for one immutable revision.
    pub fn artifact_record(
        &self,
        revision: repository::Revision,
        key: artifact::ArtifactKey,
    ) -> Result<bridge::ArtifactRecord> {
        self.root_session()?.artifact_record(revision, key)
    }

    /// Return the trace report for the latest completed artifact run.
    pub fn trace(
        &self,
        revision: repository::Revision,
        detailed: bool,
    ) -> Result<Option<bridge::TraceReport>> {
        self.root_session()?.trace(revision, detailed)
    }

    /// Build one typed language output for one immutable revision.
    pub fn build(
        &self,
        revision: repository::Revision,
        request: BuildRequest,
    ) -> Result<bridge::BuildOutput> {
        self.root_session()?.build(revision, request)
    }

    /// Return one shared content payload by exact content id.
    pub fn content(&self, id: ContentId) -> Result<bridge::Content> {
        self.root_session()?.content(id)
    }

    /// Return one text content payload by exact content id.
    pub fn text(&self, id: ContentId) -> Result<String> {
        self.root_session()?.text(id)
    }

    /// Return one binary content payload by exact content id.
    pub fn bytes(&self, id: ContentId) -> Result<Vec<u8>> {
        self.root_session()?.bytes(id)
    }

    /// Parse one loaded module.
    pub fn parse(
        &self,
        revision: repository::Revision,
        module: Module,
    ) -> Result<bridge::ParseOutput> {
        self.root_session()?.parse(revision, module)
    }

    /// Return the resolved DIR artifact for one loaded module profile.
    pub fn resolve(
        &self,
        revision: repository::Revision,
        module: Module,
        profile: ProfileId,
    ) -> Result<bridge::DirResolved> {
        self.root_session()?.resolve(revision, module, profile)
    }

    /// Check one loaded module profile.
    pub fn check(
        &self,
        revision: repository::Revision,
        module: Module,
        profile: ProfileId,
    ) -> Result<bridge::CheckOutput> {
        self.root_session()?.check(revision, module, profile)
    }

    /// Format one document for one immutable revision.
    pub fn format(
        &self,
        revision: repository::Revision,
        request: FormatRequest,
    ) -> Result<bridge::FormatOutput> {
        self.root_session()?.format(revision, request)
    }

    /// Lint one scope for one immutable revision.
    pub fn lint(
        &self,
        revision: repository::Revision,
        request: LintRequest,
    ) -> Result<bridge::LintOutput> {
        self.root_session()?.lint(revision, request)
    }

    /// Return diagnostics for one immutable revision.
    pub fn diagnostics(
        &self,
        revision: repository::Revision,
        key: Option<artifact::ArtifactKey>,
    ) -> Result<Vec<bridge::Diagnostic>> {
        self.root_session()?.diagnostics(revision, key)
    }

    /// Return sidecars for one artifact key in one immutable revision.
    pub fn sidecars(
        &self,
        revision: repository::Revision,
        key: artifact::ArtifactKey,
    ) -> Result<Vec<bridge::ArtifactSidecar>> {
        self.root_session()?.sidecars(revision, key)
    }

    /// Return the primary root session.
    fn root_session(&self) -> Result<Session> {
        let session = self
            .workspace
            .session(&self.root)
            .map_err(crate::Error::new)?;

        Ok(Session::from_session(session))
    }
}
