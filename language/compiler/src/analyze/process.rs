use crate::{AnalyzeResult, CompileOutput, CompileTask, Compiler};

use dyst_dir::{ModuleId, Program};

/// Task to analyze something.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum AnalyzeTask {
    /// Analyze a module.
    Analyze { module: ModuleId },
}

impl AnalyzeTask {
    /// Get the sub code for the task.
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::Analyze { .. } => 1,
        }
    }

    /// Get a message for the task.
    pub fn message<'a>(&self, _program: &'a Program<'a>) -> String {
        match self {
            Self::Analyze { module } => {
                format!("analyze module '{module:?}'")
            }
        }
    }
}

impl From<AnalyzeTask> for CompileTask {
    fn from(task: AnalyzeTask) -> Self {
        CompileTask::Analyze(task)
    }
}

/// Output of an analyze task.
#[derive(Debug, Clone)]
pub struct AnalyzeOutput {}

impl From<AnalyzeOutput> for CompileOutput {
    fn from(output: AnalyzeOutput) -> Self {
        CompileOutput::Analyze(output)
    }
}

impl<'a> Compiler<'a> {
    /// Process a analyze task.
    pub fn process_analyze(&self, task: AnalyzeTask) -> AnalyzeResult<AnalyzeOutput> {
        todo!("process_analyze({task:?})")
    }
}
