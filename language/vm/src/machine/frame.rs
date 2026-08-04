use destack_bytecode::{CodeOffset, CodeRange, RegisterSpan};
use destack_program::{Completion, FrameReturn, FrameStateId, FunctionId, Word};
use serde::{Deserialize, Serialize};

/// One active bytecode call frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Frame {
    /// The linked function being executed.
    pub(crate) function: FunctionId,
    /// The function's encoded bytecode range.
    pub(crate) code: CodeRange,
    /// The function-relative instruction entered when execution resumes.
    pub(crate) pc: CodeOffset,
    /// The first register word in the VM stack.
    pub(crate) register_offset: usize,
    /// The function register word count.
    pub(crate) register_count: u16,
    /// The transition taken when this frame returns.
    pub(crate) return_to: Return,
}

/// One caller transition retained across a bytecode call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum Return {
    /// Exit the root call chain.
    Exit {
        /// The terminal completion mode.
        completion: Completion,
    },
    /// Return from one ordinary or invoked call.
    Call {
        /// Caller program counter that entered the callee.
        pc: CodeOffset,
        /// Caller registers receiving returned words.
        registers: RegisterSpan,
        /// Caller bytecode offset entered after normal completion.
        normal: Option<CodeOffset>,
        /// Caller bytecode offset entered during panic unwinding.
        unwind: Option<CodeOffset>,
    },
    /// Return from one continuation control transfer.
    Continuation {
        /// Caller program counter that drove the continuation.
        pc: CodeOffset,
        /// Caller registers receiving one yielded value.
        yielded_registers: RegisterSpan,
        /// Caller register receiving the replacement continuation.
        continuation_register: u16,
        /// Caller registers receiving the final return value.
        returned_registers: RegisterSpan,
        /// Caller bytecode offset entered when the continuation yields.
        yielded: CodeOffset,
        /// Caller bytecode offset entered when the continuation returns.
        returned: CodeOffset,
        /// Caller bytecode offset entered during panic unwinding.
        unwind: Option<CodeOffset>,
    },
    /// Return from the initial eager execution of one task.
    Task {
        /// Caller program counter that started the task.
        pc: CodeOffset,
        /// Caller register containing the runtime task identity.
        task_register: u16,
    },
    /// Resume the caller after one destructor completes.
    Drop {
        /// Caller program counter that entered the destructor.
        pc: CodeOffset,
        /// Canonical caller state retained across destructor execution.
        caller_state: FrameStateId,
        /// Retained caller frames released after this destructor returns.
        frame_count: u16,
    },
}

impl Return {
    /// Return the caller program counter when this transition has one.
    pub(crate) const fn pc(self) -> Option<CodeOffset> {
        match self {
            Self::Exit { .. } => None,
            Self::Call { pc, .. }
            | Self::Continuation { pc, .. }
            | Self::Task { pc, .. }
            | Self::Drop { pc, .. } => Some(pc),
        }
    }

    /// Return the exact caller state when this transition retains one.
    pub(crate) const fn state(self) -> Option<FrameStateId> {
        match self {
            Self::Drop { caller_state, .. } => Some(caller_state),
            _ => None,
        }
    }

    /// Return the canonical child-to-parent transition.
    pub(crate) const fn frame_return(self) -> FrameReturn {
        match self {
            Self::Exit { .. } => FrameReturn::Root,
            Self::Call { .. } => FrameReturn::Call,
            Self::Continuation { .. } => FrameReturn::Continuation,
            Self::Task { .. } => FrameReturn::Task,
            Self::Drop { frame_count, .. } => FrameReturn::Drop { frame_count },
        }
    }
}

impl Frame {
    /// Create one active bytecode frame.
    pub(crate) const fn new(
        function: FunctionId,
        code: CodeRange,
        register_offset: usize,
        register_count: u16,
        return_to: Return,
    ) -> Self {
        Self {
            function,
            code,
            pc: CodeOffset(0),
            register_offset,
            register_count,
            return_to,
        }
    }

    /// Return one function-relative register word index.
    #[inline(always)]
    pub(crate) const fn register(self, register: u16) -> usize {
        self.register_offset + register as usize
    }

    /// Return one function-relative register range start.
    #[inline(always)]
    pub(crate) const fn range(self, range: RegisterSpan) -> usize {
        self.register(range.start.0)
    }

    /// Return the first byte in this frame's register window.
    #[inline(always)]
    pub(crate) const fn byte_offset(self) -> usize {
        self.register_offset * Word::BYTE_LEN
    }

    /// Return one branch target relative to this frame's next instruction.
    #[inline(always)]
    pub(crate) const fn branch_offset(self, displacement: i32) -> CodeOffset {
        CodeOffset(self.pc.0.wrapping_add_signed(displacement))
    }
}
