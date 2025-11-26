use dyst_dir::{GlobalNodeIdAny, Program};

use crate::{
    AnalyzeOutput, AnalyzeTask, BindOutput, BindTask, BuildOutput, BuildTask, CompileError,
    ElaborateOutput, ElaborateTask, ExecuteOutput, ExecuteTask, ImportOutput, ImportTask,
    LinkOutput, LinkTask, LowerOutput, LowerTask, OptimizeOutput, OptimizeTask, ResolveOutput,
    ResolveTask, ValidateOutput, ValidateTask,
};

/// Region of the compiler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompilerRegion {
    /// Front-end (import, bind, resolve, validate, elaborate).
    Front,
    /// Middle-end (lower, analyze, optimize).
    Middle,
    /// Back-end (execute, build, link).
    Back,
}

impl CompilerRegion {
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
pub enum CompilePhase {
    /// Import and parse source into AST.
    Import = 1,
    /// Bind, lower and declare AST source into DIR.
    Bind = 2,
    /// Resolve symbols, scopes and types in DIR.
    Resolve = 3,
    /// Validate and type-check DIR.
    Validate = 4,
    /// Elaborate, desugar and monomorphize DIR.
    Elaborate = 5,
    // --------------------------------------------------
    /// Lower the DIR into MIR.
    Lower = 6,
    /// Analyze and flow-check MIR.
    Analyze = 7,
    /// Optimize the MIR.
    Optimize = 8,
    // --------------------------------------------------
    /// Execute MIR statically.
    Execute = 9,
    /// Build the MIR into some artifact.
    Build = 10,
    /// Link built artifacts into final output.
    Link = 11,
}

impl std::fmt::Display for CompilePhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.letter())
    }
}

impl CompilePhase {
    /// Get the numeric code of the phase.
    pub fn code(&self) -> u8 {
        *self as u8
    }

    /// Get the region of the phase.
    pub fn region(&self) -> CompilerRegion {
        match self {
            Self::Import | Self::Bind | Self::Resolve | Self::Validate | Self::Elaborate => {
                CompilerRegion::Front
            }
            Self::Lower | Self::Analyze | Self::Optimize => CompilerRegion::Middle,
            Self::Execute | Self::Build | Self::Link => CompilerRegion::Back,
        }
    }

    /// Get the name of the phase.
    pub fn name(&self) -> &str {
        match self {
            Self::Import => "import",
            Self::Bind => "bind",
            Self::Resolve => "resolve",
            Self::Validate => "validate",
            Self::Elaborate => "elaborate",
            Self::Lower => "lower",
            Self::Analyze => "analyze",
            Self::Optimize => "optimize",
            Self::Execute => "execute",
            Self::Build => "build",
            Self::Link => "link",
        }
    }

    /// Get the description of the phase.
    pub fn description(&self) -> &str {
        match self {
            Self::Import => "import and parse source into AST",
            Self::Bind => "bind, lower and declare AST source into DIR",
            Self::Resolve => "resolve symbols, scopes and types in DIR",
            Self::Validate => "validate and check DIR",
            Self::Elaborate => "elaborate and monomorphize DIR",
            Self::Lower => "lower the DIR into MIR",
            Self::Analyze => "analyze and flow-check MIR",
            Self::Optimize => "optimize the MIR",
            Self::Execute => "execute MIR statically",
            Self::Build => "build the MIR into some artifact",
            Self::Link => "link built artifacts into final output",
        }
    }

    /// Get the letter of the phase.
    pub fn letter(&self) -> char {
        match self {
            Self::Import => 'I',
            Self::Bind => 'D',
            Self::Resolve => 'R',
            Self::Validate => 'V',
            Self::Elaborate => 'E',
            Self::Lower => 'M',
            Self::Analyze => 'A',
            Self::Optimize => 'O',
            Self::Execute => 'X',
            Self::Build => 'B',
            Self::Link => 'L',
        }
    }
}

/// Task for the compiler during compilation.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum CompileTask {
    /// Import and parse source into AST.
    Import(ImportTask),
    /// Bind, lower and declare AST source into DIR.
    Bind(BindTask),
    /// Resolve symbols, scopes and types in DIR.
    Resolve(ResolveTask),
    /// Validate and check DIR.
    Validate(ValidateTask),
    /// Elaborate and monomorphize DIR.
    Elaborate(ElaborateTask),
    // --------------------------------------------------
    /// Lower the DIR into MIR.
    Lower(LowerTask),
    /// Analyze and flow-check MIR.
    Analyze(AnalyzeTask),
    /// Optimize the MIR.
    Optimize(OptimizeTask),
    // --------------------------------------------------
    /// Execute MIR statically.
    Execute(ExecuteTask),
    /// Build the MIR into some artifact.
    Build(BuildTask),
    /// Link built artifacts into final output.
    Link(LinkTask),
}

impl CompileTask {
    /// Get the phase of the task.
    pub fn phase(&self) -> CompilePhase {
        match self {
            Self::Import(_) => CompilePhase::Import,
            Self::Bind(_) => CompilePhase::Bind,
            Self::Resolve(_) => CompilePhase::Resolve,
            Self::Validate(_) => CompilePhase::Validate,
            Self::Elaborate(_) => CompilePhase::Elaborate,
            Self::Lower(_) => CompilePhase::Lower,
            Self::Analyze(_) => CompilePhase::Analyze,
            Self::Optimize(_) => CompilePhase::Optimize,
            Self::Execute(_) => CompilePhase::Execute,
            Self::Build(_) => CompilePhase::Build,
            Self::Link(_) => CompilePhase::Link,
        }
    }

    /// Get the region of the task.
    pub fn region(&self) -> CompilerRegion {
        self.phase().region()
    }

    /// Get the sub code of the task.
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::Import(task) => task.sub_code(),
            Self::Bind(task) => task.sub_code(),
            Self::Resolve(task) => task.sub_code(),
            Self::Validate(task) => task.sub_code(),
            Self::Elaborate(task) => task.sub_code(),
            Self::Lower(task) => task.sub_code(),
            Self::Analyze(task) => task.sub_code(),
            Self::Optimize(task) => task.sub_code(),
            Self::Execute(task) => task.sub_code(),
            Self::Build(task) => task.sub_code(),
            Self::Link(task) => task.sub_code(),
        }
    }

    /// Get the full code of the task.
    pub fn full_code(&self) -> String {
        format!("T{}{:03}", self.phase().letter(), self.sub_code())
    }

    /// Get the message of the task.
    pub fn message(&self, program: &Program) -> String {
        match self {
            Self::Import(task) => task.message(program),
            Self::Bind(task) => task.message(program),
            Self::Resolve(task) => task.message(program),
            Self::Validate(task) => task.message(program),
            Self::Elaborate(task) => task.message(program),
            Self::Lower(task) => task.message(program),
            Self::Analyze(task) => task.message(program),
            Self::Optimize(task) => task.message(program),
            Self::Execute(task) => task.message(program),
            Self::Build(task) => task.message(program),
            Self::Link(task) => task.message(program),
        }
    }
}

/// Id for a compiler task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CompileTaskId(u32);

impl CompileTaskId {
    /// Get the numeric code of the id.
    pub fn code(&self) -> u32 {
        self.0
    }
}

/// Status of a compiler task.
#[derive(Debug, Clone)]
pub enum CompileTaskStatus {
    /// The task is wait for a dependency.
    Waiting,
    /// The task is queued.
    Queued,
    /// The task is running.
    Running,
    /// The task is complete.
    Complete { output: CompileOutput },
    /// The task failed.
    Failed { error: CompileError },
}

/// Handle for a compiler task.
#[derive(Debug, Clone)]
pub struct CompileTaskHandle {
    /// The handle id.
    pub id: CompileTaskId,
    /// The status of the task.
    pub status: CompileTaskStatus,
    /// The task.
    pub task: CompileTask,
}

impl CompileTaskHandle {
    /// Get the phase of the task.
    pub fn phase(&self) -> CompilePhase {
        self.task.phase()
    }

    /// Get the region of the task.
    pub fn region(&self) -> CompilerRegion {
        self.task.region()
    }

    /// Get the message of the task.
    pub fn message(&self, program: &Program) -> String {
        self.task.message(program)
    }
}

/// Task wait for other tasks.
#[derive(Debug, Clone, PartialEq)]
pub struct CompileTaskWait {
    // nocheckin: wait for tasks (and error if dependent tasks fail)
    /// The nodes involved in the wait.
    pub nodes: Vec<GlobalNodeIdAny>,
    /// The tasks to wait for.
    pub tasks: Vec<CompileTask>,
    /// The error to generate if the wait is not resolved.
    pub error: Option<Box<CompileError>>,
}

/// Output of a compiler task.
#[derive(Debug, Clone)]
pub enum CompileOutput {
    /// Output of an import task.
    Import(ImportOutput),
    /// Output of a bind task.
    Bind(BindOutput),
    /// Output of a resolve task.
    Resolve(ResolveOutput),
    /// Output of a validate task.
    Validate(ValidateOutput),
    /// Output of an elaborate task.
    Elaborate(ElaborateOutput),
    // --------------------------------------------------
    /// Output of a lower task.
    Lower(LowerOutput),
    /// Output of an analyze task.
    Analyze(AnalyzeOutput),
    /// Output of an optimize task.
    Optimize(OptimizeOutput),
    // --------------------------------------------------
    /// Output of an execute task.
    Execute(ExecuteOutput),
    /// Output of a build task.
    Build(BuildOutput),
    /// Output of a link task.
    Link(LinkOutput),
}
