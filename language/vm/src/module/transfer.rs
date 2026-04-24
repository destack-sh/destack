use {destack_engine as engine, destack_mir as mir};

use crate::Value;
use crate::diagnostic::Error;

use super::{ArgumentRange, CallTarget, CopyRange};

/// Control transfer requested by one lowered instruction.
#[derive(Debug)]
pub(crate) enum Transfer {
    /// Jump to another block.
    Jump {
        /// Target block index.
        block: u32,
        /// Copy plan for block parameters.
        copies: CopyRange,
    },
    /// Call another function.
    Call {
        /// Function to call.
        function: u32,
        /// Lowered or imported call target.
        target: CallTarget,
        /// Destination for return value.
        destination: mir::Value,
        /// Arguments to pass.
        arguments: ArgumentRange,
        /// Optional callable environment to pass.
        env: Option<Value>,
        /// Copy plan for callee parameters.
        copies: Option<CopyRange>,
        /// PC to resume at after call returns.
        resume_pc: usize,
    },
    /// Call another function and branch on normal or unwind completion.
    CallBranch {
        /// Function to call.
        function: u32,
        /// Lowered or imported call target.
        target: CallTarget,
        /// Arguments to pass.
        arguments: ArgumentRange,
        /// Optional callable environment to pass.
        env: Option<Value>,
        /// The normal continuation resume point.
        normal_resume_point: engine::ResumePointId,
        /// The unwind continuation resume point.
        unwind_resume_point: engine::ResumePointId,
    },
    /// Tail call another function.
    TailCall {
        /// Function to call.
        function: u32,
        /// Lowered or imported call target.
        target: CallTarget,
        /// Arguments to pass.
        arguments: ArgumentRange,
        /// Optional callable environment to pass.
        env: Option<Value>,
        /// Copy plan for callee parameters.
        copies: Option<CopyRange>,
    },
    /// Yield from the current function.
    Yield {
        /// The value yielded to the caller.
        value: Value,
        /// The MIR value that produced the yielded value.
        source: mir::Value,
        /// The resume point captured in the continuation.
        resume_point: engine::ResumePointId,
    },
    /// Throw one managed exception value.
    Throw(Value),
    /// Return from current function.
    Return(Value),
    /// Runtime error.
    Error(Error),
}
