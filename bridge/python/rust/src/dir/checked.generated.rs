// generated bridge target, do not edit

use destack_bridge_language as bridge;

use pyo3::prelude::*;

use crate::{ArtifactVersion, ComponentId, ModuleId, ProfileId};

/// Typed projection of one checked DIR module artifact.
#[pyclass(name = "DirChecked", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct DirChecked {
    pub(crate) value: bridge::DirChecked,
}

#[pymethods]
impl DirChecked {
    /// Create one value.
    #[new]
    pub fn new(
        version: ArtifactVersion,
        module: ModuleId,
        profile: ProfileId,
        component: ComponentId,
        entry: ModuleId,
    ) -> Self {
        Self {
            value: bridge::DirChecked {
                version: version.into_bridge(),
                module: module.into_bridge(),
                profile: profile.into_bridge(),
                component: component.into_bridge(),
                entry: entry.into_bridge(),
            },
        }
    }

    /// Exact checked facade artifact version.
    #[getter]
    pub fn version(&self) -> ArtifactVersion {
        ArtifactVersion::from_bridge(self.value.version.clone())
    }

    /// Checked module id.
    #[getter]
    pub fn module(&self) -> ModuleId {
        ModuleId::from_bridge(self.value.module.clone())
    }

    /// Checked semantic profile.
    #[getter]
    pub fn profile(&self) -> ProfileId {
        ProfileId::from_bridge(self.value.profile.clone())
    }

    /// Component that owns the checked module output.
    #[getter]
    pub fn component(&self) -> ComponentId {
        ComponentId::from_bridge(self.value.component.clone())
    }

    /// Component entry module.
    #[getter]
    pub fn entry(&self) -> ModuleId {
        ModuleId::from_bridge(self.value.entry.clone())
    }
}

#[allow(dead_code)]
impl DirChecked {
    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::DirChecked) -> Self {
        Self { value }
    }
}

/// Register generated Python bridge classes.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<DirChecked>()?;
    Ok(())
}
