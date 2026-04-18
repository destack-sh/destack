use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::BindingCallContext;

/// Status code returned by native runtime bindings.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeStatus {
    /// Status code, where zero indicates success.
    pub code: u32,
    /// Identifier for the captured runtime error, if any.
    pub error_id: u64,
}

impl RuntimeStatus {
    /// Successful status.
    pub const OK: Self = Self {
        code: 0,
        error_id: 0,
    };

    /// Build an error status from a runtime error.
    pub fn from_error(error: Box<RuntimeError>, context: Option<&BindingCallContext>) -> Self {
        let code = error.sub_code().saturating_add(1);
        let error_id = context
            .map(|context| context.worker().diagnostics.record_error(error).to_raw())
            .unwrap_or(0);
        Self { code, error_id }
    }

    /// Convert a platform result into a status.
    pub fn from_result<T>(result: RuntimeResult<T>, context: Option<&BindingCallContext>) -> Self {
        match result {
            Ok(_) => Self::OK,
            Err(error) => Self::from_error(error, context),
        }
    }
}
