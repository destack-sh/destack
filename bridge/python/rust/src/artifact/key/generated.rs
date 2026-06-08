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
