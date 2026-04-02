/// Family of published semantic artifacts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArtifactFamily {
    /// Module dependency graph for one profile.
    ModuleGraph,
    /// Language semantic environment for one profile.
    LanguageEnvironment,
    /// Intrinsic semantic environment for one profile.
    IntrinsicEnvironment,
    /// Library semantic environment for one profile.
    LibraryEnvironment,
    /// Parsed module syntax tree.
    Ast,
    /// Base DIR before semantic resolution.
    DirBase,
    /// Profile prepared DIR.
    DirPrepared,
    /// Resolved DIR.
    DirResolved,
    /// Declared DIR.
    DirDeclared,
    /// Published interface DIR.
    DirInterface,
    /// Fully analyzed DIR.
    DirAnalyzed,
    /// Elaborated DIR.
    DirElaborated,
    /// Post comptime DIR.
    DirPatched,
    /// Base MIR before optimization.
    MirBase,
    /// Optimized MIR.
    MirOptimized,
    /// One generated module artifact for one target.
    ModuleOutput,
    /// Output entries for one package target.
    PackageOutput,
}

/// Validation contract for reusing one persisted artifact image across fresh processes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PersistedImageValidation {
    /// The image can validate itself from its own stable inputs and payload metadata.
    SelfContained,
    /// The image requires dependency validation beyond its own header and payload.
    DependencyValidated,
}

impl ArtifactFamily {
    /// Return the validation contract for one persisted image in this family.
    pub fn persisted_image_validation(self) -> PersistedImageValidation {
        match self {
            Self::ModuleGraph
            | Self::LanguageEnvironment
            | Self::IntrinsicEnvironment
            | Self::LibraryEnvironment
            | Self::Ast
            | Self::DirBase => PersistedImageValidation::SelfContained,

            Self::DirPrepared
            | Self::DirResolved
            | Self::DirDeclared
            | Self::DirInterface
            | Self::DirAnalyzed
            | Self::DirElaborated
            | Self::DirPatched
            | Self::MirBase
            | Self::MirOptimized
            | Self::ModuleOutput
            | Self::PackageOutput => PersistedImageValidation::DependencyValidated,
        }
    }
}
