use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::PyModule;

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// In-process workspace protocol server exposed to Python.
#[pyclass(
    name = "LocalWorkspaceServer",
    module = "destack._native",
    skip_from_py_object
)]
#[derive(Debug, Clone)]
pub struct LocalWorkspaceServer {
    /// Local Rust server.
    server: destack::LocalWorkspaceServer,
}

#[pymethods]
impl LocalWorkspaceServer {
    /// Open an in-process workspace protocol server.
    #[staticmethod]
    pub fn open(home: String) -> PyResult<Self> {
        let server = destack::LocalWorkspaceServer::open(home).map_err(to_error)?;

        Ok(Self { server })
    }

    /// Dispatch one encoded protocol message payload.
    pub fn dispatch(&self, payload: Vec<u8>) -> PyResult<Vec<Vec<u8>>> {
        self.server.dispatch(&payload).map_err(to_error)
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
    module.add_class::<LocalWorkspaceServer>()?;
    module.add_function(wrap_pyfunction!(version, module)?)?;

    Ok(())
}

/// Convert one Rust bridge error into one Python error.
fn to_error(error: impl ToString) -> PyErr {
    PyRuntimeError::new_err(error.to_string())
}
