use serde::{Deserialize, Serialize};

use crate::diagnostic::RuntimeResult;
use crate::host::Session;
use crate::runtime::poller::HostPoller;
use crate::runtime::runtime::poller_for_backend;
use crate::runtime::world::RuntimeId;
use destack_workspace::{
    AppOptions, PlatformHostOptions, PlatformOsOptions, PollerBackend, RuntimeOptions,
};

/// Immutable runtime host reconstruction settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeHostOptions {
    /// Captured host options for runtime restore.
    pub host_options: PlatformHostOptions,
    /// Captured OS service options for runtime restore.
    pub os_options: PlatformOsOptions,
    /// Captured app declaration for runtime restore.
    pub app_declaration: AppOptions,
    /// Captured poller backend for runtime restore.
    pub poller_backend: PollerBackend,
}

impl RuntimeHostOptions {
    /// Build one runtime host reconstruction configuration from runtime options.
    pub(crate) fn from_runtime_options(options: &RuntimeOptions) -> Self {
        let (host_options, os_options, app_declaration) =
            Session::restore_config_from_runtime_options(options);

        Self {
            host_options,
            os_options,
            app_declaration,
            poller_backend: options.scheduler_options().poller_backend,
        }
    }

    /// Build one host session for the given runtime id.
    pub(crate) fn host_session(&self, runtime_id: RuntimeId) -> Session {
        Session::from_restore_config(
            runtime_id,
            self.host_options.clone(),
            self.os_options.clone(),
            self.app_declaration.clone(),
        )
    }

    /// Build one poller for this runtime configuration.
    pub(crate) fn poller(&self) -> RuntimeResult<Box<dyn HostPoller>> {
        poller_for_backend(self.poller_backend)
    }
}
