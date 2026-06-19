// generated bridge target, do not edit

use destack_bridge_language as bridge;

use pyo3::prelude::*;

use crate::{ArtifactDependency, ArtifactSidecar, ArtifactVersion, Diagnostic};

/// One interned string carried by an artifact record.
#[pyclass(name = "ArtifactString", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct ArtifactString {
    pub(crate) value: bridge::ArtifactString,
}

#[pymethods]
impl ArtifactString {
    /// Canonical lowercase hex string id.
    #[getter]
    pub fn id(&self) -> String {
        self.value.id.clone()
    }

    /// Interned string text.
    #[getter]
    pub fn text(&self) -> String {
        self.value.text.clone()
    }
}

#[allow(dead_code)]
impl ArtifactString {
    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::ArtifactString) -> Self {
        Self { value }
    }
}

/// Self-contained raw artifact body crossing bridge boundaries.
#[pyclass(name = "ArtifactRecord", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct ArtifactRecord {
    pub(crate) value: bridge::ArtifactRecord,
}

#[pymethods]
impl ArtifactRecord {
    /// The exact artifact version.
    #[getter]
    pub fn version(&self) -> ArtifactVersion {
        ArtifactVersion::from_bridge(self.value.version.clone())
    }

    /// Serialized artifact payload bytes.
    #[getter]
    pub fn payload(&self) -> Vec<u8> {
        self.value.payload.clone()
    }

    /// String pool needed to interpret interned ids in the payload.
    #[getter]
    pub fn strings(&self) -> Vec<ArtifactString> {
        self.value
            .strings
            .clone()
            .into_iter()
            .map(ArtifactString::from_bridge)
            .collect()
    }

    /// Exact artifact dependencies.
    #[getter]
    pub fn dependencies(&self) -> Vec<ArtifactDependency> {
        self.value
            .dependencies
            .clone()
            .into_iter()
            .map(ArtifactDependency::from_bridge)
            .collect()
    }

    /// Diagnostics recorded for this artifact version.
    #[getter]
    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        self.value
            .diagnostics
            .clone()
            .into_iter()
            .map(Diagnostic::from_bridge)
            .collect()
    }

    /// Artifact sidecars recorded for this artifact version.
    #[getter]
    pub fn sidecars(&self) -> Vec<ArtifactSidecar> {
        self.value
            .sidecars
            .clone()
            .into_iter()
            .map(ArtifactSidecar::from_bridge)
            .collect()
    }
}

#[allow(dead_code)]
impl ArtifactRecord {
    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::ArtifactRecord) -> Self {
        Self { value }
    }
}

/// Register generated Python bridge classes.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<ArtifactString>()?;
    module.add_class::<ArtifactRecord>()?;
    Ok(())
}
