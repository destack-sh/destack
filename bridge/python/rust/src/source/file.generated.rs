// generated bridge target, do not edit

use destack_bridge_language as bridge;

use pyo3::prelude::*;

/// External file id crossing bridge boundaries.
#[pyclass(name = "FileId", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct FileId {
    pub(crate) value: bridge::FileId,
}

#[pymethods]
impl FileId {
    /// Canonical lowercase hex file id.
    #[getter]
    pub fn id(&self) -> String {
        self.value.id.clone()
    }
}

#[allow(dead_code)]
impl FileId {
    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::FileId) -> Self {
        Self { value }
    }
}

/// External content id crossing bridge boundaries.
#[pyclass(name = "ContentId", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct ContentId {
    pub(crate) value: bridge::ContentId,
}

#[pymethods]
impl ContentId {
    /// Create one value.
    #[new]
    pub fn new(id: String) -> Self {
        Self {
            value: bridge::ContentId { id },
        }
    }

    /// Canonical lowercase hex content id.
    #[getter]
    pub fn id(&self) -> String {
        self.value.id.clone()
    }
}

#[allow(dead_code)]
impl ContentId {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::ContentId {
        self.value
    }

    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::ContentId) -> Self {
        Self { value }
    }
}

/// Full content crossing bridge boundaries.
#[pyclass(name = "Content", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct Content {
    pub(crate) value: bridge::Content,
}

#[pymethods]
impl Content {
    /// Return this enum variant label.
    #[getter]
    pub fn kind(&self) -> &'static str {
        match &self.value {
            bridge::Content::Text { .. } => "text",
            bridge::Content::Binary { .. } => "binary",
        }
    }

    /// Return this payload field when present.
    #[getter]
    pub fn get_binary_content(&self) -> Option<Vec<u8>> {
        match &self.value {
            bridge::Content::Binary { content, .. } => Some(content.clone()),
            _ => None,
        }
    }

    /// Return this payload field when present.
    #[getter]
    pub fn get_text_content(&self) -> Option<String> {
        match &self.value {
            bridge::Content::Text { content, .. } => Some(content.clone()),
            _ => None,
        }
    }
}

impl Content {
    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::Content) -> Self {
        Self { value }
    }
}

/// Register generated Python bridge classes.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<FileId>()?;
    module.add_class::<ContentId>()?;
    module.add_class::<Content>()?;
    Ok(())
}
