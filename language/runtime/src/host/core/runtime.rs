use std::sync::Arc;

use destack_workspace::{Platform, PlatformHostOptions, RuntimeAppDeclaration, RuntimeOptions};

use crate::diagnostic::RuntimeResult;
#[cfg(target_os = "android")]
use crate::host::android::AndroidHost;
#[cfg(target_os = "android")]
use crate::host::android::unregister_android_bindings;
use crate::host::common::require_declared_request;
use crate::host::core::adapter::{HostAdapter, HostPollOutcome};
use crate::host::core::event::{HostEvent, HostLifecycleEvent, HostLifecycleState};
use crate::host::core::queue::HostQueue;
use crate::host::core::registry::{HostCleanup, HostRegistrationGuard, HostRuntimeRegistry};
use crate::host::core::request::{HostRequest, HostRequestContext, HostRequestOutcome};
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
#[cfg(target_os = "ios")]
use crate::host::ios::unregister_ios_bindings;
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

/// Runtime-scoped host session.
pub struct HostSession {
    /// Active host adapter for this runtime session.
    adapter: Arc<dyn HostAdapter>,
    /// Shared host queue for this runtime instance.
    queue: Arc<HostQueue>,
    /// Shared registration guard for callback routing.
    registration_guard: HostRegistrationGuard,
    /// Runtime id used for host callback routing and ingress observers.
    runtime_id: RuntimeId,
    /// Static host capability set reported by the adapter.
    adapter_capabilities: PlatformCapabilitySet,
    /// Resolved host integration options for this runtime target.
    host_options: PlatformHostOptions,
    /// Resolved target app declaration for request availability checks.
    app_declaration: RuntimeAppDeclaration,
}

impl std::fmt::Debug for HostSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HostSession")
            .field("platform", &self.platform())
            .field("runtime_id", &self.runtime_id)
            .field(
                "registration_runtime_id",
                &self.registration_guard.runtime_id(),
            )
            .field("adapter_capability_count", &self.adapter_capabilities.len())
            .field(
                "session_capability_count",
                &self.session_capabilities().len(),
            )
            .field(
                "event_queue_capacity",
                &self.host_options.event_queue_capacity,
            )
            .finish()
    }
}

impl HostSession {
    /// Create one host session from one explicit adapter and host options.
    pub(crate) fn new_with_options(
        adapter: Arc<dyn HostAdapter>,
        cleanup: Option<HostCleanup>,
        runtime_id: RuntimeId,
        host_options: PlatformHostOptions,
        app_declaration: RuntimeAppDeclaration,
    ) -> Self {
        let platform = adapter.platform();
        let queue = Arc::new(HostQueue::new(runtime_id));

        // register callback routing before this host starts serving callers
        let registration_guard = HostRuntimeRegistry::register_queue(
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

        let adapter_capabilities = adapter.static_capabilities();
        Self {
            adapter,
            queue,
            registration_guard,
            runtime_id,
            adapter_capabilities,
            host_options,
            app_declaration,
        }
    }

    /// Create one host session from runtime options and one explicit runtime id.
    pub fn from_runtime_options(options: &RuntimeOptions, runtime_id: RuntimeId) -> Self {
        // resolve compile-target host integration once
        let (platform, adapter, cleanup) = Self::default_compile_target_parts();
        let host_options = host_options_for_target(platform, options);
        let app_declaration = options.app.clone();

        Self::new_with_options(adapter, cleanup, runtime_id, host_options, app_declaration)
    }

    /// Return the active host platform.
    pub fn platform(&self) -> Platform {
        self.adapter.platform()
    }

    /// Return effective host capabilities reported by adapter and session wiring.
    pub fn host_capabilities(&self) -> PlatformCapabilitySet {
        let session_capabilities = self.session_capabilities();

        merge_capabilities(self.adapter_capabilities.clone(), session_capabilities)
    }

    /// Return the static host capabilities reported by this adapter.
    pub fn adapter_capabilities(&self) -> &PlatformCapabilitySet {
        &self.adapter_capabilities
    }

    /// Return the dynamic session capabilities reported by the active adapter.
    pub fn session_capabilities(&self) -> PlatformCapabilitySet {
        self.adapter.session_capabilities(self.runtime_id)
    }

    /// Return whether adapter and session wiring report one host capability id.
    pub fn has_host_capability_id(&self, capability_id: PlatformCapabilityId) -> bool {
        self.adapter_capabilities.contains_id(capability_id)
            || self.session_capabilities().contains_id(capability_id)
    }

    /// Return whether adapter and session wiring report one host capability.
    pub fn has_host_capability(&self, capability: PlatformCapability) -> bool {
        self.has_host_capability_id(capability.id())
    }

    /// Submit one normalized host request through the active session.
    pub(crate) fn submit_request(&self, request: HostRequest) -> RuntimeResult<HostRequestOutcome> {
        // request declaration
        require_declared_request(self.platform(), &self.app_declaration, &request)?;

        // request context
        let request_context = HostRequestContext {
            runtime_id: self.runtime_id,
            platform: self.platform(),
            is_process_main_context: self.is_process_main_context(),
        };

        self.adapter.submit_request(&request_context, request)
    }

    /// Poll host events using the active host.
    pub fn poll_events(&self, timeout_nanos: Option<u64>) -> RuntimeResult<HostPollOutcome> {
        // drain the queued host event stream directly
        let events = self.queue.poll_events(timeout_nanos)?;

        // report queue pressure independently from the drained event batch
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
        self.adapter.is_process_main_context()
    }

    /// Service host-owned ingress for this runtime session.
    pub fn service_ingress(&self) -> RuntimeResult<()> {
        // service immediately ready native ingress from the adapter
        self.adapter.process_native_ingress()?;

        // service runtime-owned ingress observers registered for this session
        HostRuntimeRegistry::process_runtime_ingress(self.runtime_id)
    }

    /// Return the default host integration parts for the active compile target.
    fn default_compile_target_parts() -> (Platform, Arc<dyn HostAdapter>, Option<HostCleanup>) {
        #[cfg(target_os = "android")]
        return (
            Platform::Android,
            Arc::new(AndroidHost::new()),
            Some(unregister_android_bindings),
        );

        #[cfg(target_os = "dragonfly")]
        return (Platform::DragonFly, Arc::new(DragonflyHost::new()), None);

        #[cfg(target_os = "freebsd")]
        return (Platform::FreeBsd, Arc::new(FreeBsdHost::new()), None);

        #[cfg(target_os = "haiku")]
        return (Platform::Haiku, Arc::new(HaikuHost::new()), None);

        #[cfg(target_os = "illumos")]
        return (Platform::Illumos, Arc::new(IllumosHost::new()), None);

        #[cfg(target_os = "ios")]
        return (
            Platform::IOS,
            Arc::new(IosHost::new()),
            Some(unregister_ios_bindings),
        );

        #[cfg(target_os = "linux")]
        return (Platform::Linux, Arc::new(LinuxHost::new()), None);

        #[cfg(target_os = "macos")]
        return (Platform::MacOS, Arc::new(MacosHost::new()), None);

        #[cfg(target_os = "netbsd")]
        return (Platform::NetBsd, Arc::new(NetBsdHost::new()), None);

        #[cfg(target_os = "openbsd")]
        return (Platform::OpenBsd, Arc::new(OpenBsdHost::new()), None);

        #[cfg(target_os = "solaris")]
        return (Platform::Solaris, Arc::new(SolarisHost::new()), None);

        #[cfg(windows)]
        return (Platform::Windows, Arc::new(WindowsHost::new()), None);

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
        return (Platform::Universal, Arc::new(UnsupportedHost::new()), None);
    }
}

/// Return host integration options for one compile target platform.
fn host_options_for_target(platform: Platform, options: &RuntimeOptions) -> PlatformHostOptions {
    match platform {
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

/// Merge one static and one dynamic capability set into one effective set.
fn merge_capabilities(
    static_capabilities: PlatformCapabilitySet,
    dynamic_capabilities: PlatformCapabilitySet,
) -> PlatformCapabilitySet {
    let mut merged = static_capabilities;

    for capability_id in dynamic_capabilities.iter() {
        merged.insert_id(*capability_id);
    }

    merged
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use destack_workspace::{
        RuntimeAppDeclaration, RuntimeAppIntentDeclaration, RuntimeAppNotificationDeclaration,
    };

    use super::{
        HostAdapter, HostCleanup, HostRequest, HostRequestContext, HostRequestOutcome, HostSession,
        Platform, PlatformHostOptions, RuntimeId, RuntimeResult,
    };
    use crate::host::core::HostRequestResult;
    use crate::platform::os::abi_generated::DocumentPickOptionsValue;
    use crate::platform::os::{NotificationPermissionState, Permission, PermissionState};
    use crate::runtime::capability::PlatformCapabilitySet;

    /// Test host adapter that records request submissions.
    #[derive(Debug)]
    struct TestHostAdapter {
        /// Host platform reported by the adapter.
        platform: Platform,
        /// Submission counter for preflight assertions.
        submit_count: Arc<AtomicUsize>,
    }

    impl HostAdapter for TestHostAdapter {
        /// Return the configured host platform.
        fn platform(&self) -> Platform {
            self.platform
        }

        /// Return one empty static capability set for focused request tests.
        fn static_capabilities(&self) -> PlatformCapabilitySet {
            PlatformCapabilitySet::new()
        }

        /// Return one empty session capability set for focused request tests.
        fn session_capabilities(&self, _runtime_id: RuntimeId) -> PlatformCapabilitySet {
            PlatformCapabilitySet::new()
        }

        /// Submit one request and return one deterministic test payload.
        fn submit_request(
            &self,
            _context: &HostRequestContext,
            request: HostRequest,
        ) -> RuntimeResult<HostRequestOutcome> {
            // submission bookkeeping
            self.submit_count.fetch_add(1, Ordering::Relaxed);

            let outcome = match request {
                HostRequest::OsIntentCanOpenUrl { .. } => {
                    HostRequestOutcome::immediate(HostRequestResult::Bool(true))
                }
                HostRequest::OsNotificationRequestPermission => {
                    HostRequestOutcome::immediate(HostRequestResult::NotificationPermissionState(
                        NotificationPermissionState::Granted,
                    ))
                }
                HostRequest::OsDocumentPick { .. } => HostRequestOutcome::immediate(
                    HostRequestResult::DocumentDescriptors(Vec::new()),
                ),
                HostRequest::OsPermissionRequest { .. } => HostRequestOutcome::immediate(
                    HostRequestResult::PermissionState(PermissionState::Granted),
                ),
                _ => HostRequestOutcome::immediate(HostRequestResult::None),
            };

            Ok(outcome)
        }
    }

    /// Build one test host session with one explicit platform and app declaration.
    fn test_host_session(
        platform: Platform,
        app: RuntimeAppDeclaration,
    ) -> (HostSession, Arc<AtomicUsize>) {
        let submit_count = Arc::new(AtomicUsize::new(0));
        let adapter = Arc::new(TestHostAdapter {
            platform,
            submit_count: submit_count.clone(),
        });
        let cleanup: Option<HostCleanup> = None;
        let runtime_id = RuntimeId(91);
        let host_options = PlatformHostOptions::default();
        let host = HostSession::new_with_options(adapter, cleanup, runtime_id, host_options, app);

        (host, submit_count)
    }

    /// Reject undeclared permission requests before adapter submission.
    #[test]
    fn test_submit_request_rejects_undeclared_permission_before_adapter() {
        let (host, submit_count) =
            test_host_session(Platform::Android, RuntimeAppDeclaration::default());

        // submit one undeclared permission request
        let error = host
            .submit_request(HostRequest::OsPermissionRequest {
                permission: Permission::Camera,
            })
            .expect_err("undeclared permission request should fail");
        let message = error.to_string();

        // keep the adapter out of the path
        assert_eq!(submit_count.load(Ordering::Relaxed), 0);
        assert!(
            message.contains("destack.os.permission.request requires one target app declaration")
        );
        assert!(message.contains("permissions.camera"));
    }

    /// Reject undeclared iOS query schemes before adapter submission.
    #[test]
    fn test_submit_request_rejects_undeclared_ios_query_scheme() {
        let (host, submit_count) =
            test_host_session(Platform::IOS, RuntimeAppDeclaration::default());

        // submit one undeclared query-scheme request
        let error = host
            .submit_request(HostRequest::OsIntentCanOpenUrl {
                url: "mailto:test@example.com".to_string(),
            })
            .expect_err("undeclared query scheme should fail");
        let message = error.to_string();

        // keep the adapter out of the path
        assert_eq!(submit_count.load(Ordering::Relaxed), 0);
        assert!(message.contains("intents.querySchemes"));
        assert!(message.contains("mailto"));
    }

    /// Allow notification permission requests when notifications are declared.
    #[test]
    fn test_submit_request_allows_declared_notification_permission_request() {
        // notification declaration
        let app = RuntimeAppDeclaration {
            notifications: RuntimeAppNotificationDeclaration {
                enabled: true,
                ..RuntimeAppNotificationDeclaration::default()
            },
            ..RuntimeAppDeclaration::default()
        };
        let (host, submit_count) = test_host_session(Platform::IOS, app);

        // submit one declared notification permission request
        let outcome = host
            .submit_request(HostRequest::OsNotificationRequestPermission)
            .expect("declared notification permission request should succeed");
        let permission_state = outcome
            .into_notification_permission_state("destack.os.notification.requestPermission")
            .expect("notification permission request should decode one permission state");

        // ensure the adapter received the request
        assert_eq!(submit_count.load(Ordering::Relaxed), 1);
        assert_eq!(permission_state, NotificationPermissionState::Granted);
    }

    /// Allow declaration-free document picker requests to reach the adapter.
    #[test]
    fn test_submit_request_allows_declaration_free_document_pick() {
        let (host, submit_count) =
            test_host_session(Platform::MacOS, RuntimeAppDeclaration::default());

        // submit one declaration-free document picker request
        let outcome = host
            .submit_request(HostRequest::OsDocumentPick {
                options: DocumentPickOptionsValue {
                    mime_types: Vec::new(),
                    extensions: Vec::new(),
                    multiple: false,
                    allow_directories: false,
                    copy_to_sandbox: false,
                },
            })
            .expect("document picker request should remain declaration-free");
        let descriptors = outcome
            .into_document_descriptors("destack.os.document.pick")
            .expect("document picker request should decode document descriptors");

        // ensure the adapter received the request
        assert_eq!(submit_count.load(Ordering::Relaxed), 1);
        assert!(descriptors.is_empty());
    }

    /// Reject undeclared Android file sharing before adapter submission.
    #[test]
    fn test_submit_request_rejects_undeclared_android_file_share() {
        let (host, submit_count) =
            test_host_session(Platform::Android, RuntimeAppDeclaration::default());

        // submit one undeclared file-share request
        let error = host
            .submit_request(HostRequest::OsIntentSharePaths {
                paths: Vec::new(),
                mime_type: Some("text/plain".to_string()),
            })
            .expect_err("undeclared Android file share should fail");
        let message = error.to_string();

        // keep the adapter out of the path
        assert_eq!(submit_count.load(Ordering::Relaxed), 0);
        assert!(message.contains("intents.sharesFiles"));
    }

    /// Allow declared Android file sharing to reach the adapter.
    #[test]
    fn test_submit_request_allows_declared_android_file_share() {
        let app = RuntimeAppDeclaration {
            intents: RuntimeAppIntentDeclaration {
                shares_files: true,
                ..RuntimeAppIntentDeclaration::default()
            },
            ..RuntimeAppDeclaration::default()
        };
        let (host, submit_count) = test_host_session(Platform::Android, app);

        // submit one declared file-share request
        let outcome = host
            .submit_request(HostRequest::OsIntentSharePaths {
                paths: Vec::new(),
                mime_type: Some("text/plain".to_string()),
            })
            .expect("declared Android file share should succeed");

        // ensure the adapter received the request
        assert_eq!(submit_count.load(Ordering::Relaxed), 1);
        assert_eq!(outcome.result, HostRequestResult::None);
    }
}
