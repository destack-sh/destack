/// Identifier for a MIR function.
pub type FunctionId = u32;
/// Identifier for a MIR block.
pub type BlockId = u32;
/// Identifier for a MIR instruction.
pub type InstructionId = u32;

/// Identifier for a deopt map entry.
pub type DeoptMapId = u32;
/// Identifier for a GC stack map entry.
pub type StackMapId = u32;
/// Identifier for an OSR entry.
pub type OsrEntryId = u32;

/// Safepoint metadata for native execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Safepoint {
    /// Program counter for the safepoint.
    pub pc: u64,
    /// Deopt map entry for this safepoint.
    pub deopt_map: DeoptMapId,
    /// GC stack map entry for this safepoint.
    pub gc_stack_map: StackMapId,
    /// Optional OSR entry available at this safepoint.
    pub osr_entry: Option<OsrEntryId>,
}

/// Deopt metadata for reconstructing interpreter state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeoptMap {
    /// Frame maps ordered from outermost to innermost.
    pub frames: Vec<FrameMap>,
}

/// Mapping information for a single reconstructed frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameMap {
    /// Function id for the frame.
    pub function: FunctionId,
    /// Block id for the frame.
    pub block: BlockId,
    /// Instruction id for the frame.
    pub instruction: InstructionId,
    /// Value locations indexed by MIR value id.
    pub values: Vec<ValueLoc>,
    /// Local locations indexed by MIR local id.
    pub locals: Vec<ValueLoc>,
    /// Optional return destination for the frame.
    pub return_destination: Option<ValueLoc>,
}

/// Location for a value during native execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueLoc {
    /// Value stored in a register.
    Register(Register),
    /// Value stored on the native stack.
    Stack(StackSlot),
    /// Constant value materialized on demand.
    Constant(Constant),
}

/// Register location.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Register {
    /// Register index.
    pub index: u16,
}

/// Stack slot location.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StackSlot {
    /// Stack slot index.
    pub index: u32,
    /// Byte offset from the stack pointer or frame base.
    pub offset: i32,
}

/// Constant location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Constant {
    /// Constant value payload.
    pub value: ConstantValue,
}

/// Constant payload for metadata wire formats.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConstantValue {
    /// Boolean constant.
    Boolean {
        /// The boolean value.
        value: bool,
    },
    /// Signed integer constant.
    Int {
        /// The value sign-extended to 64 bits.
        value: i64,
        /// The bit width of the integer type.
        width: u8,
        /// Whether this represents a signed integer type.
        is_signed: bool,
    },
    /// Unsigned integer constant.
    UInt {
        /// The value zero-extended to 64 bits.
        value: u64,
        /// The bit width of the integer type.
        width: u8,
    },
    /// Floating point constant.
    Float {
        /// The raw IEEE-754 bits.
        bits: u64,
        /// The bit width of the float type.
        width: u8,
    },
    /// String constant encoded as UTF-8.
    String {
        /// The string value.
        value: String,
    },
    /// Character constant as a Unicode codepoint.
    Char {
        /// The character value.
        value: char,
    },
}
