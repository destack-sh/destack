use tspp_bytecode as bytecode;
use tspp_heap::HeapError;
use tspp_program as program;

use super::Error;

/// One machine or runtime failure raised during bytecode execution.
#[derive(Debug)]
pub(crate) enum ExecutionError<E> {
    /// Bytecode machine failure.
    Machine(Error),
    /// Runtime operation failure.
    Runtime(E),
}

/// Result of one bytecode execution operation.
pub(crate) type ExecutionResult<T, E> = std::result::Result<T, ExecutionError<E>>;

impl<E> ExecutionError<E> {
    /// Create one runtime operation failure.
    pub(crate) fn runtime(error: E) -> Self {
        Self::Runtime(error)
    }

    /// Collapse this internal failure into the runtime-owned error type.
    pub(crate) fn into_error(self) -> E
    where
        E: From<Error>,
    {
        match self {
            Self::Machine(error) => error.into(),
            Self::Runtime(error) => error,
        }
    }
}

impl<E> From<Error> for ExecutionError<E> {
    /// Preserve one bytecode machine failure.
    fn from(error: Error) -> Self {
        Self::Machine(error)
    }
}

impl<E> From<bytecode::Error> for ExecutionError<E> {
    /// Preserve one bytecode decoding failure.
    fn from(error: bytecode::Error) -> Self {
        Self::Machine(Error::bytecode(error))
    }
}

impl<E> From<program::Error> for ExecutionError<E> {
    /// Preserve one Program operation failure.
    fn from(error: program::Error) -> Self {
        Self::Machine(Error::program(error))
    }
}

impl<E> From<HeapError> for ExecutionError<E> {
    /// Preserve one heap operation failure.
    fn from(error: HeapError) -> Self {
        Self::Machine(Error::heap(error))
    }
}
