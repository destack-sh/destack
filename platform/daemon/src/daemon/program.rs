use std::sync::Arc;

use destack_compiler::Compiler;
use destack_workspace::Program;
use parking_lot::Mutex;

/// Per program daemon handle.
#[derive(Debug)]
pub(super) struct ProgramHandle {
    /// The program for this root.
    pub(super) program: Arc<Program>,
    /// The compiler for this program.
    pub(super) compiler: Arc<Compiler>,
    /// Serialize compilation per program.
    pub(super) compile_lock: Mutex<()>,
}
