use crate::diagnostic::RuntimeResult;
use crate::platform::PlatformPoller;
use crate::platform::poller::{PlatformPollerBackend, create_platform_poller_for_runtime};
use destack_workspace::{PollerBackend, RuntimeOptions};

/// Build a poller instance from runtime options.
pub(super) fn poller_for_options(
    options: &RuntimeOptions,
) -> RuntimeResult<Option<Box<dyn PlatformPoller>>> {
    // map runtime config enum into the canonical platform backend selector
    let backend = map_runtime_backend(options.scheduler.poller_backend);

    // create one poller instance using shared platform policy
    let poller = create_platform_poller_for_runtime(backend)?;

    Ok(Some(poller))
}

/// Map runtime config backend values into canonical platform backend values.
const fn map_runtime_backend(backend: PollerBackend) -> PlatformPollerBackend {
    match backend {
        PollerBackend::Auto => PlatformPollerBackend::Auto,
        PollerBackend::IoUring => PlatformPollerBackend::IoUring,
        PollerBackend::Epoll => PlatformPollerBackend::Epoll,
        PollerBackend::Kqueue => PlatformPollerBackend::Kqueue,
        PollerBackend::Poll => PlatformPollerBackend::Poll,
        PollerBackend::Windows => PlatformPollerBackend::Windows,
    }
}
