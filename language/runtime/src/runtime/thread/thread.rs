use std::thread::{self, JoinHandle};

use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;

use super::ExecutionPolicy;

/// Start one owned thread for one explicit execution policy.
pub(crate) fn start_with_policy<T>(
    name: impl Into<String>,
    operation: &'static str,
    policy: ExecutionPolicy,
    run: impl FnOnce() -> T + Send + 'static,
) -> RuntimeResult<JoinHandle<T>>
where
    T: Send + 'static,
{
    if !policy.mode.owns_thread() {
        return Err(core_platform::invalid_argument(
            "execution.mode",
            "execution start requires one thread-owning mode",
        ));
    }

    let name = name.into();

    thread::Builder::new()
        .name(name.clone())
        .spawn(run)
        .map_err(|error| {
            core_platform::io_operation_error(
                operation,
                None,
                format!(
                    "failed to start {scope} {mode} thread {name}: {error}",
                    scope = policy.scope.name(),
                    mode = policy.mode.name(),
                ),
            )
        })
}
