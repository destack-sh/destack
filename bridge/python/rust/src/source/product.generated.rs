// generated bridge target, do not edit

use destack_bridge_language as bridge;

use pyo3::prelude::*;

use crate::PackageId;

/// External product id crossing bridge boundaries.
#[pyclass(name = "ProductId", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct ProductId {
    pub(crate) value: bridge::ProductId,
}

#[pymethods]
impl ProductId {
    /// Create one value.
    #[new]
    pub fn new(package: PackageId, key: String) -> Self {
        Self {
            value: bridge::ProductId {
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

    /// Canonical lowercase hex product key within the package.
    #[getter]
    pub fn key(&self) -> String {
        self.value.key.clone()
    }
}

#[allow(dead_code)]
impl ProductId {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::ProductId {
        self.value
    }

    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::ProductId) -> Self {
        Self { value }
    }
}

/// Register generated Python bridge classes.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<ProductId>()?;
    Ok(())
}
