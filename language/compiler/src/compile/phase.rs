use destack_artifact::{ArtifactKey, ArtifactStore};
use destack_workspace::{Repository, Revision};

use crate::{DiagnosticAnchor, DiagnosticFormat};

/// Phase label for diagnostics, tracing, and stats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum CompilePhase {
    /// Import, parse, and bind source into base DIR.
    Import = 1,
    /// Resolve symbol references and profile environments.
    Resolve = 2,
    /// Declare, interface, analyze, and validate DIR.
    Analyze = 3,
    /// Elaborate analyzed DIR into lowered DIR form.
    Elaborate = 4,
    /// Execute comptime and patch DIR.
    Execute = 5,
    /// Lower patched DIR into MIR.
    Lower = 6,
    /// Optimize MIR.
    Optimize = 7,
    /// Generate emitted artifacts from compiler products.
    Generate = 8,
    /// Link emitted artifacts into package artifacts.
    Link = 9,
}

impl std::fmt::Display for CompilePhase {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.letter())
    }
}

impl CompilePhase {
    /// Return the numeric code for this phase.
    pub fn code(&self) -> u8 {
        *self as u8
    }

    /// Return the display name for this phase.
    pub fn name(&self) -> &str {
        match self {
            Self::Import => "import",
            Self::Resolve => "resolve",
            Self::Analyze => "analyze",
            Self::Elaborate => "elaborate",
            Self::Execute => "execute",
            Self::Lower => "lower",
            Self::Optimize => "optimize",
            Self::Generate => "generate",
            Self::Link => "link",
        }
    }

    /// Return the description for this phase.
    pub fn description(&self) -> &str {
        match self {
            Self::Import => "import, parse, and bind source into DIR",
            Self::Resolve => "resolve symbol references and build semantic environments",
            Self::Analyze => "declare, interface, analyze, and validate",
            Self::Elaborate => "desugar and reify DIR",
            Self::Execute => "execute comptime code and patch DIR",
            Self::Lower => "lower DIR into MIR",
            Self::Optimize => "optimize MIR",
            Self::Generate => "generate emitted artifacts",
            Self::Link => "link emitted artifacts",
        }
    }

    /// Return the one letter code for this phase.
    pub fn letter(&self) -> char {
        match self {
            Self::Import => 'I',
            Self::Resolve => 'R',
            Self::Analyze => 'A',
            Self::Elaborate => 'E',
            Self::Execute => 'X',
            Self::Lower => 'M',
            Self::Optimize => 'O',
            Self::Generate => 'G',
            Self::Link => 'K',
        }
    }

    /// All phases in build order.
    pub const ALL: [CompilePhase; 9] = [
        Self::Import,
        Self::Resolve,
        Self::Analyze,
        Self::Elaborate,
        Self::Execute,
        Self::Lower,
        Self::Optimize,
        Self::Generate,
        Self::Link,
    ];

    /// Iterate over all phases in build order.
    pub fn all() -> impl Iterator<Item = CompilePhase> {
        Self::ALL.into_iter()
    }
}

/// Helper methods for treating artifact keys as compiler work items.
pub trait ArtifactCompileKeyExt {
    /// Return the phase label for this artifact key.
    fn phase(&self) -> CompilePhase;

    /// Return the diagnostic anchor for this artifact key.
    fn anchor(&self) -> DiagnosticAnchor;

    /// Return trace arguments for this artifact key.
    fn trace_args(
        &self,
        revision: Revision,
        repository: &Repository,
        artifacts: &ArtifactStore,
    ) -> String;
}

impl ArtifactCompileKeyExt for ArtifactKey {
    /// Return the phase label for this artifact key.
    fn phase(&self) -> CompilePhase {
        match self {
            ArtifactKey::ModuleGraph { .. } => CompilePhase::Resolve,
            ArtifactKey::Ast { .. } | ArtifactKey::Data { .. } | ArtifactKey::DirBase { .. } => {
                CompilePhase::Import
            }
            ArtifactKey::LanguageEnvironment { .. }
            | ArtifactKey::LibraryEnvironment { .. }
            | ArtifactKey::DirPrepared { .. }
            | ArtifactKey::DirResolved { .. } => CompilePhase::Resolve,
            ArtifactKey::IntrinsicEnvironment { .. }
            | ArtifactKey::DirDeclared { .. }
            | ArtifactKey::DirInterface { .. }
            | ArtifactKey::DirAnalyzed { .. }
            | ArtifactKey::ModuleLinted { .. }
            | ArtifactKey::PackageLinted { .. }
            | ArtifactKey::WorkspaceLinted => CompilePhase::Analyze,
            ArtifactKey::DirElaborated { .. } => CompilePhase::Elaborate,
            ArtifactKey::DirPatched { .. } => CompilePhase::Execute,
            ArtifactKey::MirBase { .. } => CompilePhase::Lower,
            ArtifactKey::MirOptimized { .. } => CompilePhase::Optimize,
            ArtifactKey::ModuleOutput { .. } => CompilePhase::Generate,
            ArtifactKey::PackageOutput { .. } => CompilePhase::Link,
        }
    }

    /// Return the diagnostic anchor for this artifact key.
    fn anchor(&self) -> DiagnosticAnchor {
        match self {
            ArtifactKey::ModuleGraph { .. } => DiagnosticAnchor::Global,
            ArtifactKey::Ast { module }
            | ArtifactKey::Data { module }
            | ArtifactKey::DirBase { module }
            | ArtifactKey::DirPrepared { module, .. }
            | ArtifactKey::DirResolved { module, .. }
            | ArtifactKey::DirDeclared { module, .. }
            | ArtifactKey::DirInterface { module, .. }
            | ArtifactKey::DirAnalyzed { module, .. }
            | ArtifactKey::DirElaborated { module, .. }
            | ArtifactKey::DirPatched { module, .. }
            | ArtifactKey::MirBase { module, .. }
            | ArtifactKey::MirOptimized { module, .. }
            | ArtifactKey::ModuleOutput { module, .. }
            | ArtifactKey::ModuleLinted { module, .. } => DiagnosticAnchor::from(*module),
            ArtifactKey::LanguageEnvironment { .. }
            | ArtifactKey::IntrinsicEnvironment { .. }
            | ArtifactKey::LibraryEnvironment { .. }
            | ArtifactKey::WorkspaceLinted => DiagnosticAnchor::Global,
            ArtifactKey::PackageOutput { package, .. } | ArtifactKey::PackageLinted { package } => {
                DiagnosticAnchor::from(*package)
            }
        }
    }

    /// Return trace arguments for this artifact key.
    fn trace_args(
        &self,
        revision: Revision,
        repository: &Repository,
        artifacts: &ArtifactStore,
    ) -> String {
        match self {
            ArtifactKey::ModuleGraph { profile } => {
                let profile = profile.diagnostic_fmt(revision, repository, artifacts);
                format!("profile={profile}")
            }
            ArtifactKey::Ast { module }
            | ArtifactKey::Data { module }
            | ArtifactKey::DirBase { module } => {
                let module = module.diagnostic_fmt(revision, repository, artifacts);
                format!("module={module}")
            }
            ArtifactKey::LanguageEnvironment { profile }
            | ArtifactKey::IntrinsicEnvironment { profile }
            | ArtifactKey::LibraryEnvironment { profile } => {
                let profile = profile.diagnostic_fmt(revision, repository, artifacts);
                format!("profile={profile}")
            }
            ArtifactKey::DirPrepared { module, profile }
            | ArtifactKey::DirResolved { module, profile }
            | ArtifactKey::DirDeclared { module, profile }
            | ArtifactKey::DirInterface { module, profile }
            | ArtifactKey::DirAnalyzed { module, profile }
            | ArtifactKey::DirElaborated { module, profile }
            | ArtifactKey::DirPatched { module, profile }
            | ArtifactKey::ModuleLinted { module, profile } => {
                let module = module.diagnostic_fmt(revision, repository, artifacts);
                let profile = profile.diagnostic_fmt(revision, repository, artifacts);
                format!("module={module} profile={profile}")
            }
            ArtifactKey::MirBase {
                module,
                profile,
                target,
            }
            | ArtifactKey::MirOptimized {
                module,
                profile,
                target,
            } => {
                let module = module.diagnostic_fmt(revision, repository, artifacts);
                let profile = profile.diagnostic_fmt(revision, repository, artifacts);
                let target = target.diagnostic_fmt(revision, repository, artifacts);
                format!("module={module} profile={profile} target={target}")
            }
            ArtifactKey::ModuleOutput { module, target } => {
                let target = target.diagnostic_fmt(revision, repository, artifacts);
                let module = module.diagnostic_fmt(revision, repository, artifacts);
                format!("module={module} target={target}")
            }
            ArtifactKey::PackageOutput { package, target } => {
                let package = package.diagnostic_fmt(revision, repository, artifacts);
                let target = target.diagnostic_fmt(revision, repository, artifacts);
                format!("package={package} target={target}")
            }
            ArtifactKey::PackageLinted { package } => {
                let package = package.diagnostic_fmt(revision, repository, artifacts);
                format!("package={package}")
            }
            ArtifactKey::WorkspaceLinted => String::new(),
        }
    }
}

/// Return whether an error represents obsolete work.
pub trait ObsoleteWork {
    /// Return whether this error marks the current work obsolete.
    fn is_skipped(&self) -> bool;
}

/// Build phase errors that mark work as obsolete.
pub trait ObsoleteWorkError: Sized {
    /// Create one skipped error.
    fn skipped() -> Self;
}
