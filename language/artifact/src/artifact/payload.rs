use crate::{
    AmbientEnvironment, Ast, Data, DirChecked, DirDeclared, DirElaborated, DirExported,
    LanguageEnvironment, MirLowered, MirOptimized, ModuleLinted, ModuleOutput, ModuleQueryIndex,
    PackageLinted, PackageOutput, WorkspaceLinted, WorkspaceQueryIndex,
};

/// One typed artifact payload.
#[derive(Debug, Clone)]
pub enum ArtifactPayload {
    /// Language semantic environment for one profile.
    LanguageEnvironment(LanguageEnvironment),
    /// Ambient semantic environment for one profile.
    AmbientEnvironment(AmbientEnvironment),
    /// Parsed module syntax tree.
    Ast(Ast),
    /// Parsed non-code module data.
    Data(Data),
    /// Declared DIR.
    DirDeclared(DirDeclared),
    /// Exported DIR.
    DirExported(DirExported),
    /// Checked DIR.
    DirChecked(DirChecked),
    /// Elaborated DIR.
    DirElaborated(DirElaborated),
    /// Lowered MIR before optimization.
    MirLowered(MirLowered),
    /// Optimized MIR.
    MirOptimized(MirOptimized),
    /// Query index for one module profile.
    ModuleQueryIndex(ModuleQueryIndex),
    /// Query index for one workspace profile.
    WorkspaceQueryIndex(WorkspaceQueryIndex),
    /// One generated module output for one target.
    ModuleOutput(ModuleOutput),
    /// Output entries for one package target.
    PackageOutput(PackageOutput),
    /// Realized lint diagnostics for one module profile.
    ModuleLinted(ModuleLinted),
    /// Realized lint diagnostics for one package.
    PackageLinted(PackageLinted),
    /// Realized lint diagnostics for the workspace.
    WorkspaceLinted(WorkspaceLinted),
}

impl From<LanguageEnvironment> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: LanguageEnvironment) -> Self {
        Self::LanguageEnvironment(payload)
    }
}

impl From<AmbientEnvironment> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: AmbientEnvironment) -> Self {
        Self::AmbientEnvironment(payload)
    }
}

impl From<Ast> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: Ast) -> Self {
        Self::Ast(payload)
    }
}

impl From<Data> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: Data) -> Self {
        Self::Data(payload)
    }
}

impl From<DirDeclared> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: DirDeclared) -> Self {
        Self::DirDeclared(payload)
    }
}

impl From<DirExported> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: DirExported) -> Self {
        Self::DirExported(payload)
    }
}

impl From<DirChecked> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: DirChecked) -> Self {
        Self::DirChecked(payload)
    }
}

impl From<DirElaborated> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: DirElaborated) -> Self {
        Self::DirElaborated(payload)
    }
}

impl From<MirLowered> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: MirLowered) -> Self {
        Self::MirLowered(payload)
    }
}

impl From<MirOptimized> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: MirOptimized) -> Self {
        Self::MirOptimized(payload)
    }
}

impl From<ModuleQueryIndex> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: ModuleQueryIndex) -> Self {
        Self::ModuleQueryIndex(payload)
    }
}

impl From<WorkspaceQueryIndex> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: WorkspaceQueryIndex) -> Self {
        Self::WorkspaceQueryIndex(payload)
    }
}

impl From<ModuleOutput> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: ModuleOutput) -> Self {
        Self::ModuleOutput(payload)
    }
}

impl From<PackageOutput> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: PackageOutput) -> Self {
        Self::PackageOutput(payload)
    }
}

impl From<ModuleLinted> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: ModuleLinted) -> Self {
        Self::ModuleLinted(payload)
    }
}

impl From<PackageLinted> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: PackageLinted) -> Self {
        Self::PackageLinted(payload)
    }
}

impl From<WorkspaceLinted> for ArtifactPayload {
    /// Convert a typed artifact into an artifact payload.
    fn from(payload: WorkspaceLinted) -> Self {
        Self::WorkspaceLinted(payload)
    }
}
