use destack_workspace::{ArtifactKey, OutputKey, OutputScope, Program};

use crate::{BuildKey, BuildRequirementSet, DiagnosticAnchor, DiagnosticFormat, TaskError};

/// Region of the compiler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaskRegion {
    /// Front-end work.
    Front,
    /// Middle-end work.
    Middle,
    /// Back-end work.
    Back,
}

impl TaskRegion {
    /// Return the display name for this region.
    pub fn name(&self) -> &str {
        match self {
            Self::Front => "front-end",
            Self::Middle => "middle-end",
            Self::Back => "back-end",
        }
    }
}

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
    /// Generate build products from compiler products.
    Generate = 8,
    /// Link generated products into final outputs.
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

    /// Return the region for this phase.
    pub fn region(&self) -> TaskRegion {
        match self {
            Self::Import | Self::Resolve | Self::Analyze | Self::Elaborate => TaskRegion::Front,
            Self::Execute | Self::Lower | Self::Optimize => TaskRegion::Middle,
            Self::Generate | Self::Link => TaskRegion::Back,
        }
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
            Self::Generate => "generate build products",
            Self::Link => "link build products",
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

/// One in-flight build for one build key.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Task {
    /// The build key this task realizes.
    pub key: BuildKey,
}

impl Task {
    /// Create a new task for one build key.
    pub fn new(key: BuildKey) -> Self {
        Self { key }
    }

    /// Return the build key realized by this task.
    pub fn build_key(&self) -> &BuildKey {
        &self.key
    }

    /// Return the phase label for this task.
    pub fn phase(&self) -> TaskPhase {
        match &self.key {
            BuildKey::Artifact(ArtifactKey::Ast { .. } | ArtifactKey::DirBase { .. }) => {
                TaskPhase::Import
            }
            BuildKey::Artifact(
                ArtifactKey::LanguageEnvironment { .. }
                | ArtifactKey::LibEnvironment { .. }
                | ArtifactKey::DirPrepared { .. }
                | ArtifactKey::DirResolved { .. },
            ) => TaskPhase::Resolve,
            BuildKey::Artifact(
                ArtifactKey::IntrinsicEnvironment { .. }
                | ArtifactKey::DirDeclared { .. }
                | ArtifactKey::DirInterface { .. }
                | ArtifactKey::DirAnalyzed { .. },
            ) => TaskPhase::Analyze,
            BuildKey::Artifact(ArtifactKey::DirElaborated { .. }) => TaskPhase::Elaborate,
            BuildKey::Artifact(ArtifactKey::DirPatched { .. }) => TaskPhase::Execute,
            BuildKey::Artifact(ArtifactKey::Mir { .. }) => TaskPhase::Lower,
            BuildKey::Artifact(ArtifactKey::MirOptimized { .. }) => TaskPhase::Optimize,
            BuildKey::Output(OutputKey {
                scope: OutputScope::Module(..),
                ..
            }) => TaskPhase::Generate,
            BuildKey::Output(OutputKey {
                scope: OutputScope::Package(..),
                ..
            }) => TaskPhase::Link,
        }
    }

    /// Return the diagnostic anchor for this task.
    pub fn anchor(&self) -> DiagnosticAnchor {
        match &self.key {
            BuildKey::Artifact(ArtifactKey::Ast { module })
            | BuildKey::Artifact(ArtifactKey::DirBase { module })
            | BuildKey::Artifact(ArtifactKey::DirPrepared { module, .. })
            | BuildKey::Artifact(ArtifactKey::DirResolved { module, .. })
            | BuildKey::Artifact(ArtifactKey::DirDeclared { module, .. })
            | BuildKey::Artifact(ArtifactKey::DirInterface { module, .. })
            | BuildKey::Artifact(ArtifactKey::DirAnalyzed { module, .. })
            | BuildKey::Artifact(ArtifactKey::DirElaborated { module, .. })
            | BuildKey::Artifact(ArtifactKey::DirPatched { module, .. })
            | BuildKey::Artifact(ArtifactKey::Mir { module, .. })
            | BuildKey::Artifact(ArtifactKey::MirOptimized { module, .. }) => {
                DiagnosticAnchor::from(*module)
            }
            BuildKey::Artifact(ArtifactKey::LanguageEnvironment { .. })
            | BuildKey::Artifact(ArtifactKey::IntrinsicEnvironment { .. })
            | BuildKey::Artifact(ArtifactKey::LibEnvironment { .. }) => DiagnosticAnchor::Global,
            BuildKey::Output(OutputKey {
                scope: OutputScope::Module(module),
                ..
            }) => DiagnosticAnchor::from(*module),
            BuildKey::Output(OutputKey {
                scope: OutputScope::Package(package),
                ..
            }) => DiagnosticAnchor::from(*package),
        }
    }

    /// Return the telemetry region for this task.
    pub fn region(&self) -> TaskRegion {
        self.phase().region()
    }

    /// Return a stable short task name.
    pub fn name(&self) -> &'static str {
        match &self.key {
            BuildKey::Artifact(ArtifactKey::Ast { .. }) => "ast",
            BuildKey::Artifact(ArtifactKey::DirBase { .. }) => "dir_base",
            BuildKey::Artifact(ArtifactKey::LanguageEnvironment { .. }) => "language_environment",
            BuildKey::Artifact(ArtifactKey::IntrinsicEnvironment { .. }) => "intrinsic_environment",
            BuildKey::Artifact(ArtifactKey::LibEnvironment { .. }) => "lib_environment",
            BuildKey::Artifact(ArtifactKey::DirPrepared { .. }) => "dir_prepared",
            BuildKey::Artifact(ArtifactKey::DirResolved { .. }) => "dir_resolved",
            BuildKey::Artifact(ArtifactKey::DirDeclared { .. }) => "dir_declared",
            BuildKey::Artifact(ArtifactKey::DirInterface { .. }) => "dir_interface",
            BuildKey::Artifact(ArtifactKey::DirAnalyzed { .. }) => "dir_analyzed",
            BuildKey::Artifact(ArtifactKey::DirElaborated { .. }) => "dir_elaborated",
            BuildKey::Artifact(ArtifactKey::DirPatched { .. }) => "dir_patched",
            BuildKey::Artifact(ArtifactKey::Mir { .. }) => "mir",
            BuildKey::Artifact(ArtifactKey::MirOptimized { .. }) => "mir_optimized",
            BuildKey::Output(OutputKey {
                scope: OutputScope::Module(..),
                ..
            }) => "module_output",
            BuildKey::Output(OutputKey {
                scope: OutputScope::Package(..),
                ..
            }) => "package_output",
        }
    }

    /// Return trace arguments for this task.
    pub fn trace_args(&self, program: &Program) -> String {
        match &self.key {
            BuildKey::Artifact(ArtifactKey::Ast { module })
            | BuildKey::Artifact(ArtifactKey::DirBase { module }) => {
                let module = module.diagnostic_fmt(program);
                format!("module={module}")
            }
            BuildKey::Artifact(ArtifactKey::LanguageEnvironment { profile })
            | BuildKey::Artifact(ArtifactKey::IntrinsicEnvironment { profile })
            | BuildKey::Artifact(ArtifactKey::LibEnvironment { profile }) => {
                let profile = profile.diagnostic_fmt(program);
                format!("profile={profile}")
            }
            BuildKey::Artifact(ArtifactKey::DirPrepared { module, profile })
            | BuildKey::Artifact(ArtifactKey::DirResolved { module, profile })
            | BuildKey::Artifact(ArtifactKey::DirDeclared { module, profile })
            | BuildKey::Artifact(ArtifactKey::DirInterface { module, profile })
            | BuildKey::Artifact(ArtifactKey::DirAnalyzed { module, profile })
            | BuildKey::Artifact(ArtifactKey::DirElaborated { module, profile })
            | BuildKey::Artifact(ArtifactKey::DirPatched { module, profile }) => {
                let module = module.diagnostic_fmt(program);
                let profile = profile.diagnostic_fmt(program);
                format!("module={module} profile={profile}")
            }
            BuildKey::Artifact(ArtifactKey::Mir {
                module,
                profile,
                target,
            })
            | BuildKey::Artifact(ArtifactKey::MirOptimized {
                module,
                profile,
                target,
            }) => {
                let module = module.diagnostic_fmt(program);
                let profile = profile.diagnostic_fmt(program);
                let target = target.diagnostic_fmt(program);
                format!("module={module} profile={profile} target={target}")
            }
            BuildKey::Output(OutputKey { scope, target }) => {
                let target = target.diagnostic_fmt(program);
                match scope {
                    OutputScope::Module(module) => {
                        let module = module.diagnostic_fmt(program);
                        format!("module={module} target={target}")
                    }
                    OutputScope::Package(package) => {
                        let package = package.diagnostic_fmt(program);
                        format!("package={package} target={target}")
                    }
                }
            }
        }
    }
}

impl From<BuildKey> for Task {
    fn from(key: BuildKey) -> Self {
        Self::new(key)
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
    /// The task is waiting for more build requirements.
    Yielded { requirement: BuildRequirementSet },
    /// The task was skipped because the running attempt became obsolete.
    Skipped { reason: TaskSkipReason },
    /// The task completed successfully.
    Complete,
    /// The task failed.
    Failed { error: TaskError },
}

impl TaskStatus {
    /// Return true when this status is final.
    pub fn is_final(&self) -> bool {
        matches!(
            self,
            Self::Complete | Self::Skipped { .. } | Self::Failed { .. }
        )
    }

    /// Return true when this status is an outcome.
    pub fn is_outcome(&self) -> bool {
        matches!(
            self,
            Self::Yielded { .. } | Self::Complete | Self::Skipped { .. } | Self::Failed { .. }
        )
    }
}

impl From<TaskOutcome> for TaskStatus {
    fn from(outcome: TaskOutcome) -> Self {
        match outcome {
            TaskOutcome::Yield { requirement } => Self::Yielded { requirement },
            TaskOutcome::Skipped { reason } => Self::Skipped { reason },
            TaskOutcome::Error { error } => Self::Failed { error },
            TaskOutcome::Complete => Self::Complete,
        }
    }
}

/// Handle for one task.
#[derive(Debug, Clone, PartialEq)]
pub struct TaskHandle {
    /// The task id.
    pub id: TaskId,
    /// The current task status.
    pub status: TaskStatus,
    /// The previous outcome for repeat-yield detection.
    pub last_outcome: Option<TaskOutcome>,
    /// The task itself.
    pub task: Task,
    /// The number of times this task has yielded.
    pub yield_count: u32,
}

impl TaskHandle {
    /// Create a new task handle.
    pub fn new(id: TaskId, task: Task) -> Self {
        Self {
            id,
            status: TaskStatus::Queued,
            last_outcome: None,
            task,
            yield_count: 0,
        }
    }

    /// Return the phase label for this task.
    pub fn phase(&self) -> TaskPhase {
        self.task.phase()
    }

    /// Return the region label for this task.
    pub fn region(&self) -> TaskRegion {
        self.task.region()
    }
}

/// One step outcome for a task execution attempt.
#[derive(Debug, Clone, PartialEq)]
pub enum TaskOutcome {
    /// The task yielded more requirements.
    Yield { requirement: BuildRequirementSet },
    /// The task became obsolete while running.
    Skipped { reason: TaskSkipReason },
    /// The task failed.
    Error { error: TaskError },
    /// The task completed successfully.
    Complete,
}

impl<E> From<Result<(), E>> for TaskOutcome
where
    E: Into<TaskError> + TaskSkip,
    E: TryInto<BuildRequirementSet, Error = E>,
{
    fn from(result: Result<(), E>) -> Self {
        match result {
            Ok(()) => Self::Complete,
            Err(error) => {
                if let Some(reason) = error.skip_reason() {
                    return Self::Skipped { reason };
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

/// Reason a running task became obsolete.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskSkipReason {
    /// The module changed while the task was running.
    StaleModuleVersion,
    /// The profile changed while the task was running.
    StaleProfileVersion,
    /// The module graph changed while the task was running.
    StaleModuleGraphVersion,
}

/// Return a skip reason for errors that represent obsolete work.
pub trait TaskSkip {
    /// Return the skip reason, if any.
    fn skip_reason(&self) -> Option<TaskSkipReason>;
}

/// Build phase errors that mark work as obsolete.
pub trait TaskSkipError: Sized {
    /// Create one skipped error for the given reason.
    fn skipped(reason: TaskSkipReason) -> Self;
}
