use crate::diagnostic::RuntimeResult;

/// One caller-thread executor for one host-affine platform service.
#[cfg_attr(
    any(target_os = "ios", target_os = "android", windows),
    allow(dead_code)
)]
pub(crate) struct CallerThreadExecutor;

#[cfg_attr(
    any(target_os = "ios", target_os = "android", windows),
    allow(dead_code)
)]
impl CallerThreadExecutor {
    /// Create one caller-thread executor.
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
