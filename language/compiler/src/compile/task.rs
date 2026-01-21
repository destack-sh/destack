use destack_workspace::Program;

use crate::DiagnosticAnchor;

use crate::{
    AnalyzeTask, Compiler, ElaborateTask, EmitTask, ExecuteTask, GenerateTask, ImportTask,
    LinkTask, LintTask, LowerTask, OptimizeTask, ResolveTask, TaskError,
};

/// Trait for formatting task information.
pub trait TaskDebug {
    /// Get the task variant name (e.g., "file", "module", "specifier").
    fn name(&self) -> &'static str;

    /// Format the task arguments for tracing (resolving ids, making the arguments readable, etc.).
    fn trace_args(&self, program: &Program) -> String;
}

/// Trait for task staleness checks.
pub trait TaskSkipCheck {
    /// Return a skip reason if the task is stale.
    fn skip_reason(&self, compiler: &Compiler) -> Option<TaskSkipReason>;
}

/// Region of the compiler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaskRegion {
    /// Front-end (import, resolve, analyze, elaborate).
    Front,
    /// Middle-end (execute, lower, optimize).
    Middle,
    /// Back-end (generate, link, emit).
    Back,
    /// Lint.
    Lint,
}

impl TaskRegion {
    /// Get the name of the region.
    pub fn name(&self) -> &str {
        match self {
            Self::Front => "front-end",
            Self::Middle => "middle-end",
            Self::Back => "back-end",
            Self::Lint => "lint",
        }
    }
}

/// Phase of the compiler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TaskPhase {
    /// Import, parse, and bind source into DIR.
    Import = 1,
    /// Resolve symbol references in DIR.
    Resolve = 2,
    /// Infer types, resolve overloads, validate.
    Analyze = 3,
    /// Desugar, impute overloads, reify DIR.
    Elaborate = 4,
    // --------------------------------------------------
    /// Execute comptime code and patch DIR before target lowering.
    Execute = 5,
    /// Lower patched DIR into MIR.
    Lower = 6,
    /// Optimize MIR.
    Optimize = 7,
    // --------------------------------------------------
    /// Generate DIR or MIR into artifacts.
    Generate = 8,
    /// Link artifacts into final output.
    Link = 9,
    /// Emit linked output to disk.
    Emit = 10,
    // --------------------------------------------------
    /// Lint the program.
    Lint = 11,
}

impl std::fmt::Display for TaskPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.letter())
    }
}

impl TaskPhase {
    /// Get the numeric code of the phase.
    pub fn code(&self) -> u8 {
        *self as u8
    }

    /// Get the region of the phase.
    pub fn region(&self) -> TaskRegion {
        match self {
            Self::Import | Self::Resolve | Self::Analyze | Self::Elaborate => TaskRegion::Front,
            Self::Execute | Self::Lower | Self::Optimize => TaskRegion::Middle,
            Self::Generate | Self::Link | Self::Emit => TaskRegion::Back,
            Self::Lint => TaskRegion::Lint,
        }
    }

    /// Get the name of the phase.
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
            Self::Emit => "emit",
            Self::Lint => "lint",
        }
    }

    /// Get the description of the phase.
    pub fn description(&self) -> &str {
        match self {
            Self::Import => "import, parse, and bind source into DIR",
            Self::Resolve => "resolve symbol references in DIR",
            Self::Analyze => "infer types, resolve overloads, validate",
            Self::Elaborate => "desugar, resolve overloads, reify",
            Self::Execute => "execute comptime code and patch DIR",
            Self::Lower => "lower DIR into MIR",
            Self::Optimize => "optimize MIR",
            Self::Generate => "generate DIR or MIR into artifacts",
            Self::Link => "link artifacts into final output",
            Self::Emit => "emit linked output to disk",
            Self::Lint => "lint the program",
        }
    }

    /// Get the letter of the phase.
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
            Self::Emit => 'W',
            Self::Lint => 'L',
        }
    }

    /// All phases in compilation order.
    pub const ALL: [TaskPhase; 11] = [
        Self::Import,
        Self::Resolve,
        Self::Analyze,
        Self::Elaborate,
        Self::Execute,
        Self::Lower,
        Self::Optimize,
        Self::Generate,
        Self::Link,
        Self::Emit,
        Self::Lint,
    ];

    /// Iterate over all phases in compilation order.
    pub fn all() -> impl Iterator<Item = TaskPhase> {
        Self::ALL.into_iter()
    }
}

/// Task for the compiler during compilation.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Task {
    /// Import, parse, and bind source into DIR.
    Import(ImportTask),
    /// Resolve symbol references in DIR.
    Resolve(ResolveTask),
    /// Infer types, resolve overloads, validate.
    Analyze(AnalyzeTask),
    /// Desugar, resolve overloads, reify DIR.
    Elaborate(ElaborateTask),
    // --------------------------------------------------
    /// Execute comptime code.
    Execute(ExecuteTask),
    /// Lower DIR into MIR.
    Lower(LowerTask),
    /// Optimize MIR.
    Optimize(OptimizeTask),
    // --------------------------------------------------
    /// Generate DIR or MIR into artifacts.
    Generate(GenerateTask),
    /// Link artifacts into final output.
    Link(LinkTask),
    /// Emit linked output to disk.
    Emit(EmitTask),
    // --------------------------------------------------
    /// Lint the program.
    Lint(LintTask),
}

impl Task {
    /// Get the phase of the task.
    pub fn phase(&self) -> TaskPhase {
        match self {
            Self::Import(_) => TaskPhase::Import,
            Self::Resolve(_) => TaskPhase::Resolve,
            Self::Analyze(_) => TaskPhase::Analyze,
            Self::Elaborate(_) => TaskPhase::Elaborate,
            Self::Execute(_) => TaskPhase::Execute,
            Self::Lower(_) => TaskPhase::Lower,
            Self::Optimize(_) => TaskPhase::Optimize,
            Self::Generate(_) => TaskPhase::Generate,
            Self::Link(_) => TaskPhase::Link,
            Self::Emit(_) => TaskPhase::Emit,
            Self::Lint(_) => TaskPhase::Lint,
        }
    }

    /// Get the diagnostic anchor for this task.
    pub fn anchor(&self) -> DiagnosticAnchor {
        match self {
            Self::Import(t) => t.anchor(),
            Self::Resolve(t) => t.anchor(),
            Self::Analyze(t) => t.anchor(),
            Self::Elaborate(t) => t.anchor(),
            Self::Execute(t) => t.anchor(),
            Self::Lower(t) => t.anchor(),
            Self::Optimize(t) => t.anchor(),
            Self::Generate(t) => t.anchor(),
            Self::Link(t) => t.anchor(),
            Self::Emit(t) => t.anchor(),
            Self::Lint(t) => t.anchor(),
        }
    }

    /// Get the region of the task.
    pub fn region(&self) -> TaskRegion {
        self.phase().region()
    }

    /// Get the sub code of the task.
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::Import(task) => task.sub_code(),
            Self::Resolve(task) => task.sub_code(),
            Self::Analyze(task) => task.sub_code(),
            Self::Elaborate(task) => task.sub_code(),
            Self::Execute(task) => task.sub_code(),
            Self::Lower(task) => task.sub_code(),
            Self::Optimize(task) => task.sub_code(),
            Self::Generate(task) => task.sub_code(),
            Self::Link(task) => task.sub_code(),
            Self::Emit(task) => task.sub_code(),
            Self::Lint(task) => task.sub_code(),
        }
    }

    /// Get the full code of the task.
    pub fn full_code(&self) -> String {
        format!("T{}{:03}", self.phase().letter(), self.sub_code())
    }
}

impl TaskSkipCheck for Task {
    fn skip_reason(&self, compiler: &Compiler) -> Option<TaskSkipReason> {
        let reason = match self {
            Self::Import(task) => task.skip_reason(compiler),
            Self::Resolve(task) => task.skip_reason(compiler),
            Self::Analyze(task) => task.skip_reason(compiler),
            Self::Elaborate(task) => task.skip_reason(compiler),
            Self::Execute(task) => task.skip_reason(compiler),
            Self::Lower(task) => task.skip_reason(compiler),
            Self::Optimize(task) => task.skip_reason(compiler),
            Self::Generate(task) => task.skip_reason(compiler),
            Self::Link(task) => task.skip_reason(compiler),
            Self::Emit(task) => task.skip_reason(compiler),
            Self::Lint(task) => task.skip_reason(compiler),
        };
        if reason.is_some() {
            return reason;
        }

        if let Self::Emit(EmitTask::EmitModule {
            module, package, ..
        }) = self
        {
            let module_package = compiler.program.modules.get(module.id).read().package_id;
            if module_package != package.id {
                return Some(TaskSkipReason::StalePackageVersion);
            }
        }

        None
    }
}

impl TaskDebug for Task {
    fn name(&self) -> &'static str {
        match self {
            Self::Import(task) => task.name(),
            Self::Resolve(task) => task.name(),
            Self::Analyze(task) => task.name(),
            Self::Elaborate(task) => task.name(),
            Self::Execute(task) => task.name(),
            Self::Lower(task) => task.name(),
            Self::Optimize(task) => task.name(),
            Self::Generate(task) => task.name(),
            Self::Link(task) => task.name(),
            Self::Emit(task) => task.name(),
            Self::Lint(task) => task.name(),
        }
    }

    fn trace_args(&self, program: &Program) -> String {
        match self {
            Self::Import(task) => task.trace_args(program),
            Self::Resolve(task) => task.trace_args(program),
            Self::Analyze(task) => task.trace_args(program),
            Self::Elaborate(task) => task.trace_args(program),
            Self::Execute(task) => task.trace_args(program),
            Self::Lower(task) => task.trace_args(program),
            Self::Optimize(task) => task.trace_args(program),
            Self::Generate(task) => task.trace_args(program),
            Self::Link(task) => task.trace_args(program),
            Self::Emit(task) => task.trace_args(program),
            Self::Lint(task) => task.trace_args(program),
        }
    }
}

/// Id for a compiler task.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct TaskId(pub u32);

impl std::fmt::Debug for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

impl std::fmt::Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

impl TaskId {
    /// Create a new task id.
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    /// Get the numeric code of the id.
    pub fn code(&self) -> u32 {
        self.0
    }
}

/// Status of a compiler task.
#[derive(Debug, Clone, PartialEq)]
pub enum TaskStatus {
    /// The task is queued.
    Queued,
    /// The task is running.
    Running,
    /// The task is waiting for a dependency.
    Yielded { dependency: TaskDependency },
    /// The task was skipped.
    Skipped { reason: TaskSkipReason },
    /// The task is complete.
    Complete,
    /// The task failed.
    Failed { error: TaskError },
}

impl TaskStatus {
    /// Whether the status is final (i.e., will not change).
    pub fn is_final(&self) -> bool {
        matches!(
            self,
            Self::Complete { .. } | Self::Failed { .. } | Self::Skipped { .. }
        )
    }

    /// Whether the status is an outcome (i.e., yield, complete, skip, or fail).
    pub fn is_outcome(&self) -> bool {
        matches!(
            self,
            Self::Yielded { .. }
                | Self::Complete { .. }
                | Self::Failed { .. }
                | Self::Skipped { .. }
        )
    }
}

impl From<TaskOutcome> for TaskStatus {
    fn from(outcome: TaskOutcome) -> Self {
        match outcome {
            TaskOutcome::Yield { dependency } => Self::Yielded { dependency },
            TaskOutcome::Error { error } => Self::Failed { error },
            TaskOutcome::Skipped { reason } => Self::Skipped { reason },
            TaskOutcome::Complete => Self::Complete,
        }
    }
}

/// Handle for a compiler task.
#[derive(Debug, Clone, PartialEq)]
pub struct TaskHandle {
    /// The task id.
    pub id: TaskId,
    /// The status of the task.
    pub status: TaskStatus,
    /// The previous outcome of the task.
    pub last_outcome: Option<TaskOutcome>,
    /// The task.
    pub task: Task,
    /// Number of times this task has yielded (for debugging).
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

    /// Get the phase of the task.
    pub fn phase(&self) -> TaskPhase {
        self.task.phase()
    }

    /// Get the region of the task.
    pub fn region(&self) -> TaskRegion {
        self.task.region()
    }
}

/// Outcome of a compiler task.
#[derive(Debug, Clone, PartialEq)]
pub enum TaskOutcome {
    /// The task yielded a dependency.
    Yield { dependency: TaskDependency },
    /// The task failed with an error.
    Error { error: TaskError },
    /// The task was skipped.
    Skipped { reason: TaskSkipReason },
    /// The task completed successfully.
    Complete,
}

impl<E> From<Result<(), E>> for TaskOutcome
where
    E: Into<TaskError> + TaskSkip,
    E: TryInto<TaskDependency, Error = E>,
{
    fn from(result: Result<(), E>) -> Self {
        match result {
            Ok(()) => Self::Complete,
            Err(error) => {
                if let Some(reason) = error.skip_reason() {
                    return Self::Skipped { reason };
                }
                match error.try_into() {
                    Ok(dependency) => Self::Yield { dependency },
                    Err(error) => Self::Error {
                        error: error.into(),
                    },
                }
            }
        }
    }
}

impl TaskOutcome {
    /// Check if the outcome is final (i.e., will not change).
    pub fn is_final(&self) -> bool {
        match self {
            Self::Yield { .. } => false,
            Self::Error { .. } => true,
            Self::Skipped { .. } => true,
            Self::Complete => true,
        }
    }
}

/// Task dependency to wait for.
#[derive(Debug, Clone, PartialEq)]
pub enum TaskDependency {
    /// Wait for a single task dependency to complete.
    Complete {
        anchor: DiagnosticAnchor,
        task: Task,
        error: Option<Box<TaskError>>,
    },
    /// Wait for all of the given task dependencies to be satisfied.
    CompleteAll {
        dependencies: Vec<Box<TaskDependency>>,
    },
    /// Wait for any of the given task dependencies to be satisfied.
    CompleteAny {
        dependencies: Vec<Box<TaskDependency>>,
    },
}

impl TaskDependency {
    /// Get the anchor for this dependency.
    pub fn anchor(&self) -> DiagnosticAnchor {
        match self {
            Self::Complete { anchor, .. } => anchor.clone(),
            Self::CompleteAll { dependencies } => dependencies
                .first()
                .map(|dependency| dependency.anchor())
                .unwrap_or(DiagnosticAnchor::Global),
            Self::CompleteAny { dependencies } => dependencies
                .first()
                .map(|dependency| dependency.anchor())
                .unwrap_or(DiagnosticAnchor::Global),
        }
    }

    /// Get all anchors involved in this dependency.
    pub fn anchors(&self) -> Vec<DiagnosticAnchor> {
        match self {
            Self::Complete { anchor, .. } => vec![anchor.clone()],
            Self::CompleteAll { dependencies } => dependencies
                .iter()
                .flat_map(|dependency| dependency.anchors())
                .collect(),
            Self::CompleteAny { dependencies } => dependencies
                .iter()
                .flat_map(|dependency| dependency.anchors())
                .collect(),
        }
    }
}

/// Error when a task dependency is not satisfied.
#[derive(Debug, Clone, PartialEq)]
pub enum TaskDependencyError {
    /// Task is not yet complete, need to yield.
    NotReady { dependency: TaskDependency },
    /// Task has failed.
    Failed { dependency: TaskDependency },
}

impl TaskDependencyError {
    /// Get the dependency from this error.
    pub fn dependency(&self) -> &TaskDependency {
        match self {
            Self::NotReady { dependency } | Self::Failed { dependency } => dependency,
        }
    }

    /// Convert to owned dependency.
    pub fn into_dependency(self) -> TaskDependency {
        match self {
            Self::NotReady { dependency } | Self::Failed { dependency } => dependency,
        }
    }
}

impl TryFrom<TaskDependencyError> for TaskDependency {
    type Error = TaskDependencyError;

    fn try_from(error: TaskDependencyError) -> Result<Self, Self::Error> {
        match error {
            TaskDependencyError::NotReady { dependency } => Ok(dependency),
            TaskDependencyError::Failed { .. } => Err(error),
        }
    }
}

/// Reason a task was skipped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskSkipReason {
    /// Task was stale due to a module version change.
    StaleModuleVersion,
    /// Task was stale due to a profile version change.
    StaleProfileVersion,
    /// Task was stale due to a module graph version change.
    StaleModuleGraphVersion,
    /// Task was stale due to a package version change.
    StalePackageVersion,
    /// Task was stale due to a program stamp change.
    StaleProgramStamp,
}

/// Return a skip reason for errors that represent task skips.
pub trait TaskSkip {
    /// Get the skip reason, if this error represents a skipped task.
    fn skip_reason(&self) -> Option<TaskSkipReason>;
}

/// Build phase errors that represent skipped tasks.
pub trait TaskSkipError: Sized {
    /// Create an error that marks a task as skipped for the given reason.
    fn skipped(reason: TaskSkipReason) -> Self;
}

/// Collector for coalescing task dependencies from multiple operations.
/// Accumulates Yield errors and lets non-yield errors pass through for handling.
#[derive(Debug, Default)]
pub struct TaskResultCollector {
    dependencies: Vec<TaskDependency>,
}

impl TaskResultCollector {
    /// Create a new empty collector.
    pub fn new() -> Self {
        Self {
            dependencies: Vec::new(),
        }
    }

    /// Collect a result as a TaskDependency (error if not a yield).
    pub fn try_collect<T, E>(&mut self, result: Result<T, E>) -> Option<E>
    where
        E: TryInto<TaskDependency, Error = E>,
    {
        match result {
            Ok(_) => None,
            Err(error) => match error.try_into() {
                Ok(dependency) => {
                    self.dependencies.push(dependency);
                    None
                }
                Err(error) => Some(error),
            },
        }
    }

    /// Check if any dependencies were collected.
    pub fn has_dependencies(&self) -> bool {
        !self.dependencies.is_empty()
    }

    /// Finish collection and return a combined CompleteAny dependency if any were collected.
    pub fn try_into_yield_any(self) -> Option<TaskDependency> {
        match self.dependencies.len() {
            0 => None,
            1 => Some(self.dependencies.into_iter().next().unwrap()),
            _ => Some(TaskDependency::CompleteAny {
                dependencies: self.dependencies.into_iter().map(Box::new).collect(),
            }),
        }
    }

    /// Finish collection and return a combined CompleteAll dependency if any were collected.
    pub fn try_into_yield_all(self) -> Option<TaskDependency> {
        match self.dependencies.len() {
            0 => None,
            _ => Some(TaskDependency::CompleteAll {
                dependencies: self.dependencies.into_iter().map(Box::new).collect(),
            }),
        }
    }
}
