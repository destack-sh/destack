use crate::{
    BindTask, BuildTask, ElaborateTask, ExecuteTask, ImportTask, LinkTask, LowerTask, OptimizeTask,
    ResolveTask, ValidateTask,
};

/// Stage of the compiler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum CompileStage {
    /// Import and parse source into AST.
    Import = 1,
    /// Bind, lower and declare AST source into DIR.
    Bind = 2,
    /// Resolve symbols, scopes and types in DIR.
    Resolve = 3,
    /// Validate and check DIR.
    Validate = 4,
    /// Elaborate, desugar and monomorphize DIR.
    Elaborate = 5,
    /// Lower the DIR into MIR.
    Lower = 6,
    /// Execute MIR statically.
    Execute = 7,
    /// Optimize the MIR.
    Optimize = 8,
    /// Build the MIR into some artifact.
    Build = 9,
    /// Link built artifacts into final output.
    Link = 10,
}

impl std::fmt::Display for CompileStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.letter())
    }
}

impl CompileStage {
    /// Get the numeric code of the stage.
    pub fn code(&self) -> u8 {
        *self as u8
    }

    /// Get the name of the stage.
    pub fn name(&self) -> &str {
        match self {
            Self::Import => "import",
            Self::Bind => "bind",
            Self::Resolve => "resolve",
            Self::Validate => "validate",
            Self::Elaborate => "elaborate",
            Self::Lower => "lower",
            Self::Execute => "execute",
            Self::Optimize => "optimize",
            Self::Build => "build",
            Self::Link => "link",
        }
    }

    /// Get the description of the stage.
    pub fn description(&self) -> &str {
        match self {
            Self::Import => "import and parse source into AST",
            Self::Bind => "bind, lower and declare AST source into DIR",
            Self::Resolve => "resolve symbols, scopes and types in DIR",
            Self::Validate => "validate and check DIR",
            Self::Elaborate => "elaborate and monomorphize DIR",
            Self::Lower => "lower the DIR into MIR",
            Self::Execute => "execute MIR statically",
            Self::Optimize => "optimize the MIR",
            Self::Build => "build the MIR into some artifact",
            Self::Link => "link built artifacts into final output",
        }
    }

    /// Get the letter of the stage.
    pub fn letter(&self) -> char {
        match self {
            Self::Import => 'I',
            Self::Bind => 'D',
            Self::Resolve => 'R',
            Self::Validate => 'V',
            Self::Elaborate => 'E',
            Self::Lower => 'M',
            Self::Execute => 'X',
            Self::Optimize => 'O',
            Self::Build => 'B',
            Self::Link => 'F',
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
    /// Lower the DIR into MIR.
    Lower(LowerTask),
    /// Execute MIR statically.
    Execute(ExecuteTask),
    /// Optimize the MIR.
    Optimize(OptimizeTask),
    /// Build the MIR into some artifact.
    Build(BuildTask),
    /// Link built artifacts into final output.
    Link(LinkTask),
}

impl CompileTask {
    /// Get the stage of the task.
    pub fn stage(&self) -> CompileStage {
        match self {
            Self::Import(_) => CompileStage::Import,
            Self::Bind(_) => CompileStage::Bind,
            Self::Resolve(_) => CompileStage::Resolve,
            Self::Validate(_) => CompileStage::Validate,
            Self::Elaborate(_) => CompileStage::Elaborate,
            Self::Lower(_) => CompileStage::Lower,
            Self::Execute(_) => CompileStage::Execute,
            Self::Optimize(_) => CompileStage::Optimize,
            Self::Build(_) => CompileStage::Build,
            Self::Link(_) => CompileStage::Link,
        }
    }
}
