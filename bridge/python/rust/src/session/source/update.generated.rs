// generated bridge target, do not edit

use destack_bridge_language as bridge;

use pyo3::prelude::*;

use crate::{FileUpdate, Revision};

/// Source text range in byte offsets.
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

    /// Inclusive start byte offset.
    #[getter]
    pub fn start(&self) -> u32 {
        self.value.start.clone()
    }

    /// Exclusive end byte offset.
    #[getter]
    pub fn end(&self) -> u32 {
        self.value.end.clone()
    }
}

#[allow(dead_code)]
impl TextRange {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::TextRange {
        self.value
    }

    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::TextRange) -> Self {
        Self { value }
    }
}

/// Source text replacement.
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

    /// Replaced byte range.
    #[getter]
    pub fn range(&self) -> TextRange {
        TextRange::from_bridge(self.value.range.clone())
    }

    /// Replacement text.
    #[getter]
    pub fn text(&self) -> String {
        self.value.text.clone()
    }
}

#[allow(dead_code)]
impl TextEdit {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::TextEdit {
        self.value
    }
}

/// One source edit accepted by a session update.
#[pyclass(name = "SourceEdit", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct SourceEdit {
    pub(crate) value: bridge::SourceEdit,
}

#[pymethods]
impl SourceEdit {
    /// Replace or create one text file.
    #[staticmethod]
    pub fn set_text(path: String, text: String) -> Self {
        Self {
            value: bridge::SourceEdit::SetText { path, text },
        }
    }

    /// Apply text replacements to one tracked text file.
    #[staticmethod]
    pub fn edit_text(path: String, edits: Vec<TextEdit>) -> Self {
        Self {
            value: bridge::SourceEdit::EditText {
                path,
                edits: edits.into_iter().map(|item| item.into_bridge()).collect(),
            },
        }
    }

    /// Replace or create one binary file.
    #[staticmethod]
    pub fn set_bytes(path: String, bytes: Vec<u8>) -> Self {
        Self {
            value: bridge::SourceEdit::SetBytes { path, bytes },
        }
    }

    /// Remove one file.
    #[staticmethod]
    pub fn remove(path: String) -> Self {
        Self {
            value: bridge::SourceEdit::Remove { path },
        }
    }

    /// Move one file.
    #[staticmethod]
    pub fn move_file(from: String, to: String) -> Self {
        Self {
            value: bridge::SourceEdit::Move { from, to },
        }
    }

    /// Return this enum variant label.
    #[getter]
    pub fn kind(&self) -> &'static str {
        match &self.value {
            bridge::SourceEdit::SetText { .. } => "setText",
            bridge::SourceEdit::EditText { .. } => "editText",
            bridge::SourceEdit::SetBytes { .. } => "setBytes",
            bridge::SourceEdit::Remove { .. } => "remove",
            bridge::SourceEdit::Move { .. } => "move",
        }
    }
}

impl SourceEdit {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::SourceEdit {
        self.value
    }

    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::SourceEdit) -> Self {
        Self { value }
    }
}

/// Source update applied through one session ref.
#[pyclass(name = "SourceUpdate", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct SourceUpdate {
    pub(crate) value: bridge::SourceUpdate,
}

#[pymethods]
impl SourceUpdate {
    /// Create one value.
    #[new]
    pub fn new(base: Option<Revision>, edits: Vec<SourceEdit>) -> Self {
        Self {
            value: bridge::SourceUpdate {
                base: base.map(|item| item.into_bridge()),
                edits: edits.into_iter().map(|item| item.into_bridge()).collect(),
            },
        }
    }

    /// Expected base revision.
    #[getter]
    pub fn base(&self) -> Option<Revision> {
        self.value
            .base
            .clone()
            .map(|item| Revision::from_bridge(item))
    }

    /// Source edits in this atomic update.
    #[getter]
    pub fn edits(&self) -> Vec<SourceEdit> {
        self.value
            .edits
            .clone()
            .into_iter()
            .map(|item| SourceEdit::from_bridge(item))
            .collect()
    }
}

#[allow(dead_code)]
impl SourceUpdate {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::SourceUpdate {
        self.value
    }
}

/// Source update result.
#[pyclass(
    name = "SourceUpdateResult",
    module = "destack._native",
    from_py_object
)]
#[derive(Debug, Clone)]
pub struct SourceUpdateResult {
    pub(crate) value: bridge::SourceUpdateResult,
}

#[pymethods]
impl SourceUpdateResult {
    /// Create one value.
    #[new]
    pub fn new(before: Revision, after: Revision, files: Vec<FileUpdate>) -> Self {
        Self {
            value: bridge::SourceUpdateResult {
                before: before.into_bridge(),
                after: after.into_bridge(),
                files: files.into_iter().map(|item| item.into_bridge()).collect(),
            },
        }
    }

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
    pub fn files(&self) -> Vec<FileUpdate> {
        self.value
            .files
            .clone()
            .into_iter()
            .map(|item| FileUpdate::from_bridge(item))
            .collect()
    }
}

#[allow(dead_code)]
impl SourceUpdateResult {
    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::SourceUpdateResult) -> Self {
        Self { value }
    }
}

/// Register generated Python bridge classes.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<TextRange>()?;
    module.add_class::<TextEdit>()?;
    module.add_class::<SourceEdit>()?;
    module.add_class::<SourceUpdate>()?;
    module.add_class::<SourceUpdateResult>()?;
    Ok(())
}
