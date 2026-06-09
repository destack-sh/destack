// generated bridge target, do not edit

use destack_bridge_language as bridge;

use pyo3::prelude::*;

use crate::ModuleId;

/// One file change observed by a session.
#[pyclass(name = "Change", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct Change {
    pub(crate) value: bridge::Change,
}

#[pymethods]
impl Change {
    /// Create one value.
    #[new]
    pub fn new(path: String, uri: String, is_removed: bool, module_id: Option<ModuleId>) -> Self {
        Self {
            value: bridge::Change {
                path,
                uri,
                is_removed,
                module_id: module_id.map(|item| item.into_bridge()),
            },
        }
    }

    /// Repository logical path.
    #[getter]
    pub fn path(&self) -> String {
        self.value.path.clone()
    }

    /// External file URI.
    #[getter]
    pub fn uri(&self) -> String {
        self.value.uri.clone()
    }

    /// Whether the file was removed.
    #[getter]
    pub fn is_removed(&self) -> bool {
        self.value.is_removed.clone()
    }

    /// Updated module id when known.
    #[getter]
    pub fn module_id(&self) -> Option<ModuleId> {
        self.value
            .module_id
            .clone()
            .map(|item| ModuleId::from_bridge(item))
    }
}

#[allow(dead_code)]
impl Change {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::Change {
        self.value
    }

    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::Change) -> Self {
        Self { value }
    }
}

/// Register generated Python bridge classes.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<Change>()?;
    Ok(())
}
