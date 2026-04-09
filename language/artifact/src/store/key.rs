use serde::{Deserialize, Serialize};

use destack_source::{ModuleId, PackageId, ProfileId, TargetId};

use crate::{ArtifactFamily, ProfileKey};

/// Provider that owns one artifact key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ArtifactProvider {
    /// Compiler owned artifacts.
    Compiler,
    /// Linter owned artifacts.
    Linter,
}

/// Stamp captured for one artifact build.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize)]
pub struct ArtifactStamp(pub u64);

impl std::fmt::Debug for ArtifactStamp {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "d{:016x}", self.0)
    }
}

impl std::fmt::Display for ArtifactStamp {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "d{:016x}", self.0)
    }
}

impl ArtifactStamp {
    /// Create a new artifact stamp.
    pub fn new(value: u64) -> Self {
        Self(value)
    }
}

/// Stable artifact image identity for one persisted profile key.
pub type ArtifactImageKey = ArtifactKey<ProfileKey>;

/// One exact live artifact version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ArtifactVersion {
    /// The semantic artifact slot.
    pub key: ArtifactKey,
    /// The exact reusable stamp.
    pub stamp: ArtifactStamp,
}

impl ArtifactVersion {
    /// Create one artifact version from one key and stamp.
    pub fn new(key: ArtifactKey, stamp: ArtifactStamp) -> Self {
        Self { key, stamp }
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ArtifactKey<P = ProfileId> {
    /// Module dependency graph for one profile.
    ModuleGraph { profile: P },
    /// Language semantic environment for one profile.
    LanguageEnvironment { profile: P },
    /// Intrinsic semantic environment for one profile.
    IntrinsicEnvironment { profile: P },
    /// Library semantic environment for one profile.
    LibraryEnvironment { profile: P },
    /// Parsed module syntax tree.
    Ast { module: ModuleId },
    /// Parsed non-code module data.
    Data { module: ModuleId },
    /// Base DIR.
    DirBase { module: ModuleId },
    /// Profile prepared DIR.
    DirPrepared { module: ModuleId, profile: P },
    /// Resolved DIR.
    DirResolved { module: ModuleId, profile: P },
    /// Declared DIR.
    DirDeclared { module: ModuleId, profile: P },
    /// Interface DIR.
    DirInterface { module: ModuleId, profile: P },
    /// Analyzed DIR.
    DirAnalyzed { module: ModuleId, profile: P },
    /// Elaborated DIR.
    DirElaborated { module: ModuleId, profile: P },
    /// Patched DIR.
    DirPatched { module: ModuleId, profile: P },
    /// Base MIR before optimization.
    MirBase {
        module: ModuleId,
        profile: P,
        target: TargetId,
    },
    /// Optimized MIR.
    MirOptimized {
        module: ModuleId,
        profile: P,
        target: TargetId,
    },
    /// One generated module artifact for one target.
    ModuleOutput { module: ModuleId, target: TargetId },
    /// Output entries for one package target.
    PackageOutput {
        package: PackageId,
        target: TargetId,
    },
    /// Realized lint diagnostics for one module profile.
    ModuleLinted { module: ModuleId, profile: P },
    /// Realized lint diagnostics for one package.
    PackageLinted { package: PackageId },
    /// Realized lint diagnostics for the workspace.
    WorkspaceLinted,
}

impl<P> ArtifactKey<P> {
    /// Return the package referenced by this artifact key when one exists.
    pub fn package_id(&self) -> Option<PackageId> {
        match self {
            Self::PackageOutput { package, .. } | Self::PackageLinted { package } => Some(*package),
            _ => None,
        }
    }

    /// Build one module graph artifact key.
    pub fn module_graph(profile: P) -> Self {
        Self::ModuleGraph { profile }
    }

    /// Build one language environment artifact key.
    pub fn language_environment(profile: P) -> Self {
        Self::LanguageEnvironment { profile }
    }

    /// Build one intrinsic environment artifact key.
    pub fn intrinsic_environment(profile: P) -> Self {
        Self::IntrinsicEnvironment { profile }
    }

    /// Build one library environment artifact key.
    pub fn library_environment(profile: P) -> Self {
        Self::LibraryEnvironment { profile }
    }

    /// Build one AST artifact key.
    pub fn ast(module: ModuleId) -> Self {
        Self::Ast { module }
    }

    /// Build one data artifact key.
    pub fn data(module: ModuleId) -> Self {
        Self::Data { module }
    }

    /// Build one base DIR artifact key.
    pub fn dir_base(module: ModuleId) -> Self {
        Self::DirBase { module }
    }

    /// Build one prepared DIR artifact key.
    pub fn dir_prepared(module: ModuleId, profile: P) -> Self {
        Self::DirPrepared { module, profile }
    }

    /// Build one resolved DIR artifact key.
    pub fn dir_resolved(module: ModuleId, profile: P) -> Self {
        Self::DirResolved { module, profile }
    }

    /// Build one declared DIR artifact key.
    pub fn dir_declared(module: ModuleId, profile: P) -> Self {
        Self::DirDeclared { module, profile }
    }

    /// Build one interface DIR artifact key.
    pub fn dir_interface(module: ModuleId, profile: P) -> Self {
        Self::DirInterface { module, profile }
    }

    /// Build one analyzed DIR artifact key.
    pub fn dir_analyzed(module: ModuleId, profile: P) -> Self {
        Self::DirAnalyzed { module, profile }
    }

    /// Build one elaborated DIR artifact key.
    pub fn dir_elaborated(module: ModuleId, profile: P) -> Self {
        Self::DirElaborated { module, profile }
    }

    /// Build one patched DIR artifact key.
    pub fn dir_patched(module: ModuleId, profile: P) -> Self {
        Self::DirPatched { module, profile }
    }

    /// Build one base MIR artifact key.
    pub fn mir_base(module: ModuleId, profile: P, target: TargetId) -> Self {
        Self::MirBase {
            module,
            profile,
            target,
        }
    }

    /// Build one optimized MIR artifact key.
    pub fn mir_optimized(module: ModuleId, profile: P, target: TargetId) -> Self {
        Self::MirOptimized {
            module,
            profile,
            target,
        }
    }

    /// Build one module artifact key.
    pub fn module_output(module: ModuleId, target: TargetId) -> Self {
        Self::ModuleOutput { module, target }
    }

    /// Build one package output artifact key.
    pub fn package_output(package: PackageId, target: TargetId) -> Self {
        Self::PackageOutput { package, target }
    }

    /// Build one module lint artifact key.
    pub fn module_linted(module: ModuleId, profile: P) -> Self {
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

    /// Return the artifact family for this key.
    pub fn family(&self) -> ArtifactFamily {
        match self {
            Self::ModuleGraph { .. } => ArtifactFamily::ModuleGraph,
            Self::LanguageEnvironment { .. } => ArtifactFamily::LanguageEnvironment,
            Self::IntrinsicEnvironment { .. } => ArtifactFamily::IntrinsicEnvironment,
            Self::LibraryEnvironment { .. } => ArtifactFamily::LibraryEnvironment,
            Self::Ast { .. } => ArtifactFamily::Ast,
            Self::Data { .. } => ArtifactFamily::Data,
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
            Self::ModuleLinted { .. } => ArtifactFamily::ModuleLinted,
            Self::PackageLinted { .. } => ArtifactFamily::PackageLinted,
            Self::WorkspaceLinted => ArtifactFamily::WorkspaceLinted,
        }
    }

    /// Return the provider that owns this key.
    pub fn provider(&self) -> ArtifactProvider {
        match self {
            Self::ModuleLinted { .. } | Self::PackageLinted { .. } | Self::WorkspaceLinted => {
                ArtifactProvider::Linter
            }
            Self::ModuleGraph { .. }
            | Self::LanguageEnvironment { .. }
            | Self::IntrinsicEnvironment { .. }
            | Self::LibraryEnvironment { .. }
            | Self::Ast { .. }
            | Self::DirBase { .. }
            | Self::DirPrepared { .. }
            | Self::DirResolved { .. }
            | Self::DirDeclared { .. }
            | Self::DirInterface { .. }
            | Self::DirAnalyzed { .. }
            | Self::DirElaborated { .. }
            | Self::DirPatched { .. }
            | Self::MirBase { .. }
            | Self::MirOptimized { .. }
            | Self::ModuleOutput { .. }
            | Self::PackageOutput { .. } => ArtifactProvider::Compiler,
        }
    }

    /// Return the stable short name for this key.
    pub fn name(&self) -> &'static str {
        match self {
            Self::ModuleGraph { .. } => "module_graph",
            Self::LanguageEnvironment { .. } => "language_environment",
            Self::IntrinsicEnvironment { .. } => "intrinsic_environment",
            Self::LibraryEnvironment { .. } => "library_environment",
            Self::Ast { .. } => "ast",
            Self::DirBase { .. } => "dir_base",
            Self::DirPrepared { .. } => "dir_prepared",
            Self::DirResolved { .. } => "dir_resolved",
            Self::DirDeclared { .. } => "dir_declared",
            Self::DirInterface { .. } => "dir_interface",
            Self::DirAnalyzed { .. } => "dir_analyzed",
            Self::DirElaborated { .. } => "dir_elaborated",
            Self::DirPatched { .. } => "dir_patched",
            Self::MirBase { .. } => "mir_base",
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
            | Self::ModuleOutput { module, .. }
            | Self::ModuleLinted { module, .. } => Some(*module),
            Self::ModuleGraph { .. }
            | Self::LanguageEnvironment { .. }
            | Self::IntrinsicEnvironment { .. }
            | Self::LibraryEnvironment { .. }
            | Self::PackageOutput { .. }
            | Self::PackageLinted { .. }
            | Self::WorkspaceLinted => None,
        }
    }
}

impl ArtifactKey<ProfileId> {
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
            | Self::MirOptimized { profile, .. }
            | Self::ModuleLinted { profile, .. } => Some(*profile),
            Self::Ast { .. }
            | Self::Data { .. }
            | Self::DirBase { .. }
            | Self::ModuleOutput { .. }
            | Self::PackageOutput { .. }
            | Self::PackageLinted { .. }
            | Self::WorkspaceLinted => None,
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
            Self::Data { module } => ArtifactImageKey::Data { module: *module },
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
            Self::ModuleOutput { module, target } => ArtifactImageKey::ModuleOutput {
                module: *module,
                target: *target,
            },
            Self::PackageOutput { package, target } => ArtifactImageKey::PackageOutput {
                package: *package,
                target: *target,
            },
            Self::ModuleLinted { module, profile } => ArtifactImageKey::ModuleLinted {
                module: *module,
                profile: profile_key_for_id(*profile),
            },
            Self::PackageLinted { package } => {
                ArtifactImageKey::PackageLinted { package: *package }
            }
            Self::WorkspaceLinted => ArtifactImageKey::WorkspaceLinted,
        }
    }
}
