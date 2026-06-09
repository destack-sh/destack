// generated bridge target, do not edit

use destack_bridge_language as bridge;

use pyo3::prelude::*;

use crate::PackageId;

/// External target id crossing bridge boundaries.
#[pyclass(name = "TargetId", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct TargetId {
    pub(crate) value: bridge::TargetId,
}

#[pymethods]
impl TargetId {
    /// Create one value.
    #[new]
    pub fn new(package: PackageId, key: String) -> Self {
        Self {
            value: bridge::TargetId {
                package: package.into_bridge(),
                key,
            },
        }
    }

    /// Owning package.
    #[getter]
    pub fn package(&self) -> PackageId {
        PackageId::from_bridge(self.value.package.clone())
    }

    /// Canonical lowercase hex target key within the package.
    #[getter]
    pub fn key(&self) -> String {
        self.value.key.clone()
    }
}

#[allow(dead_code)]
impl TargetId {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::TargetId {
        self.value
    }

    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::TargetId) -> Self {
        Self { value }
    }
}

/// Register generated Python bridge classes.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<TargetId>()?;
    Ok(())
}
