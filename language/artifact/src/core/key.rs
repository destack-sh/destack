use serde::{Deserialize, Serialize};

use destack_source::{ModuleId, PackageId, ProfileId, TargetId};

/// Provider family for one artifact key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ArtifactProvider {
    /// Source artifacts derived directly from repository file contents.
    Source,
    /// Compiler artifacts derived by compiler phases.
    Compiler,
    /// Linter artifacts derived by lint rules.
    Linter,
}

/// Semantic artifact identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ArtifactKey {
    /// Language semantic environment for one profile.
    LanguageEnvironment { profile: ProfileId },
    /// Ambient semantic environment for one profile.
    AmbientEnvironment { profile: ProfileId },

    /// Parsed module syntax tree.
    Ast { module: ModuleId },
    /// Parsed non-code module data.
    Data { module: ModuleId },

    /// Declared DIR.
    DirDeclared {
        module: ModuleId,
        profile: ProfileId,
    },
    /// Exported DIR.
    DirExported {
        module: ModuleId,
        profile: ProfileId,
    },
    /// Checked DIR.
    DirChecked {
        module: ModuleId,
        profile: ProfileId,
    },
    /// Elaborated DIR.
    DirElaborated {
        module: ModuleId,
        profile: ProfileId,
    },

    /// Lowered MIR before optimization.
    MirLowered {
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

    /// One generated module output for one target.
    ModuleOutput { module: ModuleId, target: TargetId },
    /// Output entries for one package target.
    PackageOutput {
        package: PackageId,
        target: TargetId,
    },

    /// Realized lint diagnostics for one module profile.
    ModuleLinted {
        module: ModuleId,
        profile: ProfileId,
    },
    /// Realized lint diagnostics for one package.
    PackageLinted { package: PackageId },
    /// Realized lint diagnostics for the workspace.
    WorkspaceLinted,
}

impl ArtifactKey {
    /// Return the provider family responsible for this artifact.
    pub fn provider(self) -> ArtifactProvider {
        match self {
            Self::Ast { .. } | Self::Data { .. } => ArtifactProvider::Source,
            Self::LanguageEnvironment { .. }
            | Self::AmbientEnvironment { .. }
            | Self::DirDeclared { .. }
            | Self::DirExported { .. }
            | Self::DirChecked { .. }
            | Self::DirElaborated { .. }
            | Self::MirLowered { .. }
            | Self::MirOptimized { .. }
            | Self::ModuleOutput { .. }
            | Self::PackageOutput { .. } => ArtifactProvider::Compiler,
            Self::ModuleLinted { .. } | Self::PackageLinted { .. } | Self::WorkspaceLinted => {
                ArtifactProvider::Linter
            }
        }
    }

    /// Return the package referenced by this artifact key when one exists.
    pub fn package_id(&self) -> Option<PackageId> {
        match self {
            Self::PackageOutput { package, .. } | Self::PackageLinted { package } => Some(*package),
            _ => None,
        }
    }

    /// Build one language environment artifact key.
    pub fn language_environment(profile: ProfileId) -> Self {
        Self::LanguageEnvironment { profile }
    }

    /// Build one ambient environment artifact key.
    pub fn ambient_environment(profile: ProfileId) -> Self {
        Self::AmbientEnvironment { profile }
    }

    /// Build one AST artifact key.
    pub fn ast(module: ModuleId) -> Self {
        Self::Ast { module }
    }

    /// Build one data artifact key.
    pub fn data(module: ModuleId) -> Self {
        Self::Data { module }
    }

    /// Build one declared DIR artifact key.
    pub fn dir_declared(module: ModuleId, profile: ProfileId) -> Self {
        Self::DirDeclared { module, profile }
    }

    /// Build one exported DIR artifact key.
    pub fn dir_exported(module: ModuleId, profile: ProfileId) -> Self {
        Self::DirExported { module, profile }
    }

    /// Build one checked DIR artifact key.
    pub fn dir_checked(module: ModuleId, profile: ProfileId) -> Self {
        Self::DirChecked { module, profile }
    }

    /// Build one elaborated DIR artifact key.
    pub fn dir_elaborated(module: ModuleId, profile: ProfileId) -> Self {
        Self::DirElaborated { module, profile }
    }

    /// Build one lowered MIR artifact key.
    pub fn mir_lowered(module: ModuleId, profile: ProfileId, target: TargetId) -> Self {
        Self::MirLowered {
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

    /// Build one module output key.
    pub fn module_output(module: ModuleId, target: TargetId) -> Self {
        Self::ModuleOutput { module, target }
    }

    /// Build one package output artifact key.
    pub fn package_output(package: PackageId, target: TargetId) -> Self {
        Self::PackageOutput { package, target }
    }

    /// Build one module lint artifact key.
    pub fn module_linted(module: ModuleId, profile: ProfileId) -> Self {
        Self::ModuleLinted { module, profile }
    }

    /// Build one package lint artifact key.
    pub fn package_linted(package: PackageId) -> Self {
        Self::PackageLinted { package }
    }

    /// Build one workspace lint artifact key.
    pub fn workspace_linted() -> Self {
        Self::WorkspaceLinted
    }

    /// Return the stable short name for this key.
    pub fn name(&self) -> &'static str {
        match self {
            Self::LanguageEnvironment { .. } => "language_environment",
            Self::AmbientEnvironment { .. } => "ambient_environment",
            Self::Ast { .. } => "ast",
            Self::Data { .. } => "data",
            Self::DirDeclared { .. } => "dir_declared",
            Self::DirExported { .. } => "dir_exported",
            Self::DirChecked { .. } => "dir_checked",
            Self::DirElaborated { .. } => "dir_elaborated",
            Self::MirLowered { .. } => "mir_lowered",
            Self::MirOptimized { .. } => "mir_optimized",
            Self::ModuleOutput { .. } => "module_output",
            Self::PackageOutput { .. } => "package_output",
            Self::ModuleLinted { .. } => "module_linted",
            Self::PackageLinted { .. } => "package_linted",
            Self::WorkspaceLinted => "workspace_linted",
        }
    }

    /// Return the module id encoded in this key when one exists.
    pub fn module_id(&self) -> Option<ModuleId> {
        match self {
            Self::Ast { module }
            | Self::Data { module }
            | Self::DirDeclared { module, .. }
            | Self::DirExported { module, .. }
            | Self::DirChecked { module, .. }
            | Self::DirElaborated { module, .. }
            | Self::MirLowered { module, .. }
            | Self::MirOptimized { module, .. }
            | Self::ModuleOutput { module, .. }
            | Self::ModuleLinted { module, .. } => Some(*module),
            Self::LanguageEnvironment { .. }
            | Self::AmbientEnvironment { .. }
            | Self::PackageOutput { .. }
            | Self::PackageLinted { .. }
            | Self::WorkspaceLinted => None,
        }
    }
}

impl ArtifactKey {
    /// Return the profile id encoded in this key when one exists.
    pub fn profile_id(&self) -> Option<ProfileId> {
        match self {
            Self::LanguageEnvironment { profile }
            | Self::AmbientEnvironment { profile }
            | Self::DirDeclared { profile, .. }
            | Self::DirExported { profile, .. }
            | Self::DirChecked { profile, .. }
            | Self::DirElaborated { profile, .. }
            | Self::MirLowered { profile, .. }
            | Self::MirOptimized { profile, .. }
            | Self::ModuleLinted { profile, .. } => Some(*profile),
            Self::Ast { .. }
            | Self::Data { .. }
            | Self::ModuleOutput { .. }
            | Self::PackageOutput { .. }
            | Self::PackageLinted { .. }
            | Self::WorkspaceLinted => None,
        }
    }
}
