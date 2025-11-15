use dyst_dir::NodeIdAny;

use crate::CompileError;

/// Error when evaluating something statically.
#[derive(Debug, Clone)]
#[repr(u8)]
pub enum ExecuteError {
    /// Dynamic dependency cannot be statically resolved.
    NotExecutable { node: NodeIdAny } = 1,
}

impl ExecuteError {
    /// Get the numeric sub-code of the error.
    #[inline]
    pub fn sub_code(&self) -> u8 {
        match self {
            Self::NotExecutable { .. } => 1,
        }
    }
}

impl std::fmt::Display for ExecuteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExecuteError")
            .field("code", &format!("XE{:03}", self.sub_code()))
            .finish()
    }
}

pub type ExecuteResult<T> = Result<T, ExecuteError>;

impl From<ExecuteError> for CompileError {
    #[inline]
    fn from(error: ExecuteError) -> Self {
        CompileError::Execute(error)
    }
}

