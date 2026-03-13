use crate::diagnostic::RuntimeResult;
use crate::host::apple::message::is_process_main_context;
use dispatch2::run_on_main;

use super::super::executor::host::HostLoopExecutor;

/// Execute one callback on the process main thread.
pub(crate) fn call_process_main_thread<R>(
    operation: &'static str,
    service: &HostLoopExecutor,
    callback: impl FnOnce() -> RuntimeResult<R> + Send,
) -> RuntimeResult<R>
where
    R: Send,
{
    if is_process_main_context() {
        service.ensure_host_loop(operation)?;
        return callback();
    }

    run_on_main(|_| {
        service.ensure_host_loop(operation)?;
        callback()
    })
}
