/// Family of published semantic artifacts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArtifactFamily {
    /// Language semantic environment for one profile.
    LanguageEnvironment,
    /// Intrinsic semantic environment for one profile.
    IntrinsicEnvironment,
    /// Lib semantic environment for one profile.
    LibEnvironment,
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
    /// Lowered MIR.
    Mir,
    /// Optimized MIR.
    MirOptimized,
}
