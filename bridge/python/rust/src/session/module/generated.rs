// generated bridge target, do not edit

use destack_bridge_language as bridge;

use pyo3::prelude::*;

use crate::ModuleId;

/// One module loaded through a live session.
#[pyclass(name = "Module", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct Module {
    pub(crate) value: bridge::Module,
}

#[pymethods]
impl Module {
    /// Create one value.
    #[new]
    pub fn new(id: ModuleId) -> Self {
        Self {
            value: bridge::Module {
                id: id.into_bridge(),
            },
        }
    }

    /// Stable source module id.
    #[getter]
    pub fn id(&self) -> ModuleId {
        ModuleId::from_bridge(self.value.id.clone())
    }
}

#[allow(dead_code)]
impl Module {
    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::Module) -> Self {
        Self { value }
    }
}

/// Register generated Python bridge classes.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<Module>()?;
    Ok(())
}
