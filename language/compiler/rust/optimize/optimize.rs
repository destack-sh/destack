use crate::Compiler;

/// Request to optimize something.
#[derive(Debug, Clone)]
pub enum OptimizeTask {}

impl<'a> Compiler<'a> {
    /// Process a optimize request.
    pub fn process_optimize(&mut self, request: OptimizeTask) {
        todo!("process_optimize({request:?})")
    }
}
