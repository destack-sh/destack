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
    /// MIR.
    Mir {
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
            Self::Mir { .. } => ArtifactFamily::Mir,
            Self::MirOptimized { .. } => ArtifactFamily::MirOptimized,
        }
    }
}
