// generated bridge target, do not edit

use destack_bridge_language as bridge;

use pyo3::prelude::*;

use crate::ModuleId;

/// Observed file change projected from a file update.
#[pyclass(name = "FileChange", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct FileChange {
    pub(crate) value: bridge::FileChange,
}

#[pymethods]
impl FileChange {
    /// Create one value.
    #[new]
    pub fn new(
        path: String,
        uri: String,
        kind: FileChangeKind,
        is_removed: bool,
        module_id: Option<ModuleId>,
    ) -> Self {
        Self {
            value: bridge::FileChange {
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

    /// Coarse file change kind.
    #[getter]
    pub fn kind(&self) -> FileChangeKind {
        FileChangeKind::from_bridge(self.value.kind.clone())
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
impl FileChange {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::FileChange {
        self.value
    }

    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::FileChange) -> Self {
        Self { value }
    }
}

/// One coarse kind for a file change.
#[pyclass(name = "FileChangeKind", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct FileChangeKind {
    pub(crate) value: bridge::FileChangeKind,
}

#[pymethods]
impl FileChangeKind {
    /// One ordinary source change.
    #[staticmethod]
    pub fn source() -> Self {
        Self {
            value: bridge::FileChangeKind::Source,
        }
    }

    /// One `destack.json` change.
    #[staticmethod]
    pub fn config() -> Self {
        Self {
            value: bridge::FileChangeKind::Config,
        }
    }

    /// Return this enum label.
    #[getter]
    pub fn label(&self) -> &'static str {
        match self.value {
            bridge::FileChangeKind::Source => "source",
            bridge::FileChangeKind::Config => "config",
        }
    }
}

impl FileChangeKind {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::FileChangeKind {
        self.value
    }

    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::FileChangeKind) -> Self {
        Self { value }
    }
}

/// Register generated Python bridge classes.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<FileChange>()?;
    module.add_class::<FileChangeKind>()?;
    Ok(())
}
