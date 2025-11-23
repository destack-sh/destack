use crate::{
    AnalyzeTask, BindTask, BuildTask, ElaborateTask, ExecuteTask, ImportTask, LinkTask, LowerTask,
    OptimizeTask, ResolveTask, ValidateTask,
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
#[derive(Debug, Clone)]
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
}
