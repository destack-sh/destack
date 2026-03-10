/// Family of published semantic artifacts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArtifactFamily {
    /// Parsed module syntax tree.
    Ast,
    /// Base DIR before semantic resolution.
    DirBase,
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
    DirComptime,
    /// Lowered MIR.
    Mir,
}
