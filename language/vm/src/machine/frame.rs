use destack_bytecode::{CodeOffset, CodeRange, RegisterRange};
use destack_program::{FrameStateId, FunctionId};

/// One active bytecode call frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Frame {
    /// The linked function being executed.
    pub(crate) function: FunctionId,
    /// The function's encoded bytecode range.
    pub(crate) code: CodeRange,
    /// The next function-relative instruction byte.
    pub(crate) code_offset: CodeOffset,
    /// The first byte owned by this frame in the VM stack.
    pub(crate) stack_offset: usize,
    /// The first canonical frame byte in the VM stack.
    pub(crate) frame_offset: usize,
    /// The canonical frame byte length.
    pub(crate) frame_byte_len: u32,
    /// The durable state used when this frame is suspended by a callee.
    pub(crate) frame_state: Option<FrameStateId>,
    /// The state entered when the suspended callee returns normally.
    pub(crate) normal_state: Option<FrameStateId>,
    /// The state entered when the suspended callee unwinds.
    pub(crate) unwind_state: Option<FrameStateId>,
    /// The first register word in the VM stack.
    pub(crate) register_offset: usize,
    /// The function register word count.
    pub(crate) register_count: u16,
    /// The caller destination when this is not the entry frame.
    pub(crate) return_to: Option<Return>,
}

/// One caller transition retained across a bytecode call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Return {
    /// Copy returned words into the caller register range.
    Values(RegisterRange),
    /// Resume the caller after one destructor completes.
    Drop,
}

impl Frame {
    /// Create one active bytecode frame.
    pub(crate) const fn new(
        function: FunctionId,
        code: CodeRange,
        stack_offset: usize,
        frame_offset: usize,
        frame_byte_len: u32,
        register_offset: usize,
        register_count: u16,
        return_to: Option<Return>,
    ) -> Self {
        Self {
            function,
            code,
            code_offset: CodeOffset(0),
            stack_offset,
            frame_offset,
            frame_byte_len,
            frame_state: None,
            normal_state: None,
            unwind_state: None,
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
    pub(crate) const fn range(self, range: RegisterRange) -> usize {
        self.register(range.start.0)
    }

    /// Return one canonical frame slot address.
    #[inline(always)]
    pub(crate) const fn slot(self, byte_offset: u32) -> usize {
        self.frame_offset + byte_offset as usize
    }

    /// Retain the caller states required while one callee is active.
    pub(crate) fn suspend(
        &mut self,
        frame_state: FrameStateId,
        normal_state: Option<FrameStateId>,
        unwind_state: Option<FrameStateId>,
    ) {
        self.frame_state = Some(frame_state);
        self.normal_state = normal_state;
        self.unwind_state = unwind_state;
    }

    /// Clear caller states after the active callee returns.
    pub(crate) fn resume(&mut self) {
        self.frame_state = None;
        self.normal_state = None;
        self.unwind_state = None;
    }

    /// Return whether this frame is executing one destructor.
    pub(crate) const fn is_destructor(self) -> bool {
        matches!(self.return_to, Some(Return::Drop))
    }

    /// Advance to the instruction following one encoded instruction.
    #[inline(always)]
    pub(crate) fn advance(&mut self, byte_len: usize) {
        self.code_offset.0 += byte_len as u32;
    }

    /// Branch relative to the end of the current instruction.
    #[inline(always)]
    pub(crate) fn branch(&mut self, displacement: i32) {
        self.code_offset.0 = self.code_offset.0.wrapping_add_signed(displacement);
    }
}
