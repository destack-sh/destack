use {destack_engine as engine, destack_mir as mir};

use crate::Word;
use crate::diagnostic::Error;

use super::{ArgumentRange, CallTarget, MoveRange};

/// Control transfer requested by one lowered instruction.
#[derive(Debug)]
pub(crate) enum Transfer {
    /// Continue to the next instruction in the current block.
    Continue,
    /// Jump to another block.
    Jump {
        /// Target block index.
        block: u32,
        /// Move plan for block parameters.
        moves: MoveRange,
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
        env: Option<Word>,
        /// Move plan for callee parameters.
        moves: Option<MoveRange>,
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
        env: Option<Word>,
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
        env: Option<Word>,
        /// Move plan for callee parameters.
        moves: Option<MoveRange>,
    },
    /// Yield from the current function.
    Yield {
        /// The value yielded to the caller.
        value: Word,
        /// The MIR value that produced the yielded value.
        source: mir::Value,
        /// The resume point captured in the continuation.
        resume_point: engine::ResumePointId,
    },
    /// Throw one managed exception value.
    Throw(Word),
    /// Return from current function.
    Return(Word),
    /// Runtime error.
    Error(Error),
}
