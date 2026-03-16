use destack_source::ModuleId;

use crate::{ProfileId, TargetId};

use super::ArtifactFamily;

/// Semantic artifact identity.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ArtifactKey {
    /// Language semantic environment for one profile.
    LanguageEnvironment { profile: ProfileId },
    /// Intrinsic semantic environment for one profile.
    IntrinsicEnvironment { profile: ProfileId },
    /// Lib semantic environment for one profile.
    LibEnvironment { profile: ProfileId },
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
}

impl ArtifactKey {
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

    /// Return the artifact family for this key.
    pub fn family(&self) -> ArtifactFamily {
        match self {
            Self::LanguageEnvironment { .. } => ArtifactFamily::LanguageEnvironment,
            Self::IntrinsicEnvironment { .. } => ArtifactFamily::IntrinsicEnvironment,
            Self::LibEnvironment { .. } => ArtifactFamily::LibEnvironment,
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
            | Self::MirOptimized { module, .. } => Some(*module),
            Self::LanguageEnvironment { .. }
            | Self::IntrinsicEnvironment { .. }
            | Self::LibEnvironment { .. } => None,
        }
    }

    /// Return the profile id encoded in this key when one exists.
    pub fn profile_id(&self) -> Option<ProfileId> {
        match self {
            Self::LanguageEnvironment { profile }
            | Self::IntrinsicEnvironment { profile }
            | Self::LibEnvironment { profile }
            | Self::DirPrepared { profile, .. }
            | Self::DirResolved { profile, .. }
            | Self::DirDeclared { profile, .. }
            | Self::DirInterface { profile, .. }
            | Self::DirAnalyzed { profile, .. }
            | Self::DirElaborated { profile, .. }
            | Self::DirPatched { profile, .. }
            | Self::MirBase { profile, .. }
            | Self::MirOptimized { profile, .. } => Some(*profile),
            Self::Ast { .. } | Self::DirBase { .. } => None,
        }
    }
}
