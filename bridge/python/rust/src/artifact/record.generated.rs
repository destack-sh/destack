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
    /// Create one value.
    #[new]
    pub fn new(id: String, text: String) -> Self {
        Self {
            value: bridge::ArtifactString { id, text },
        }
    }

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
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::ArtifactString {
        self.value
    }

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
    /// Create one value.
    #[new]
    pub fn new(
        version: ArtifactVersion,
        image: Vec<u8>,
        strings: Vec<ArtifactString>,
        dependencies: Vec<ArtifactDependency>,
        diagnostics: Vec<Diagnostic>,
        sidecars: Vec<ArtifactSidecar>,
    ) -> Self {
        Self {
            value: bridge::ArtifactRecord {
                version: version.into_bridge(),
                image,
                strings: strings.into_iter().map(|item| item.into_bridge()).collect(),
                dependencies: dependencies
                    .into_iter()
                    .map(|item| item.into_bridge())
                    .collect(),
                diagnostics: diagnostics
                    .into_iter()
                    .map(|item| item.into_bridge())
                    .collect(),
                sidecars: sidecars
                    .into_iter()
                    .map(|item| item.into_bridge())
                    .collect(),
            },
        }
    }

    /// The exact artifact version.
    #[getter]
    pub fn version(&self) -> ArtifactVersion {
        ArtifactVersion::from_bridge(self.value.version.clone())
    }

    /// Serialized artifact image bytes.
    #[getter]
    pub fn image(&self) -> Vec<u8> {
        self.value.image.clone()
    }

    /// String pool needed to interpret interned ids in the payload.
    #[getter]
    pub fn strings(&self) -> Vec<ArtifactString> {
        self.value
            .strings
            .clone()
            .into_iter()
            .map(|item| ArtifactString::from_bridge(item))
            .collect()
    }

    /// Exact artifact dependencies.
    #[getter]
    pub fn dependencies(&self) -> Vec<ArtifactDependency> {
        self.value
            .dependencies
            .clone()
            .into_iter()
            .map(|item| ArtifactDependency::from_bridge(item))
            .collect()
    }

    /// Diagnostics recorded for this artifact version.
    #[getter]
    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        self.value
            .diagnostics
            .clone()
            .into_iter()
            .map(|item| Diagnostic::from_bridge(item))
            .collect()
    }

    /// Artifact sidecars recorded for this artifact version.
    #[getter]
    pub fn sidecars(&self) -> Vec<ArtifactSidecar> {
        self.value
            .sidecars
            .clone()
            .into_iter()
            .map(|item| ArtifactSidecar::from_bridge(item))
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
