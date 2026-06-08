// generated bridge target, do not edit

use destack_bridge_language as bridge;

use pyo3::prelude::*;

/// External package id crossing bridge boundaries.
#[pyclass(name = "PackageId", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct PackageId {
    pub(crate) value: bridge::PackageId,
}

#[pymethods]
impl PackageId {
    /// Create one value.
    #[new]
    pub fn new(id: String) -> Self {
        Self {
            value: bridge::PackageId { id },
        }
    }

    /// Canonical lowercase hex package id.
    #[getter]
    pub fn id(&self) -> String {
        self.value.id.clone()
    }
}

#[allow(dead_code)]
impl PackageId {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::PackageId {
        self.value
    }

    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::PackageId) -> Self {
        Self { value }
    }
}

/// Register generated Python bridge classes.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PackageId>()?;
    Ok(())
}
