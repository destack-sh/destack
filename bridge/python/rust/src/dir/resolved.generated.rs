// generated bridge target, do not edit

use destack_bridge_language as bridge;

use pyo3::prelude::*;

use crate::{ArtifactVersion, ModuleId, ProfileId};

/// Typed projection of one resolved DIR artifact.
#[pyclass(name = "DirResolved", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct DirResolved {
    pub(crate) value: bridge::DirResolved,
}

#[pymethods]
impl DirResolved {
    /// Create one value.
    #[new]
    pub fn new(version: ArtifactVersion, module: ModuleId, profile: ProfileId) -> Self {
        Self {
            value: bridge::DirResolved {
                version: version.into_bridge(),
                module: module.into_bridge(),
                profile: profile.into_bridge(),
            },
        }
    }

    /// Exact resolved artifact version.
    #[getter]
    pub fn version(&self) -> ArtifactVersion {
        ArtifactVersion::from_bridge(self.value.version.clone())
    }

    /// Resolved module id.
    #[getter]
    pub fn module(&self) -> ModuleId {
        ModuleId::from_bridge(self.value.module.clone())
    }

    /// Resolved semantic profile.
    #[getter]
    pub fn profile(&self) -> ProfileId {
        ProfileId::from_bridge(self.value.profile.clone())
    }
}

#[allow(dead_code)]
impl DirResolved {
    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::DirResolved) -> Self {
        Self { value }
    }
}

/// Register generated Python bridge classes.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<DirResolved>()?;
    Ok(())
}
