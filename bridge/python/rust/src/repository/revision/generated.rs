// generated bridge target, do not edit

use destack_bridge_language as bridge;

use pyo3::prelude::*;

/// External revision value crossing bridge boundaries.
#[pyclass(name = "Revision", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct Revision {
    pub(crate) value: bridge::Revision,
}

#[pymethods]
impl Revision {
    /// Create one value.
    #[new]
    pub fn new(id: String) -> Self {
        Self {
            value: bridge::Revision { id },
        }
    }

    /// Displayed repository revision id.
    #[getter]
    pub fn id(&self) -> String {
        self.value.id.clone()
    }
}

#[allow(dead_code)]
impl Revision {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::Revision {
        self.value
    }

    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::Revision) -> Self {
        Self { value }
    }
}

/// Register generated Python bridge classes.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<Revision>()?;
    Ok(())
}
