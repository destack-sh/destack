use crate::Compiler;

/// task to build something into an artifact.
#[derive(Debug, Clone)]
pub enum BuildTask {}

impl<'a> Compiler<'a> {
    /// Process a build task.
    pub fn process_build(&mut self, task: BuildTask) {
        todo!("process_build({task:?})")
    }
}
