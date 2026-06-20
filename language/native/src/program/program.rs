use std::sync::Arc;

use destack_mir as mir;
use destack_program::{EntryPoint, FunctionId, StaticSpace, TypeTable};

use crate::{Code, Entry, ResumeEntry};

/// Process-local native program.
#[derive(Debug, Clone)]
pub struct Program {
    /// The durable program artifact.
    program: Arc<destack_program::Program>,
    /// The process-local native code table.
    code: Code,
}

impl Program {
    /// Create one native program.
    pub fn new(program: Arc<destack_program::Program>, code: Code) -> Self {
        Self { program, code }
    }

    /// Borrow the durable program artifact.
    pub fn program(&self) -> &destack_program::Program {
        self.program.as_ref()
    }

    /// Return the durable program artifact handle.
    pub fn program_handle(&self) -> Arc<destack_program::Program> {
        self.program.clone()
    }

    /// Borrow the native code table.
    pub const fn code(&self) -> &Code {
        &self.code
    }

    /// Borrow the native code map.
    pub const fn code_map(&self) -> &destack_program::native::CodeMap {
        self.code.map()
    }

    /// Borrow immutable program constants.
    pub fn constants(&self) -> &StaticSpace {
        self.program.constants()
    }

    /// Borrow initial shared static storage.
    pub fn shared_statics(&self) -> &StaticSpace {
        self.program.shared_statics()
    }

    /// Borrow initial local static storage.
    pub fn local_statics(&self) -> &StaticSpace {
        self.program.local_statics()
    }

    /// Borrow runtime type metadata.
    pub fn types(&self) -> &TypeTable {
        self.program.types()
    }

    /// Borrow runtime layouts.
    pub fn layouts(&self) -> &mir::LayoutTable {
        self.program.layouts()
    }

    /// Borrow runtime frame metadata.
    pub fn frames(&self) -> &mir::FrameTable {
        &self.program.frames
    }

    /// Borrow runtime function metadata.
    pub fn functions(&self) -> &destack_program::FunctionTable {
        self.program.functions()
    }

    /// Return the program heap trace table.
    pub fn trace_table(&self) -> Arc<mir::TraceTable> {
        self.program.trace_table_handle()
    }

    /// Return one native entry for one function.
    pub fn entry(&self, function: FunctionId) -> Option<&Entry> {
        self.code.entry(function)
    }

    /// Return one native resume entry for one frame state.
    pub fn resume(&self, frame_state: mir::FrameStateId) -> Option<&ResumeEntry> {
        self.code.resume(frame_state)
    }

    /// Return one native entry for one program entrypoint.
    pub fn entry_point(&self, entry: EntryPoint) -> Option<&Entry> {
        self.entry(entry.function())
    }

    /// Return one native entry by runtime name.
    pub fn entry_by_name(&self, name: &str) -> Option<&Entry> {
        let function = self.program.function_id_by_name(name)?;

        self.entry(function)
    }
}
