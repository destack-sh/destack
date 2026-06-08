use destack as rust;
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::PyModule;

use crate::{
    ArtifactKey, ArtifactSidecar, ArtifactVersion, Diagnostic, FileUpdate, Module, Revision,
    SessionFile, SourceSnapshot, SourceUpdate, SourceUpdateResult, artifact, diagnostic,
    repository, session, source,
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
    /// Open one session from a native filesystem path.
    #[staticmethod]
    pub fn open_path(path: String) -> PyResult<Self> {
        let session = rust::Session::open_path(path).map_err(to_error)?;

        Ok(Self { session })
    }

    /// Open one session from an explicit source snapshot.
    #[staticmethod]
    pub fn open_source(root: String, source: SourceSnapshot) -> PyResult<Self> {
        let session = rust::Session::open_source(root, source.into_bridge()).map_err(to_error)?;

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

    /// Apply one source update.
    pub fn update(&self, update: SourceUpdate) -> PyResult<SourceUpdateResult> {
        let value = self
            .session
            .update(update.into_bridge())
            .map_err(to_error)?;

        Ok(SourceUpdateResult::from_bridge(value))
    }

    /// Reload tracked files from this session source.
    pub fn reload(&self) -> PyResult<Vec<FileUpdate>> {
        let updates = self.session.reload().map_err(to_error)?;
        let updates = updates.into_iter().map(FileUpdate::from_bridge).collect();

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
