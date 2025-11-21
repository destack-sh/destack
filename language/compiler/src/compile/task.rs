use crate::{
    BindTask, BuildTask, ExecuteTask, ImportTask, LinkTask, LowerTask, OptimizeTask, ResolveTask,
    ValidateTask,
};

/// Stage of the compiler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum CompilerStage {
    /// Import & parse source into AST.
    Import = 1,
    /// Bind, lower and declare AST source in DIR.
    Bind = 2,
    /// Resolve references, types and static DIR constructs.
    Resolve = 3,
    /// Validate and check all DIR constructs.
    Validate = 4,
    /// Lower the DIR into MIR.
    Lower = 5,
    /// Execute something statically.
    Execute = 6,
    /// Optimize the MIR.
    Optimize = 7,
    /// Build the MIR into something.
    Build = 8,
    /// Link the artifacts into a final artifact.
    Link = 9,
}

impl std::fmt::Display for CompilerStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.letter())
    }
}

impl CompilerStage {
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
            Self::Lower => "lower",
            Self::Execute => "execute",
            Self::Optimize => "optimize",
            Self::Build => "build",
            Self::Link => "link",
        }
    }

    /// Get the letter of the stage.
    pub fn letter(&self) -> char {
        match self {
            Self::Import => 'I',
            Self::Bind => 'D',
            Self::Resolve => 'R',
            Self::Validate => 'V',
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
pub enum CompilerTask {
    /// Import/parse/lower source into DIR.
    Import(ImportTask),
    /// Bind & lower AST source into DIR.
    Bind(BindTask),
    /// Resolve references, types and static DIR constructs.
    Resolve(ResolveTask),
    /// Validate and check all DIR constructs.
    Validate(ValidateTask),
    /// Lower the DIR into MIR.
    Lower(LowerTask),
    /// Execute something statically.
    Execute(ExecuteTask),
    /// Optimize the MIR.
    Optimize(OptimizeTask),
    /// Build the MIR into something.
    Build(BuildTask),
    /// Link the artifacts into a final artifact.
    Link(LinkTask),
}

impl CompilerTask {
    /// Get the stage of the task.
    pub fn stage(&self) -> CompilerStage {
        match self {
            Self::Import(_) => CompilerStage::Import,
            Self::Bind(_) => CompilerStage::Bind,
            Self::Resolve(_) => CompilerStage::Resolve,
            Self::Validate(_) => CompilerStage::Validate,
            Self::Lower(_) => CompilerStage::Lower,
            Self::Execute(_) => CompilerStage::Execute,
            Self::Optimize(_) => CompilerStage::Optimize,
            Self::Build(_) => CompilerStage::Build,
            Self::Link(_) => CompilerStage::Link,
        }
    }
}
