// generated bridge target, do not edit

use destack_bridge_language as bridge;

use pyo3::prelude::*;

use crate::SourceFile;

/// Source truth used to open a live session.
#[pyclass(name = "SourceSnapshot", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct SourceSnapshot {
    pub(crate) value: bridge::SourceSnapshot,
}

#[pymethods]
impl SourceSnapshot {
    /// Create one value.
    #[new]
    pub fn new(files: Vec<SourceFile>) -> Self {
        Self {
            value: bridge::SourceSnapshot {
                files: files.into_iter().map(|item| item.into_bridge()).collect(),
            },
        }
    }

    /// Files visible to the session source root.
    #[getter]
    pub fn files(&self) -> Vec<SourceFile> {
        self.value
            .files
            .clone()
            .into_iter()
            .map(|item| SourceFile::from_bridge(item))
            .collect()
    }
}

#[allow(dead_code)]
impl SourceSnapshot {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::SourceSnapshot {
        self.value
    }
}

/// Register generated Python bridge classes.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<SourceSnapshot>()?;
    Ok(())
}
