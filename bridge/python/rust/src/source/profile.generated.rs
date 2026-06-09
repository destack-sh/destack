// generated bridge target, do not edit

use destack_bridge_language as bridge;

use pyo3::prelude::*;

/// External profile id crossing bridge boundaries.
#[pyclass(name = "ProfileId", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct ProfileId {
    pub(crate) value: bridge::ProfileId,
}

#[pymethods]
impl ProfileId {
    /// Create one value.
    #[new]
    pub fn new(id: String) -> Self {
        Self {
            value: bridge::ProfileId { id },
        }
    }

    /// Canonical lowercase hex profile id.
    #[getter]
    pub fn id(&self) -> String {
        self.value.id.clone()
    }
}

#[allow(dead_code)]
impl ProfileId {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::ProfileId {
        self.value
    }

    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::ProfileId) -> Self {
        Self { value }
    }
}

/// Register generated Python bridge classes.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<ProfileId>()?;
    Ok(())
}
