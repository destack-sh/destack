use std::collections::HashMap;

use destack::{source, workspace};
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
#[derive(Debug)]
pub struct LocalWorkspaceServer {
    /// Local Rust server.
    server: workspace::Server,
}

/// Remote workspace protocol server exposed to Python.
#[derive(Debug)]
#[pyclass(
    name = "RemoteWorkspaceServer",
    module = "destack._native",
    skip_from_py_object
)]
pub struct RemoteWorkspaceServer {
    /// Workspace WebSocket server.
    server: workspace::WebSocketServer,
}

#[pymethods]
impl LocalWorkspaceServer {
    /// Open an in-process workspace protocol server.
    #[staticmethod]
    pub fn open(home: String) -> PyResult<Self> {
        let server = workspace::Server::open(home).map_err(to_error)?;

        Ok(Self { server })
    }

    /// Open an in-process workspace protocol server from in-memory files.
    #[staticmethod]
    pub fn memory(
        root: String,
        text_files: HashMap<String, String>,
        byte_files: HashMap<String, Vec<u8>>,
    ) -> PyResult<Self> {
        let files = text_files
            .into_iter()
            .map(|(path, text)| source::Edit::SetText {
                path: path.into(),
                text,
            })
            .chain(
                byte_files
                    .into_iter()
                    .map(|(path, bytes)| source::Edit::SetBytes {
                        path: path.into(),
                        bytes,
                    }),
            )
            .collect();
        let server = workspace::Server::memory(root, files).map_err(to_error)?;

        Ok(Self { server })
    }

    /// Dispatch one encoded protocol message payload.
    pub fn dispatch(&self, payload: Vec<u8>) -> PyResult<Vec<Vec<u8>>> {
        self.server.dispatch(&payload).map_err(to_error)
    }
}

#[pymethods]
impl RemoteWorkspaceServer {
    /// Open a remote workspace protocol server over WebSocket.
    #[staticmethod]
    pub fn open(root: String) -> PyResult<Self> {
        let server = workspace::WebSocketServer::open(root).map_err(to_error)?;

        Ok(Self { server })
    }

    /// Return the WebSocket URL for this server.
    pub fn url(&self) -> String {
        self.server.url().to_string()
    }

    /// Close this server.
    pub fn close(&mut self) -> PyResult<()> {
        self.server
            .close()
            .map_err(|_| to_error("remote workspace server thread panicked"))
    }
}

/// Return the package version.
#[pyfunction]
pub fn version() -> &'static str {
    VERSION
}

/// Load the native Python client module.
#[pymodule]
fn _native(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add("VERSION", VERSION)?;
    module.add_class::<LocalWorkspaceServer>()?;
    module.add_class::<RemoteWorkspaceServer>()?;
    module.add_function(wrap_pyfunction!(version, module)?)?;

    Ok(())
}

/// Convert one Rust client error into one Python error.
fn to_error(error: impl ToString) -> PyErr {
    PyRuntimeError::new_err(error.to_string())
}
