// generated bridge target, do not edit

use destack_bridge_language as bridge;

use pyo3::prelude::*;

use crate::{ComponentId, ModuleId, PackageId, ProfileId, TargetId};

/// External artifact key crossing bridge boundaries.
#[pyclass(name = "ArtifactKey", module = "destack._native", from_py_object)]
#[derive(Debug, Clone)]
pub struct ArtifactKey {
    pub(crate) value: bridge::ArtifactKey,
}

#[pymethods]
impl ArtifactKey {
    /// Parsed module DIR.
    #[staticmethod]
    pub fn dir_parsed(module: ModuleId) -> Self {
        Self {
            value: bridge::ArtifactKey::DirParsed {
                module: module.into_bridge(),
            },
        }
    }

    /// Parsed non-code module data.
    #[staticmethod]
    pub fn data(module: ModuleId) -> Self {
        Self {
            value: bridge::ArtifactKey::Data {
                module: module.into_bridge(),
            },
        }
    }

    /// Explicit global environment for one profile.
    #[staticmethod]
    pub fn global_environment(profile: ProfileId) -> Self {
        Self {
            value: bridge::ArtifactKey::GlobalEnvironment {
                profile: profile.into_bridge(),
            },
        }
    }

    /// Active dependency index for one profile.
    #[staticmethod]
    pub fn dependency_index(profile: ProfileId) -> Self {
        Self {
            value: bridge::ArtifactKey::DependencyIndex {
                profile: profile.into_bridge(),
            },
        }
    }

    /// Bound DIR.
    #[staticmethod]
    pub fn dir_bound(module: ModuleId, profile: ProfileId) -> Self {
        Self {
            value: bridge::ArtifactKey::DirBound {
                module: module.into_bridge(),
                profile: profile.into_bridge(),
            },
        }
    }

    /// Imported DIR.
    #[staticmethod]
    pub fn dir_imported(module: ModuleId, profile: ProfileId) -> Self {
        Self {
            value: bridge::ArtifactKey::DirImported {
                module: module.into_bridge(),
                profile: profile.into_bridge(),
            },
        }
    }

    /// Expanded DIR.
    #[staticmethod]
    pub fn dir_expanded(module: ModuleId, profile: ProfileId) -> Self {
        Self {
            value: bridge::ArtifactKey::DirExpanded {
                module: module.into_bridge(),
                profile: profile.into_bridge(),
            },
        }
    }

    /// Exported DIR.
    #[staticmethod]
    pub fn dir_exported(module: ModuleId, profile: ProfileId) -> Self {
        Self {
            value: bridge::ArtifactKey::DirExported {
                module: module.into_bridge(),
                profile: profile.into_bridge(),
            },
        }
    }

    /// Resolved DIR imports.
    #[staticmethod]
    pub fn dir_resolved(module: ModuleId, profile: ProfileId) -> Self {
        Self {
            value: bridge::ArtifactKey::DirResolved {
                module: module.into_bridge(),
                profile: profile.into_bridge(),
            },
        }
    }

    /// Checked DIR component.
    #[staticmethod]
    pub fn dir_checked_component(
        entry: ModuleId,
        component: ComponentId,
        profile: ProfileId,
    ) -> Self {
        Self {
            value: bridge::ArtifactKey::DirCheckedComponent {
                entry: entry.into_bridge(),
                component: component.into_bridge(),
                profile: profile.into_bridge(),
            },
        }
    }

    /// Checked DIR facade.
    #[staticmethod]
    pub fn dir_checked(module: ModuleId, profile: ProfileId) -> Self {
        Self {
            value: bridge::ArtifactKey::DirChecked {
                module: module.into_bridge(),
                profile: profile.into_bridge(),
            },
        }
    }

    /// Materialized DIR.
    #[staticmethod]
    pub fn dir_materialized(module: ModuleId, profile: ProfileId) -> Self {
        Self {
            value: bridge::ArtifactKey::DirMaterialized {
                module: module.into_bridge(),
                profile: profile.into_bridge(),
            },
        }
    }

    /// Elaborated DIR.
    #[staticmethod]
    pub fn dir_elaborated(module: ModuleId, profile: ProfileId) -> Self {
        Self {
            value: bridge::ArtifactKey::DirElaborated {
                module: module.into_bridge(),
                profile: profile.into_bridge(),
            },
        }
    }

    /// Lowered MIR before optimization.
    #[staticmethod]
    pub fn mir_lowered(module: ModuleId, profile: ProfileId, target: TargetId) -> Self {
        Self {
            value: bridge::ArtifactKey::MirLowered {
                module: module.into_bridge(),
                profile: profile.into_bridge(),
                target: target.into_bridge(),
            },
        }
    }

    /// Verified MIR after required semantic verification.
    #[staticmethod]
    pub fn mir_verified(module: ModuleId, profile: ProfileId, target: TargetId) -> Self {
        Self {
            value: bridge::ArtifactKey::MirVerified {
                module: module.into_bridge(),
                profile: profile.into_bridge(),
                target: target.into_bridge(),
            },
        }
    }

    /// Optimized MIR.
    #[staticmethod]
    pub fn mir_optimized(module: ModuleId, profile: ProfileId, target: TargetId) -> Self {
        Self {
            value: bridge::ArtifactKey::MirOptimized {
                module: module.into_bridge(),
                profile: profile.into_bridge(),
                target: target.into_bridge(),
            },
        }
    }

    /// Query index for one module profile.
    #[staticmethod]
    pub fn module_query_index(module: ModuleId, profile: ProfileId) -> Self {
        Self {
            value: bridge::ArtifactKey::ModuleQueryIndex {
                module: module.into_bridge(),
                profile: profile.into_bridge(),
            },
        }
    }

    /// Query index for one workspace profile.
    #[staticmethod]
    pub fn workspace_query_index(profile: ProfileId) -> Self {
        Self {
            value: bridge::ArtifactKey::WorkspaceQueryIndex {
                profile: profile.into_bridge(),
            },
        }
    }

    /// One generated module output for one target.
    #[staticmethod]
    pub fn module_output(module: ModuleId, target: TargetId) -> Self {
        Self {
            value: bridge::ArtifactKey::ModuleOutput {
                module: module.into_bridge(),
                target: target.into_bridge(),
            },
        }
    }

    /// Output entries for one package target.
    #[staticmethod]
    pub fn package_output(package: PackageId, target: TargetId) -> Self {
        Self {
            value: bridge::ArtifactKey::PackageOutput {
                package: package.into_bridge(),
                target: target.into_bridge(),
            },
        }
    }

    /// Realized lint diagnostics for one module profile.
    #[staticmethod]
    pub fn module_linted(module: ModuleId, profile: ProfileId) -> Self {
        Self {
            value: bridge::ArtifactKey::ModuleLinted {
                module: module.into_bridge(),
                profile: profile.into_bridge(),
            },
        }
    }

    /// Realized lint diagnostics for one package.
    #[staticmethod]
    pub fn package_linted(package: PackageId) -> Self {
        Self {
            value: bridge::ArtifactKey::PackageLinted {
                package: package.into_bridge(),
            },
        }
    }

    /// Realized lint diagnostics for the workspace.
    #[staticmethod]
    pub fn workspace_linted() -> Self {
        Self {
            value: bridge::ArtifactKey::WorkspaceLinted,
        }
    }

    /// Return this enum variant label.
    #[getter]
    pub fn kind(&self) -> &'static str {
        match &self.value {
            bridge::ArtifactKey::DirParsed { .. } => "dirParsed",
            bridge::ArtifactKey::Data { .. } => "data",
            bridge::ArtifactKey::GlobalEnvironment { .. } => "globalEnvironment",
            bridge::ArtifactKey::DependencyIndex { .. } => "dependencyIndex",
            bridge::ArtifactKey::DirBound { .. } => "dirBound",
            bridge::ArtifactKey::DirImported { .. } => "dirImported",
            bridge::ArtifactKey::DirExpanded { .. } => "dirExpanded",
            bridge::ArtifactKey::DirExported { .. } => "dirExported",
            bridge::ArtifactKey::DirResolved { .. } => "dirResolved",
            bridge::ArtifactKey::DirCheckedComponent { .. } => "dirCheckedComponent",
            bridge::ArtifactKey::DirChecked { .. } => "dirChecked",
            bridge::ArtifactKey::DirMaterialized { .. } => "dirMaterialized",
            bridge::ArtifactKey::DirElaborated { .. } => "dirElaborated",
            bridge::ArtifactKey::MirLowered { .. } => "mirLowered",
            bridge::ArtifactKey::MirVerified { .. } => "mirVerified",
            bridge::ArtifactKey::MirOptimized { .. } => "mirOptimized",
            bridge::ArtifactKey::ModuleQueryIndex { .. } => "moduleQueryIndex",
            bridge::ArtifactKey::WorkspaceQueryIndex { .. } => "workspaceQueryIndex",
            bridge::ArtifactKey::ModuleOutput { .. } => "moduleOutput",
            bridge::ArtifactKey::PackageOutput { .. } => "packageOutput",
            bridge::ArtifactKey::ModuleLinted { .. } => "moduleLinted",
            bridge::ArtifactKey::PackageLinted { .. } => "packageLinted",
            bridge::ArtifactKey::WorkspaceLinted => "workspaceLinted",
        }
    }

    /// Return this payload field when present.
    #[getter]
    pub fn component(&self) -> Option<ComponentId> {
        match &self.value {
            bridge::ArtifactKey::DirCheckedComponent { component, .. } => {
                Some(ComponentId::from_bridge(component.clone()))
            }
            _ => None,
        }
    }

    /// Return this payload field when present.
    #[getter]
    pub fn entry(&self) -> Option<ModuleId> {
        match &self.value {
            bridge::ArtifactKey::DirCheckedComponent { entry, .. } => {
                Some(ModuleId::from_bridge(entry.clone()))
            }
            _ => None,
        }
    }

    /// Return this payload field when present.
    #[getter]
    pub fn module(&self) -> Option<ModuleId> {
        match &self.value {
            bridge::ArtifactKey::DirParsed { module, .. } => {
                Some(ModuleId::from_bridge(module.clone()))
            }
            bridge::ArtifactKey::Data { module, .. } => Some(ModuleId::from_bridge(module.clone())),
            bridge::ArtifactKey::DirBound { module, .. } => {
                Some(ModuleId::from_bridge(module.clone()))
            }
            bridge::ArtifactKey::DirImported { module, .. } => {
                Some(ModuleId::from_bridge(module.clone()))
            }
            bridge::ArtifactKey::DirExpanded { module, .. } => {
                Some(ModuleId::from_bridge(module.clone()))
            }
            bridge::ArtifactKey::DirExported { module, .. } => {
                Some(ModuleId::from_bridge(module.clone()))
            }
            bridge::ArtifactKey::DirResolved { module, .. } => {
                Some(ModuleId::from_bridge(module.clone()))
            }
            bridge::ArtifactKey::DirChecked { module, .. } => {
                Some(ModuleId::from_bridge(module.clone()))
            }
            bridge::ArtifactKey::DirMaterialized { module, .. } => {
                Some(ModuleId::from_bridge(module.clone()))
            }
            bridge::ArtifactKey::DirElaborated { module, .. } => {
                Some(ModuleId::from_bridge(module.clone()))
            }
            bridge::ArtifactKey::MirLowered { module, .. } => {
                Some(ModuleId::from_bridge(module.clone()))
            }
            bridge::ArtifactKey::MirVerified { module, .. } => {
                Some(ModuleId::from_bridge(module.clone()))
            }
            bridge::ArtifactKey::MirOptimized { module, .. } => {
                Some(ModuleId::from_bridge(module.clone()))
            }
            bridge::ArtifactKey::ModuleQueryIndex { module, .. } => {
                Some(ModuleId::from_bridge(module.clone()))
            }
            bridge::ArtifactKey::ModuleOutput { module, .. } => {
                Some(ModuleId::from_bridge(module.clone()))
            }
            bridge::ArtifactKey::ModuleLinted { module, .. } => {
                Some(ModuleId::from_bridge(module.clone()))
            }
            _ => None,
        }
    }

    /// Return this payload field when present.
    #[getter]
    pub fn package(&self) -> Option<PackageId> {
        match &self.value {
            bridge::ArtifactKey::PackageOutput { package, .. } => {
                Some(PackageId::from_bridge(package.clone()))
            }
            bridge::ArtifactKey::PackageLinted { package, .. } => {
                Some(PackageId::from_bridge(package.clone()))
            }
            _ => None,
        }
    }

    /// Return this payload field when present.
    #[getter]
    pub fn profile(&self) -> Option<ProfileId> {
        match &self.value {
            bridge::ArtifactKey::GlobalEnvironment { profile, .. } => {
                Some(ProfileId::from_bridge(profile.clone()))
            }
            bridge::ArtifactKey::DependencyIndex { profile, .. } => {
                Some(ProfileId::from_bridge(profile.clone()))
            }
            bridge::ArtifactKey::DirBound { profile, .. } => {
                Some(ProfileId::from_bridge(profile.clone()))
            }
            bridge::ArtifactKey::DirImported { profile, .. } => {
                Some(ProfileId::from_bridge(profile.clone()))
            }
            bridge::ArtifactKey::DirExpanded { profile, .. } => {
                Some(ProfileId::from_bridge(profile.clone()))
            }
            bridge::ArtifactKey::DirExported { profile, .. } => {
                Some(ProfileId::from_bridge(profile.clone()))
            }
            bridge::ArtifactKey::DirResolved { profile, .. } => {
                Some(ProfileId::from_bridge(profile.clone()))
            }
            bridge::ArtifactKey::DirCheckedComponent { profile, .. } => {
                Some(ProfileId::from_bridge(profile.clone()))
            }
            bridge::ArtifactKey::DirChecked { profile, .. } => {
                Some(ProfileId::from_bridge(profile.clone()))
            }
            bridge::ArtifactKey::DirMaterialized { profile, .. } => {
                Some(ProfileId::from_bridge(profile.clone()))
            }
            bridge::ArtifactKey::DirElaborated { profile, .. } => {
                Some(ProfileId::from_bridge(profile.clone()))
            }
            bridge::ArtifactKey::MirLowered { profile, .. } => {
                Some(ProfileId::from_bridge(profile.clone()))
            }
            bridge::ArtifactKey::MirVerified { profile, .. } => {
                Some(ProfileId::from_bridge(profile.clone()))
            }
            bridge::ArtifactKey::MirOptimized { profile, .. } => {
                Some(ProfileId::from_bridge(profile.clone()))
            }
            bridge::ArtifactKey::ModuleQueryIndex { profile, .. } => {
                Some(ProfileId::from_bridge(profile.clone()))
            }
            bridge::ArtifactKey::WorkspaceQueryIndex { profile, .. } => {
                Some(ProfileId::from_bridge(profile.clone()))
            }
            bridge::ArtifactKey::ModuleLinted { profile, .. } => {
                Some(ProfileId::from_bridge(profile.clone()))
            }
            _ => None,
        }
    }

    /// Return this payload field when present.
    #[getter]
    pub fn target(&self) -> Option<TargetId> {
        match &self.value {
            bridge::ArtifactKey::MirLowered { target, .. } => {
                Some(TargetId::from_bridge(target.clone()))
            }
            bridge::ArtifactKey::MirVerified { target, .. } => {
                Some(TargetId::from_bridge(target.clone()))
            }
            bridge::ArtifactKey::MirOptimized { target, .. } => {
                Some(TargetId::from_bridge(target.clone()))
            }
            bridge::ArtifactKey::ModuleOutput { target, .. } => {
                Some(TargetId::from_bridge(target.clone()))
            }
            bridge::ArtifactKey::PackageOutput { target, .. } => {
                Some(TargetId::from_bridge(target.clone()))
            }
            _ => None,
        }
    }
}

impl ArtifactKey {
    /// Convert this Python value into one bridge value.
    pub(crate) fn into_bridge(self) -> bridge::ArtifactKey {
        self.value
    }

    /// Convert one bridge value into one Python value.
    pub(crate) fn from_bridge(value: bridge::ArtifactKey) -> Self {
        Self { value }
    }
}

/// Register generated Python bridge classes.
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<ArtifactKey>()?;
    Ok(())
}
