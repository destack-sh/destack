use destack as rust;
use napi::Result;
use napi_derive::napi;

use crate::{
    ArtifactKey, ArtifactSidecar, ArtifactVersion, Diagnostic, FileUpdate, Module, Revision,
    SessionFile, SourceSnapshot, SourceUpdate, SourceUpdateResult,
};

/// Live language session exposed to Node API bindings.
#[derive(Debug)]
#[napi]
pub struct Session {
    /// Live language session.
    session: rust::Session,
}

#[napi]
impl Session {
    /// Open one session from an explicit source snapshot.
    #[napi(factory)]
    pub fn open_source(root: String, source: SourceSnapshot) -> Result<Self> {
        let source = source.into_bridge()?;
        let session = rust::Session::open_source(root, source).map_err(to_error)?;

        Ok(Self { session })
    }

    /// Open one session from a native filesystem path.
    #[napi(factory)]
    pub fn open_path(path: String) -> Result<Self> {
        let session = rust::Session::open_path(path).map_err(to_error)?;

        Ok(Self { session })
    }

    /// Return the current session revision.
    #[napi]
    pub fn revision(&self) -> Result<Revision> {
        let revision = self.session.revision().map_err(to_error)?;

        Ok(Revision::from_bridge(revision))
    }

    /// Return editable repository file paths at the current revision.
    #[napi]
    pub fn files(&self) -> Result<Vec<SessionFile>> {
        let files = self.session.files().map_err(to_error)?;
        let files = files.into_iter().map(SessionFile::from_bridge).collect();

        Ok(files)
    }

    /// Apply one source update through the default session ref.
    #[napi]
    pub fn update(&self, update: SourceUpdate) -> Result<SourceUpdateResult> {
        let result = self
            .session
            .update(update.into_bridge()?)
            .map_err(to_error)?;

        Ok(SourceUpdateResult::from_bridge(result))
    }

    /// Reload tracked files from this session filesystem.
    #[napi]
    pub fn reload(&self) -> Result<Vec<FileUpdate>> {
        let updates = self.session.reload().map_err(to_error)?;
        let updates = updates.into_iter().map(FileUpdate::from_bridge).collect();

        Ok(updates)
    }

    /// Load one module path into the default session ref.
    #[napi]
    pub fn load_module(&self, path: String) -> Result<Module> {
        let module = self.session.load_module(path).map_err(to_error)?;

        Ok(Module::from_bridge(module))
    }

    /// Provide root artifacts for one immutable revision.
    #[napi]
    pub fn provide(&self, revision: Revision, keys: Vec<ArtifactKey>) -> Result<()> {
        let revision = revision.into_bridge()?;
        let keys = keys
            .into_iter()
            .map(ArtifactKey::into_bridge)
            .collect::<Result<Vec<_>>>()?;

        self.session.provide(revision, keys).map_err(to_error)
    }

    /// Require one root artifact for one immutable revision.
    #[napi]
    pub fn require(&self, revision: Revision, key: ArtifactKey) -> Result<ArtifactVersion> {
        let revision = revision.into_bridge()?;
        let key = key.into_bridge()?;
        let version = self.session.require(revision, key).map_err(to_error)?;

        Ok(ArtifactVersion::from_bridge(version))
    }

    /// Return diagnostics for one immutable revision.
    #[napi]
    pub fn diagnostics(
        &self,
        revision: Revision,
        key: Option<ArtifactKey>,
    ) -> Result<Vec<Diagnostic>> {
        let revision = revision.into_bridge()?;
        let key = key.map(ArtifactKey::into_bridge).transpose()?;
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
        let revision = revision.into_bridge()?;
        let key = key.into_bridge()?;
        let sidecars = self.session.sidecars(revision, key).map_err(to_error)?;
        let sidecars = sidecars
            .into_iter()
            .map(ArtifactSidecar::from_bridge)
            .collect();

        Ok(sidecars)
    }
}

/// Convert one bridge error into one NAPI error.
fn to_error(error: impl ToString) -> napi::Error {
    napi::Error::from_reason(error.to_string())
}
