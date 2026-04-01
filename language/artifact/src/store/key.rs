use serde::{Deserialize, Serialize};

use destack_source::{ModuleId, PackageId, ProfileId, TargetId};

use crate::{ArtifactFamily, ProfileKey};

/// Dependency stamp captured for one artifact build.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize)]
pub struct ArtifactDependency(pub u64);

impl std::fmt::Debug for ArtifactDependency {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "d{:016x}", self.0)
    }
}

impl std::fmt::Display for ArtifactDependency {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "d{:016x}", self.0)
    }
}

impl ArtifactDependency {
    /// Create a new artifact dependency.
    pub fn new(value: u64) -> Self {
        Self(value)
    }
}

/// One exact live artifact version.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ArtifactVersion {
    /// The semantic artifact slot.
    pub key: ArtifactKey,
    /// The exact reusable dependency stamp.
    pub dependency: ArtifactDependency,
}

impl ArtifactVersion {
    /// Create one artifact version from one key and dependency.
    pub fn new(key: ArtifactKey, dependency: ArtifactDependency) -> Self {
        Self { key, dependency }
    }

    /// Return the package referenced by this artifact version when one exists.
    pub fn package_id(&self) -> Option<PackageId> {
        self.key.package_id()
    }

    /// Return the module referenced by this artifact version when one exists.
    pub fn module_id(&self) -> Option<ModuleId> {
        self.key.module_id()
    }

    /// Return the profile referenced by this artifact version when one exists.
    pub fn profile_id(&self) -> Option<ProfileId> {
        self.key.profile_id()
    }

    /// Return the family of this artifact version.
    pub fn family(&self) -> ArtifactFamily {
        self.key.family()
    }

    /// Convert this live artifact version into one stable persisted image key.
    pub fn image_key_with(
        &self,
        profile_key_for_id: impl Fn(ProfileId) -> ProfileKey,
    ) -> ArtifactImageKey {
        self.key.image_key_with(profile_key_for_id)
    }
}

/// Semantic artifact identity.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ArtifactKey {
    /// Module dependency graph for one profile.
    ModuleGraph { profile: ProfileId },
    /// Language semantic environment for one profile.
    LanguageEnvironment { profile: ProfileId },
    /// Intrinsic semantic environment for one profile.
    IntrinsicEnvironment { profile: ProfileId },
    /// Library semantic environment for one profile.
    LibraryEnvironment { profile: ProfileId },
    /// Parsed module syntax tree.
    Ast { module: ModuleId },
    /// Base DIR.
    DirBase { module: ModuleId },
    /// Profile prepared DIR.
    DirPrepared {
        module: ModuleId,
        profile: ProfileId,
    },
    /// Resolved DIR.
    DirResolved {
        module: ModuleId,
        profile: ProfileId,
    },
    /// Declared DIR.
    DirDeclared {
        module: ModuleId,
        profile: ProfileId,
    },
    /// Interface DIR.
    DirInterface {
        module: ModuleId,
        profile: ProfileId,
    },
    /// Analyzed DIR.
    DirAnalyzed {
        module: ModuleId,
        profile: ProfileId,
    },
    /// Elaborated DIR.
    DirElaborated {
        module: ModuleId,
        profile: ProfileId,
    },
    /// Patched DIR.
    DirPatched {
        module: ModuleId,
        profile: ProfileId,
    },
    /// Base MIR before optimization.
    MirBase {
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
    },
    /// Optimized MIR.
    MirOptimized {
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
    },
    /// One generated module artifact for one target.
    ModuleArtifact { module: ModuleId, target: TargetId },
    /// Output entries for one package target.
    PackageOutput {
        package: PackageId,
        target: TargetId,
    },
}

/// Stable artifact image identity for persisted cache entries.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ArtifactImageKey {
    /// Module dependency graph for one profile.
    ModuleGraph { profile: ProfileKey },
    /// Parsed module base DIR.
    DirBase { module: ModuleId },
    /// Parsed module syntax tree.
    Ast { module: ModuleId },
    /// Profile prepared DIR.
    DirPrepared {
        module: ModuleId,
        profile: ProfileKey,
    },
    /// Resolved DIR.
    DirResolved {
        module: ModuleId,
        profile: ProfileKey,
    },
    /// Declared DIR.
    DirDeclared {
        module: ModuleId,
        profile: ProfileKey,
    },
    /// Interface DIR.
    DirInterface {
        module: ModuleId,
        profile: ProfileKey,
    },
    /// Analyzed DIR.
    DirAnalyzed {
        module: ModuleId,
        profile: ProfileKey,
    },
    /// Elaborated DIR.
    DirElaborated {
        module: ModuleId,
        profile: ProfileKey,
    },
    /// Patched DIR.
    DirPatched {
        module: ModuleId,
        profile: ProfileKey,
    },
    /// Base MIR before optimization.
    MirBase {
        module: ModuleId,
        profile: ProfileKey,
        target: TargetId,
    },
    /// Optimized MIR.
    MirOptimized {
        module: ModuleId,
        profile: ProfileKey,
        target: TargetId,
    },
    /// One generated module artifact for one target.
    ModuleArtifact { module: ModuleId, target: TargetId },
    /// Output entries for one package target.
    PackageOutput {
        package: PackageId,
        target: TargetId,
    },
    /// Language semantic environment for one profile.
    LanguageEnvironment { profile: ProfileKey },
    /// Intrinsic semantic environment for one profile.
    IntrinsicEnvironment { profile: ProfileKey },
    /// Library semantic environment for one profile.
    LibraryEnvironment { profile: ProfileKey },
}

impl ArtifactKey {
    /// Return the package referenced by this artifact key when one exists.
    pub fn package_id(&self) -> Option<PackageId> {
        match self {
            Self::PackageOutput { package, .. } => Some(*package),
            _ => None,
        }
    }

    /// Build one module graph artifact key.
    pub fn module_graph(profile: ProfileId) -> Self {
        Self::ModuleGraph { profile }
    }

    /// Build one language environment artifact key.
    pub fn language_environment(profile: ProfileId) -> Self {
        Self::LanguageEnvironment { profile }
    }

    /// Build one intrinsic environment artifact key.
    pub fn intrinsic_environment(profile: ProfileId) -> Self {
        Self::IntrinsicEnvironment { profile }
    }

    /// Build one library environment artifact key.
    pub fn library_environment(profile: ProfileId) -> Self {
        Self::LibraryEnvironment { profile }
    }

    /// Build one AST artifact key.
    pub fn ast(module: ModuleId) -> Self {
        Self::Ast { module }
    }

    /// Build one base DIR artifact key.
    pub fn dir_base(module: ModuleId) -> Self {
        Self::DirBase { module }
    }

    /// Build one prepared DIR artifact key.
    pub fn dir_prepared(module: ModuleId, profile: ProfileId) -> Self {
        Self::DirPrepared { module, profile }
    }

    /// Build one resolved DIR artifact key.
    pub fn dir_resolved(module: ModuleId, profile: ProfileId) -> Self {
        Self::DirResolved { module, profile }
    }

    /// Build one declared DIR artifact key.
    pub fn dir_declared(module: ModuleId, profile: ProfileId) -> Self {
        Self::DirDeclared { module, profile }
    }

    /// Build one interface DIR artifact key.
    pub fn dir_interface(module: ModuleId, profile: ProfileId) -> Self {
        Self::DirInterface { module, profile }
    }

    /// Build one analyzed DIR artifact key.
    pub fn dir_analyzed(module: ModuleId, profile: ProfileId) -> Self {
        Self::DirAnalyzed { module, profile }
    }

    /// Build one elaborated DIR artifact key.
    pub fn dir_elaborated(module: ModuleId, profile: ProfileId) -> Self {
        Self::DirElaborated { module, profile }
    }

    /// Build one patched DIR artifact key.
    pub fn dir_patched(module: ModuleId, profile: ProfileId) -> Self {
        Self::DirPatched { module, profile }
    }

    /// Build one base MIR artifact key.
    pub fn mir_base(module: ModuleId, profile: ProfileId, target: TargetId) -> Self {
        Self::MirBase {
            module,
            profile,
            target,
        }
    }

    /// Build one optimized MIR artifact key.
    pub fn mir_optimized(module: ModuleId, profile: ProfileId, target: TargetId) -> Self {
        Self::MirOptimized {
            module,
            profile,
            target,
        }
    }

    /// Build one module artifact key.
    pub fn module_artifact(module: ModuleId, target: TargetId) -> Self {
        Self::ModuleArtifact { module, target }
    }

    /// Build one package output artifact key.
    pub fn package_output(package: PackageId, target: TargetId) -> Self {
        Self::PackageOutput { package, target }
    }

    /// Return the artifact family for this key.
    pub fn family(&self) -> ArtifactFamily {
        match self {
            Self::ModuleGraph { .. } => ArtifactFamily::ModuleGraph,
            Self::LanguageEnvironment { .. } => ArtifactFamily::LanguageEnvironment,
            Self::IntrinsicEnvironment { .. } => ArtifactFamily::IntrinsicEnvironment,
            Self::LibraryEnvironment { .. } => ArtifactFamily::LibraryEnvironment,
            Self::Ast { .. } => ArtifactFamily::Ast,
            Self::DirBase { .. } => ArtifactFamily::DirBase,
            Self::DirPrepared { .. } => ArtifactFamily::DirPrepared,
            Self::DirResolved { .. } => ArtifactFamily::DirResolved,
            Self::DirDeclared { .. } => ArtifactFamily::DirDeclared,
            Self::DirInterface { .. } => ArtifactFamily::DirInterface,
            Self::DirAnalyzed { .. } => ArtifactFamily::DirAnalyzed,
            Self::DirElaborated { .. } => ArtifactFamily::DirElaborated,
            Self::DirPatched { .. } => ArtifactFamily::DirPatched,
            Self::MirBase { .. } => ArtifactFamily::MirBase,
            Self::MirOptimized { .. } => ArtifactFamily::MirOptimized,
            Self::ModuleArtifact { .. } => ArtifactFamily::ModuleArtifact,
            Self::PackageOutput { .. } => ArtifactFamily::PackageOutput,
        }
    }

    /// Return the module id encoded in this key when one exists.
    pub fn module_id(&self) -> Option<ModuleId> {
        match self {
            Self::Ast { module }
            | Self::DirBase { module }
            | Self::DirPrepared { module, .. }
            | Self::DirResolved { module, .. }
            | Self::DirDeclared { module, .. }
            | Self::DirInterface { module, .. }
            | Self::DirAnalyzed { module, .. }
            | Self::DirElaborated { module, .. }
            | Self::DirPatched { module, .. }
            | Self::MirBase { module, .. }
            | Self::MirOptimized { module, .. }
            | Self::ModuleArtifact { module, .. } => Some(*module),
            Self::ModuleGraph { .. }
            | Self::LanguageEnvironment { .. }
            | Self::IntrinsicEnvironment { .. }
            | Self::LibraryEnvironment { .. }
            | Self::PackageOutput { .. } => None,
        }
    }

    /// Return the profile id encoded in this key when one exists.
    pub fn profile_id(&self) -> Option<ProfileId> {
        match self {
            Self::ModuleGraph { profile }
            | Self::LanguageEnvironment { profile }
            | Self::IntrinsicEnvironment { profile }
            | Self::LibraryEnvironment { profile }
            | Self::DirPrepared { profile, .. }
            | Self::DirResolved { profile, .. }
            | Self::DirDeclared { profile, .. }
            | Self::DirInterface { profile, .. }
            | Self::DirAnalyzed { profile, .. }
            | Self::DirElaborated { profile, .. }
            | Self::DirPatched { profile, .. }
            | Self::MirBase { profile, .. }
            | Self::MirOptimized { profile, .. } => Some(*profile),
            Self::Ast { .. }
            | Self::DirBase { .. }
            | Self::ModuleArtifact { .. }
            | Self::PackageOutput { .. } => None,
        }
    }

    /// Convert this live artifact key into one stable persisted image key.
    pub fn image_key_with(
        &self,
        profile_key_for_id: impl Fn(ProfileId) -> ProfileKey,
    ) -> ArtifactImageKey {
        match self {
            Self::ModuleGraph { profile } => ArtifactImageKey::ModuleGraph {
                profile: profile_key_for_id(*profile),
            },
            Self::LanguageEnvironment { profile } => ArtifactImageKey::LanguageEnvironment {
                profile: profile_key_for_id(*profile),
            },
            Self::IntrinsicEnvironment { profile } => ArtifactImageKey::IntrinsicEnvironment {
                profile: profile_key_for_id(*profile),
            },
            Self::LibraryEnvironment { profile } => ArtifactImageKey::LibraryEnvironment {
                profile: profile_key_for_id(*profile),
            },
            Self::Ast { module } => ArtifactImageKey::Ast { module: *module },
            Self::DirBase { module } => ArtifactImageKey::DirBase { module: *module },
            Self::DirPrepared { module, profile } => ArtifactImageKey::DirPrepared {
                module: *module,
                profile: profile_key_for_id(*profile),
            },
            Self::DirResolved { module, profile } => ArtifactImageKey::DirResolved {
                module: *module,
                profile: profile_key_for_id(*profile),
            },
            Self::DirDeclared { module, profile } => ArtifactImageKey::DirDeclared {
                module: *module,
                profile: profile_key_for_id(*profile),
            },
            Self::DirInterface { module, profile } => ArtifactImageKey::DirInterface {
                module: *module,
                profile: profile_key_for_id(*profile),
            },
            Self::DirAnalyzed { module, profile } => ArtifactImageKey::DirAnalyzed {
                module: *module,
                profile: profile_key_for_id(*profile),
            },
            Self::DirElaborated { module, profile } => ArtifactImageKey::DirElaborated {
                module: *module,
                profile: profile_key_for_id(*profile),
            },
            Self::DirPatched { module, profile } => ArtifactImageKey::DirPatched {
                module: *module,
                profile: profile_key_for_id(*profile),
            },
            Self::MirBase {
                module,
                profile,
                target,
            } => ArtifactImageKey::MirBase {
                module: *module,
                profile: profile_key_for_id(*profile),
                target: *target,
            },
            Self::MirOptimized {
                module,
                profile,
                target,
            } => ArtifactImageKey::MirOptimized {
                module: *module,
                profile: profile_key_for_id(*profile),
                target: *target,
            },
            Self::ModuleArtifact { module, target } => ArtifactImageKey::ModuleArtifact {
                module: *module,
                target: *target,
            },
            Self::PackageOutput { package, target } => ArtifactImageKey::PackageOutput {
                package: *package,
                target: *target,
            },
        }
    }
}

impl ArtifactImageKey {
    /// Return the artifact family for this image key.
    pub fn family(&self) -> ArtifactFamily {
        match self {
            Self::ModuleGraph { .. } => ArtifactFamily::ModuleGraph,
            Self::DirBase { .. } => ArtifactFamily::DirBase,
            Self::Ast { .. } => ArtifactFamily::Ast,
            Self::DirPrepared { .. } => ArtifactFamily::DirPrepared,
            Self::DirResolved { .. } => ArtifactFamily::DirResolved,
            Self::DirDeclared { .. } => ArtifactFamily::DirDeclared,
            Self::DirInterface { .. } => ArtifactFamily::DirInterface,
            Self::DirAnalyzed { .. } => ArtifactFamily::DirAnalyzed,
            Self::DirElaborated { .. } => ArtifactFamily::DirElaborated,
            Self::DirPatched { .. } => ArtifactFamily::DirPatched,
            Self::MirBase { .. } => ArtifactFamily::MirBase,
            Self::MirOptimized { .. } => ArtifactFamily::MirOptimized,
            Self::ModuleArtifact { .. } => ArtifactFamily::ModuleArtifact,
            Self::PackageOutput { .. } => ArtifactFamily::PackageOutput,
            Self::LanguageEnvironment { .. } => ArtifactFamily::LanguageEnvironment,
            Self::IntrinsicEnvironment { .. } => ArtifactFamily::IntrinsicEnvironment,
            Self::LibraryEnvironment { .. } => ArtifactFamily::LibraryEnvironment,
        }
    }
}
