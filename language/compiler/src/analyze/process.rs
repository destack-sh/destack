use crate::{AnalyzeResult, CompileTask, Compiler};

/// Task to analyze something.
#[derive(Debug, Clone)]
pub enum AnalyzeTask {}

impl From<AnalyzeTask> for CompileTask {
    fn from(task: AnalyzeTask) -> Self {
        CompileTask::Analyze(task)
    }
}

impl<'a> Compiler<'a> {
    /// Process a analyze task.
    pub fn process_analyze(&self, task: AnalyzeTask) -> AnalyzeResult<()> {
        todo!("process_analyze({task:?})")
    }
}
