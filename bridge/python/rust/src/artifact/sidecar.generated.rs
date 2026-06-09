// generated bridge target, do not edit

use destack_bridge_language as bridge;

use pyo3::prelude::*;

use crate::FileContent;

/// One stable sidecar label crossing bridge boundaries.
#[pyclass(
    name = "ArtifactSidecarLabel",
    module = "destack._native",
    from_py_object
)]
#[derive(Debug, Clone)]
pub struct ArtifactSidecarLabel {
    pub(crate) value: bridge::ArtifactSidecarLabel,
}

#[pymethods]
impl ArtifactSidecarLabel {
    /// Create one value.
    #[new]
    pub fn new(key: String, value: String) -> Self {
        Self {
            value: bridge::ArtifactSidecarLabel { key, value },
        }
    }

    /// Label key.
    #[getter]
    pub fn key(&self) -> String {
        self.value.key.clone()
    }

    /// Label value.
    #[getter]
    pub fn value(&self) -> String {
        self.value.value.clone()
    }
}

#[allow(dead_code)]
impl ArtifactSidecarLabel {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::ArtifactSidecarLabel {
        self.value
    }

    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::ArtifactSidecarLabel) -> Self {
        Self { value }
    }
}

/// One named artifact sidecar crossing bridge boundaries.
#[pyclass(name = "ArtifactSidecar", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct ArtifactSidecar {
    pub(crate) value: bridge::ArtifactSidecar,
}

#[pymethods]
impl ArtifactSidecar {
    /// Create one value.
    #[new]
    pub fn new(name: String, labels: Vec<ArtifactSidecarLabel>, content: FileContent) -> Self {
        Self {
            value: bridge::ArtifactSidecar {
                name,
                labels: labels.into_iter().map(|item| item.into_bridge()).collect(),
                content: content.into_bridge(),
            },
        }
    }

    /// Sidecar name.
    #[getter]
    pub fn name(&self) -> String {
        self.value.name.clone()
    }

    /// Stable labels describing this sidecar.
    #[getter]
    pub fn labels(&self) -> Vec<ArtifactSidecarLabel> {
        self.value
            .labels
            .clone()
            .into_iter()
            .map(|item| ArtifactSidecarLabel::from_bridge(item))
            .collect()
    }

    /// Sidecar content.
    #[getter]
    pub fn content(&self) -> FileContent {
        FileContent::from_bridge(self.value.content.clone())
    }
}

#[allow(dead_code)]
impl ArtifactSidecar {
    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::ArtifactSidecar) -> Self {
        Self { value }
    }
}

/// Register generated Python bridge classes.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<ArtifactSidecarLabel>()?;
    module.add_class::<ArtifactSidecar>()?;
    Ok(())
}
