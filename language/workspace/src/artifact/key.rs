use destack_source::ModuleId;

use crate::{ProfileId, TargetId};

use super::ArtifactFamily;

/// Semantic artifact identity.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ArtifactKey {
    /// Parsed module syntax tree.
    Ast { module: ModuleId },
    /// Base DIR.
    DirBase { module: ModuleId },
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
    /// Comptime DIR.
    DirComptime {
        module: ModuleId,
        profile: ProfileId,
    },
    /// MIR.
    Mir {
        module: ModuleId,
        profile: ProfileId,
        target: TargetId,
    },
}

impl ArtifactKey {
    /// Return the artifact family for this key.
    pub fn family(&self) -> ArtifactFamily {
        match self {
            Self::Ast { .. } => ArtifactFamily::Ast,
            Self::DirBase { .. } => ArtifactFamily::DirBase,
            Self::DirResolved { .. } => ArtifactFamily::DirResolved,
            Self::DirDeclared { .. } => ArtifactFamily::DirDeclared,
            Self::DirInterface { .. } => ArtifactFamily::DirInterface,
            Self::DirAnalyzed { .. } => ArtifactFamily::DirAnalyzed,
            Self::DirElaborated { .. } => ArtifactFamily::DirElaborated,
            Self::DirComptime { .. } => ArtifactFamily::DirComptime,
            Self::Mir { .. } => ArtifactFamily::Mir,
        }
    }
}
