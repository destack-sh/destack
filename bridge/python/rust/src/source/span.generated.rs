// generated bridge target, do not edit

use destack_bridge_language as bridge;

use pyo3::prelude::*;

use crate::FileId;

/// Source byte span crossing bridge boundaries.
#[pyclass(name = "Span", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct Span {
    pub(crate) value: bridge::Span,
}

#[pymethods]
impl Span {
    /// Create one value.
    #[new]
    pub fn new(file: FileId, start: u32, end: u32) -> Self {
        Self {
            value: bridge::Span {
                file: file.into_bridge(),
                start,
                end,
            },
        }
    }

    /// File containing this span.
    #[getter]
    pub fn file(&self) -> FileId {
        FileId::from_bridge(self.value.file.clone())
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
impl Span {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::Span {
        self.value
    }

    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::Span) -> Self {
        Self { value }
    }
}

/// Source span with a display label.
#[pyclass(name = "LabeledSpan", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct LabeledSpan {
    pub(crate) value: bridge::LabeledSpan,
}

#[pymethods]
impl LabeledSpan {
    /// Create one value.
    #[new]
    pub fn new(span: Span, label: String) -> Self {
        Self {
            value: bridge::LabeledSpan {
                span: span.into_bridge(),
                label,
            },
        }
    }

    /// Source span.
    #[getter]
    pub fn span(&self) -> Span {
        Span::from_bridge(self.value.span.clone())
    }

    /// Display label.
    #[getter]
    pub fn label(&self) -> String {
        self.value.label.clone()
    }
}

#[allow(dead_code)]
impl LabeledSpan {}

/// Register generated Python bridge classes.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<Span>()?;
    module.add_class::<LabeledSpan>()?;
    Ok(())
}
