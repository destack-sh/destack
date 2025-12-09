use destack_mir as mir;

/// Diagnostic anchor for MIR-level error locations.
#[derive(Debug, Clone, PartialEq)]
pub enum DiagnosticAnchor {
    /// No specific location.
    None,
    /// A specific function.
    Function(mir::LocalNodeId<mir::Function>),
    /// A specific block within a function.
    Block {
        function: mir::LocalNodeId<mir::Function>,
        block: mir::LocalNodeId<mir::Block>,
    },
    /// A specific instruction within a block.
    Instruction {
        function: mir::LocalNodeId<mir::Function>,
        block: mir::LocalNodeId<mir::Block>,
        instruction: mir::LocalNodeId<mir::Instruction>,
    },
}

/// Information about a call frame for stack traces.
#[derive(Debug, Clone, PartialEq)]
pub struct FrameInfo {
    /// The function being executed.
    pub function: mir::LocalNodeId<mir::Function>,
    /// The block being executed.
    pub block: mir::LocalNodeId<mir::Block>,
    /// Function name (if available).
    pub function_name: Option<String>,
}

/// Errors that can occur during interpreter execution.
#[derive(Debug, Clone, PartialEq)]
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

    /// External function not found.
    ExternalFunctionNotFound { name: String } = 10,

    /// Invalid instruction.
    InvalidInstruction = 11,

    /// Memory allocation failed (heap limit exceeded).
    AllocationFailed = 12,

    /// Invalid cast operation.
    InvalidCast = 13,

    /// Execution step limit exceeded (infinite loop protection).
    StepLimitExceeded = 14,

    /// Attempted to access an undefined local variable.
    UndefinedLocal { local: mir::LocalNodeId<mir::Local> } = 15,

    /// Invalid field access (index out of bounds for struct/tuple).
    InvalidFieldAccess { index: u32, field_count: usize } = 16,

    /// Invalid array element access.
    InvalidArrayAccess { index: u64, length: u64 } = 17,

    /// Invalid heap handle (dangling reference).
    InvalidHeapHandle = 18,

    /// Unsupported instruction for comptime evaluation.
    UnsupportedInstruction { name: String } = 19,

    /// Attempted to use a non-pointer value as a pointer (in Load/Store).
    InvalidPointerType { actual: String } = 20,

    /// Attempted to access an undefined global variable.
    UndefinedGlobal {
        global: mir::LocalNodeId<mir::Global>,
    } = 21,

    /// Attempted to write to an immutable global.
    ImmutableGlobalWrite {
        global: mir::LocalNodeId<mir::Global>,
    } = 22,
}

impl Error {
    /// Get the numeric sub-code of the error.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        // Safety: repr(u8) ensures the discriminant is valid
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
            Self::ExternalFunctionNotFound { name } => {
                format!("external function not found: {name}")
            }
            Self::InvalidInstruction => "invalid instruction".to_string(),
            Self::AllocationFailed => "memory allocation failed (heap limit exceeded)".to_string(),
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
            Self::InvalidHeapHandle => "invalid heap handle (dangling reference)".to_string(),
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
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EM{:03}: {}", self.sub_code(), self.message())
    }
}

impl std::error::Error for Error {}

/// A runtime error with call stack and location information.
#[derive(Debug, Clone)]
pub struct RuntimeError {
    /// The underlying error.
    pub error: Error,
    /// The call stack at the time of the error.
    pub call_stack: Vec<FrameInfo>,
    /// The location where the error occurred.
    pub anchor: DiagnosticAnchor,
}

impl RuntimeError {
    /// Create a new runtime error.
    pub fn new(error: Error) -> Self {
        Self {
            error,
            call_stack: Vec::new(),
            anchor: DiagnosticAnchor::None,
        }
    }

    /// Add call stack information.
    pub fn with_call_stack(mut self, call_stack: Vec<FrameInfo>) -> Self {
        self.call_stack = call_stack;
        self
    }

    /// Add location information.
    pub fn with_anchor(mut self, anchor: DiagnosticAnchor) -> Self {
        self.anchor = anchor;
        self
    }

    /// Format a stack trace for display.
    pub fn format_stack_trace(&self) -> String {
        if self.call_stack.is_empty() {
            return String::new();
        }

        let mut trace = String::from("\nStack trace:\n");
        for (i, frame) in self.call_stack.iter().rev().enumerate() {
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

/// Result type for machine operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Result type for runtime operations that include stack traces.
pub type RuntimeResult<T> = std::result::Result<T, RuntimeError>;
