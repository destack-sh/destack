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
    /// File containing this span.
    #[getter]
    pub fn file(&self) -> FileId {
        FileId::from_bridge(self.value.file.clone())
    }

    /// Inclusive start byte offset.
    #[getter]
    pub fn start(&self) -> u32 {
        self.value.start
    }

    /// Exclusive end byte offset.
    #[getter]
    pub fn end(&self) -> u32 {
        self.value.end
    }
}

#[allow(dead_code)]
impl Span {
    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::Span) -> Self {
        Self { value }
    }
}

/// Register generated Python bridge classes.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<Span>()?;
    Ok(())
}
