// generated bridge target, do not edit

use destack_bridge_language as bridge;

use pyo3::prelude::*;

use crate::{ArtifactVersion, ContentId, FileId};

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
    pub fn get_content(&self) -> Option<ContentId> {
        match &self.value {
            bridge::ArtifactDependency::Source { content, .. } => {
                Some(ContentId::from_bridge(content.clone()))
            }
            _ => None,
        }
    }

    /// Return this payload field when present.
    #[getter]
    pub fn get_file(&self) -> Option<FileId> {
        match &self.value {
            bridge::ArtifactDependency::Source { file, .. } => {
                Some(FileId::from_bridge(file.clone()))
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
    module.add_class::<ArtifactDependency>()?;
    Ok(())
}
