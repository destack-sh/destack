use destack_workspace::{ArtifactKey, Program};

use crate::{
    ArtifactRequirement, ArtifactRequirementSet, DiagnosticAnchor, DiagnosticFormat, TaskError,
};

/// Phase label for diagnostics, tracing, and stats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TaskPhase {
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

impl std::fmt::Display for TaskPhase {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.letter())
    }
}

impl TaskPhase {
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

    /// Return the one-letter code for this phase.
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
    pub const ALL: [TaskPhase; 9] = [
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
    pub fn all() -> impl Iterator<Item = TaskPhase> {
        Self::ALL.into_iter()
    }
}

/// Helper methods for treating artifact keys as scheduler tasks.
pub trait ArtifactTaskKeyExt {
    /// Return the phase label for this artifact key.
    fn phase(&self) -> TaskPhase;

    /// Return the diagnostic anchor for this artifact key.
    fn anchor(&self) -> DiagnosticAnchor;

    /// Return a stable short name for this artifact key.
    fn name(&self) -> &'static str;

    /// Return trace arguments for this artifact key.
    fn trace_args(&self, program: &Program) -> String;
}

impl ArtifactTaskKeyExt for ArtifactKey {
    /// Return the phase label for this artifact key.
    fn phase(&self) -> TaskPhase {
        match self {
            ArtifactKey::ModuleGraph { .. } => TaskPhase::Resolve,
            ArtifactKey::Ast { .. } | ArtifactKey::DirBase { .. } => TaskPhase::Import,
            ArtifactKey::LanguageEnvironment { .. }
            | ArtifactKey::LibraryEnvironment { .. }
            | ArtifactKey::DirPrepared { .. }
            | ArtifactKey::DirResolved { .. } => TaskPhase::Resolve,
            ArtifactKey::IntrinsicEnvironment { .. }
            | ArtifactKey::DirDeclared { .. }
            | ArtifactKey::DirInterface { .. }
            | ArtifactKey::DirAnalyzed { .. } => TaskPhase::Analyze,
            ArtifactKey::DirElaborated { .. } => TaskPhase::Elaborate,
            ArtifactKey::DirPatched { .. } => TaskPhase::Execute,
            ArtifactKey::MirBase { .. } => TaskPhase::Lower,
            ArtifactKey::MirOptimized { .. } => TaskPhase::Optimize,
            ArtifactKey::ModuleOutput { .. } => TaskPhase::Generate,
            ArtifactKey::PackageOutput { .. } => TaskPhase::Link,
        }
    }

    /// Return the diagnostic anchor for this artifact key.
    fn anchor(&self) -> DiagnosticAnchor {
        match self {
            ArtifactKey::ModuleGraph { .. } => DiagnosticAnchor::Global,
            ArtifactKey::Ast { module }
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
            | ArtifactKey::ModuleOutput { module, .. } => DiagnosticAnchor::from(*module),
            ArtifactKey::LanguageEnvironment { .. }
            | ArtifactKey::IntrinsicEnvironment { .. }
            | ArtifactKey::LibraryEnvironment { .. } => DiagnosticAnchor::Global,
            ArtifactKey::PackageOutput { package, .. } => DiagnosticAnchor::from(*package),
        }
    }

    /// Return a stable short name for this artifact key.
    fn name(&self) -> &'static str {
        match self {
            ArtifactKey::ModuleGraph { .. } => "module_graph",
            ArtifactKey::Ast { .. } => "ast",
            ArtifactKey::DirBase { .. } => "dir_base",
            ArtifactKey::LanguageEnvironment { .. } => "language_environment",
            ArtifactKey::IntrinsicEnvironment { .. } => "intrinsic_environment",
            ArtifactKey::LibraryEnvironment { .. } => "library_environment",
            ArtifactKey::DirPrepared { .. } => "dir_prepared",
            ArtifactKey::DirResolved { .. } => "dir_resolved",
            ArtifactKey::DirDeclared { .. } => "dir_declared",
            ArtifactKey::DirInterface { .. } => "dir_interface",
            ArtifactKey::DirAnalyzed { .. } => "dir_analyzed",
            ArtifactKey::DirElaborated { .. } => "dir_elaborated",
            ArtifactKey::DirPatched { .. } => "dir_patched",
            ArtifactKey::MirBase { .. } => "mir_base",
            ArtifactKey::MirOptimized { .. } => "mir_optimized",
            ArtifactKey::ModuleOutput { .. } => "module_output",
            ArtifactKey::PackageOutput { .. } => "package_output",
        }
    }

    /// Return trace arguments for this artifact key.
    fn trace_args(&self, program: &Program) -> String {
        match self {
            ArtifactKey::ModuleGraph { profile } => {
                let profile = profile.diagnostic_fmt(program);
                format!("profile={profile}")
            }
            ArtifactKey::Ast { module } | ArtifactKey::DirBase { module } => {
                let module = module.diagnostic_fmt(program);
                format!("module={module}")
            }
            ArtifactKey::LanguageEnvironment { profile }
            | ArtifactKey::IntrinsicEnvironment { profile }
            | ArtifactKey::LibraryEnvironment { profile } => {
                let profile = profile.diagnostic_fmt(program);
                format!("profile={profile}")
            }
            ArtifactKey::DirPrepared { module, profile }
            | ArtifactKey::DirResolved { module, profile }
            | ArtifactKey::DirDeclared { module, profile }
            | ArtifactKey::DirInterface { module, profile }
            | ArtifactKey::DirAnalyzed { module, profile }
            | ArtifactKey::DirElaborated { module, profile }
            | ArtifactKey::DirPatched { module, profile } => {
                let module = module.diagnostic_fmt(program);
                let profile = profile.diagnostic_fmt(program);
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
                let module = module.diagnostic_fmt(program);
                let profile = profile.diagnostic_fmt(program);
                let target = target.diagnostic_fmt(program);
                format!("module={module} profile={profile} target={target}")
            }
            ArtifactKey::ModuleOutput { module, target } => {
                let target = target.diagnostic_fmt(program);
                let module = module.diagnostic_fmt(program);
                format!("module={module} target={target}")
            }
            ArtifactKey::PackageOutput { package, target } => {
                let package = package.diagnostic_fmt(program);
                let target = target.diagnostic_fmt(program);
                format!("package={package} target={target}")
            }
        }
    }
}

/// Id for a compiler task.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct TaskId(pub u32);

impl std::fmt::Debug for TaskId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "#{}", self.0)
    }
}

impl std::fmt::Display for TaskId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "#{}", self.0)
    }
}

impl TaskId {
    /// Create a new task id.
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    /// Return the numeric code of this id.
    pub fn code(&self) -> u32 {
        self.0
    }
}

/// Status of one task.
#[derive(Debug, Clone, PartialEq)]
pub enum TaskStatus {
    /// The task is queued.
    Queued,
    /// The task is running.
    Running,
    /// The task is waiting for more artifact requirements.
    Yielded { requirement: ArtifactRequirementSet },
    /// The task was skipped because the running attempt became obsolete.
    Skipped,
    /// The task completed successfully.
    Complete,
    /// The task failed.
    Failed { error: TaskError },
}

impl TaskStatus {
    /// Return true when this status is final.
    pub fn is_final(&self) -> bool {
        matches!(self, Self::Complete | Self::Skipped | Self::Failed { .. })
    }

    /// Return true when this status is an outcome.
    pub fn is_outcome(&self) -> bool {
        matches!(
            self,
            Self::Yielded { .. } | Self::Complete | Self::Skipped | Self::Failed { .. }
        )
    }
}

impl From<TaskOutcome> for TaskStatus {
    fn from(outcome: TaskOutcome) -> Self {
        match outcome {
            TaskOutcome::Yield { requirement } => Self::Yielded { requirement },
            TaskOutcome::Skipped => Self::Skipped,
            TaskOutcome::Error { error } => Self::Failed { error },
            TaskOutcome::Complete { .. } => Self::Complete,
        }
    }
}

/// Handle for one task.
#[derive(Debug, Clone)]
pub struct TaskHandle {
    /// The task id.
    pub id: TaskId,
    /// The current task status.
    pub status: TaskStatus,
    /// The previous outcome for repeat-yield detection.
    pub last_outcome: Option<TaskOutcome>,
    /// The artifact key realized by this task.
    pub artifact_key: ArtifactKey,
    /// The number of times this task has yielded.
    pub yield_count: u32,
    /// The exact requirements satisfied by the last completed build.
    pub final_requirements: Vec<ArtifactRequirement>,
}

impl TaskHandle {
    /// Create a new task handle.
    pub fn new(id: TaskId, artifact_key: ArtifactKey) -> Self {
        Self {
            id,
            status: TaskStatus::Queued,
            last_outcome: None,
            artifact_key,
            yield_count: 0,
            final_requirements: Vec::new(),
        }
    }

    /// Return the phase label for this task.
    pub fn phase(&self) -> TaskPhase {
        self.artifact_key.phase()
    }
}

/// One step outcome for a task execution attempt.
#[derive(Debug, Clone)]
pub enum TaskOutcome {
    /// The task yielded more requirements.
    Yield { requirement: ArtifactRequirementSet },
    /// The task became obsolete while running.
    Skipped,
    /// The task failed.
    Error { error: TaskError },
    /// The task completed successfully.
    Complete,
}

impl<E> From<Result<(), E>> for TaskOutcome
where
    E: Into<TaskError> + TaskSkip,
    E: TryInto<ArtifactRequirementSet, Error = E>,
{
    fn from(result: Result<(), E>) -> Self {
        match result {
            Ok(()) => Self::Complete,
            Err(error) => {
                if error.is_skipped() {
                    return Self::Skipped;
                }

                match error.try_into() {
                    Ok(requirement) => Self::Yield { requirement },
                    Err(error) => Self::Error {
                        error: error.into(),
                    },
                }
            }
        }
    }
}

impl TaskOutcome {
    /// Return true when this outcome is final.
    pub fn is_final(&self) -> bool {
        !matches!(self, Self::Yield { .. })
    }
}

/// Return whether an error represents obsolete work.
pub trait TaskSkip {
    /// Return whether this error marks the current task obsolete.
    fn is_skipped(&self) -> bool;
}

/// Build phase errors that mark work as obsolete.
pub trait TaskSkipError: Sized {
    /// Create one skipped error.
    fn skipped() -> Self;
}
