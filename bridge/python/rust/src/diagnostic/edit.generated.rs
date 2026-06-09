// generated bridge target, do not edit

use destack_bridge_language as bridge;

use pyo3::prelude::*;

use crate::{FileId, Span};

/// One source replacement crossing bridge boundaries.
#[pyclass(name = "Replacement", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct Replacement {
    pub(crate) value: bridge::Replacement,
}

#[pymethods]
impl Replacement {
    /// Create one value.
    #[new]
    pub fn new(span: Span, new_text: String) -> Self {
        Self {
            value: bridge::Replacement {
                span: span.into_bridge(),
                new_text,
            },
        }
    }

    /// Source span to replace.
    #[getter]
    pub fn span(&self) -> Span {
        Span::from_bridge(self.value.span.clone())
    }

    /// Replacement text.
    #[getter]
    pub fn new_text(&self) -> String {
        self.value.new_text.clone()
    }
}

#[allow(dead_code)]
impl Replacement {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::Replacement {
        self.value
    }

    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::Replacement) -> Self {
        Self { value }
    }
}

/// Edits for a single file.
#[pyclass(name = "FilePatch", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct FilePatch {
    pub(crate) value: bridge::FilePatch,
}

#[pymethods]
impl FilePatch {
    /// Create one value.
    #[new]
    pub fn new(file: FileId, replacements: Vec<Replacement>) -> Self {
        Self {
            value: bridge::FilePatch {
                file: file.into_bridge(),
                replacements: replacements
                    .into_iter()
                    .map(|item| item.into_bridge())
                    .collect(),
            },
        }
    }

    /// Edited file.
    #[getter]
    pub fn file(&self) -> FileId {
        FileId::from_bridge(self.value.file.clone())
    }

    /// Source replacements.
    #[getter]
    pub fn replacements(&self) -> Vec<Replacement> {
        self.value
            .replacements
            .clone()
            .into_iter()
            .map(|item| Replacement::from_bridge(item))
            .collect()
    }
}

#[allow(dead_code)]
impl FilePatch {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::FilePatch {
        self.value
    }

    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::FilePatch) -> Self {
        Self { value }
    }
}

/// Edits across multiple files.
#[pyclass(name = "BatchEdit", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct BatchEdit {
    pub(crate) value: bridge::BatchEdit,
}

#[pymethods]
impl BatchEdit {
    /// Create one value.
    #[new]
    pub fn new(files: Vec<FilePatch>) -> Self {
        Self {
            value: bridge::BatchEdit {
                files: files.into_iter().map(|item| item.into_bridge()).collect(),
            },
        }
    }

    /// Per-file edits.
    #[getter]
    pub fn files(&self) -> Vec<FilePatch> {
        self.value
            .files
            .clone()
            .into_iter()
            .map(|item| FilePatch::from_bridge(item))
            .collect()
    }
}

#[allow(dead_code)]
impl BatchEdit {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::BatchEdit {
        self.value
    }

    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::BatchEdit) -> Self {
        Self { value }
    }
}

/// Register generated Python bridge classes.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<Replacement>()?;
    module.add_class::<FilePatch>()?;
    module.add_class::<BatchEdit>()?;
    Ok(())
}
