// generated bridge target, do not edit

use destack_bridge_language as bridge;

use pyo3::prelude::*;

use crate::{FileChange, Revision};

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

    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::TextEdit) -> Self {
        Self { value }
    }
}

/// One file edit accepted by a session update.
#[pyclass(name = "FileEdit", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct FileEdit {
    pub(crate) value: bridge::FileEdit,
}

#[pymethods]
impl FileEdit {
    /// Replace or create one text file.
    #[staticmethod]
    pub fn set_text(path: String, text: String) -> Self {
        Self {
            value: bridge::FileEdit::SetText { path, text },
        }
    }

    /// Apply text replacements to one tracked text file.
    #[staticmethod]
    pub fn edit_text(path: String, edits: Vec<TextEdit>) -> Self {
        Self {
            value: bridge::FileEdit::EditText {
                path,
                edits: edits.into_iter().map(|item| item.into_bridge()).collect(),
            },
        }
    }

    /// Replace or create one binary file.
    #[staticmethod]
    pub fn set_bytes(path: String, bytes: Vec<u8>) -> Self {
        Self {
            value: bridge::FileEdit::SetBytes { path, bytes },
        }
    }

    /// Remove one file.
    #[staticmethod]
    pub fn remove(path: String) -> Self {
        Self {
            value: bridge::FileEdit::Remove { path },
        }
    }

    /// Move one file.
    #[staticmethod]
    pub fn move_file(from: String, to: String) -> Self {
        Self {
            value: bridge::FileEdit::Move { from, to },
        }
    }

    /// Return this enum variant label.
    #[getter]
    pub fn kind(&self) -> &'static str {
        match &self.value {
            bridge::FileEdit::SetText { .. } => "setText",
            bridge::FileEdit::EditText { .. } => "editText",
            bridge::FileEdit::SetBytes { .. } => "setBytes",
            bridge::FileEdit::Remove { .. } => "remove",
            bridge::FileEdit::Move { .. } => "move",
        }
    }

    /// Return this payload field when present.
    #[getter]
    pub fn bytes(&self) -> Option<Vec<u8>> {
        match &self.value {
            bridge::FileEdit::SetBytes { bytes, .. } => Some(bytes.clone()),
            _ => None,
        }
    }

    /// Return this payload field when present.
    #[getter]
    pub fn edits(&self) -> Option<Vec<TextEdit>> {
        match &self.value {
            bridge::FileEdit::EditText { edits, .. } => Some(
                edits
                    .clone()
                    .into_iter()
                    .map(|item| TextEdit::from_bridge(item))
                    .collect(),
            ),
            _ => None,
        }
    }

    /// Return this payload field when present.
    #[getter]
    pub fn from(&self) -> Option<String> {
        match &self.value {
            bridge::FileEdit::Move { from, .. } => Some(from.clone()),
            _ => None,
        }
    }

    /// Return this payload field when present.
    #[getter]
    pub fn path(&self) -> Option<String> {
        match &self.value {
            bridge::FileEdit::SetText { path, .. } => Some(path.clone()),
            bridge::FileEdit::EditText { path, .. } => Some(path.clone()),
            bridge::FileEdit::SetBytes { path, .. } => Some(path.clone()),
            bridge::FileEdit::Remove { path, .. } => Some(path.clone()),
            _ => None,
        }
    }

    /// Return this payload field when present.
    #[getter]
    pub fn text(&self) -> Option<String> {
        match &self.value {
            bridge::FileEdit::SetText { text, .. } => Some(text.clone()),
            _ => None,
        }
    }

    /// Return this payload field when present.
    #[getter]
    pub fn to(&self) -> Option<String> {
        match &self.value {
            bridge::FileEdit::Move { to, .. } => Some(to.clone()),
            _ => None,
        }
    }
}

impl FileEdit {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::FileEdit {
        self.value
    }

    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::FileEdit) -> Self {
        Self { value }
    }
}

/// One file update applied through one session ref.
#[pyclass(name = "FileUpdate", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct FileUpdate {
    pub(crate) value: bridge::FileUpdate,
}

#[pymethods]
impl FileUpdate {
    /// Create one value.
    #[new]
    pub fn new(base: Option<Revision>, edits: Vec<FileEdit>) -> Self {
        Self {
            value: bridge::FileUpdate {
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

    /// File edits in this atomic update.
    #[getter]
    pub fn edits(&self) -> Vec<FileEdit> {
        self.value
            .edits
            .clone()
            .into_iter()
            .map(|item| FileEdit::from_bridge(item))
            .collect()
    }
}

#[allow(dead_code)]
impl FileUpdate {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::FileUpdate {
        self.value
    }
}

/// File update result.
#[pyclass(name = "FileUpdateResult", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct FileUpdateResult {
    pub(crate) value: bridge::FileUpdateResult,
}

#[pymethods]
impl FileUpdateResult {
    /// Create one value.
    #[new]
    pub fn new(before: Revision, after: Revision, files: Vec<FileChange>) -> Self {
        Self {
            value: bridge::FileUpdateResult {
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
    pub fn files(&self) -> Vec<FileChange> {
        self.value
            .files
            .clone()
            .into_iter()
            .map(|item| FileChange::from_bridge(item))
            .collect()
    }
}

#[allow(dead_code)]
impl FileUpdateResult {
    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::FileUpdateResult) -> Self {
        Self { value }
    }
}

/// Register generated Python bridge classes.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<TextRange>()?;
    module.add_class::<TextEdit>()?;
    module.add_class::<FileEdit>()?;
    module.add_class::<FileUpdate>()?;
    module.add_class::<FileUpdateResult>()?;
    Ok(())
}
