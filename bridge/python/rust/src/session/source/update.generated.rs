// generated bridge target, do not edit

use destack_bridge_language as bridge;

use pyo3::prelude::*;

use crate::{Change, Revision};

/// One text range in byte offsets.
#[pyclass(name = "TextRange", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct TextRange {
    pub(crate) value: bridge::TextRange,
}

#[pymethods]
impl TextRange {
    /// Create one value.
    #[new]
    pub fn new(start: u32, end: u32) -> Self {
        Self {
            value: bridge::TextRange { start, end },
        }
    }
}

#[allow(dead_code)]
impl TextRange {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::TextRange {
        self.value
    }
}

/// One text replacement.
#[pyclass(name = "TextEdit", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct TextEdit {
    pub(crate) value: bridge::TextEdit,
}

#[pymethods]
impl TextEdit {
    /// Create one value.
    #[new]
    pub fn new(range: TextRange, text: String) -> Self {
        Self {
            value: bridge::TextEdit {
                range: range.into_bridge(),
                text,
            },
        }
    }
}

#[allow(dead_code)]
impl TextEdit {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::TextEdit {
        self.value
    }
}

/// One edit accepted by a session.
#[pyclass(name = "Edit", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct Edit {
    pub(crate) value: bridge::Edit,
}

#[pymethods]
impl Edit {
    /// Replace or create one text file.
    #[staticmethod]
    pub fn set_text(path: String, text: String) -> Self {
        Self {
            value: bridge::Edit::SetText { path, text },
        }
    }

    /// Apply text replacements to one tracked text file.
    #[staticmethod]
    pub fn edit_text(path: String, edits: Vec<TextEdit>) -> Self {
        Self {
            value: bridge::Edit::EditText {
                path,
                edits: edits.into_iter().map(|item| item.into_bridge()).collect(),
            },
        }
    }

    /// Replace or create one binary file.
    #[staticmethod]
    pub fn set_bytes(path: String, bytes: Vec<u8>) -> Self {
        Self {
            value: bridge::Edit::SetBytes { path, bytes },
        }
    }

    /// Remove one file.
    #[staticmethod]
    pub fn remove(path: String) -> Self {
        Self {
            value: bridge::Edit::Remove { path },
        }
    }

    /// Move one file.
    #[staticmethod]
    pub fn move_file(from: String, to: String) -> Self {
        Self {
            value: bridge::Edit::Move { from, to },
        }
    }

    /// Return this enum variant label.
    #[getter]
    pub fn kind(&self) -> &'static str {
        match &self.value {
            bridge::Edit::SetText { .. } => "setText",
            bridge::Edit::EditText { .. } => "editText",
            bridge::Edit::SetBytes { .. } => "setBytes",
            bridge::Edit::Remove { .. } => "remove",
            bridge::Edit::Move { .. } => "move",
        }
    }
}

impl Edit {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::Edit {
        self.value
    }
}

/// One committed edit batch.
#[pyclass(name = "Commit", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct Commit {
    pub(crate) value: bridge::Commit,
}

#[pymethods]
impl Commit {
    /// Previous revision.
    #[getter]
    pub fn before(&self) -> Revision {
        Revision::from_bridge(self.value.before.clone())
    }

    /// Updated revision.
    #[getter]
    pub fn after(&self) -> Revision {
        Revision::from_bridge(self.value.after.clone())
    }

    /// Changed files.
    #[getter]
    pub fn changes(&self) -> Vec<Change> {
        self.value
            .changes
            .clone()
            .into_iter()
            .map(Change::from_bridge)
            .collect()
    }
}

#[allow(dead_code)]
impl Commit {
    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::Commit) -> Self {
        Self { value }
    }
}

/// Register generated Python bridge classes.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<TextRange>()?;
    module.add_class::<TextEdit>()?;
    module.add_class::<Edit>()?;
    module.add_class::<Commit>()?;
    Ok(())
}
