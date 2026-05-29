use destack_heap as heap;
use destack_memory as memory;
use destack_mir as mir;
use serde::{Deserialize, Serialize};

/// Anchor for MIR-level error locations.
#[derive(Debug, Clone, PartialEq)]
pub enum DiagnosticAnchor {
    /// No specific location.
    None,
    /// Specific function.
    Function(mir::LocalNodeId<mir::Function>),
    /// Specific block within a function.
    Block {
        function: mir::LocalNodeId<mir::Function>,
        block: mir::LocalNodeId<mir::Block>,
    },
    /// Specific instruction within a block.
    Instruction {
        function: mir::LocalNodeId<mir::Function>,
        block: mir::LocalNodeId<mir::Block>,
        instruction: mir::LocalNodeId<mir::Instruction>,
    },
}

/// One frame in a diagnostic call stack.
#[derive(Debug, Clone, PartialEq)]
pub struct StackTraceFrame {
    /// The function being executed.
    pub function: mir::LocalNodeId<mir::Function>,
    /// The block being executed.
    pub block: mir::LocalNodeId<mir::Block>,
    /// Function name (if available).
    pub function_name: Option<String>,
}

/// One VM reference space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReferenceKind {
    /// Local managed heap reference.
    Heap,
    /// Local raw pointer.
    Raw,
    /// Shared managed heap reference.
    SharedHeap,
    /// Shared raw pointer.
    SharedRaw,
}

/// Errors that can occur during VM execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Error {
    /// The program representation is invalid or unsupported.
    Program { reason: ProgramError },
    /// Execution reached a language trap.
    Trap { reason: Trap },
    /// An imported host function failed at the VM boundary.
    Import { name: String, reason: ImportError },
    /// VM resource budget or capacity was exhausted.
    Resource { reason: ResourceError },
    /// One internal VM invariant failed.
    Internal { context: String },
}

/// Invalid or unsupported VM program representation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProgramError {
    /// Attempted to execute an undefined function.
    UndefinedFunction {
        function: mir::LocalNodeId<mir::Function>,
    },
    /// Attempted to access an undefined value.
    UndefinedValue { value: mir::Value },
    /// Attempted to jump to an undefined block.
    UndefinedBlock { block: mir::LocalNodeId<mir::Block> },
    /// Attempted to access an undefined local variable.
    UndefinedLocal { local: mir::LocalNodeId<mir::Local> },
    /// Attempted to access an undefined global variable.
    UndefinedGlobal {
        global: mir::LocalNodeId<mir::Global>,
    },
    /// Type mismatch during execution.
    TypeMismatch { expected: String, actual: String },
    /// Invalid instruction.
    InvalidInstruction,
    /// Invalid field access.
    InvalidFieldAccess { index: u32, field_count: usize },
    /// Invalid array element access.
    InvalidArrayAccess { index: u64, length: u64 },
    /// Unsupported instruction for comptime evaluation.
    UnsupportedInstruction { name: String },
    /// Invalid arguments to intrinsic.
    InvalidIntrinsicArguments { intrinsic: String },
    /// Unsupported zero initialization for a MIR type.
    UnsupportedZeroValue { ty: String },
    /// The VM program representation is invalid or incomplete.
    InvalidProgram { context: String },
    /// Native pointer width is incompatible with the host VM.
    IncompatiblePointerWidth { bytes: u8, host_bytes: u8 },
}

/// Language trap reached while executing code.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Trap {
    /// Division by zero.
    DivisionByZero,
    /// Integer overflow.
    IntegerOverflow,
    /// Null pointer dereference.
    NullPointerDereference,
    /// Out of bounds access.
    IndexOutOfBounds { index: u64, length: u64 },
    /// Invalid reference value.
    InvalidReference { kind: ReferenceKind },
    /// Attempted to use a non-pointer value as a pointer.
    InvalidPointerType { actual: String },
    /// Reference space does not match the pointer value.
    InvalidSpace { expected: String, actual: String },
    /// Attempted to write to an immutable global.
    ImmutableGlobalWrite {
        global: mir::LocalNodeId<mir::Global>,
    },
    /// Attempted to write through a readonly reference.
    ImmutableReferenceWrite { reference: String },
    /// Reached unreachable code.
    Unreachable,
    /// Invalid cast operation.
    InvalidCast,
    /// Yielded during a non-yielding execution.
    UnexpectedYield,
    /// Attempted to resume without a pending yield.
    ResumeWithoutYield,
    /// Attempted to resume with an invalid continuation.
    InvalidContinuation,
    /// Attempted to suspend while frame-local state was still live.
    SuspendWithFrameLocalState,
    /// Abort trap triggered.
    Abort,
    /// Panic trap triggered.
    Panic { message: String },
    /// Float to integer conversion failed.
    BadConversionToInteger,
}

/// Import boundary failure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImportError {
    /// Imported function not found.
    NotFound,
    /// Imported function call forbidden by policy.
    Forbidden,
}

/// VM resource failure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceError {
    /// Heap allocation failed.
    AllocationFailed,
    /// One heap hard limit was exceeded.
    HeapLimitExceeded {
        scope: String,
        used_bytes: u64,
        max_bytes: u64,
    },
    /// Stack overflow.
    StackOverflow,
    /// Execution step limit exceeded.
    StepLimitExceeded,
}

impl Error {
    /// Return an undefined function error.
    #[inline]
    pub fn undefined_function(function: mir::LocalNodeId<mir::Function>) -> Self {
        Self::Program {
            reason: ProgramError::UndefinedFunction { function },
        }
    }

    /// Return an undefined value error.
    #[inline]
    pub fn undefined_value(value: mir::Value) -> Self {
        Self::Program {
            reason: ProgramError::UndefinedValue { value },
        }
    }

    /// Return an undefined block error.
    #[inline]
    pub fn undefined_block(block: mir::LocalNodeId<mir::Block>) -> Self {
        Self::Program {
            reason: ProgramError::UndefinedBlock { block },
        }
    }

    /// Return an undefined local error.
    #[inline]
    pub fn undefined_local(local: mir::LocalNodeId<mir::Local>) -> Self {
        Self::Program {
            reason: ProgramError::UndefinedLocal { local },
        }
    }

    /// Return an undefined global error.
    #[inline]
    pub fn undefined_global(global: mir::LocalNodeId<mir::Global>) -> Self {
        Self::Program {
            reason: ProgramError::UndefinedGlobal { global },
        }
    }

    /// Return a type mismatch error.
    #[inline]
    pub fn type_mismatch(expected: impl Into<String>, actual: impl Into<String>) -> Self {
        Self::Program {
            reason: ProgramError::TypeMismatch {
                expected: expected.into(),
                actual: actual.into(),
            },
        }
    }

    /// Return an invalid instruction error.
    #[inline]
    pub const fn invalid_instruction() -> Self {
        Self::Program {
            reason: ProgramError::InvalidInstruction,
        }
    }

    /// Return an invalid field access error.
    #[inline]
    pub fn invalid_field_access(index: u32, field_count: usize) -> Self {
        Self::Program {
            reason: ProgramError::InvalidFieldAccess { index, field_count },
        }
    }

    /// Return an invalid array access error.
    #[inline]
    pub fn invalid_array_access(index: u64, length: u64) -> Self {
        Self::Program {
            reason: ProgramError::InvalidArrayAccess { index, length },
        }
    }

    /// Return an unsupported instruction error.
    #[inline]
    pub fn unsupported_instruction(name: impl Into<String>) -> Self {
        Self::Program {
            reason: ProgramError::UnsupportedInstruction { name: name.into() },
        }
    }

    /// Return an invalid intrinsic arguments error.
    #[inline]
    pub fn invalid_intrinsic_arguments(intrinsic: impl Into<String>) -> Self {
        Self::Program {
            reason: ProgramError::InvalidIntrinsicArguments {
                intrinsic: intrinsic.into(),
            },
        }
    }

    /// Return an unsupported zero value error.
    #[inline]
    pub fn unsupported_zero_value(ty: impl Into<String>) -> Self {
        Self::Program {
            reason: ProgramError::UnsupportedZeroValue { ty: ty.into() },
        }
    }

    /// Return an invalid program error.
    #[inline]
    pub fn invalid_program(context: impl Into<String>) -> Self {
        Self::Program {
            reason: ProgramError::InvalidProgram {
                context: context.into(),
            },
        }
    }

    /// Return an incompatible pointer width error.
    #[inline]
    pub fn incompatible_pointer_width(bytes: u8, host_bytes: u8) -> Self {
        Self::Program {
            reason: ProgramError::IncompatiblePointerWidth { bytes, host_bytes },
        }
    }

    /// Return a division by zero trap.
    #[inline]
    pub fn division_by_zero() -> Self {
        Self::Trap {
            reason: Trap::DivisionByZero,
        }
    }

    /// Return an integer overflow trap.
    #[inline]
    pub fn integer_overflow() -> Self {
        Self::Trap {
            reason: Trap::IntegerOverflow,
        }
    }

    /// Return a null pointer dereference trap.
    #[inline]
    pub fn null_pointer_dereference() -> Self {
        Self::Trap {
            reason: Trap::NullPointerDereference,
        }
    }

    /// Return an index out of bounds trap.
    #[inline]
    pub fn index_out_of_bounds(index: u64, length: u64) -> Self {
        Self::Trap {
            reason: Trap::IndexOutOfBounds { index, length },
        }
    }

    /// Return a stack overflow trap.
    #[inline]
    pub fn stack_overflow() -> Self {
        Self::Resource {
            reason: ResourceError::StackOverflow,
        }
    }

    /// Return an unreachable trap.
    #[inline]
    pub fn unreachable() -> Self {
        Self::Trap {
            reason: Trap::Unreachable,
        }
    }

    /// Return an invalid cast trap.
    #[inline]
    pub fn invalid_cast() -> Self {
        Self::Trap {
            reason: Trap::InvalidCast,
        }
    }

    /// Return an abort trap.
    #[inline]
    pub fn abort() -> Self {
        Self::Trap {
            reason: Trap::Abort,
        }
    }

    /// Return a panic trap.
    #[inline]
    pub fn panic(message: impl Into<String>) -> Self {
        Self::Trap {
            reason: Trap::Panic {
                message: message.into(),
            },
        }
    }

    /// Return a bad integer conversion trap.
    #[inline]
    pub fn bad_conversion_to_integer() -> Self {
        Self::Trap {
            reason: Trap::BadConversionToInteger,
        }
    }

    /// Return a missing import error.
    #[inline]
    pub fn import_not_found(name: impl Into<String>) -> Self {
        Self::Import {
            name: name.into(),
            reason: ImportError::NotFound,
        }
    }

    /// Return a forbidden import call error.
    #[inline]
    pub fn import_forbidden(name: impl Into<String>) -> Self {
        Self::Import {
            name: name.into(),
            reason: ImportError::Forbidden,
        }
    }

    /// Return an invalid reference error.
    #[inline]
    pub fn invalid_reference(kind: ReferenceKind) -> Self {
        Self::Trap {
            reason: Trap::InvalidReference { kind },
        }
    }

    /// Return an invalid pointer type error.
    #[inline]
    pub fn invalid_pointer_type(actual: impl Into<String>) -> Self {
        Self::Trap {
            reason: Trap::InvalidPointerType {
                actual: actual.into(),
            },
        }
    }

    /// Return an invalid pointer space error.
    #[inline]
    pub fn invalid_space(expected: impl Into<String>, actual: impl Into<String>) -> Self {
        Self::Trap {
            reason: Trap::InvalidSpace {
                expected: expected.into(),
                actual: actual.into(),
            },
        }
    }

    /// Return an immutable global write error.
    #[inline]
    pub fn immutable_global_write(global: mir::LocalNodeId<mir::Global>) -> Self {
        Self::Trap {
            reason: Trap::ImmutableGlobalWrite { global },
        }
    }

    /// Return an immutable reference write error.
    #[inline]
    pub fn immutable_reference_write(reference: impl Into<String>) -> Self {
        Self::Trap {
            reason: Trap::ImmutableReferenceWrite {
                reference: reference.into(),
            },
        }
    }

    /// Return a step limit error.
    #[inline]
    pub fn step_limit_exceeded() -> Self {
        Self::Resource {
            reason: ResourceError::StepLimitExceeded,
        }
    }

    /// Return an unexpected yield error.
    #[inline]
    pub fn unexpected_yield() -> Self {
        Self::Trap {
            reason: Trap::UnexpectedYield,
        }
    }

    /// Return a resume without yield error.
    #[inline]
    pub fn resume_without_yield() -> Self {
        Self::Trap {
            reason: Trap::ResumeWithoutYield,
        }
    }

    /// Return an invalid continuation error.
    #[inline]
    pub fn invalid_continuation() -> Self {
        Self::Trap {
            reason: Trap::InvalidContinuation,
        }
    }

    /// Return a frame local suspension error.
    #[inline]
    pub fn suspend_with_frame_local_state() -> Self {
        Self::Trap {
            reason: Trap::SuspendWithFrameLocalState,
        }
    }

    /// Return an allocation failed error.
    #[inline]
    pub fn allocation_failed() -> Self {
        Self::Resource {
            reason: ResourceError::AllocationFailed,
        }
    }

    /// Return a heap limit error.
    #[inline]
    pub fn heap_limit_exceeded(scope: impl Into<String>, used_bytes: u64, max_bytes: u64) -> Self {
        Self::Resource {
            reason: ResourceError::HeapLimitExceeded {
                scope: scope.into(),
                used_bytes,
                max_bytes,
            },
        }
    }

    /// Return an internal VM error.
    #[inline]
    pub fn internal(context: impl Into<String>) -> Self {
        Self::Internal {
            context: context.into(),
        }
    }

    /// Return the numeric error code.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::Program { reason } => reason.sub_code(),
            Self::Trap { reason } => reason.sub_code(),
            Self::Import { reason, .. } => reason.sub_code(),
            Self::Resource { reason } => reason.sub_code(),
            Self::Internal { .. } => 39,
        }
    }

    /// Return the message of the error.
    pub fn message(&self) -> String {
        match self {
            Self::Program { reason } => reason.message(),
            Self::Trap { reason } => reason.message(),
            Self::Import { name, reason } => reason.message(name),
            Self::Resource { reason } => reason.message(),
            Self::Internal { context } => {
                format!("internal vm error: {context}")
            }
        }
    }
}

impl ProgramError {
    /// Return the numeric error code.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::UndefinedFunction { .. } => 0,
            Self::UndefinedValue { .. } => 1,
            Self::UndefinedBlock { .. } => 2,
            Self::TypeMismatch { .. } => 3,
            Self::InvalidInstruction => 11,
            Self::UndefinedLocal { .. } => 16,
            Self::InvalidFieldAccess { .. } => 17,
            Self::InvalidArrayAccess { .. } => 18,
            Self::UnsupportedInstruction { .. } => 20,
            Self::UndefinedGlobal { .. } => 22,
            Self::InvalidIntrinsicArguments { .. } => 25,
            Self::UnsupportedZeroValue { .. } => 34,
            Self::InvalidProgram { .. } => 38,
            Self::IncompatiblePointerWidth { .. } => 40,
        }
    }

    /// Return the message of the error.
    pub fn message(&self) -> String {
        match self {
            Self::UndefinedFunction { function } => {
                format!("undefined function: {function:?}")
            }
            Self::UndefinedValue { value } => {
                format!("undefined value: {value:?}")
            }
            Self::UndefinedBlock { block } => {
                format!("undefined block: {block:?}")
            }
            Self::UndefinedLocal { local } => {
                format!("undefined local variable: {local:?}")
            }
            Self::UndefinedGlobal { global } => {
                format!("undefined global variable: {global:?}")
            }
            Self::TypeMismatch { expected, actual } => {
                format!("type mismatch: expected {expected}, got {actual}")
            }
            Self::InvalidInstruction => "invalid instruction".to_string(),
            Self::InvalidFieldAccess { index, field_count } => {
                format!("invalid field access: index {index}, struct has {field_count} fields")
            }
            Self::InvalidArrayAccess { index, length } => {
                format!("invalid array access: index {index}, array has {length} elements")
            }
            Self::UnsupportedInstruction { name } => {
                format!("unsupported instruction for comptime: {name}")
            }
            Self::InvalidIntrinsicArguments { intrinsic } => {
                format!("invalid arguments to intrinsic: {intrinsic}")
            }
            Self::UnsupportedZeroValue { ty } => {
                format!("unsupported zero initialization for type {ty}")
            }
            Self::InvalidProgram { context } => {
                format!("invalid vm program: {context}")
            }
            Self::IncompatiblePointerWidth { bytes, host_bytes } => {
                format!("incompatible pointer width: program {bytes} bytes, host {host_bytes}")
            }
        }
    }
}

impl Trap {
    /// Return the numeric error code.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::DivisionByZero => 4,
            Self::IntegerOverflow => 5,
            Self::NullPointerDereference => 6,
            Self::IndexOutOfBounds { .. } => 7,
            Self::InvalidReference { .. } => 19,
            Self::InvalidPointerType { .. } => 21,
            Self::ImmutableGlobalWrite { .. } => 23,
            Self::ImmutableReferenceWrite { .. } => 26,
            Self::UnexpectedYield => 28,
            Self::ResumeWithoutYield => 29,
            Self::InvalidContinuation => 30,
            Self::InvalidSpace { .. } => 33,
            Self::SuspendWithFrameLocalState => 37,
            Self::Unreachable => 9,
            Self::InvalidCast => 14,
            Self::Abort => 24,
            Self::Panic { .. } => 35,
            Self::BadConversionToInteger => 36,
        }
    }

    /// Return the message of the trap.
    pub fn message(&self) -> String {
        match self {
            Self::DivisionByZero => "division by zero".to_string(),
            Self::IntegerOverflow => "integer overflow".to_string(),
            Self::NullPointerDereference => "null pointer dereference".to_string(),
            Self::IndexOutOfBounds { index, length } => {
                format!("index out of bounds: index {index}, length {length}")
            }
            Self::InvalidReference { kind } => {
                format!("invalid {} reference", kind.name())
            }
            Self::InvalidPointerType { actual } => {
                format!("invalid pointer type: expected pointer, got {actual}")
            }
            Self::InvalidSpace { expected, actual } => {
                format!("invalid space: expected {expected}, got {actual}")
            }
            Self::ImmutableGlobalWrite { global } => {
                format!("cannot write to immutable global: {global:?}")
            }
            Self::ImmutableReferenceWrite { reference } => {
                format!("cannot write through readonly reference: {reference}")
            }
            Self::Unreachable => "reached unreachable code".to_string(),
            Self::InvalidCast => "invalid cast".to_string(),
            Self::UnexpectedYield => "yielded during non-yielding execution".to_string(),
            Self::ResumeWithoutYield => "attempted to resume without a pending yield".to_string(),
            Self::InvalidContinuation => {
                "attempted to resume with an invalid continuation".to_string()
            }
            Self::SuspendWithFrameLocalState => {
                "cannot suspend while frame-local state is still live".to_string()
            }
            Self::Abort => "abort called".to_string(),
            Self::Panic { message } => {
                if message.is_empty() {
                    "panic".to_string()
                } else {
                    format!("panic: {message}")
                }
            }
            Self::BadConversionToInteger => "bad conversion to integer".to_string(),
        }
    }
}

impl ImportError {
    /// Return the numeric error code.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::NotFound => 10,
            Self::Forbidden => 32,
        }
    }

    /// Return the message of the error.
    pub fn message(&self, name: &str) -> String {
        match self {
            Self::NotFound => format!("imported function not found: {name}"),
            Self::Forbidden => format!("imported function call forbidden: {name}"),
        }
    }
}

impl ResourceError {
    /// Return the numeric error code.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::AllocationFailed => 12,
            Self::HeapLimitExceeded { .. } => 13,
            Self::StackOverflow => 8,
            Self::StepLimitExceeded => 15,
        }
    }

    /// Return the message of the error.
    pub fn message(&self) -> String {
        match self {
            Self::AllocationFailed => "allocation failed".to_string(),
            Self::HeapLimitExceeded {
                scope,
                used_bytes,
                max_bytes,
            } => {
                format!(
                    "{scope} heap limit exceeded: using {used_bytes} bytes with limit {max_bytes}"
                )
            }
            Self::StackOverflow => "stack overflow".to_string(),
            Self::StepLimitExceeded => "execution step limit exceeded".to_string(),
        }
    }
}

impl ReferenceKind {
    /// Return the diagnostic reference kind name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Heap => "heap",
            Self::Raw => "raw",
            Self::SharedHeap => "shared heap",
            Self::SharedRaw => "shared raw",
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EM{:03}: {}", self.sub_code(), self.message())
    }
}

impl std::error::Error for Error {}

impl From<heap::HeapError> for Error {
    fn from(error: heap::HeapError) -> Self {
        match error {
            heap::HeapError::LimitExceeded {
                region,
                used_bytes,
                max_bytes,
            } => Self::heap_limit_exceeded(region.to_string(), used_bytes, max_bytes),
            heap::HeapError::InvalidReference { kind, .. } => match kind {
                heap::HeapReferenceKind::Heap => Self::invalid_reference(ReferenceKind::Heap),
                heap::HeapReferenceKind::Raw => Self::invalid_reference(ReferenceKind::Raw),
                heap::HeapReferenceKind::SharedHeap => {
                    Self::invalid_reference(ReferenceKind::SharedHeap)
                }
                heap::HeapReferenceKind::SharedRaw => {
                    Self::invalid_reference(ReferenceKind::SharedRaw)
                }
            },
            error => Self::internal(error.to_string()),
        }
    }
}

impl From<memory::MemoryError> for Error {
    fn from(error: memory::MemoryError) -> Self {
        Self::internal(error.to_string())
    }
}

/// A runtime error with call stack and location information.
#[derive(Debug, Clone)]
pub struct RuntimeError {
    /// The underlying error.
    pub error: Error,
    /// The call stack at the time of the error.
    pub stack: Vec<StackTraceFrame>,
    /// The location where the error occurred.
    pub anchor: DiagnosticAnchor,
}

impl RuntimeError {
    /// Create a new runtime error.
    pub fn new(error: Error) -> Self {
        Self {
            error,
            stack: Vec::new(),
            anchor: DiagnosticAnchor::None,
        }
    }

    /// Add call stack information.
    pub fn with_call_stack(mut self, stack: Vec<StackTraceFrame>) -> Self {
        self.stack = stack;
        self
    }

    /// Add location information.
    pub fn with_anchor(mut self, anchor: DiagnosticAnchor) -> Self {
        self.anchor = anchor;
        self
    }

    /// Format a stack trace for display.
    pub fn format_stack_trace(&self) -> String {
        if self.stack.is_empty() {
            return String::new();
        }

        let mut trace = String::from("\nStack trace:\n");
        for (i, frame) in self.stack.iter().rev().enumerate() {
            let name = frame.function_name.as_deref().unwrap_or("<anonymous>");
            trace.push_str(&format!("  {i}: {name} (block {:?})\n", frame.block));
        }
        trace
    }
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.error, self.format_stack_trace())
    }
}

impl std::error::Error for RuntimeError {}

impl From<Error> for RuntimeError {
    fn from(error: Error) -> Self {
        Self::new(error)
    }
}

/// Result type for VM operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Result type for runtime operations that include stack traces.
pub type RuntimeResult<T> = std::result::Result<T, RuntimeError>;
