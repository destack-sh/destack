use std::sync::Arc;

use destack_workspace::{Platform, PlatformHostOptions, RuntimeOptions};

use super::backend::{HostBackend, HostPollOutcome};
use super::event::{HostEvent, HostLifecycleEvent, HostLifecycleState};
use super::observer::RuntimeIngressObserverRegistry;
use super::queue::HostQueue;
use super::registry::{HostCleanup, HostQueueRegistry, HostRegistrationGuard};
use crate::diagnostic::RuntimeResult;
#[cfg(target_os = "android")]
use crate::host::android::AndroidHost;
#[cfg(target_os = "android")]
use crate::host::android::unregister_android_bindings;
#[cfg(target_os = "dragonfly")]
use crate::host::dragonfly::DragonflyHost;
#[cfg(target_os = "freebsd")]
use crate::host::freebsd::FreeBsdHost;
#[cfg(target_os = "haiku")]
use crate::host::haiku::HaikuHost;
#[cfg(target_os = "illumos")]
use crate::host::illumos::IllumosHost;
#[cfg(target_os = "ios")]
use crate::host::ios::IosHost;
#[cfg(target_os = "linux")]
use crate::host::linux::LinuxHost;
#[cfg(target_os = "macos")]
use crate::host::macos::MacosHost;
#[cfg(target_os = "netbsd")]
use crate::host::netbsd::NetBsdHost;
#[cfg(target_os = "openbsd")]
use crate::host::openbsd::OpenBsdHost;
#[cfg(target_os = "solaris")]
use crate::host::solaris::SolarisHost;
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
use crate::host::unsupported::UnsupportedHost;
#[cfg(windows)]
use crate::host::windows::WindowsHost;
use crate::runtime::capability::{PlatformCapability, PlatformCapabilityId, PlatformCapabilitySet};
use crate::runtime::poller::PollerWakeHandle;
use crate::runtime::world::RuntimeId;

/// Runtime host integration container.
pub struct Host {
    /// Active host implementation for this runtime instance.
    backend: Arc<dyn HostBackend>,
    /// Shared host queue for this runtime instance.
    queue: Arc<HostQueue>,
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
        cleanup: Option<HostCleanup>,
        runtime_id: RuntimeId,
        host_options: PlatformHostOptions,
    ) -> Self {
        let platform = backend.platform();
        let queue = Arc::new(HostQueue::new());

        // register callback routing before this host starts serving callers
        let registration_guard = HostQueueRegistry::shared().write().register(
            platform,
            runtime_id,
            Arc::downgrade(&queue),
            cleanup,
        );

        // apply queue policy to the shared host event queue
        let queue_capacity = host_options
            .event_queue_capacity
            .and_then(|capacity| usize::try_from(capacity).ok());
        queue.configure_capacity(queue_capacity);

        // seed one initializing lifecycle event for the new runtime
        queue.enqueue(HostEvent::Lifecycle(HostLifecycleEvent {
            state: HostLifecycleState::Initializing,
        }));

        let host_capabilities = backend.host_capabilities();

        Self {
            backend,
            queue,
            registration_guard,
            runtime_id,
            host_capabilities,
            host_options,
        }
    }

    /// Create one host runtime from runtime options and one explicit runtime id.
    pub fn from_runtime_options(options: &RuntimeOptions, runtime_id: RuntimeId) -> Self {
        // select the host for this compile target
        let (host, cleanup) = Self::default_backend_parts();
        let host_options = Self::host_options_for_target(options);

        Self::new_with_options(host, cleanup, runtime_id, host_options)
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
        let events = self.queue.poll_events(timeout_nanos)?;
        let events = self.filter_events(events);
        let dropped_event_count = self.queue.take_dropped_event_count();

        Ok(HostPollOutcome {
            events,
            dropped_event_count,
        })
    }

    /// Return one shared host wake handle.
    pub fn poll_wake_handle(&self) -> Arc<dyn PollerWakeHandle> {
        self.queue.poll_wake_handle()
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
        RuntimeIngressObserverRegistry::shared()
            .write()
            .process_runtime(self.runtime_id.0)
    }

    /// Return the default backend and runtime cleanup for the active compile target.
    fn default_backend_parts() -> (Arc<dyn HostBackend>, Option<HostCleanup>) {
        #[cfg(target_os = "android")]
        return (
            Arc::new(AndroidHost::new()),
            Some(unregister_android_bindings),
        );

        #[cfg(target_os = "dragonfly")]
        return (Arc::new(DragonflyHost::new()), None);

        #[cfg(target_os = "freebsd")]
        return (Arc::new(FreeBsdHost::new()), None);

        #[cfg(target_os = "haiku")]
        return (Arc::new(HaikuHost::new()), None);

        #[cfg(target_os = "illumos")]
        return (Arc::new(IllumosHost::new()), None);

        #[cfg(target_os = "ios")]
        return (Arc::new(IosHost::new()), None);

        #[cfg(target_os = "linux")]
        return (Arc::new(LinuxHost::new()), None);

        #[cfg(target_os = "macos")]
        return (Arc::new(MacosHost::new()), None);

        #[cfg(target_os = "netbsd")]
        return (Arc::new(NetBsdHost::new()), None);

        #[cfg(target_os = "openbsd")]
        return (Arc::new(OpenBsdHost::new()), None);

        #[cfg(target_os = "solaris")]
        return (Arc::new(SolarisHost::new()), None);

        #[cfg(windows)]
        return (Arc::new(WindowsHost::new()), None);

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
        return (Arc::new(UnsupportedHost::new()), None);
    }

    /// Return host integration options for the current compile target.
    fn host_options_for_target(options: &RuntimeOptions) -> PlatformHostOptions {
        match Self::compile_target_host_platform() {
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
    fn filter_events(&self, events: Vec<HostEvent>) -> Vec<HostEvent> {
        let mut filtered_events = Vec::with_capacity(events.len());

        for event in events {
            let is_enabled = match event {
                HostEvent::Lifecycle(_) => self.host_options.enable_lifecycle_events,
                HostEvent::Permission(_) => self.host_options.enable_permission_events,
                HostEvent::Interruption(_) => self.host_options.enable_interruption_events,
                HostEvent::MemoryPressure(_) => self.host_options.enable_interruption_events,
                HostEvent::ThermalState(_) => self.host_options.enable_interruption_events,
                HostEvent::PowerMode(_) => self.host_options.enable_interruption_events,
                HostEvent::WallClock(_) => self.host_options.enable_lifecycle_events,
            };

            if is_enabled {
                filtered_events.push(event);
            }
        }

        filtered_events
    }

    /// Return the host platform for the current compile target.
    const fn compile_target_host_platform() -> Platform {
        #[cfg(target_os = "android")]
        {
            Platform::Android
        }

        #[cfg(target_os = "dragonfly")]
        {
            Platform::DragonFly
        }

        #[cfg(target_os = "freebsd")]
        {
            Platform::FreeBsd
        }

        #[cfg(target_os = "haiku")]
        {
            Platform::Haiku
        }

        #[cfg(target_os = "illumos")]
        {
            Platform::Illumos
        }

        #[cfg(target_os = "ios")]
        {
            Platform::IOS
        }

        #[cfg(target_os = "linux")]
        {
            Platform::Linux
        }

        #[cfg(target_os = "macos")]
        {
            Platform::MacOS
        }

        #[cfg(target_os = "netbsd")]
        {
            Platform::NetBsd
        }

        #[cfg(target_os = "openbsd")]
        {
            Platform::OpenBsd
        }

        #[cfg(target_os = "solaris")]
        {
            Platform::Solaris
        }

        #[cfg(windows)]
        {
            Platform::Windows
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
            Platform::Universal
        }
    }
}
