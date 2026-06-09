// generated bridge target, do not edit

use destack_bridge_language as bridge;

use pyo3::prelude::*;

use crate::Edit;

/// Source input used to open a live session.
#[pyclass(name = "Source", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct Source {
    pub(crate) value: bridge::Source,
}

#[pymethods]
impl Source {
    /// Filesystem source rooted at a path.
    #[staticmethod]
    pub fn file_system(path: String) -> Self {
        Self {
            value: bridge::Source::FileSystem { path },
        }
    }

    /// In-memory filesystem source seeded by edits.
    #[staticmethod]
    pub fn memory(root: String, edits: Vec<Edit>) -> Self {
        Self {
            value: bridge::Source::Memory {
                root,
                edits: edits.into_iter().map(|item| item.into_bridge()).collect(),
            },
        }
    }

    /// Return this enum variant label.
    #[getter]
    pub fn kind(&self) -> &'static str {
        match &self.value {
            bridge::Source::FileSystem { .. } => "fileSystem",
            bridge::Source::Memory { .. } => "memory",
        }
    }

    /// Return this payload field when present.
    #[getter]
    pub fn edits(&self) -> Option<Vec<Edit>> {
        match &self.value {
            bridge::Source::Memory { edits, .. } => Some(
                edits
                    .clone()
                    .into_iter()
                    .map(|item| Edit::from_bridge(item))
                    .collect(),
            ),
            _ => None,
        }
    }

    /// Return this payload field when present.
    #[getter]
    pub fn path(&self) -> Option<String> {
        match &self.value {
            bridge::Source::FileSystem { path, .. } => Some(path.clone()),
            _ => None,
        }
    }

    /// Return this payload field when present.
    #[getter]
    pub fn root(&self) -> Option<String> {
        match &self.value {
            bridge::Source::Memory { root, .. } => Some(root.clone()),
            _ => None,
        }
    }
}

impl Source {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::Source {
        self.value
    }
}

/// Register generated Python bridge classes.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<Source>()?;
    Ok(())
}
