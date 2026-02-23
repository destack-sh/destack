use std::sync::Arc;

use destack_workspace::{PlatformHostOptions, RuntimeOptions};

use super::adapter::{HostAdapter, HostPlatform};
use super::event::HostEvent;
use super::select::{compile_target_host_platform, default_host_adapter};
use super::service::HostServices;
use crate::diagnostic::RuntimeResult;
use crate::runtime::capability::{PlatformCapability, PlatformCapabilityId, PlatformCapabilitySet};
use crate::runtime::poller::HostPollerWakeHandle;

/// Runtime host adapter container.
#[derive(Clone)]
pub struct HostRuntime {
    /// Active host adapter for this runtime instance.
    adapter: Arc<dyn HostAdapter>,
    /// Host capability set reported by the host adapter.
    host_capabilities: PlatformCapabilitySet,
    /// Resolved host integration options for this runtime target.
    host_options: PlatformHostOptions,
    /// Filtered service surfaces enabled for this runtime target.
    services: HostServices,
}

impl std::fmt::Debug for HostRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HostRuntime")
            .field("platform", &self.platform())
            .field("host_capability_count", &self.host_capabilities.len())
            .field(
                "enable_lifecycle_events",
                &self.host_options.enable_lifecycle_events,
            )
            .field(
                "enable_window_events",
                &self.host_options.enable_window_events,
            )
            .field(
                "enable_permission_events",
                &self.host_options.enable_permission_events,
            )
            .field(
                "enable_interruption_events",
                &self.host_options.enable_interruption_events,
            )
            .field(
                "event_queue_capacity",
                &self.host_options.event_queue_capacity,
            )
            .field("callback_runtime_id", &self.callback_runtime_id())
            .finish()
    }
}

impl HostRuntime {
    /// Create one host runtime from one explicit adapter.
    pub fn new(adapter: Arc<dyn HostAdapter>) -> Self {
        Self::new_with_options(adapter, PlatformHostOptions::default())
    }

    /// Create one host runtime from one explicit adapter and host options.
    pub fn new_with_options(
        adapter: Arc<dyn HostAdapter>,
        host_options: PlatformHostOptions,
    ) -> Self {
        adapter.configure_host_options(&host_options);
        let host_capabilities = adapter.host_capabilities();
        let services = filtered_services(&adapter, &host_options);

        Self {
            adapter,
            host_capabilities,
            host_options,
            services,
        }
    }

    /// Create one host runtime from runtime options.
    pub fn from_runtime_options(options: &RuntimeOptions) -> Self {
        // select the host adapter for this compile target
        let adapter = default_host_adapter();
        let host_options = host_options_for_target(options);

        Self::new_with_options(adapter, host_options)
    }

    /// Return the active host platform.
    pub fn platform(&self) -> HostPlatform {
        self.adapter.platform()
    }

    /// Return the active adapter.
    pub fn adapter(&self) -> &Arc<dyn HostAdapter> {
        &self.adapter
    }

    /// Return host platform capabilities reported by this runtime target.
    pub fn host_capabilities(&self) -> &PlatformCapabilitySet {
        &self.host_capabilities
    }

    /// Return the resolved host options for this runtime target.
    pub fn host_options(&self) -> &PlatformHostOptions {
        &self.host_options
    }

    /// Return the configured host event queue capacity for this target.
    pub fn host_event_queue_capacity(&self) -> Option<usize> {
        self.host_options
            .event_queue_capacity
            .and_then(|capacity| usize::try_from(capacity).ok())
    }

    /// Return whether this runtime target reports one host capability id.
    pub fn has_host_capability_id(&self, capability_id: PlatformCapabilityId) -> bool {
        self.host_capabilities.contains_id(capability_id)
    }

    /// Return whether this runtime target reports one host capability.
    pub fn has_host_capability(&self, capability: PlatformCapability) -> bool {
        self.host_capabilities.contains_capability(capability)
    }

    /// Return the service surfaces exposed by this host.
    pub fn services(&self) -> &HostServices {
        &self.services
    }

    /// Poll host events using the active adapter.
    pub fn poll_events(&self, timeout_nanos: Option<u64>) -> RuntimeResult<Vec<HostEvent>> {
        let events = self.adapter.poll_events(timeout_nanos)?;

        Ok(filter_events(events, &self.host_options))
    }

    /// Return one shared host wake handle when supported.
    pub fn wake_handle(&self) -> Option<Arc<dyn HostPollerWakeHandle>> {
        self.adapter.wake_handle()
    }

    /// Return the callback runtime id for native host callback routing.
    pub fn callback_runtime_id(&self) -> Option<u64> {
        self.adapter.callback_runtime_id()
    }

    /// Take the number of dropped host events observed by this adapter.
    pub fn take_dropped_event_count(&self) -> u64 {
        self.adapter.take_dropped_event_count()
    }
}

impl Default for HostRuntime {
    fn default() -> Self {
        Self::new(default_host_adapter())
    }
}

/// Return host integration options for the current compile target.
fn host_options_for_target(options: &RuntimeOptions) -> PlatformHostOptions {
    match compile_target_host_platform() {
        HostPlatform::Android => options.platform.android.host.clone(),
        HostPlatform::DragonFly => options.platform.dragonfly.host.clone(),
        HostPlatform::FreeBsd => options.platform.freebsd.host.clone(),
        HostPlatform::Haiku => options.platform.haiku.host.clone(),
        HostPlatform::Illumos => options.platform.illumos.host.clone(),
        HostPlatform::IOS => options.platform.ios.host.clone(),
        HostPlatform::Linux => options.platform.linux.host.clone(),
        HostPlatform::MacOS => options.platform.macos.host.clone(),
        HostPlatform::NetBsd => options.platform.netbsd.host.clone(),
        HostPlatform::OpenBsd => options.platform.openbsd.host.clone(),
        HostPlatform::Solaris => options.platform.solaris.host.clone(),
        HostPlatform::Windows => options.platform.windows.host.clone(),
        _ => PlatformHostOptions::default(),
    }
}

/// Return service surfaces filtered by host integration options.
fn filtered_services(
    adapter: &Arc<dyn HostAdapter>,
    host_options: &PlatformHostOptions,
) -> HostServices {
    let adapter_services = adapter.services();
    let mut services = HostServices::default();

    // lifecycle service kind
    if host_options.enable_lifecycle_events
        && let Some(service) = adapter_services.lifecycle()
    {
        services = services.with_lifecycle(Arc::clone(service));
    }

    // window service kind
    if host_options.enable_window_events
        && let Some(service) = adapter_services.window()
    {
        services = services.with_window(Arc::clone(service));
    }

    // permission service kind
    if host_options.enable_permission_events
        && let Some(service) = adapter_services.permission()
    {
        services = services.with_permission(Arc::clone(service));
    }

    // interruption service kind
    if host_options.enable_interruption_events
        && let Some(service) = adapter_services.interruption()
    {
        services = services.with_interruption(Arc::clone(service));
    }

    services
}

/// Filter one host event vector with host integration options.
fn filter_events(events: Vec<HostEvent>, host_options: &PlatformHostOptions) -> Vec<HostEvent> {
    let mut filtered_events = Vec::with_capacity(events.len());

    for event in events {
        let is_enabled = match event {
            HostEvent::Poller(_) => true,
            HostEvent::Lifecycle(_) => host_options.enable_lifecycle_events,
            HostEvent::Window(_) => host_options.enable_window_events,
            HostEvent::Permission(_) => host_options.enable_permission_events,
            HostEvent::Interruption(_) => host_options.enable_interruption_events,
        };

        if is_enabled {
            filtered_events.push(event);
        }
    }

    filtered_events
}
