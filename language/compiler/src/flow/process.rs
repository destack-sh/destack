use crate::{CompileTask, Compiler, FlowResult};

/// Task to flow something.
#[derive(Debug, Clone)]
pub enum FlowTask {}

impl From<FlowTask> for CompileTask {
    fn from(task: FlowTask) -> Self {
        CompileTask::Flow(task)
    }
}

impl<'a> Compiler<'a> {
    /// Process a flow task.
    pub fn process_flow(&self, task: FlowTask) -> FlowResult<()> {
        todo!("process_flow({task:?})")
    }
}
