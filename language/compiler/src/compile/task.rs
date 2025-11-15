use crate::{BuildTask, ExecuteTask, ImportTask, OptimizeTask, ResolveTask, ValidateTask};

/// Task for the compiler during compilation.
#[derive(Debug, Clone)]
pub enum CompilerTask {
    /// Import/parse/lower source into DIR.
    Import(ImportTask),
    /// Resolve references and static DIR constructs.
    Resolve(ResolveTask),
    /// Validate and check all DIR constructs.
    Validate(ValidateTask),
    /// Execute something statically.
    Execute(ExecuteTask),
    /// Optimize the DIR.
    Optimize(OptimizeTask),
    /// Build the DIR into something (JS/TS/MIR/...).
    Build(BuildTask),
}

impl CompilerTask {}
