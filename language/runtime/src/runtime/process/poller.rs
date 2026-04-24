use crate::diagnostic::RuntimeResult;
use crate::runtime::poller::{HostPoller, HostPollerBackend, create_host_poller_for_runtime};
use destack_workspace::{PollerBackend, RuntimeOptions};

/// Build a poller instance from runtime options.
pub(super) fn poller_for_options(
    options: &RuntimeOptions,
) -> RuntimeResult<Option<Box<dyn HostPoller>>> {
    poller_for_backend(options.scheduler_options().poller_backend)
}

/// Build a poller instance from one explicit backend selector.
pub(super) fn poller_for_backend(
    backend: PollerBackend,
) -> RuntimeResult<Option<Box<dyn HostPoller>>> {
    // map runtime config enum into the canonical platform backend selector
    let backend = map_runtime_backend(backend);

    // create one poller instance using shared platform policy
    let poller = create_host_poller_for_runtime(backend)?;

    Ok(Some(poller))
}

/// Map runtime config backend values into canonical platform backend values.
const fn map_runtime_backend(backend: PollerBackend) -> HostPollerBackend {
    match backend {
        PollerBackend::Auto => HostPollerBackend::Auto,
        PollerBackend::IoUring => HostPollerBackend::IoUring,
        PollerBackend::Epoll => HostPollerBackend::Epoll,
        PollerBackend::Kqueue => HostPollerBackend::Kqueue,
        PollerBackend::Poll => HostPollerBackend::Poll,
        PollerBackend::Windows => HostPollerBackend::Windows,
    }
}
