use std::fmt::{self, Display, Formatter};

use destack_artifact as artifact;

use crate::{ComponentId, ModuleId, PackageId, ProfileId, SourceIdParseError, TargetId, bridge};

/// External artifact key crossing bridge boundaries.
#[bridge(capi_handle)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ArtifactKey {
    /// Parsed module DIR.
    DirParsed {
        /// Source module.
        module: ModuleId,
    },
    /// Parsed non-code module data.
    Data {
        /// Source module.
        module: ModuleId,
    },
    /// Explicit global environment for one profile.
    GlobalEnvironment {
        /// Semantic profile.
        profile: ProfileId,
    },
    /// Active dependency index for one profile.
    DependencyIndex {
        /// Semantic profile.
        profile: ProfileId,
    },
    /// Bound DIR.
    DirBound {
        /// Source module.
        module: ModuleId,
        /// Semantic profile.
        profile: ProfileId,
    },
    /// Imported DIR.
    DirImported {
        /// Source module.
        module: ModuleId,
        /// Semantic profile.
        profile: ProfileId,
    },
    /// Expanded DIR.
    DirExpanded {
        /// Source module.
        module: ModuleId,
        /// Semantic profile.
        profile: ProfileId,
    },
    /// Exported DIR.
    DirExported {
        /// Source module.
        module: ModuleId,
        /// Semantic profile.
        profile: ProfileId,
    },
    /// Resolved DIR imports.
    DirResolved {
        /// Source module.
        module: ModuleId,
        /// Semantic profile.
        profile: ProfileId,
    },
    /// Checked DIR component.
    DirCheckedComponent {
        /// Component entry module.
        entry: ModuleId,
        /// Checked component id.
        component: ComponentId,
        /// Semantic profile.
        profile: ProfileId,
    },
    /// Checked DIR facade.
    DirChecked {
        /// Source module.
        module: ModuleId,
        /// Semantic profile.
        profile: ProfileId,
    },
    /// Materialized DIR.
    DirMaterialized {
        /// Source module.
        module: ModuleId,
        /// Semantic profile.
        profile: ProfileId,
    },
    /// Elaborated DIR.
    DirElaborated {
        /// Source module.
        module: ModuleId,
        /// Semantic profile.
        profile: ProfileId,
    },
    /// Lowered MIR before optimization.
    MirLowered {
        /// Source module.
        module: ModuleId,
        /// Semantic profile.
        profile: ProfileId,
        /// Build target.
        target: TargetId,
    },
    /// Verified MIR after required semantic verification.
    MirVerified {
        /// Source module.
        module: ModuleId,
        /// Semantic profile.
        profile: ProfileId,
        /// Build target.
        target: TargetId,
    },
    /// Optimized MIR.
    MirOptimized {
        /// Source module.
        module: ModuleId,
        /// Semantic profile.
        profile: ProfileId,
        /// Build target.
        target: TargetId,
    },
    /// Query index for one module profile.
    ModuleQueryIndex {
        /// Source module.
        module: ModuleId,
        /// Semantic profile.
        profile: ProfileId,
    },
    /// Query index for one workspace profile.
    WorkspaceQueryIndex {
        /// Semantic profile.
        profile: ProfileId,
    },
    /// One generated module output for one target.
    ModuleOutput {
        /// Source module.
        module: ModuleId,
        /// Build target.
        target: TargetId,
    },
    /// Output entries for one package target.
    PackageOutput {
        /// Source package.
        package: PackageId,
        /// Build target.
        target: TargetId,
    },
    /// Realized lint diagnostics for one module profile.
    ModuleLinted {
        /// Source module.
        module: ModuleId,
        /// Semantic profile.
        profile: ProfileId,
    },
    /// Realized lint diagnostics for one package.
    PackageLinted {
        /// Source package.
        package: PackageId,
    },
    /// Realized lint diagnostics for the workspace.
    WorkspaceLinted,
}

impl ArtifactKey {
    /// Convert one artifact key into one bridge artifact key.
    pub fn from_artifact(key: artifact::ArtifactKey) -> Self {
        match key {
            artifact::ArtifactKey::DirParsed { module } => Self::DirParsed {
                module: module.into(),
            },
            artifact::ArtifactKey::Data { module } => Self::Data {
                module: module.into(),
            },
            artifact::ArtifactKey::GlobalEnvironment { profile } => Self::GlobalEnvironment {
                profile: profile.into(),
            },
            artifact::ArtifactKey::DependencyIndex { profile } => Self::DependencyIndex {
                profile: profile.into(),
            },
            artifact::ArtifactKey::DirBound { module, profile } => Self::DirBound {
                module: module.into(),
                profile: profile.into(),
            },
            artifact::ArtifactKey::DirImported { module, profile } => Self::DirImported {
                module: module.into(),
                profile: profile.into(),
            },
            artifact::ArtifactKey::DirExpanded { module, profile } => Self::DirExpanded {
                module: module.into(),
                profile: profile.into(),
            },
            artifact::ArtifactKey::DirExported { module, profile } => Self::DirExported {
                module: module.into(),
                profile: profile.into(),
            },
            artifact::ArtifactKey::DirResolved { module, profile } => Self::DirResolved {
                module: module.into(),
                profile: profile.into(),
            },
            artifact::ArtifactKey::DirCheckedComponent {
                entry,
                component,
                profile,
            } => Self::DirCheckedComponent {
                entry: entry.into(),
                component: component.into(),
                profile: profile.into(),
            },
            artifact::ArtifactKey::DirChecked { module, profile } => Self::DirChecked {
                module: module.into(),
                profile: profile.into(),
            },
            artifact::ArtifactKey::DirMaterialized { module, profile } => Self::DirMaterialized {
                module: module.into(),
                profile: profile.into(),
            },
            artifact::ArtifactKey::DirElaborated { module, profile } => Self::DirElaborated {
                module: module.into(),
                profile: profile.into(),
            },
            artifact::ArtifactKey::MirLowered {
                module,
                profile,
                target,
            } => Self::MirLowered {
                module: module.into(),
                profile: profile.into(),
                target: target.into(),
            },
            artifact::ArtifactKey::MirVerified {
                module,
                profile,
                target,
            } => Self::MirVerified {
                module: module.into(),
                profile: profile.into(),
                target: target.into(),
            },
            artifact::ArtifactKey::MirOptimized {
                module,
                profile,
                target,
            } => Self::MirOptimized {
                module: module.into(),
                profile: profile.into(),
                target: target.into(),
            },
            artifact::ArtifactKey::ModuleQueryIndex { module, profile } => Self::ModuleQueryIndex {
                module: module.into(),
                profile: profile.into(),
            },
            artifact::ArtifactKey::WorkspaceQueryIndex { profile } => Self::WorkspaceQueryIndex {
                profile: profile.into(),
            },
            artifact::ArtifactKey::ModuleOutput { module, target } => Self::ModuleOutput {
                module: module.into(),
                target: target.into(),
            },
            artifact::ArtifactKey::PackageOutput { package, target } => Self::PackageOutput {
                package: package.into(),
                target: target.into(),
            },
            artifact::ArtifactKey::ModuleLinted { module, profile } => Self::ModuleLinted {
                module: module.into(),
                profile: profile.into(),
            },
            artifact::ArtifactKey::PackageLinted { package } => Self::PackageLinted {
                package: package.into(),
            },
            artifact::ArtifactKey::WorkspaceLinted => Self::WorkspaceLinted,
        }
    }

    /// Convert this bridge artifact key into one artifact key.
    pub fn into_artifact(self) -> Result<artifact::ArtifactKey, ArtifactBridgeError> {
        match self {
            Self::DirParsed { module } => Ok(artifact::ArtifactKey::DirParsed {
                module: module.into_source()?,
            }),
            Self::Data { module } => Ok(artifact::ArtifactKey::Data {
                module: module.into_source()?,
            }),
            Self::GlobalEnvironment { profile } => Ok(artifact::ArtifactKey::GlobalEnvironment {
                profile: profile.into_source()?,
            }),
            Self::DependencyIndex { profile } => Ok(artifact::ArtifactKey::DependencyIndex {
                profile: profile.into_source()?,
            }),
            Self::DirBound { module, profile } => Ok(artifact::ArtifactKey::DirBound {
                module: module.into_source()?,
                profile: profile.into_source()?,
            }),
            Self::DirImported { module, profile } => Ok(artifact::ArtifactKey::DirImported {
                module: module.into_source()?,
                profile: profile.into_source()?,
            }),
            Self::DirExpanded { module, profile } => Ok(artifact::ArtifactKey::DirExpanded {
                module: module.into_source()?,
                profile: profile.into_source()?,
            }),
            Self::DirExported { module, profile } => Ok(artifact::ArtifactKey::DirExported {
                module: module.into_source()?,
                profile: profile.into_source()?,
            }),
            Self::DirResolved { module, profile } => Ok(artifact::ArtifactKey::DirResolved {
                module: module.into_source()?,
                profile: profile.into_source()?,
            }),
            Self::DirCheckedComponent {
                entry,
                component,
                profile,
            } => Ok(artifact::ArtifactKey::DirCheckedComponent {
                entry: entry.into_source()?,
                component: component.into_source()?,
                profile: profile.into_source()?,
            }),
            Self::DirChecked { module, profile } => Ok(artifact::ArtifactKey::DirChecked {
                module: module.into_source()?,
                profile: profile.into_source()?,
            }),
            Self::DirMaterialized { module, profile } => {
                Ok(artifact::ArtifactKey::DirMaterialized {
                    module: module.into_source()?,
                    profile: profile.into_source()?,
                })
            }
            Self::DirElaborated { module, profile } => Ok(artifact::ArtifactKey::DirElaborated {
                module: module.into_source()?,
                profile: profile.into_source()?,
            }),
            Self::MirLowered {
                module,
                profile,
                target,
            } => Ok(artifact::ArtifactKey::MirLowered {
                module: module.into_source()?,
                profile: profile.into_source()?,
                target: target.into_source()?,
            }),
            Self::MirVerified {
                module,
                profile,
                target,
            } => Ok(artifact::ArtifactKey::MirVerified {
                module: module.into_source()?,
                profile: profile.into_source()?,
                target: target.into_source()?,
            }),
            Self::MirOptimized {
                module,
                profile,
                target,
            } => Ok(artifact::ArtifactKey::MirOptimized {
                module: module.into_source()?,
                profile: profile.into_source()?,
                target: target.into_source()?,
            }),
            Self::ModuleQueryIndex { module, profile } => {
                Ok(artifact::ArtifactKey::ModuleQueryIndex {
                    module: module.into_source()?,
                    profile: profile.into_source()?,
                })
            }
            Self::WorkspaceQueryIndex { profile } => {
                Ok(artifact::ArtifactKey::WorkspaceQueryIndex {
                    profile: profile.into_source()?,
                })
            }
            Self::ModuleOutput { module, target } => Ok(artifact::ArtifactKey::ModuleOutput {
                module: module.into_source()?,
                target: target.into_source()?,
            }),
            Self::PackageOutput { package, target } => Ok(artifact::ArtifactKey::PackageOutput {
                package: package.into_source()?,
                target: target.into_source()?,
            }),
            Self::ModuleLinted { module, profile } => Ok(artifact::ArtifactKey::ModuleLinted {
                module: module.into_source()?,
                profile: profile.into_source()?,
            }),
            Self::PackageLinted { package } => Ok(artifact::ArtifactKey::PackageLinted {
                package: package.into_source()?,
            }),
            Self::WorkspaceLinted => Ok(artifact::ArtifactKey::WorkspaceLinted),
        }
    }
}

impl From<artifact::ArtifactKey> for ArtifactKey {
    /// Convert one artifact key into one bridge artifact key.
    fn from(key: artifact::ArtifactKey) -> Self {
        Self::from_artifact(key)
    }
}

impl TryFrom<ArtifactKey> for artifact::ArtifactKey {
    type Error = ArtifactBridgeError;

    /// Convert one bridge artifact key into one artifact key.
    fn try_from(key: ArtifactKey) -> Result<Self, Self::Error> {
        key.into_artifact()
    }
}

/// Error returned when an artifact bridge value is invalid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactBridgeError {
    /// One source id inside the artifact key is invalid.
    Source(SourceIdParseError),
}

impl Display for ArtifactBridgeError {
    /// Format this artifact bridge error.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Source(error) => Display::fmt(error, formatter),
        }
    }
}

impl std::error::Error for ArtifactBridgeError {
    /// Return the underlying error source.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Source(error) => Some(error),
        }
    }
}

impl From<SourceIdParseError> for ArtifactBridgeError {
    /// Convert one source id parse error into one artifact bridge error.
    fn from(error: SourceIdParseError) -> Self {
        Self::Source(error)
    }
}
