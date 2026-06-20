use crate::{
    NativeContext, NativeContinuation, NativeEntry, NativeExitCode, NativeResumeEntry, NativeValue,
};
use destack_mir as mir;
use destack_program::FunctionId;

/// One native entry.
#[derive(Clone)]
pub struct Entry {
    /// The function implemented by this entry.
    pub function: FunctionId,
    /// The native entry function pointer.
    pub entry: NativeEntry,
}

impl Entry {
    /// Create one native entry.
    pub fn new(function: FunctionId, entry: NativeEntry) -> Self {
        Self { function, entry }
    }

    /// Call this native entry.
    pub fn call(
        &self,
        context: &mut NativeContext,
        args: &[NativeValue],
        out: &mut NativeValue,
    ) -> NativeExitCode {
        // native entries are produced by the native loader with this ABI
        unsafe { (self.entry)(context, args.as_ptr(), args.len(), out) }
    }
}

/// One native continuation resume entry.
#[derive(Clone)]
pub struct ResumeEntry {
    /// The frame state resumed by this entry.
    pub frame_state: mir::FrameStateId,
    /// The native resume function pointer.
    pub entry: NativeResumeEntry,
}

impl ResumeEntry {
    /// Create one native resume entry.
    pub fn new(frame_state: mir::FrameStateId, entry: NativeResumeEntry) -> Self {
        Self { frame_state, entry }
    }

    /// Call this native resume entry.
    pub fn call(
        &self,
        context: &mut NativeContext,
        continuation: NativeContinuation,
        received: NativeValue,
        out: &mut NativeValue,
    ) -> NativeExitCode {
        // native resume entries are produced by the native loader with this ABI
        unsafe { (self.entry)(context, continuation, received, out) }
    }
}

impl std::fmt::Debug for ResumeEntry {
    /// Format this entry without exposing a raw function pointer.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ResumeEntry")
            .field("frame_state", &self.frame_state)
            .finish()
    }
}

impl std::fmt::Debug for Entry {
    /// Format this entry without exposing a raw function pointer.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Entry")
            .field("function", &self.function)
            .finish()
    }
}
