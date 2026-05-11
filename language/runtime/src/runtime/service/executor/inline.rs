use crate::diagnostic::RuntimeResult;

/// One inline executor for one runtime-owned service.
#[cfg_attr(
    any(target_os = "macos", target_os = "ios", target_os = "android", windows),
    allow(dead_code)
)]
pub(crate) struct InlineExecutor;

#[cfg_attr(
    any(target_os = "macos", target_os = "ios", target_os = "android", windows),
    allow(dead_code)
)]
impl InlineExecutor {
    /// Create one inline executor.
    pub(crate) fn new(_name: &str) -> Self {
        Self
    }

    /// Execute one callback directly on the caller thread.
    pub(crate) fn call<R>(
        &self,
        _operation: &'static str,
        callback: impl FnOnce() -> RuntimeResult<R>,
    ) -> RuntimeResult<R> {
        callback()
    }
}
