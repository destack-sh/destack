use std::sync::Arc;

use destack_workspace::{PlatformHostOptions, RuntimeOptions};

use super::adapter::{HostAdapter, HostPlatform};
use super::event::HostEvent;
use super::select::default_host_adapter;
use super::service::HostServices;
use crate::diagnostic::RuntimeResult;
use crate::runtime::capability::PlatformCapabilitySet;
use crate::runtime::poller::HostPollerWakeHandle;

/// Runtime host adapter container.
#[derive(Clone)]
pub struct HostRuntime {
    /// Active host adapter for this runtime instance.
    adapter: Arc<dyn HostAdapter>,
    /// Active capability set reported by the host adapter.
    capabilities: PlatformCapabilitySet,
    /// Resolved host integration options for this runtime target.
    host_options: PlatformHostOptions,
    /// Filtered service surfaces enabled for this runtime target.
    services: HostServices,
}

impl std::fmt::Debug for HostRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HostRuntime")
            .field("platform", &self.platform())
            .field("capability_count", &self.capabilities.len())
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
        let capabilities = adapter.capabilities();
        let services = filtered_services(&adapter, &host_options);

        Self {
            adapter,
            capabilities,
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

    /// Return the active host capability set.
    pub fn capabilities(&self) -> &PlatformCapabilitySet {
        &self.capabilities
    }

    /// Return the resolved host options for this runtime target.
    pub fn host_options(&self) -> &PlatformHostOptions {
        &self.host_options
    }

    /// Return the configured host event queue capacity for this target.
    pub fn event_queue_capacity(&self) -> Option<usize> {
        self.host_options
            .event_queue_capacity
            .and_then(|capacity| usize::try_from(capacity).ok())
    }

    /// Return whether the active host reports one capability name.
    pub fn has_capability(&self, capability_name: &str) -> bool {
        self.capabilities.contains_name(capability_name)
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
}

impl Default for HostRuntime {
    fn default() -> Self {
        Self::new(default_host_adapter())
    }
}

/// Return host integration options for the current compile target.
fn host_options_for_target(options: &RuntimeOptions) -> PlatformHostOptions {
    #[cfg(target_os = "android")]
    {
        options.platform.android.host.clone()
    }

    #[cfg(target_os = "dragonfly")]
    {
        options.platform.dragonfly.host.clone()
    }

    #[cfg(target_os = "freebsd")]
    {
        options.platform.freebsd.host.clone()
    }

    #[cfg(target_os = "haiku")]
    {
        options.platform.haiku.host.clone()
    }

    #[cfg(target_os = "illumos")]
    {
        options.platform.illumos.host.clone()
    }

    #[cfg(target_os = "ios")]
    {
        options.platform.ios.host.clone()
    }

    #[cfg(target_os = "linux")]
    {
        options.platform.linux.host.clone()
    }

    #[cfg(target_os = "macos")]
    {
        options.platform.macos.host.clone()
    }

    #[cfg(target_os = "netbsd")]
    {
        options.platform.netbsd.host.clone()
    }

    #[cfg(target_os = "openbsd")]
    {
        options.platform.openbsd.host.clone()
    }

    #[cfg(target_os = "solaris")]
    {
        options.platform.solaris.host.clone()
    }

    #[cfg(windows)]
    {
        options.platform.windows.host.clone()
    }

    #[cfg(not(any(
        target_os = "android",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "haiku",
        target_os = "illumos",
        target_os = "ios",
        target_os = "linux",
        target_os = "macos",
        target_os = "netbsd",
        target_os = "openbsd",
        target_os = "solaris",
        windows,
    )))]
    {
        PlatformHostOptions::default()
    }
}

/// Return service surfaces filtered by host integration options.
fn filtered_services(
    adapter: &Arc<dyn HostAdapter>,
    host_options: &PlatformHostOptions,
) -> HostServices {
    let adapter_services = adapter.services();
    let mut services = HostServices::default();

    // lifecycle service lane
    if host_options.enable_lifecycle_events
        && let Some(service) = adapter_services.lifecycle()
    {
        services = services.with_lifecycle(Arc::clone(service));
    }

    // window service lane
    if host_options.enable_window_events
        && let Some(service) = adapter_services.window()
    {
        services = services.with_window(Arc::clone(service));
    }

    // permission service lane
    if host_options.enable_permission_events
        && let Some(service) = adapter_services.permission()
    {
        services = services.with_permission(Arc::clone(service));
    }

    // interruption service lane
    if host_options.enable_interruption_events
        && let Some(service) = adapter_services.interruption()
    {
        services = services.with_interruption(Arc::clone(service));
    }

    // host core services
    if let Some(service) = adapter_services.asset() {
        services = services.with_asset(Arc::clone(service));
    }
    if let Some(service) = adapter_services.jni() {
        services = services.with_jni(Arc::clone(service));
    }

    // host integration surfaces
    if let Some(service) = adapter_services.display() {
        services = services.with_display(Arc::clone(service));
    }
    if let Some(service) = adapter_services.power() {
        services = services.with_power(Arc::clone(service));
    }
    if let Some(service) = adapter_services.text_input() {
        services = services.with_text_input(Arc::clone(service));
    }
    if let Some(service) = adapter_services.haptics() {
        services = services.with_haptics(Arc::clone(service));
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
