use destack_vm::Error;

/// Error type for host binding handlers.
pub type HostError = Error;

/// Result type for host binding handlers.
pub type HostResult<T> = Result<T, HostError>;
