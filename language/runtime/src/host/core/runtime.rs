use std::sync::Arc;

use destack_workspace::{PlatformHostOptions, RuntimeOptions};

use super::adapter::{HostAdapter, HostPlatform, HostPollOutcome};
use super::event::HostEvent;
use super::select::{compile_target_host_platform, default_host};
use crate::diagnostic::RuntimeResult;
use crate::runtime::capability::{PlatformCapability, PlatformCapabilityId, PlatformCapabilitySet};
use crate::runtime::poller::HostPollerWakeHandle;

/// Runtime host integration container.
#[derive(Clone)]
pub struct Host {
    /// Active host implementation for this runtime instance.
    adapter: Arc<dyn HostAdapter>,
    /// Host capability set reported by the host implementation.
    host_capabilities: PlatformCapabilitySet,
    /// Resolved host integration options for this runtime target.
    host_options: PlatformHostOptions,
}

impl std::fmt::Debug for Host {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Host")
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

impl Host {
    /// Create one host runtime from one explicit host.
    pub fn new(adapter: Arc<dyn HostAdapter>) -> Self {
        Self::new_with_options(adapter, PlatformHostOptions::default())
    }

    /// Create one host runtime from one explicit host and host options.
    pub fn new_with_options(
        adapter: Arc<dyn HostAdapter>,
        host_options: PlatformHostOptions,
    ) -> Self {
        adapter.configure_host_options(&host_options);
        let host_capabilities = adapter.host_capabilities();

        Self {
            adapter,
            host_capabilities,
            host_options,
        }
    }

    /// Create one host runtime from runtime options.
    pub fn from_runtime_options(options: &RuntimeOptions) -> Self {
        // select the host for this compile target
        let host = default_host();
        let host_options = host_options_for_target(options);

        Self::new_with_options(host, host_options)
    }

    /// Return the active host platform.
    pub fn platform(&self) -> HostPlatform {
        self.adapter.platform()
    }

    /// Return the active host implementation.
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

    /// Poll host events using the active host.
    pub fn poll_events(&self, timeout_nanos: Option<u64>) -> RuntimeResult<HostPollOutcome> {
        let poll_result = self.adapter.poll_events(timeout_nanos)?;
        let events = filter_events(poll_result.events, &self.host_options);

        Ok(HostPollOutcome {
            events,
            dropped_event_count: poll_result.dropped_event_count,
        })
    }

    /// Return one shared host wake handle when supported.
    pub fn wake_handle(&self) -> Option<Arc<dyn HostPollerWakeHandle>> {
        self.adapter.wake_handle()
    }

    /// Return the callback runtime id for native host callback routing.
    pub fn callback_runtime_id(&self) -> Option<u64> {
        self.adapter.callback_runtime_id()
    }
}

impl Default for Host {
    fn default() -> Self {
        Self::new(default_host())
    }
}

/// Return host integration options for the current compile target.
fn host_options_for_target(options: &RuntimeOptions) -> PlatformHostOptions {
    match compile_target_host_platform() {
        HostPlatform::Android => options.platform.android.clone(),
        HostPlatform::DragonFly => options.platform.dragonfly.clone(),
        HostPlatform::FreeBsd => options.platform.freebsd.clone(),
        HostPlatform::Haiku => options.platform.haiku.clone(),
        HostPlatform::Illumos => options.platform.illumos.clone(),
        HostPlatform::IOS => options.platform.ios.clone(),
        HostPlatform::Linux => options.platform.linux.clone(),
        HostPlatform::MacOS => options.platform.macos.clone(),
        HostPlatform::NetBsd => options.platform.netbsd.clone(),
        HostPlatform::OpenBsd => options.platform.openbsd.clone(),
        HostPlatform::Solaris => options.platform.solaris.clone(),
        HostPlatform::Windows => options.platform.windows.host_options(),
        _ => PlatformHostOptions::default(),
    }
}

/// Filter one host event vector with host integration options.
fn filter_events(events: Vec<HostEvent>, host_options: &PlatformHostOptions) -> Vec<HostEvent> {
    let mut filtered_events = Vec::with_capacity(events.len());

    for event in events {
        let is_enabled = match event {
            HostEvent::Lifecycle(_) => host_options.enable_lifecycle_events,
            HostEvent::Window(_) => host_options.enable_window_events,
            HostEvent::WindowFocus(_) => host_options.enable_window_events,
            HostEvent::Permission(_) => host_options.enable_permission_events,
            HostEvent::Interruption(_) => host_options.enable_interruption_events,
            HostEvent::MemoryPressure(_) => host_options.enable_interruption_events,
            HostEvent::ThermalState(_) => host_options.enable_interruption_events,
            HostEvent::PowerMode(_) => host_options.enable_interruption_events,
            HostEvent::WallClock(_) => host_options.enable_lifecycle_events,
        };

        if is_enabled {
            filtered_events.push(event);
        }
    }

    filtered_events
}
