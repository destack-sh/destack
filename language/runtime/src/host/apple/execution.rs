use crate::diagnostic::RuntimeResult;
#[cfg(target_os = "macos")]
use crate::host::apple::message::is_process_main_context;

/// Execute one callback on the process main context when the host requires it.
#[allow(dead_code)]
pub(crate) fn call_process_main_context_if_needed<R>(
    callback: impl FnOnce() -> RuntimeResult<R> + Send,
) -> RuntimeResult<R>
where
    R: Send,
{
    #[cfg(target_os = "macos")]
    {
        if is_process_main_context() {
            return callback();
        }

        return dispatch2::run_on_main(|_| callback());
    }

    #[cfg(not(target_os = "macos"))]
    {
        callback()
    }
}

/// Execute one callback on the Apple process main context with one main-thread marker.
#[cfg(target_os = "macos")]
#[allow(dead_code)]
pub(crate) fn with_process_main_context_marker_if_needed<R>(
    callback: impl FnOnce(objc2::MainThreadMarker) -> RuntimeResult<R> + Send,
) -> RuntimeResult<R>
where
    R: Send,
{
    if let Some(marker) = objc2::MainThreadMarker::new() {
        return callback(marker);
    }

    dispatch2::run_on_main(|marker| callback(marker))
}
