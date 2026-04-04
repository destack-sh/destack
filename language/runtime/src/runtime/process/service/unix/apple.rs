use crate::diagnostic::RuntimeResult;
use crate::host::os::apple::call::call_process_main_context_if_needed;
use crate::host::os::apple::ingress::r#loop::is_process_main_context;

use super::super::executor::host::HostExecutor;

/// Execute one callback on the process main thread.
pub(crate) fn call_process_main_thread<R>(
    operation: &'static str,
    service: &HostExecutor,
    callback: impl FnOnce() -> RuntimeResult<R> + Send,
) -> RuntimeResult<R>
where
    R: Send,
{
    if is_process_main_context() {
        service.ensure_host_loop(operation)?;
        return callback();
    }

    call_process_main_context_if_needed(|| {
        service.ensure_host_loop(operation)?;
        callback()
    })
}
