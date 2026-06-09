// generated bridge target, do not edit

use destack_bridge_language as bridge;

use pyo3::prelude::*;

/// One editable file visible to a live session.
#[pyclass(name = "SessionFile", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct SessionFile {
    pub(crate) value: bridge::SessionFile,
}

#[pymethods]
impl SessionFile {
    /// Create one value.
    #[new]
    pub fn new(path: String) -> Self {
        Self {
            value: bridge::SessionFile { path },
        }
    }

    /// Repository logical path.
    #[getter]
    pub fn path(&self) -> String {
        self.value.path.clone()
    }
}

#[allow(dead_code)]
impl SessionFile {
    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::SessionFile) -> Self {
        Self { value }
    }
}

/// Register generated Python bridge classes.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<SessionFile>()?;
    Ok(())
}
