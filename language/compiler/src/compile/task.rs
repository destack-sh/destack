use crate::{BuildTask, EvaluateTask, ExecuteTask, LoadTask, OptimizeTask, ValidateTask};

/// Task for the compiler during compilation.
#[derive(Debug, Clone)]
pub enum CompilerTask {
    /// Load/parse/lower something into the compiler.
    Load(LoadTask),
    /// Evaluate something.
    Evaluate(EvaluateTask),
    /// Validate something.
    Validate(ValidateTask),
    /// Execute something.
    Execute(ExecuteTask),
    /// Optimize something.
    Optimize(OptimizeTask),
    /// Build something.
    Build(BuildTask),
}

impl CompilerTask {}
