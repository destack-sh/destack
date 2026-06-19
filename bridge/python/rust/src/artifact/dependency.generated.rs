// generated bridge target, do not edit

use destack_bridge_language as bridge;

use pyo3::prelude::*;

use crate::{ArtifactVersion, ContentId, FileId};

/// Exact source path state observed by one artifact computation.
#[pyclass(name = "ArtifactPathState", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct ArtifactPathState {
    pub(crate) value: bridge::ArtifactPathState,
}

#[pymethods]
impl ArtifactPathState {
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
    /// The entry path.
    #[getter]
    pub fn path(&self) -> String {
        self.value.path.clone()
    }

    /// The exact entry path state.
    #[getter]
    pub fn state(&self) -> ArtifactPathState {
        ArtifactPathState::from_bridge(self.value.state.clone())
    }
}

#[allow(dead_code)]
impl ArtifactDirectoryEntry {
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
    pub fn get_content(&self) -> Option<ContentId> {
        match &self.value {
            bridge::ArtifactSourceDependency::FileContent { content, .. } => {
                Some(ContentId::from_bridge(content.clone()))
            }
            _ => None,
        }
    }

    /// Return this payload field when present.
    #[getter]
    pub fn get_directory(&self) -> Option<String> {
        match &self.value {
            bridge::ArtifactSourceDependency::DirectoryEntries { directory, .. } => {
                Some(directory.clone())
            }
            _ => None,
        }
    }

    /// Return this payload field when present.
    #[getter]
    pub fn get_entries(&self) -> Option<Vec<ArtifactDirectoryEntry>> {
        match &self.value {
            bridge::ArtifactSourceDependency::DirectoryEntries { entries, .. } => Some(
                entries
                    .clone()
                    .into_iter()
                    .map(ArtifactDirectoryEntry::from_bridge)
                    .collect(),
            ),
            _ => None,
        }
    }

    /// Return this payload field when present.
    #[getter]
    pub fn get_file(&self) -> Option<FileId> {
        match &self.value {
            bridge::ArtifactSourceDependency::FileContent { file, .. } => {
                Some(FileId::from_bridge(file.clone()))
            }
            _ => None,
        }
    }

    /// Return this payload field when present.
    #[getter]
    pub fn get_path(&self) -> Option<String> {
        match &self.value {
            bridge::ArtifactSourceDependency::PathState { path, .. } => Some(path.clone()),
            _ => None,
        }
    }

    /// Return this payload field when present.
    #[getter]
    pub fn get_state(&self) -> Option<ArtifactPathState> {
        match &self.value {
            bridge::ArtifactSourceDependency::PathState { state, .. } => {
                Some(ArtifactPathState::from_bridge(state.clone()))
            }
            _ => None,
        }
    }
}

impl ArtifactSourceDependency {
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
    pub fn get_dependency(&self) -> Option<ArtifactSourceDependency> {
        match &self.value {
            bridge::ArtifactDependency::Source { dependency, .. } => {
                Some(ArtifactSourceDependency::from_bridge(dependency.clone()))
            }
            _ => None,
        }
    }

    /// Return this payload field when present.
    #[getter]
    pub fn get_version(&self) -> Option<ArtifactVersion> {
        match &self.value {
            bridge::ArtifactDependency::Artifact { version, .. } => {
                Some(ArtifactVersion::from_bridge(version.clone()))
            }
            _ => None,
        }
    }
}

impl ArtifactDependency {
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
