use serde::{Deserialize, Serialize};
use {destack_heap as heap, destack_mir as mir};

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

/// Errors that can occur during interpreter execution.
/// FUGU #Cleanup: reorganize VM errors / diagnostics (and also establish proper tracing?)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[repr(u8)]
pub enum Error {
    /// Attempted to execute an undefined function.
    UndefinedFunction {
        function: mir::LocalNodeId<mir::Function>,
    } = 0,

    /// Attempted to access an undefined value.
    UndefinedValue { value: mir::Value } = 1,

    /// Attempted to jump to an undefined block.
    UndefinedBlock { block: mir::LocalNodeId<mir::Block> } = 2,

    /// Type mismatch during execution.
    TypeMismatch { expected: String, actual: String } = 3,

    /// Division by zero.
    DivisionByZero = 4,

    /// Integer overflow.
    IntegerOverflow = 5,

    /// Null pointer dereference.
    NullPointerDereference = 6,

    /// Out of bounds access.
    IndexOutOfBounds { index: u64, length: u64 } = 7,

    /// Stack overflow.
    StackOverflow = 8,

    /// Reached unreachable code.
    Unreachable = 9,

    /// Binding function not found.
    BindingFunctionNotFound { name: String } = 10,

    /// Binding call forbidden by policy.
    BindingCallForbidden { name: String } = 32,

    /// Invalid instruction.
    InvalidInstruction = 11,

    /// Heap allocation failed.
    AllocationFailed = 12,

    /// One heap hard limit was exceeded.
    HeapLimitExceeded {
        scope: String,
        used_bytes: u64,
        max_bytes: u64,
    } = 13,

    /// Invalid cast operation.
    InvalidCast = 14,

    /// Execution step limit exceeded (infinite loop protection).
    StepLimitExceeded = 15,

    /// Attempted to access an undefined local variable.
    UndefinedLocal { local: mir::LocalNodeId<mir::Local> } = 16,

    /// Invalid field access (index out of bounds for struct/tuple).
    InvalidFieldAccess { index: u32, field_count: usize } = 17,

    /// Invalid array element access.
    InvalidArrayAccess { index: u64, length: u64 } = 18,

    /// Invalid heap reference.
    InvalidHeapReference = 19,

    /// Unsupported instruction for comptime evaluation.
    UnsupportedInstruction { name: String } = 20,

    /// Attempted to use a non-pointer value as a pointer (in Load/Store).
    InvalidPointerType { actual: String } = 21,

    /// Attempted to access an undefined global variable.
    UndefinedGlobal {
        global: mir::LocalNodeId<mir::Global>,
    } = 22,

    /// Attempted to write to an immutable global.
    ImmutableGlobalWrite {
        global: mir::LocalNodeId<mir::Global>,
    } = 23,

    /// Abort trap triggered.
    Abort = 24,

    /// Invalid arguments to intrinsic.
    InvalidIntrinsicArguments { intrinsic: String } = 25,

    /// Attempted to write through an immutable reference.
    ImmutableReferenceWrite { reference: String } = 26,

    /// Yielded during a non-yielding execution.
    UnexpectedYield = 28,

    /// Attempted to resume without a pending yield.
    ResumeWithoutYield = 29,

    /// Attempted to resume with an invalid continuation.
    InvalidContinuation = 30,

    /// Reference address space does not match the pointer value.
    InvalidAddressSpace { expected: String, actual: String } = 33,

    /// Unsupported zero initialization for a MIR type.
    UnsupportedZeroValue { ty: String } = 34,

    /// Panic trap triggered.
    Panic { message: String } = 35,

    /// Float to integer conversion failed.
    BadConversionToInteger = 36,

    /// Attempted to suspend while frame-local state was still live.
    SuspendWithFrameLocalState = 37,

    /// One VM stage required concrete MIR at the given use site.
    MissingRepresentation { context: String } = 38,

    /// One internal VM invariant was violated.
    InvariantViolation { context: String } = 39,

    /// Native pointer width is incompatible with the host VM.
    IncompatiblePointerWidth { bytes: u8, host_bytes: u8 } = 40,

    /// Invalid raw pointer.
    InvalidRawPointer = 41,

    /// Invalid shared raw pointer.
    InvalidSharedRawPointer = 42,

    /// Invalid shared heap reference.
    InvalidSharedHeapReference = 43,
}

impl Error {
    /// Get the numeric sub-code of the error.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        // safety: repr(u8) stores the discriminant in the first byte
        unsafe { *(self as *const Self as *const u8) }
    }

    /// Get the message of the error.
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
            Self::TypeMismatch { expected, actual } => {
                format!("type mismatch: expected {expected}, got {actual}")
            }
            Self::DivisionByZero => "division by zero".to_string(),
            Self::IntegerOverflow => "integer overflow".to_string(),
            Self::NullPointerDereference => "null pointer dereference".to_string(),
            Self::IndexOutOfBounds { index, length } => {
                format!("index out of bounds: index {index}, length {length}")
            }
            Self::StackOverflow => "stack overflow".to_string(),
            Self::Unreachable => "reached unreachable code".to_string(),
            Self::BindingFunctionNotFound { name } => {
                format!("binding function not found: {name}")
            }
            Self::BindingCallForbidden { name } => {
                format!("binding call forbidden: {name}")
            }
            Self::InvalidInstruction => "invalid instruction".to_string(),
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
            Self::InvalidCast => "invalid cast".to_string(),
            Self::StepLimitExceeded => "execution step limit exceeded".to_string(),
            Self::UndefinedLocal { local } => {
                format!("undefined local variable: {local:?}")
            }
            Self::InvalidFieldAccess { index, field_count } => {
                format!("invalid field access: index {index}, struct has {field_count} fields")
            }
            Self::InvalidArrayAccess { index, length } => {
                format!("invalid array access: index {index}, array has {length} elements")
            }
            Self::InvalidHeapReference => "invalid heap reference".to_string(),
            Self::UnsupportedInstruction { name } => {
                format!("unsupported instruction for comptime: {name}")
            }
            Self::InvalidPointerType { actual } => {
                format!("invalid pointer type: expected pointer, got {actual}")
            }
            Self::UndefinedGlobal { global } => {
                format!("undefined global variable: {global:?}")
            }
            Self::ImmutableGlobalWrite { global } => {
                format!("cannot write to immutable global: {global:?}")
            }
            Self::Abort => "abort called".to_string(),
            Self::InvalidIntrinsicArguments { intrinsic } => {
                format!("invalid arguments to intrinsic: {intrinsic}")
            }
            Self::ImmutableReferenceWrite { reference } => {
                format!("cannot write through immutable reference: {reference}")
            }
            Self::UnexpectedYield => "yielded during non-yielding execution".to_string(),
            Self::ResumeWithoutYield => "attempted to resume without a pending yield".to_string(),
            Self::InvalidContinuation => {
                "attempted to resume with an invalid continuation".to_string()
            }
            Self::InvalidAddressSpace { expected, actual } => {
                format!("invalid address space: expected {expected}, got {actual}")
            }
            Self::UnsupportedZeroValue { ty } => {
                format!("unsupported zero initialization for type {ty}")
            }
            Self::Panic { message } => {
                // normalize empty messages
                if message.is_empty() {
                    "panic".to_string()
                } else {
                    format!("panic: {message}")
                }
            }
            Self::BadConversionToInteger => "bad conversion to integer".to_string(),
            Self::SuspendWithFrameLocalState => {
                "cannot suspend while frame-local state is still live".to_string()
            }
            Self::MissingRepresentation { context } => {
                format!("concrete MIR required: {context}")
            }
            Self::InvariantViolation { context } => {
                format!("vm invariant violated: {context}")
            }
            Self::IncompatiblePointerWidth { bytes, host_bytes } => {
                format!("incompatible pointer width: program {bytes} bytes, host {host_bytes}")
            }
            Self::InvalidRawPointer => "invalid raw pointer".to_string(),
            Self::InvalidSharedRawPointer => "invalid shared raw pointer".to_string(),
            Self::InvalidSharedHeapReference => "invalid shared heap reference".to_string(),
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
            } => Self::HeapLimitExceeded {
                scope: region.to_string(),
                used_bytes,
                max_bytes,
            },
            heap::HeapError::TotalLimitExceeded {
                used_bytes,
                max_bytes,
            } => Self::HeapLimitExceeded {
                scope: "total".to_string(),
                used_bytes,
                max_bytes,
            },
            heap::HeapError::InvalidRawPointer { .. } => Self::InvalidRawPointer,
            heap::HeapError::InvalidSharedRawPointer { .. } => Self::InvalidSharedRawPointer,
            heap::HeapError::InvalidHeapReference { .. } => Self::InvalidHeapReference,
            heap::HeapError::InvalidSharedHeapReference { .. } => Self::InvalidSharedHeapReference,
            error => Self::InvariantViolation {
                context: error.to_string(),
            },
        }
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
