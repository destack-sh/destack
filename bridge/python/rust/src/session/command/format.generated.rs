// generated bridge target, do not edit

use destack_bridge_language as bridge;

use pyo3::prelude::*;

use crate::Module;

/// One document accepted by formatter operations.
#[pyclass(name = "Document", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct Document {
    pub(crate) value: bridge::Document,
}

#[pymethods]
impl Document {
    /// Repository module at one immutable revision.
    #[staticmethod]
    pub fn module(module: Module) -> Self {
        Self {
            value: bridge::Document::Module {
                module: module.into_bridge(),
            },
        }
    }

    /// Ad hoc source text.
    #[staticmethod]
    pub fn text(path: String, text: String) -> Self {
        Self {
            value: bridge::Document::Text { path, text },
        }
    }

    /// Return this enum variant label.
    #[getter]
    pub fn kind(&self) -> &'static str {
        match &self.value {
            bridge::Document::Module { .. } => "module",
            bridge::Document::Text { .. } => "text",
        }
    }
}

impl Document {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::Document {
        self.value
    }
}

/// One formatter request.
#[pyclass(name = "FormatRequest", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct FormatRequest {
    pub(crate) value: bridge::FormatRequest,
}

#[pymethods]
impl FormatRequest {
    /// Create one value.
    #[new]
    pub fn new(document: Document) -> Self {
        Self {
            value: bridge::FormatRequest {
                document: document.into_bridge(),
            },
        }
    }
}

#[allow(dead_code)]
impl FormatRequest {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::FormatRequest {
        self.value
    }
}

/// One formatter output.
#[pyclass(name = "FormatOutput", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct FormatOutput {
    pub(crate) value: bridge::FormatOutput,
}

#[pymethods]
impl FormatOutput {
    /// Formatted source text.
    #[getter]
    pub fn text(&self) -> String {
        self.value.text.clone()
    }
}

#[allow(dead_code)]
impl FormatOutput {
    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::FormatOutput) -> Self {
        Self { value }
    }
}

/// Register generated Python bridge classes.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<Document>()?;
    module.add_class::<FormatRequest>()?;
    module.add_class::<FormatOutput>()?;
    Ok(())
}
