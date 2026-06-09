use destack as rust;
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::PyModule;

use crate::{
    ArtifactKey, ArtifactRecord, ArtifactSidecar, ArtifactVersion, Diagnostic, DirChecked,
    DirParsed, DirResolved, FileChange, FileUpdate, FileUpdateResult, Module, ProfileId, Revision,
    SessionFile, Source, artifact, diagnostic, dir, repository, session, source,
};

const VERSION: &str = env!("CARGO_PKG_VERSION");

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

        Ok(Revision::from_bridge(value))
    }

    /// Return editable repository file paths.
    pub fn files(&self) -> PyResult<Vec<SessionFile>> {
        let files = self.session.files().map_err(to_error)?;
        let files = files.into_iter().map(SessionFile::from_bridge).collect();

        Ok(files)
    }

    /// Apply one file update.
    pub fn update(&self, update: FileUpdate) -> PyResult<FileUpdateResult> {
        let value = self
            .session
            .update(update.into_bridge())
            .map_err(to_error)?;

        Ok(FileUpdateResult::from_bridge(value))
    }

    /// Reload tracked files from this session backing source.
    pub fn reload(&self) -> PyResult<Vec<FileChange>> {
        let updates = self.session.reload().map_err(to_error)?;
        let updates = updates.into_iter().map(FileChange::from_bridge).collect();

        Ok(updates)
    }

    /// Load one module path into the current session.
    pub fn load_module(&self, path: String) -> PyResult<Module> {
        let value = self.session.load_module(path).map_err(to_error)?;

        Ok(Module::from_bridge(value))
    }

    /// Provide root artifacts for one immutable revision.
    pub fn provide(&self, revision: Revision, keys: Vec<ArtifactKey>) -> PyResult<()> {
        let keys = keys.into_iter().map(ArtifactKey::into_bridge).collect();

        self.session
            .provide(revision.into_bridge(), keys)
            .map_err(to_error)
    }

    /// Require one root artifact for one immutable revision.
    pub fn require(&self, revision: Revision, key: ArtifactKey) -> PyResult<ArtifactVersion> {
        let version = self
            .session
            .require(revision.into_bridge(), key.into_bridge())
            .map_err(to_error)?;

        Ok(ArtifactVersion::from_bridge(version))
    }

    /// Return one raw artifact record for one immutable revision.
    pub fn artifact_record(
        &self,
        revision: Revision,
        key: ArtifactKey,
    ) -> PyResult<ArtifactRecord> {
        let record = self
            .session
            .artifact_record(revision.into_bridge(), key.into_bridge())
            .map_err(to_error)?;

        Ok(ArtifactRecord::from_bridge(record))
    }

    /// Return the parsed DIR artifact for one loaded module.
    pub fn parse(&self, revision: Revision, module: Module) -> PyResult<DirParsed> {
        let parsed = self
            .session
            .parse(revision.into_bridge(), module.into_bridge())
            .map_err(to_error)?;

        Ok(DirParsed::from_bridge(parsed))
    }

    /// Return the resolved DIR artifact for one loaded module profile.
    pub fn resolve(
        &self,
        revision: Revision,
        module: Module,
        profile: ProfileId,
    ) -> PyResult<DirResolved> {
        let resolved = self
            .session
            .resolve(
                revision.into_bridge(),
                module.into_bridge(),
                profile.into_bridge(),
            )
            .map_err(to_error)?;

        Ok(DirResolved::from_bridge(resolved))
    }

    /// Return the checked DIR facade artifact for one loaded module profile.
    pub fn check(
        &self,
        revision: Revision,
        module: Module,
        profile: ProfileId,
    ) -> PyResult<DirChecked> {
        let checked = self
            .session
            .check(
                revision.into_bridge(),
                module.into_bridge(),
                profile.into_bridge(),
            )
            .map_err(to_error)?;

        Ok(DirChecked::from_bridge(checked))
    }

    /// Return diagnostics for one immutable revision.
    pub fn diagnostics(
        &self,
        revision: Revision,
        key: Option<ArtifactKey>,
    ) -> PyResult<Vec<Diagnostic>> {
        let diagnostics = self
            .session
            .diagnostics(revision.into_bridge(), key.map(ArtifactKey::into_bridge))
            .map_err(to_error)?;
        let diagnostics = diagnostics
            .into_iter()
            .map(Diagnostic::from_bridge)
            .collect();

        Ok(diagnostics)
    }

    /// Return sidecars for one artifact key in one immutable revision.
    pub fn sidecars(&self, revision: Revision, key: ArtifactKey) -> PyResult<Vec<ArtifactSidecar>> {
        let sidecars = self
            .session
            .sidecars(revision.into_bridge(), key.into_bridge())
            .map_err(to_error)?;
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
    module.add_class::<Session>()?;
    artifact::register(module)?;
    diagnostic::register(module)?;
    dir::register(module)?;
    repository::register(module)?;
    session::register(module)?;
    source::register(module)?;
    module.add_function(wrap_pyfunction!(version, module)?)?;

    Ok(())
}

/// Convert one Rust bridge error into one Python error.
fn to_error(error: rust::Error) -> PyErr {
    PyRuntimeError::new_err(error.to_string())
}
