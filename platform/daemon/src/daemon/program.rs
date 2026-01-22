use std::sync::Arc;

use destack_compiler::Compiler;
use destack_workspace::Program;
use parking_lot::Mutex;

/// Per program daemon handle.
#[derive(Debug)]
pub(crate) struct ProgramHandle {
    /// The program for this root.
    pub(crate) program: Arc<Program>,
    /// The compiler for this program.
    pub(crate) compiler: Arc<Compiler>,
    /// Serialize compilation per program.
    pub(crate) compile_lock: Mutex<()>,
}
