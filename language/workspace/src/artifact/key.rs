use serde::{Deserialize, Serialize};

use destack_source::{ModuleId, PackageId};

use crate::{ProfileId, ProfileKey, TargetId};

use super::ArtifactFamily;

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
    /// Output entries for one module target.
    ModuleOutput { module: ModuleId, target: TargetId },
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
    /// Output entries for one module target.
    ModuleOutput { module: ModuleId, target: TargetId },
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

    /// Build one module output artifact key.
    pub fn module_output(module: ModuleId, target: TargetId) -> Self {
        Self::ModuleOutput { module, target }
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
            Self::ModuleOutput { .. } => ArtifactFamily::ModuleOutput,
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
            | Self::ModuleOutput { module, .. } => Some(*module),
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
            | Self::ModuleOutput { .. }
            | Self::PackageOutput { .. } => None,
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
            Self::ModuleOutput { .. } => ArtifactFamily::ModuleOutput,
            Self::PackageOutput { .. } => ArtifactFamily::PackageOutput,
            Self::LanguageEnvironment { .. } => ArtifactFamily::LanguageEnvironment,
            Self::IntrinsicEnvironment { .. } => ArtifactFamily::IntrinsicEnvironment,
            Self::LibraryEnvironment { .. } => ArtifactFamily::LibraryEnvironment,
        }
    }
}
