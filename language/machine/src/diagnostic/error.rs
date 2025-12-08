use destack_mir as mir;

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
    UndefinedBlock {
        block: mir::LocalNodeId<mir::Block>,
    } = 2,

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

    /// Memory allocation failed.
    AllocationFailed = 12,

    /// Invalid cast operation.
    InvalidCast = 13,
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
            Self::AllocationFailed => "memory allocation failed".to_string(),
            Self::InvalidCast => "invalid cast".to_string(),
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EM{:03}: {}", self.sub_code(), self.message())
    }
}

impl std::error::Error for Error {}

/// Result type for machine operations.
pub type Result<T> = std::result::Result<T, Error>;
