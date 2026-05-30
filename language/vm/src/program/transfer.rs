use destack_engine as engine;
use destack_mir as mir;

use crate::Word;
use crate::diagnostic::Error;

use super::{ArgumentRange, CallTarget, MoveRange};

/// Control transfer requested by one lowered instruction.
#[derive(Debug)]
pub(crate) enum Transfer {
    /// Continue at the current frame's block.
    Enter,
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
        /// Arguments to pass.
        arguments: ArgumentRange,
        /// Optional closure environment to pass.
        env: Option<Word>,
        /// Move plan for callee parameters.
        moves: Option<MoveRange>,
        /// PC to resume at after call returns.
        resume_pc: usize,
    },
    /// Call another function and enter an explicit continuation.
    CallBranch {
        /// Function to call.
        function: u32,
        /// Lowered or imported call target.
        target: CallTarget,
        /// Arguments to pass.
        arguments: ArgumentRange,
        /// Optional closure environment to pass.
        env: Option<Word>,
        /// The continuation frame state.
        target_state: engine::FrameStateId,
    },
    /// Tail call another function.
    TailCall {
        /// Function to call.
        function: u32,
        /// Lowered or imported call target.
        target: CallTarget,
        /// Arguments to pass.
        arguments: ArgumentRange,
        /// Optional closure environment to pass.
        env: Option<Word>,
        /// Move plan for callee parameters.
        moves: Option<MoveRange>,
    },
    /// Yield from the current function.
    Yield {
        /// The value yielded to the caller.
        value: Word,
        /// The yielded value type.
        source_type: mir::LocalNodeId<mir::Type>,
        /// The frame state captured in the continuation.
        frame_state: engine::FrameStateId,
    },
    /// Return from current function.
    Return(Word),
    /// Runtime error.
    Error(Error),
}
