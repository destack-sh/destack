// generated bridge target, do not edit

use destack_bridge_language as bridge;

use pyo3::prelude::*;

use crate::ModuleId;

/// One source file in an explicit source snapshot.
#[pyclass(name = "SourceFile", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct SourceFile {
    pub(crate) value: bridge::SourceFile,
}

#[pymethods]
impl SourceFile {
    /// Create one value.
    #[new]
    pub fn new(path: String, content: SourceFileContent) -> Self {
        Self {
            value: bridge::SourceFile {
                path,
                content: content.into_bridge(),
            },
        }
    }

    /// Repository-root relative path.
    #[getter]
    pub fn path(&self) -> String {
        self.value.path.clone()
    }

    /// Full source file content.
    #[getter]
    pub fn content(&self) -> SourceFileContent {
        SourceFileContent::from_bridge(self.value.content.clone())
    }

    /// Create one text source file.
    #[staticmethod]
    pub fn text(path: String, text: String) -> Self {
        Self::new(path, SourceFileContent::text(text))
    }

    /// Create one binary source file.
    #[staticmethod]
    pub fn bytes(path: String, bytes: Vec<u8>) -> Self {
        Self::new(path, SourceFileContent::bytes(bytes))
    }
}

#[allow(dead_code)]
impl SourceFile {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::SourceFile {
        self.value
    }

    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::SourceFile) -> Self {
        Self { value }
    }
}

/// Full content for one source snapshot file.
#[pyclass(name = "SourceFileContent", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct SourceFileContent {
    pub(crate) value: bridge::SourceFileContent,
}

#[pymethods]
impl SourceFileContent {
    /// Text file content.
    #[staticmethod]
    pub fn text(text: String) -> Self {
        Self {
            value: bridge::SourceFileContent::Text(text),
        }
    }

    /// Binary file content.
    #[staticmethod]
    pub fn bytes(bytes: Vec<u8>) -> Self {
        Self {
            value: bridge::SourceFileContent::Bytes(bytes),
        }
    }

    /// Return this enum variant label.
    #[getter]
    pub fn kind(&self) -> &'static str {
        match &self.value {
            bridge::SourceFileContent::Text(_) => "text",
            bridge::SourceFileContent::Bytes(_) => "bytes",
        }
    }
}

impl SourceFileContent {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::SourceFileContent {
        self.value
    }

    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::SourceFileContent) -> Self {
        Self { value }
    }
}

/// File update projected from a source update.
#[pyclass(name = "FileUpdate", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct FileUpdate {
    pub(crate) value: bridge::FileUpdate,
}

#[pymethods]
impl FileUpdate {
    /// Create one value.
    #[new]
    pub fn new(
        path: String,
        uri: String,
        kind: FileUpdateKind,
        is_removed: bool,
        module_id: Option<ModuleId>,
    ) -> Self {
        Self {
            value: bridge::FileUpdate {
                path,
                uri,
                kind: kind.into_bridge(),
                is_removed,
                module_id: module_id.map(|item| item.into_bridge()),
            },
        }
    }

    /// Repository logical path.
    #[getter]
    pub fn path(&self) -> String {
        self.value.path.clone()
    }

    /// External file URI.
    #[getter]
    pub fn uri(&self) -> String {
        self.value.uri.clone()
    }

    /// Coarse file update kind.
    #[getter]
    pub fn kind(&self) -> FileUpdateKind {
        FileUpdateKind::from_bridge(self.value.kind.clone())
    }

    /// Whether the file was removed.
    #[getter]
    pub fn is_removed(&self) -> bool {
        self.value.is_removed.clone()
    }

    /// Updated module id when known.
    #[getter]
    pub fn module_id(&self) -> Option<ModuleId> {
        self.value
            .module_id
            .clone()
            .map(|item| ModuleId::from_bridge(item))
    }
}

#[allow(dead_code)]
impl FileUpdate {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::FileUpdate {
        self.value
    }

    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::FileUpdate) -> Self {
        Self { value }
    }
}

/// One coarse kind for a file change.
#[pyclass(name = "FileUpdateKind", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct FileUpdateKind {
    pub(crate) value: bridge::FileUpdateKind,
}

#[pymethods]
impl FileUpdateKind {
    /// One ordinary source change.
    #[staticmethod]
    pub fn source() -> Self {
        Self {
            value: bridge::FileUpdateKind::Source,
        }
    }

    /// One `destack.json` change.
    #[staticmethod]
    pub fn config() -> Self {
        Self {
            value: bridge::FileUpdateKind::Config,
        }
    }

    /// Return this enum label.
    #[getter]
    pub fn label(&self) -> &'static str {
        match self.value {
            bridge::FileUpdateKind::Source => "source",
            bridge::FileUpdateKind::Config => "config",
        }
    }
}

impl FileUpdateKind {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::FileUpdateKind {
        self.value
    }

    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::FileUpdateKind) -> Self {
        Self { value }
    }
}

/// Register generated Python bridge classes.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<SourceFile>()?;
    module.add_class::<SourceFileContent>()?;
    module.add_class::<FileUpdate>()?;
    module.add_class::<FileUpdateKind>()?;
    Ok(())
}
