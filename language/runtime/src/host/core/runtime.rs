use std::sync::Arc;

use destack_workspace::{Platform, PlatformHostOptions, RuntimeOptions};

use super::backend::{HostBackend, HostPollOutcome};
use super::event::{HostEvent, HostLifecycleState};
use super::observer::process_runtime_observer;
use super::registry::{HostRegistrationGuard, register_host_state};
use super::select::{compile_target_host_platform, default_host};
use super::state::HostState;
use crate::diagnostic::RuntimeResult;
use crate::runtime::capability::{PlatformCapability, PlatformCapabilityId, PlatformCapabilitySet};
use crate::runtime::poller::PollerWakeHandle;
use crate::runtime::world::RuntimeId;

/// Runtime host integration container.
pub struct Host {
    /// Active host implementation for this runtime instance.
    backend: Arc<dyn HostBackend>,
    /// Shared runtime state for this host instance.
    runtime_state: Arc<HostState>,
    /// Shared registration guard for callback routing.
    registration_guard: HostRegistrationGuard,
    /// Runtime id used for host callback routing and ingress observers.
    runtime_id: RuntimeId,
    /// Host capability set reported by the host implementation.
    host_capabilities: PlatformCapabilitySet,
    /// Resolved host integration options for this runtime target.
    host_options: PlatformHostOptions,
}

impl std::fmt::Debug for Host {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Host")
            .field("platform", &self.platform())
            .field("runtime_id", &self.runtime_id)
            .field(
                "registration_runtime_id",
                &self.registration_guard.runtime_id(),
            )
            .field("host_capability_count", &self.host_capabilities.len())
            .field(
                "enable_lifecycle_events",
                &self.host_options.enable_lifecycle_events,
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

impl Host {
    /// Create one host runtime from one explicit host and host options.
    pub(crate) fn new_with_options(
        backend: Arc<dyn HostBackend>,
        runtime_id: RuntimeId,
        host_options: PlatformHostOptions,
    ) -> Self {
        let platform = backend.platform();
        let runtime_state = HostState::new();

        // register callback routing before this host starts serving callers
        let registration_guard = register_host_state(
            platform,
            runtime_id,
            Arc::downgrade(&runtime_state),
            backend.runtime_state_cleanup(),
        );

        // apply queue policy to the shared host runtime state
        runtime_state.apply_host_options(&host_options);
        runtime_state.push_lifecycle(HostLifecycleState::Initializing);

        let host_capabilities = backend.host_capabilities();

        Self {
            backend,
            runtime_state,
            registration_guard,
            runtime_id,
            host_capabilities,
            host_options,
        }
    }

    /// Create one host runtime from runtime options and one explicit runtime id.
    pub fn from_runtime_options(options: &RuntimeOptions, runtime_id: RuntimeId) -> Self {
        // select the host for this compile target
        let host = default_host();
        let host_options = host_options_for_target(options);

        Self::new_with_options(host, runtime_id, host_options)
    }

    /// Return the active host platform.
    pub fn platform(&self) -> Platform {
        self.backend.platform()
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
        let poll_result = self.runtime_state.poll_events(timeout_nanos)?;
        let events = filter_events(poll_result.events, &self.host_options);

        Ok(HostPollOutcome {
            events,
            dropped_event_count: poll_result.dropped_event_count,
        })
    }

    /// Return one shared host wake handle.
    pub fn poll_wake_handle(&self) -> Arc<dyn PollerWakeHandle> {
        self.runtime_state.poll_wake_handle()
    }

    /// Return the runtime id used for host callback routing.
    pub const fn runtime_id(&self) -> RuntimeId {
        self.runtime_id
    }

    /// Return whether the current execution context is the process main context.
    pub fn is_process_main_context(&self) -> bool {
        self.backend.is_process_main_context()
    }

    /// Service immediately ready native host ingress without blocking.
    pub fn process_ingress(&self) -> RuntimeResult<bool> {
        self.backend.process_native_ingress()
    }

    /// Service host-owned ingress for the active runtime.
    pub fn process_runtime_ingress(&self) -> RuntimeResult<()> {
        // service immediately ready native ingress for this host
        self.process_ingress()?;

        // service registered runtime observers for this runtime
        process_runtime_observer(self.runtime_id.0)
    }
}

/// Return host integration options for the current compile target.
fn host_options_for_target(options: &RuntimeOptions) -> PlatformHostOptions {
    match compile_target_host_platform() {
        Platform::Android => options.platform.android.clone(),
        Platform::DragonFly => options.platform.dragonfly.clone(),
        Platform::FreeBsd => options.platform.freebsd.clone(),
        Platform::Haiku => options.platform.haiku.clone(),
        Platform::Illumos => options.platform.illumos.clone(),
        Platform::IOS => options.platform.ios.clone(),
        Platform::Linux => options.platform.linux.clone(),
        Platform::MacOS => options.platform.macos.clone(),
        Platform::NetBsd => options.platform.netbsd.clone(),
        Platform::OpenBsd => options.platform.openbsd.clone(),
        Platform::Solaris => options.platform.solaris.clone(),
        Platform::Windows => options.platform.windows.host_options(),
        _ => PlatformHostOptions::default(),
    }
}

/// Filter one host event vector with host integration options.
fn filter_events(events: Vec<HostEvent>, host_options: &PlatformHostOptions) -> Vec<HostEvent> {
    let mut filtered_events = Vec::with_capacity(events.len());

    for event in events {
        let is_enabled = match event {
            HostEvent::Lifecycle(_) => host_options.enable_lifecycle_events,
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
