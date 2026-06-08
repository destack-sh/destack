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
    /// Create one value.
    #[new]
    pub fn new(id: String) -> Self {
        Self {
            value: bridge::FileId { id },
        }
    }

    /// Canonical lowercase hex file id.
    #[getter]
    pub fn id(&self) -> String {
        self.value.id.clone()
    }
}

#[allow(dead_code)]
impl FileId {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::FileId {
        self.value
    }

    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::FileId) -> Self {
        Self { value }
    }
}

/// External file content id crossing bridge boundaries.
#[pyclass(name = "FileContentId", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct FileContentId {
    pub(crate) value: bridge::FileContentId,
}

#[pymethods]
impl FileContentId {
    /// Create one value.
    #[new]
    pub fn new(id: String) -> Self {
        Self {
            value: bridge::FileContentId { id },
        }
    }

    /// Canonical lowercase hex file content id.
    #[getter]
    pub fn id(&self) -> String {
        self.value.id.clone()
    }
}

#[allow(dead_code)]
impl FileContentId {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::FileContentId {
        self.value
    }

    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::FileContentId) -> Self {
        Self { value }
    }
}

/// Full file content crossing bridge boundaries.
#[pyclass(name = "FileContent", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct FileContent {
    pub(crate) value: bridge::FileContent,
}

#[pymethods]
impl FileContent {
    /// Text file content.
    #[staticmethod]
    pub fn text(content: String) -> Self {
        Self {
            value: bridge::FileContent::Text { content },
        }
    }

    /// Binary file content.
    #[staticmethod]
    pub fn binary(content: Vec<u8>) -> Self {
        Self {
            value: bridge::FileContent::Binary { content },
        }
    }

    /// Return this enum variant label.
    #[getter]
    pub fn kind(&self) -> &'static str {
        match &self.value {
            bridge::FileContent::Text { .. } => "text",
            bridge::FileContent::Binary { .. } => "binary",
        }
    }
}

impl FileContent {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::FileContent {
        self.value
    }

    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::FileContent) -> Self {
        Self { value }
    }
}

/// Register generated Python bridge classes.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<FileId>()?;
    module.add_class::<FileContentId>()?;
    module.add_class::<FileContent>()?;
    Ok(())
}
