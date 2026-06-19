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
            .map(Replacement::from_bridge)
            .collect()
    }
}

#[allow(dead_code)]
impl FilePatch {
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
    /// Per-file edits.
    #[getter]
    pub fn files(&self) -> Vec<FilePatch> {
        self.value
            .files
            .clone()
            .into_iter()
            .map(FilePatch::from_bridge)
            .collect()
    }
}

#[allow(dead_code)]
impl BatchEdit {
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
