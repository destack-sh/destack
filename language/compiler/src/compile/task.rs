use destack_dir::{GlobalNodeIdAny, Program};

use crate::{
    VerifyOutput, VerifyTask, BindOutput, BindTask, GenerateOutput, GenerateTask, ElaborateOutput,
    ElaborateTask, ExecuteOutput, ExecuteTask, ImportOutput, ImportTask, LinkOutput, LinkTask,
    LowerOutput, LowerTask, OptimizeOutput, OptimizeTask, ResolveOutput, ResolveTask, TaskError,
    AnalyzeOutput, AnalyzeTask,
};

/// Trait for formatting task information.
pub trait TaskDebug {
    /// Get the task variant name (e.g., "file", "module", "specifier").
    fn name(&self) -> &'static str;

    /// Format the task arguments for tracing (resolving ids, making the arguments readable, etc.).
    fn trace_args(&self, program: &Program) -> String;
}

/// Region of the compiler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaskRegion {
    /// Front-end (import, bind, resolve, analyze, elaborate).
    Front,
    /// Middle-end (lower, verify, optimize).
    Middle,
    /// Back-end (execute, generate, link).
    Back,
}

impl TaskRegion {
    /// Get the name of the region.
    pub fn name(&self) -> &str {
        match self {
            Self::Front => "front-end",
            Self::Middle => "middle-end",
            Self::Back => "back-end",
        }
    }
}

/// Phase of the compiler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TaskPhase {
    /// Import and parse source into AST.
    Import = 1,
    /// Bind, lower and declare AST source into DIR.
    Bind = 2,
    /// Resolve symbols, scopes and types in DIR.
    Resolve = 3,
    /// Analyze and type-check DIR.
    Analyze = 4,
    /// Elaborate, desugar and monomorphize DIR.
    Elaborate = 5,
    // --------------------------------------------------
    /// Lower the DIR into MIR.
    Lower = 6,
    /// Verify and flow-check MIR.
    Verify = 7,
    /// Optimize the MIR.
    Optimize = 8,
    // --------------------------------------------------
    /// Execute MIR statically.
    Execute = 9,
    /// Generate the MIR into some artifact.
    Generate = 10,
    /// Link generated artifacts into final output.
    Link = 11,
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
            Self::Import | Self::Bind | Self::Resolve | Self::Analyze | Self::Elaborate => {
                TaskRegion::Front
            }
            Self::Lower | Self::Verify | Self::Optimize => TaskRegion::Middle,
            Self::Execute | Self::Generate | Self::Link => TaskRegion::Back,
        }
    }

    /// Get the name of the phase.
    pub fn name(&self) -> &str {
        match self {
            Self::Import => "import",
            Self::Bind => "bind",
            Self::Resolve => "resolve",
            Self::Analyze => "analyze",
            Self::Elaborate => "elaborate",
            Self::Lower => "lower",
            Self::Verify => "verify",
            Self::Optimize => "optimize",
            Self::Execute => "execute",
            Self::Generate => "generate",
            Self::Link => "link",
        }
    }

    /// Get the description of the phase.
    pub fn description(&self) -> &str {
        match self {
            Self::Import => "import and parse source into AST",
            Self::Bind => "bind, lower and declare AST source into DIR",
            Self::Resolve => "resolve symbols, scopes and types in DIR",
            Self::Analyze => "analyze and check DIR",
            Self::Elaborate => "elaborate and monomorphize DIR",
            Self::Lower => "lower the DIR into MIR",
            Self::Verify => "verify and flow-check MIR",
            Self::Optimize => "optimize the MIR",
            Self::Execute => "execute MIR statically",
            Self::Generate => "generate the MIR into some artifact",
            Self::Link => "link generated artifacts into final output",
        }
    }

    /// Get the letter of the phase.
    pub fn letter(&self) -> char {
        match self {
            Self::Import => 'I',
            Self::Bind => 'B',
            Self::Resolve => 'R',
            Self::Analyze => 'A',
            Self::Elaborate => 'E',
            Self::Lower => 'L',
            Self::Verify => 'V',
            Self::Optimize => 'O',
            Self::Execute => 'X',
            Self::Generate => 'G',
            Self::Link => 'K',
        }
    }
}

/// Task for the compiler during compilation.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Task {
    /// Import and parse source into AST.
    Import(ImportTask),
    /// Bind, lower and declare AST source into DIR.
    Bind(BindTask),
    /// Resolve symbols, scopes and types in DIR.
    Resolve(ResolveTask),
    /// Analyze and check DIR.
    Analyze(AnalyzeTask),
    /// Elaborate and monomorphize DIR.
    Elaborate(ElaborateTask),
    // --------------------------------------------------
    /// Lower the DIR into MIR.
    Lower(LowerTask),
    /// Verify and flow-check MIR.
    Verify(VerifyTask),
    /// Optimize the MIR.
    Optimize(OptimizeTask),
    // --------------------------------------------------
    /// Execute MIR statically.
    Execute(ExecuteTask),
    /// Generate the MIR into some artifact.
    Generate(GenerateTask),
    /// Link generated artifacts into final output.
    Link(LinkTask),
}

impl Task {
    /// Get the phase of the task.
    pub fn phase(&self) -> TaskPhase {
        match self {
            Self::Import(_) => TaskPhase::Import,
            Self::Bind(_) => TaskPhase::Bind,
            Self::Resolve(_) => TaskPhase::Resolve,
            Self::Analyze(_) => TaskPhase::Analyze,
            Self::Elaborate(_) => TaskPhase::Elaborate,
            Self::Lower(_) => TaskPhase::Lower,
            Self::Verify(_) => TaskPhase::Verify,
            Self::Optimize(_) => TaskPhase::Optimize,
            Self::Execute(_) => TaskPhase::Execute,
            Self::Generate(_) => TaskPhase::Generate,
            Self::Link(_) => TaskPhase::Link,
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
            Self::Bind(task) => task.sub_code(),
            Self::Resolve(task) => task.sub_code(),
            Self::Analyze(task) => task.sub_code(),
            Self::Elaborate(task) => task.sub_code(),
            Self::Lower(task) => task.sub_code(),
            Self::Verify(task) => task.sub_code(),
            Self::Optimize(task) => task.sub_code(),
            Self::Execute(task) => task.sub_code(),
            Self::Generate(task) => task.sub_code(),
            Self::Link(task) => task.sub_code(),
        }
    }

    /// Get the full code of the task.
    pub fn full_code(&self) -> String {
        format!("T{}{:03}", self.phase().letter(), self.sub_code())
    }
}

impl TaskDebug for Task {
    fn name(&self) -> &'static str {
        match self {
            Self::Import(task) => task.name(),
            Self::Bind(task) => task.name(),
            Self::Resolve(task) => task.name(),
            Self::Analyze(task) => task.name(),
            Self::Elaborate(task) => task.name(),
            Self::Lower(task) => task.name(),
            Self::Verify(task) => task.name(),
            Self::Optimize(task) => task.name(),
            Self::Execute(task) => task.name(),
            Self::Generate(task) => task.name(),
            Self::Link(task) => task.name(),
        }
    }

    fn trace_args(&self, program: &Program) -> String {
        match self {
            Self::Import(task) => task.trace_args(program),
            Self::Bind(task) => task.trace_args(program),
            Self::Resolve(task) => task.trace_args(program),
            Self::Analyze(task) => task.trace_args(program),
            Self::Elaborate(task) => task.trace_args(program),
            Self::Lower(task) => task.trace_args(program),
            Self::Verify(task) => task.trace_args(program),
            Self::Optimize(task) => task.trace_args(program),
            Self::Execute(task) => task.trace_args(program),
            Self::Generate(task) => task.trace_args(program),
            Self::Link(task) => task.trace_args(program),
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
    /// The task is wait for a dependency.
    Yielded { dependency: TaskDependency },
    /// The task is complete.
    Complete { output: TaskOutput },
    /// The task failed.
    Failed { error: TaskError },
}

impl TaskStatus {
    /// Whether the status is final (i.e., will not change).
    pub fn is_final(&self) -> bool {
        matches!(self, Self::Complete { .. } | Self::Failed { .. })
    }

    /// Whether the status is an outcome (i.e., yield, complete or fail).
    pub fn is_outcome(&self) -> bool {
        matches!(
            self,
            Self::Yielded { .. } | Self::Complete { .. } | Self::Failed { .. }
        )
    }
}

impl From<TaskOutcome> for TaskStatus {
    fn from(outcome: TaskOutcome) -> Self {
        match outcome {
            TaskOutcome::Yield { dependency } => Self::Yielded { dependency },
            TaskOutcome::Error { error } => Self::Failed { error },
            TaskOutcome::Complete { output } => Self::Complete { output },
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
    /// The task completed with an output.
    Complete { output: TaskOutput },
}

impl<O, E> From<Result<O, E>> for TaskOutcome
where
    O: Into<TaskOutput>,
    E: Into<TaskError>,
    E: TryInto<TaskDependency, Error = E>,
{
    fn from(result: Result<O, E>) -> Self {
        match result {
            Ok(output) => Self::Complete {
                output: output.into(),
            },
            Err(error) => match error.try_into() {
                Ok(dependency) => Self::Yield { dependency },
                Err(error) => Self::Error {
                    error: error.into(),
                },
            },
        }
    }
}

impl TaskOutcome {
    /// Check if the outcome is final (i.e., will not change).
    pub fn is_final(&self) -> bool {
        match self {
            Self::Yield { .. } => false,
            Self::Error { .. } => true,
            Self::Complete { .. } => true,
        }
    }
}

/// Task dependency to wait for.
#[derive(Debug, Clone, PartialEq)]
pub enum TaskDependency {
    /// Wait for a single task dependency to complete.
    Complete {
        node: GlobalNodeIdAny,
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
    /// Get the first node involved in the wait.
    pub fn node(&self) -> GlobalNodeIdAny {
        match self {
            Self::Complete { node, .. } => *node,
            Self::CompleteAll { dependencies } => dependencies
                .first()
                .map(|dependency| dependency.node())
                .unwrap(),
            Self::CompleteAny { dependencies } => dependencies
                .first()
                .map(|dependency| dependency.node())
                .unwrap(),
        }
    }

    /// Get the nodes involved in the wait.
    pub fn nodes(&self) -> Vec<GlobalNodeIdAny> {
        match self {
            Self::Complete { node, .. } => vec![*node],
            Self::CompleteAll { dependencies } => dependencies
                .iter()
                .flat_map(|dependency| dependency.nodes())
                .collect(),
            Self::CompleteAny { dependencies } => dependencies
                .iter()
                .flat_map(|dependency| dependency.nodes())
                .collect(),
        }
    }
}

/// Output of a compiler task.
#[derive(Debug, Clone, PartialEq)]
pub enum TaskOutput {
    /// Output of an import task.
    Import(ImportOutput),
    /// Output of a bind task.
    Bind(BindOutput),
    /// Output of a resolve task.
    Resolve(ResolveOutput),
    /// Output of a analyze task.
    Analyze(AnalyzeOutput),
    /// Output of an elaborate task.
    Elaborate(ElaborateOutput),
    // --------------------------------------------------
    /// Output of a lower task.
    Lower(LowerOutput),
    /// Output of an verify task.
    Verify(VerifyOutput),
    /// Output of an optimize task.
    Optimize(OptimizeOutput),
    // --------------------------------------------------
    /// Output of an execute task.
    Execute(ExecuteOutput),
    /// Output of a generate task.
    Generate(GenerateOutput),
    /// Output of a link task.
    Link(LinkOutput),
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
