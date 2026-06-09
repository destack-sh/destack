use destack as rust;
use napi::Result;
use napi_derive::napi;

use crate::{
    ArtifactKey, ArtifactRecord, ArtifactSidecar, ArtifactVersion, Diagnostic, DirChecked,
    DirParsed, DirResolved, FileChange, FileUpdate, FileUpdateResult, Module, ProfileId, Revision,
    SessionFile, Source,
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

        Ok(Revision::from_bridge(revision))
    }

    /// Return editable repository file paths at the current revision.
    #[napi]
    pub fn files(&self) -> Result<Vec<SessionFile>> {
        let files = self.session.files().map_err(to_error)?;
        let files = files.into_iter().map(SessionFile::from_bridge).collect();

        Ok(files)
    }

    /// Apply one file update through the default session ref.
    #[napi]
    pub fn update(&self, update: FileUpdate) -> Result<FileUpdateResult> {
        let result = self
            .session
            .update(update.into_bridge()?)
            .map_err(to_error)?;

        Ok(FileUpdateResult::from_bridge(result))
    }

    /// Reload tracked files from this session backing source.
    #[napi]
    pub fn reload(&self) -> Result<Vec<FileChange>> {
        let updates = self.session.reload().map_err(to_error)?;
        let updates = updates.into_iter().map(FileChange::from_bridge).collect();

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

    /// Return one raw artifact record for one immutable revision.
    #[napi(js_name = "artifactRecord")]
    pub fn artifact_record(&self, revision: Revision, key: ArtifactKey) -> Result<ArtifactRecord> {
        let revision = revision.into_bridge()?;
        let key = key.into_bridge()?;
        let record = self
            .session
            .artifact_record(revision, key)
            .map_err(to_error)?;

        Ok(ArtifactRecord::from_bridge(record))
    }

    /// Return the parsed DIR artifact for one loaded module.
    #[napi]
    pub fn parse(&self, revision: Revision, module: Module) -> Result<DirParsed> {
        let revision = revision.into_bridge()?;
        let module = module.into_bridge()?;
        let parsed = self.session.parse(revision, module).map_err(to_error)?;

        Ok(DirParsed::from_bridge(parsed))
    }

    /// Return the resolved DIR artifact for one loaded module profile.
    #[napi]
    pub fn resolve(
        &self,
        revision: Revision,
        module: Module,
        profile: ProfileId,
    ) -> Result<DirResolved> {
        let revision = revision.into_bridge()?;
        let module = module.into_bridge()?;
        let profile = profile.into_bridge()?;
        let resolved = self
            .session
            .resolve(revision, module, profile)
            .map_err(to_error)?;

        Ok(DirResolved::from_bridge(resolved))
    }

    /// Return the checked DIR facade artifact for one loaded module profile.
    #[napi]
    pub fn check(
        &self,
        revision: Revision,
        module: Module,
        profile: ProfileId,
    ) -> Result<DirChecked> {
        let revision = revision.into_bridge()?;
        let module = module.into_bridge()?;
        let profile = profile.into_bridge()?;
        let checked = self
            .session
            .check(revision, module, profile)
            .map_err(to_error)?;

        Ok(DirChecked::from_bridge(checked))
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
