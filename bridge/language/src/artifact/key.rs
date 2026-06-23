use std::fmt::{self, Display, Formatter};

use crate::{
    ComponentId, ModuleId, PackageId, ProductId, ProfileId, SourceIdParseError, TargetId, bridge,
};

/// External artifact key crossing bridge boundaries.
#[bridge(capi_handle)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ArtifactKey {
    /// Toolchain build payload for one target.
    Build {
        /// Build target.
        target: TargetId,
    },
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
    PackageIndex {
        /// Semantic profile.
        profile: ProfileId,
    },
    /// Component partition for one profile.
    ComponentGraph {
        /// Semantic profile.
        profile: ProfileId,
    },
    /// Whole-program analysis for one profile and target.
    ProgramAnalysis {
        /// Semantic profile.
        profile: ProfileId,
        /// Build target.
        target: TargetId,
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
    /// Per-module link summary for whole-program analysis.
    MirAnalyzed {
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
    /// One structured linker input for one target.
    Script {
        /// Source module.
        module: ModuleId,
        /// Build target.
        target: TargetId,
    },
    /// One compiled-code linker input for one target.
    Object {
        /// Source module.
        module: ModuleId,
        /// Build target.
        target: TargetId,
    },
    /// One opaque linker input for one target.
    Asset {
        /// Source module.
        module: ModuleId,
        /// Build target.
        target: TargetId,
    },
    /// Linked file graph for one package target.
    Bundle {
        /// Source package.
        package: PackageId,
        /// Build target.
        target: TargetId,
    },
    /// Executable program for one package target.
    Program {
        /// Source package.
        package: PackageId,
        /// Build target.
        target: TargetId,
    },
    /// Linked product assembled from configured target artifacts.
    Product {
        /// Source package.
        package: PackageId,
        /// Product.
        product: ProductId,
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
    pub fn from_artifact(key: destack_artifact::ArtifactKey) -> Self {
        match key {
            destack_artifact::ArtifactKey::Build { target } => Self::Build {
                target: target.into(),
            },
            destack_artifact::ArtifactKey::DirParsed { module } => Self::DirParsed {
                module: module.into(),
            },
            destack_artifact::ArtifactKey::Data { module } => Self::Data {
                module: module.into(),
            },
            destack_artifact::ArtifactKey::GlobalEnvironment { profile } => {
                Self::GlobalEnvironment {
                    profile: profile.into(),
                }
            }
            destack_artifact::ArtifactKey::PackageIndex { profile } => Self::PackageIndex {
                profile: profile.into(),
            },
            destack_artifact::ArtifactKey::ComponentGraph { profile } => Self::ComponentGraph {
                profile: profile.into(),
            },
            destack_artifact::ArtifactKey::ProgramAnalysis { profile, target } => {
                Self::ProgramAnalysis {
                    profile: profile.into(),
                    target: target.into(),
                }
            }
            destack_artifact::ArtifactKey::DirBound { module, profile } => Self::DirBound {
                module: module.into(),
                profile: profile.into(),
            },
            destack_artifact::ArtifactKey::DirImported { module, profile } => Self::DirImported {
                module: module.into(),
                profile: profile.into(),
            },
            destack_artifact::ArtifactKey::DirExpanded { module, profile } => Self::DirExpanded {
                module: module.into(),
                profile: profile.into(),
            },
            destack_artifact::ArtifactKey::DirExported { module, profile } => Self::DirExported {
                module: module.into(),
                profile: profile.into(),
            },
            destack_artifact::ArtifactKey::DirResolved { module, profile } => Self::DirResolved {
                module: module.into(),
                profile: profile.into(),
            },
            destack_artifact::ArtifactKey::DirCheckedComponent {
                entry,
                component,
                profile,
            } => Self::DirCheckedComponent {
                entry: entry.into(),
                component: component.into(),
                profile: profile.into(),
            },
            destack_artifact::ArtifactKey::DirChecked { module, profile } => Self::DirChecked {
                module: module.into(),
                profile: profile.into(),
            },
            destack_artifact::ArtifactKey::DirMaterialized { module, profile } => {
                Self::DirMaterialized {
                    module: module.into(),
                    profile: profile.into(),
                }
            }
            destack_artifact::ArtifactKey::DirElaborated { module, profile } => {
                Self::DirElaborated {
                    module: module.into(),
                    profile: profile.into(),
                }
            }
            destack_artifact::ArtifactKey::MirLowered {
                module,
                profile,
                target,
            } => Self::MirLowered {
                module: module.into(),
                profile: profile.into(),
                target: target.into(),
            },
            destack_artifact::ArtifactKey::MirVerified {
                module,
                profile,
                target,
            } => Self::MirVerified {
                module: module.into(),
                profile: profile.into(),
                target: target.into(),
            },
            destack_artifact::ArtifactKey::MirAnalyzed {
                module,
                profile,
                target,
            } => Self::MirAnalyzed {
                module: module.into(),
                profile: profile.into(),
                target: target.into(),
            },
            destack_artifact::ArtifactKey::MirOptimized {
                module,
                profile,
                target,
            } => Self::MirOptimized {
                module: module.into(),
                profile: profile.into(),
                target: target.into(),
            },
            destack_artifact::ArtifactKey::ModuleQueryIndex { module, profile } => {
                Self::ModuleQueryIndex {
                    module: module.into(),
                    profile: profile.into(),
                }
            }
            destack_artifact::ArtifactKey::WorkspaceQueryIndex { profile } => {
                Self::WorkspaceQueryIndex {
                    profile: profile.into(),
                }
            }
            destack_artifact::ArtifactKey::Script { module, target } => Self::Script {
                module: module.into(),
                target: target.into(),
            },
            destack_artifact::ArtifactKey::Object { module, target } => Self::Object {
                module: module.into(),
                target: target.into(),
            },
            destack_artifact::ArtifactKey::Asset { module, target } => Self::Asset {
                module: module.into(),
                target: target.into(),
            },
            destack_artifact::ArtifactKey::Bundle { package, target } => Self::Bundle {
                package: package.into(),
                target: target.into(),
            },
            destack_artifact::ArtifactKey::Program { package, target } => Self::Program {
                package: package.into(),
                target: target.into(),
            },
            destack_artifact::ArtifactKey::Product { package, product } => Self::Product {
                package: package.into(),
                product: product.into(),
            },
            destack_artifact::ArtifactKey::ModuleLinted { module, profile } => Self::ModuleLinted {
                module: module.into(),
                profile: profile.into(),
            },
            destack_artifact::ArtifactKey::PackageLinted { package } => Self::PackageLinted {
                package: package.into(),
            },
            destack_artifact::ArtifactKey::WorkspaceLinted => Self::WorkspaceLinted,
        }
    }

    /// Convert this bridge artifact key into one artifact key.
    pub fn into_artifact(self) -> Result<destack_artifact::ArtifactKey, ArtifactBridgeError> {
        match self {
            Self::Build { target } => Ok(destack_artifact::ArtifactKey::Build {
                target: target.into_source()?,
            }),
            Self::DirParsed { module } => Ok(destack_artifact::ArtifactKey::DirParsed {
                module: module.into_source()?,
            }),
            Self::Data { module } => Ok(destack_artifact::ArtifactKey::Data {
                module: module.into_source()?,
            }),
            Self::GlobalEnvironment { profile } => {
                Ok(destack_artifact::ArtifactKey::GlobalEnvironment {
                    profile: profile.into_source()?,
                })
            }
            Self::PackageIndex { profile } => Ok(destack_artifact::ArtifactKey::PackageIndex {
                profile: profile.into_source()?,
            }),
            Self::ComponentGraph { profile } => Ok(destack_artifact::ArtifactKey::ComponentGraph {
                profile: profile.into_source()?,
            }),
            Self::ProgramAnalysis { profile, target } => {
                Ok(destack_artifact::ArtifactKey::ProgramAnalysis {
                    profile: profile.into_source()?,
                    target: target.into_source()?,
                })
            }
            Self::DirBound { module, profile } => Ok(destack_artifact::ArtifactKey::DirBound {
                module: module.into_source()?,
                profile: profile.into_source()?,
            }),
            Self::DirImported { module, profile } => {
                Ok(destack_artifact::ArtifactKey::DirImported {
                    module: module.into_source()?,
                    profile: profile.into_source()?,
                })
            }
            Self::DirExpanded { module, profile } => {
                Ok(destack_artifact::ArtifactKey::DirExpanded {
                    module: module.into_source()?,
                    profile: profile.into_source()?,
                })
            }
            Self::DirExported { module, profile } => {
                Ok(destack_artifact::ArtifactKey::DirExported {
                    module: module.into_source()?,
                    profile: profile.into_source()?,
                })
            }
            Self::DirResolved { module, profile } => {
                Ok(destack_artifact::ArtifactKey::DirResolved {
                    module: module.into_source()?,
                    profile: profile.into_source()?,
                })
            }
            Self::DirCheckedComponent {
                entry,
                component,
                profile,
            } => Ok(destack_artifact::ArtifactKey::DirCheckedComponent {
                entry: entry.into_source()?,
                component: component.into_source()?,
                profile: profile.into_source()?,
            }),
            Self::DirChecked { module, profile } => Ok(destack_artifact::ArtifactKey::DirChecked {
                module: module.into_source()?,
                profile: profile.into_source()?,
            }),
            Self::DirMaterialized { module, profile } => {
                Ok(destack_artifact::ArtifactKey::DirMaterialized {
                    module: module.into_source()?,
                    profile: profile.into_source()?,
                })
            }
            Self::DirElaborated { module, profile } => {
                Ok(destack_artifact::ArtifactKey::DirElaborated {
                    module: module.into_source()?,
                    profile: profile.into_source()?,
                })
            }
            Self::MirLowered {
                module,
                profile,
                target,
            } => Ok(destack_artifact::ArtifactKey::MirLowered {
                module: module.into_source()?,
                profile: profile.into_source()?,
                target: target.into_source()?,
            }),
            Self::MirVerified {
                module,
                profile,
                target,
            } => Ok(destack_artifact::ArtifactKey::MirVerified {
                module: module.into_source()?,
                profile: profile.into_source()?,
                target: target.into_source()?,
            }),
            Self::MirAnalyzed {
                module,
                profile,
                target,
            } => Ok(destack_artifact::ArtifactKey::MirAnalyzed {
                module: module.into_source()?,
                profile: profile.into_source()?,
                target: target.into_source()?,
            }),
            Self::MirOptimized {
                module,
                profile,
                target,
            } => Ok(destack_artifact::ArtifactKey::MirOptimized {
                module: module.into_source()?,
                profile: profile.into_source()?,
                target: target.into_source()?,
            }),
            Self::ModuleQueryIndex { module, profile } => {
                Ok(destack_artifact::ArtifactKey::ModuleQueryIndex {
                    module: module.into_source()?,
                    profile: profile.into_source()?,
                })
            }
            Self::WorkspaceQueryIndex { profile } => {
                Ok(destack_artifact::ArtifactKey::WorkspaceQueryIndex {
                    profile: profile.into_source()?,
                })
            }
            Self::Script { module, target } => Ok(destack_artifact::ArtifactKey::Script {
                module: module.into_source()?,
                target: target.into_source()?,
            }),
            Self::Object { module, target } => Ok(destack_artifact::ArtifactKey::Object {
                module: module.into_source()?,
                target: target.into_source()?,
            }),
            Self::Asset { module, target } => Ok(destack_artifact::ArtifactKey::Asset {
                module: module.into_source()?,
                target: target.into_source()?,
            }),
            Self::Bundle { package, target } => Ok(destack_artifact::ArtifactKey::Bundle {
                package: package.into_source()?,
                target: target.into_source()?,
            }),
            Self::Program { package, target } => Ok(destack_artifact::ArtifactKey::Program {
                package: package.into_source()?,
                target: target.into_source()?,
            }),
            Self::Product { package, product } => Ok(destack_artifact::ArtifactKey::Product {
                package: package.into_source()?,
                product: product.into_source()?,
            }),
            Self::ModuleLinted { module, profile } => {
                Ok(destack_artifact::ArtifactKey::ModuleLinted {
                    module: module.into_source()?,
                    profile: profile.into_source()?,
                })
            }
            Self::PackageLinted { package } => Ok(destack_artifact::ArtifactKey::PackageLinted {
                package: package.into_source()?,
            }),
            Self::WorkspaceLinted => Ok(destack_artifact::ArtifactKey::WorkspaceLinted),
        }
    }
}

impl From<destack_artifact::ArtifactKey> for ArtifactKey {
    /// Convert one artifact key into one bridge artifact key.
    fn from(key: destack_artifact::ArtifactKey) -> Self {
        Self::from_artifact(key)
    }
}

impl TryFrom<ArtifactKey> for destack_artifact::ArtifactKey {
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
