// generated bridge target, do not edit

use destack_bridge_language as bridge;

use pyo3::prelude::*;

use crate::ArtifactKey;

/// External artifact version crossing bridge boundaries.
#[pyclass(name = "ArtifactVersion", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct ArtifactVersion {
    pub(crate) value: bridge::ArtifactVersion,
}

#[pymethods]
impl ArtifactVersion {
    /// Create one value.
    #[new]
    pub fn new(key: ArtifactKey, fingerprint: String) -> Self {
        Self {
            value: bridge::ArtifactVersion {
                key: key.into_bridge(),
                fingerprint,
            },
        }
    }

    /// Semantic artifact slot.
    #[getter]
    pub fn key(&self) -> ArtifactKey {
        ArtifactKey::from_bridge(self.value.key.clone())
    }

    /// Exact semantic fingerprint.
    #[getter]
    pub fn fingerprint(&self) -> String {
        self.value.fingerprint.clone()
    }
}

#[allow(dead_code)]
impl ArtifactVersion {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::ArtifactVersion {
        self.value
    }

    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::ArtifactVersion) -> Self {
        Self { value }
    }
}

/// Register generated Python bridge classes.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<ArtifactVersion>()?;
    Ok(())
}
