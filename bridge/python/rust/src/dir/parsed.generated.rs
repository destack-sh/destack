// generated bridge target, do not edit

use destack_bridge_language as bridge;

use pyo3::prelude::*;

use crate::{ArtifactVersion, ModuleId};

/// Typed projection of one parsed DIR artifact.
#[pyclass(name = "DirParsed", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct DirParsed {
    pub(crate) value: bridge::DirParsed,
}

#[pymethods]
impl DirParsed {
    /// Create one value.
    #[new]
    pub fn new(version: ArtifactVersion, module: ModuleId) -> Self {
        Self {
            value: bridge::DirParsed {
                version: version.into_bridge(),
                module: module.into_bridge(),
            },
        }
    }

    /// Exact parsed artifact version.
    #[getter]
    pub fn version(&self) -> ArtifactVersion {
        ArtifactVersion::from_bridge(self.value.version.clone())
    }

    /// Parsed module id.
    #[getter]
    pub fn module(&self) -> ModuleId {
        ModuleId::from_bridge(self.value.module.clone())
    }
}

#[allow(dead_code)]
impl DirParsed {
    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::DirParsed) -> Self {
        Self { value }
    }
}

/// Register generated Python bridge classes.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<DirParsed>()?;
    Ok(())
}
