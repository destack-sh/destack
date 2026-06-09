// generated bridge target, do not edit

use destack_bridge_language as bridge;

use pyo3::prelude::*;

use crate::{ArtifactVersion, FileContentId, FileId};

/// Exact source path state observed by one artifact computation.
#[pyclass(name = "ArtifactPathState", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct ArtifactPathState {
    pub(crate) value: bridge::ArtifactPathState,
}

#[pymethods]
impl ArtifactPathState {
    /// The path did not exist.
    #[staticmethod]
    pub fn missing() -> Self {
        Self {
            value: bridge::ArtifactPathState::Missing,
        }
    }

    /// The path was a regular file.
    #[staticmethod]
    pub fn file() -> Self {
        Self {
            value: bridge::ArtifactPathState::File,
        }
    }

    /// The path was a directory.
    #[staticmethod]
    pub fn directory() -> Self {
        Self {
            value: bridge::ArtifactPathState::Directory,
        }
    }

    /// The path was a symbolic link.
    #[staticmethod]
    pub fn symlink() -> Self {
        Self {
            value: bridge::ArtifactPathState::Symlink,
        }
    }

    /// The path existed with another host-specific kind.
    #[staticmethod]
    pub fn other() -> Self {
        Self {
            value: bridge::ArtifactPathState::Other,
        }
    }

    /// Return this enum label.
    #[getter]
    pub fn label(&self) -> &'static str {
        match self.value {
            bridge::ArtifactPathState::Missing => "missing",
            bridge::ArtifactPathState::File => "file",
            bridge::ArtifactPathState::Directory => "directory",
            bridge::ArtifactPathState::Symlink => "symlink",
            bridge::ArtifactPathState::Other => "other",
        }
    }
}

impl ArtifactPathState {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::ArtifactPathState {
        self.value
    }

    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::ArtifactPathState) -> Self {
        Self { value }
    }
}

/// One exact directory entry observed by one artifact computation.
#[pyclass(
    name = "ArtifactDirectoryEntry",
    module = "destack._native",
    from_py_object
)]
#[derive(Debug, Clone)]
pub struct ArtifactDirectoryEntry {
    pub(crate) value: bridge::ArtifactDirectoryEntry,
}

#[pymethods]
impl ArtifactDirectoryEntry {
    /// Create one value.
    #[new]
    pub fn new(path: FileId, state: ArtifactPathState) -> Self {
        Self {
            value: bridge::ArtifactDirectoryEntry {
                path: path.into_bridge(),
                state: state.into_bridge(),
            },
        }
    }

    /// The entry path identity.
    #[getter]
    pub fn path(&self) -> FileId {
        FileId::from_bridge(self.value.path.clone())
    }

    /// The exact entry path state.
    #[getter]
    pub fn state(&self) -> ArtifactPathState {
        ArtifactPathState::from_bridge(self.value.state.clone())
    }
}

#[allow(dead_code)]
impl ArtifactDirectoryEntry {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::ArtifactDirectoryEntry {
        self.value
    }

    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::ArtifactDirectoryEntry) -> Self {
        Self { value }
    }
}

/// One primitive source observation read while building an artifact.
#[pyclass(
    name = "ArtifactSourceDependency",
    module = "destack._native",
    from_py_object
)]
#[derive(Debug, Clone)]
pub struct ArtifactSourceDependency {
    pub(crate) value: bridge::ArtifactSourceDependency,
}

#[pymethods]
impl ArtifactSourceDependency {
    /// The exact state observed for one source path.
    #[staticmethod]
    pub fn path_state(path: FileId, state: ArtifactPathState) -> Self {
        Self {
            value: bridge::ArtifactSourceDependency::PathState {
                path: path.into_bridge(),
                state: state.into_bridge(),
            },
        }
    }

    /// The exact direct entries observed for one directory.
    #[staticmethod]
    pub fn directory_entries(directory: FileId, entries: Vec<ArtifactDirectoryEntry>) -> Self {
        Self {
            value: bridge::ArtifactSourceDependency::DirectoryEntries {
                directory: directory.into_bridge(),
                entries: entries.into_iter().map(|item| item.into_bridge()).collect(),
            },
        }
    }

    /// The exact source content read for one file.
    #[staticmethod]
    pub fn file_content(file: FileId, content: FileContentId) -> Self {
        Self {
            value: bridge::ArtifactSourceDependency::FileContent {
                file: file.into_bridge(),
                content: content.into_bridge(),
            },
        }
    }

    /// Return this enum variant label.
    #[getter]
    pub fn kind(&self) -> &'static str {
        match &self.value {
            bridge::ArtifactSourceDependency::PathState { .. } => "pathState",
            bridge::ArtifactSourceDependency::DirectoryEntries { .. } => "directoryEntries",
            bridge::ArtifactSourceDependency::FileContent { .. } => "fileContent",
        }
    }

    /// Return this payload field when present.
    #[getter]
    pub fn content(&self) -> Option<FileContentId> {
        match &self.value {
            bridge::ArtifactSourceDependency::FileContent { content, .. } => {
                Some(FileContentId::from_bridge(content.clone()))
            }
            _ => None,
        }
    }

    /// Return this payload field when present.
    #[getter]
    pub fn directory(&self) -> Option<FileId> {
        match &self.value {
            bridge::ArtifactSourceDependency::DirectoryEntries { directory, .. } => {
                Some(FileId::from_bridge(directory.clone()))
            }
            _ => None,
        }
    }

    /// Return this payload field when present.
    #[getter]
    pub fn entries(&self) -> Option<Vec<ArtifactDirectoryEntry>> {
        match &self.value {
            bridge::ArtifactSourceDependency::DirectoryEntries { entries, .. } => Some(
                entries
                    .clone()
                    .into_iter()
                    .map(|item| ArtifactDirectoryEntry::from_bridge(item))
                    .collect(),
            ),
            _ => None,
        }
    }

    /// Return this payload field when present.
    #[getter]
    pub fn file(&self) -> Option<FileId> {
        match &self.value {
            bridge::ArtifactSourceDependency::FileContent { file, .. } => {
                Some(FileId::from_bridge(file.clone()))
            }
            _ => None,
        }
    }

    /// Return this payload field when present.
    #[getter]
    pub fn path(&self) -> Option<FileId> {
        match &self.value {
            bridge::ArtifactSourceDependency::PathState { path, .. } => {
                Some(FileId::from_bridge(path.clone()))
            }
            _ => None,
        }
    }

    /// Return this payload field when present.
    #[getter]
    pub fn state(&self) -> Option<ArtifactPathState> {
        match &self.value {
            bridge::ArtifactSourceDependency::PathState { state, .. } => {
                Some(ArtifactPathState::from_bridge(state.clone()))
            }
            _ => None,
        }
    }
}

impl ArtifactSourceDependency {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::ArtifactSourceDependency {
        self.value
    }

    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::ArtifactSourceDependency) -> Self {
        Self { value }
    }
}

/// One exact dependency read while building an artifact.
#[pyclass(
    name = "ArtifactDependency",
    module = "destack._native",
    from_py_object
)]
#[derive(Debug, Clone)]
pub struct ArtifactDependency {
    pub(crate) value: bridge::ArtifactDependency,
}

#[pymethods]
impl ArtifactDependency {
    /// Another exact artifact version.
    #[staticmethod]
    pub fn artifact(version: ArtifactVersion) -> Self {
        Self {
            value: bridge::ArtifactDependency::Artifact {
                version: version.into_bridge(),
            },
        }
    }

    /// One exact primitive source observation.
    #[staticmethod]
    pub fn source(dependency: ArtifactSourceDependency) -> Self {
        Self {
            value: bridge::ArtifactDependency::Source {
                dependency: dependency.into_bridge(),
            },
        }
    }

    /// Return this enum variant label.
    #[getter]
    pub fn kind(&self) -> &'static str {
        match &self.value {
            bridge::ArtifactDependency::Artifact { .. } => "artifact",
            bridge::ArtifactDependency::Source { .. } => "source",
        }
    }

    /// Return this payload field when present.
    #[getter]
    pub fn dependency(&self) -> Option<ArtifactSourceDependency> {
        match &self.value {
            bridge::ArtifactDependency::Source { dependency, .. } => {
                Some(ArtifactSourceDependency::from_bridge(dependency.clone()))
            }
            _ => None,
        }
    }

    /// Return this payload field when present.
    #[getter]
    pub fn version(&self) -> Option<ArtifactVersion> {
        match &self.value {
            bridge::ArtifactDependency::Artifact { version, .. } => {
                Some(ArtifactVersion::from_bridge(version.clone()))
            }
            _ => None,
        }
    }
}

impl ArtifactDependency {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::ArtifactDependency {
        self.value
    }

    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::ArtifactDependency) -> Self {
        Self { value }
    }
}

/// Register generated Python bridge classes.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<ArtifactPathState>()?;
    module.add_class::<ArtifactDirectoryEntry>()?;
    module.add_class::<ArtifactSourceDependency>()?;
    module.add_class::<ArtifactDependency>()?;
    Ok(())
}
