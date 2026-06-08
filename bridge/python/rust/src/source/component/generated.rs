// generated bridge target, do not edit

use destack_bridge_language as bridge;

use pyo3::prelude::*;

/// External component id crossing bridge boundaries.
#[pyclass(name = "ComponentId", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct ComponentId {
    pub(crate) value: bridge::ComponentId,
}

#[pymethods]
impl ComponentId {
    /// Create one value.
    #[new]
    pub fn new(id: String) -> Self {
        Self {
            value: bridge::ComponentId { id },
        }
    }

    /// Canonical lowercase hex component id.
    #[getter]
    pub fn id(&self) -> String {
        self.value.id.clone()
    }
}

#[allow(dead_code)]
impl ComponentId {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::ComponentId {
        self.value
    }

    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::ComponentId) -> Self {
        Self { value }
    }
}

/// Register generated Python bridge classes.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<ComponentId>()?;
    Ok(())
}
