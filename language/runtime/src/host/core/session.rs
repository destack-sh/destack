use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use destack_artifact::Platform;
use destack_workspace::{
    PlatformHostOptions, PlatformOsOptions, RuntimeAppDeclaration, RuntimeOptions,
};

use crate::diagnostic::RuntimeResult;
use crate::host::core::adapter::{HostAdapter, PollResult};
use crate::host::core::queue::HostQueue;
use crate::host::core::registry::{
    HostCleanup, HostSessionId, HostSessionRegistrationGuard, HostSessionRegistry,
};
use crate::host::core::request::{
    HostRequest, HostRequestId, HostRequestOutcome, RequestContext, SessionContext,
};
use crate::host::core::target::{default_compile_target_parts, host_options_for_target};
use crate::host::operation::HostOperation;
use crate::host::policy::require_declared_request;
use crate::runtime::capability::{PlatformCapability, PlatformCapabilityId, PlatformCapabilitySet};
use crate::runtime::poller::PollerWakeHandle;
use crate::runtime::world::RuntimeId;

/// Runtime-scoped session boundary between one runtime, one host adapter, and one host ingress queue.
pub struct Session {
    /// Active host adapter for this runtime session.
    adapter: Arc<dyn HostAdapter>,
    /// Shared ingress queue for this runtime instance.
    queue: Arc<HostQueue>,
    /// Shared registration guard for host ingress routing.
    registration_guard: HostSessionRegistrationGuard,
    /// Logical runtime id used by the world/runtime layer.
    runtime_id: RuntimeId,
    /// Process-global routing id used for host ingress routing and queue servicing.
    host_session_id: HostSessionId,
    /// Whether ambient native ingress should be advanced during poll.
    is_native_ingress_enabled: bool,
    /// Session-scoped request id allocator for outbound host requests.
    next_host_request_id: AtomicU64,

    /// Static host capability set reported by the host adapter.
    adapter_capabilities: PlatformCapabilitySet,
    /// Resolved host integration options for this runtime target.
    host_options: PlatformHostOptions,
    /// Resolved OS runtime options for host-backed services.
    os_options: PlatformOsOptions,
    /// Resolved target app declaration for request availability checks.
    app_declaration: RuntimeAppDeclaration,
}

impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Session")
            .field("platform", &self.platform())
            .field("runtime_id", &self.runtime_id)
            .field("host_session_id", &self.host_session_id)
            .field(
                "registration_runtime_id",
                &self.registration_guard.host_session_id(),
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

impl Session {
    /// Capture host restore configuration from runtime options.
    pub(crate) fn restore_config_from_runtime_options(
        options: &RuntimeOptions,
    ) -> (
        PlatformHostOptions,
        PlatformOsOptions,
        RuntimeAppDeclaration,
    ) {
        // resolve compile-target host integration once
        let (platform, _adapter, _cleanup) = default_compile_target_parts();
        let host_options = host_options_for_target(platform, options);
        let os_options = options.os.clone();
        let app_declaration = options.app.clone();

        (host_options, os_options, app_declaration)
    }

    /// Create one session from one explicit host adapter and host options.
    pub(crate) fn new_with_options(
        adapter: Arc<dyn HostAdapter>,
        cleanup: Option<HostCleanup>,
        runtime_id: RuntimeId,
        host_options: PlatformHostOptions,
        os_options: PlatformOsOptions,
        app_declaration: RuntimeAppDeclaration,
        is_native_ingress_enabled: bool,
    ) -> Self {
        Self::new_with_options_inner(
            adapter,
            cleanup,
            runtime_id,
            host_options,
            os_options,
            app_declaration,
            is_native_ingress_enabled,
        )
    }

    /// Create one session from one explicit host adapter and host options.
    fn new_with_options_inner(
        adapter: Arc<dyn HostAdapter>,
        cleanup: Option<HostCleanup>,
        runtime_id: RuntimeId,
        host_options: PlatformHostOptions,
        os_options: PlatformOsOptions,
        app_declaration: RuntimeAppDeclaration,
        is_native_ingress_enabled: bool,
    ) -> Self {
        let platform = adapter.platform();
        let host_session_id = HostSessionRegistry::allocate_session_id();
        let queue = Arc::new(HostQueue::new(host_session_id));

        // register host ingress routing before this host starts serving callers
        let registration_guard = HostSessionRegistry::register_queue(
            platform,
            host_session_id,
            Arc::clone(&queue),
            cleanup,
        );

        // apply queue policy to the shared host event queue
        let queue_capacity = host_options
            .event_queue_capacity
            .and_then(|capacity| usize::try_from(capacity).ok());
        queue.configure_capacity(queue_capacity);

        let adapter_capabilities = adapter.static_capabilities();
        Self {
            adapter,
            queue,
            registration_guard,
            runtime_id,
            host_session_id,
            is_native_ingress_enabled,
            next_host_request_id: AtomicU64::new(1),
            adapter_capabilities,
            host_options,
            os_options,
            app_declaration,
        }
    }

    /// Create one session from runtime options and one explicit runtime id.
    pub fn from_runtime_options(options: &RuntimeOptions, runtime_id: RuntimeId) -> Self {
        // resolve compile-target host integration once
        let (_platform, adapter, cleanup) = default_compile_target_parts();
        let (host_options, os_options, app_declaration) =
            Self::restore_config_from_runtime_options(options);

        Self::new_with_options(
            adapter,
            cleanup,
            runtime_id,
            host_options,
            os_options,
            app_declaration,
            true,
        )
    }

    /// Create one session from one captured host-restore configuration.
    pub(crate) fn from_restore_config(
        runtime_id: RuntimeId,
        host_options: PlatformHostOptions,
        os_options: PlatformOsOptions,
        app_declaration: RuntimeAppDeclaration,
    ) -> Self {
        // resolve compile-target host integration once
        let (_platform, adapter, cleanup) = default_compile_target_parts();

        Self::new_with_options(
            adapter,
            cleanup,
            runtime_id,
            host_options,
            os_options,
            app_declaration,
            true,
        )
    }

    /// Create one session from runtime options with one explicit native-ingress policy.
    pub(crate) fn from_runtime_options_with_native_ingress(
        options: &RuntimeOptions,
        runtime_id: RuntimeId,
        is_native_ingress_enabled: bool,
    ) -> Self {
        // resolve compile-target host integration once
        let (_platform, adapter, cleanup) = default_compile_target_parts();
        let (host_options, os_options, app_declaration) =
            Self::restore_config_from_runtime_options(options);

        Self::new_with_options_inner(
            adapter,
            cleanup,
            runtime_id,
            host_options,
            os_options,
            app_declaration,
            is_native_ingress_enabled,
        )
    }

    /// Return the active host platform.
    pub fn platform(&self) -> Platform {
        self.adapter.platform()
    }

    /// Return effective host capabilities reported by the host adapter and session wiring.
    pub fn host_capabilities(&self) -> PlatformCapabilitySet {
        let session_capabilities = self.session_capabilities();

        merge_capabilities(self.adapter_capabilities.clone(), session_capabilities)
    }

    /// Return the static host capabilities reported by this host adapter.
    pub fn adapter_capabilities(&self) -> &PlatformCapabilitySet {
        &self.adapter_capabilities
    }

    /// Return the dynamic session capabilities reported by the active host adapter.
    pub fn session_capabilities(&self) -> PlatformCapabilitySet {
        self.adapter.session_capabilities(self.host_session_id)
    }

    /// Return whether the host adapter and session wiring report one host capability id.
    pub fn has_host_capability_id(&self, capability_id: PlatformCapabilityId) -> bool {
        self.adapter_capabilities.contains_id(capability_id)
            || self.session_capabilities().contains_id(capability_id)
    }

    /// Return whether shell and session wiring report one host capability.
    pub fn has_host_capability(&self, capability: PlatformCapability) -> bool {
        self.has_host_capability_id(capability.id())
    }

    /// Submit one normalized runtime-owned host request through the active session.
    pub(crate) fn submit_request(&self, request: HostRequest) -> RuntimeResult<HostRequestOutcome> {
        // request declaration
        require_declared_request(self.platform(), &self.app_declaration, &request)?;

        // request context
        let request_id = self.allocate_request_id();

        self.submit_with_id(request_id, request)
    }

    /// Submit one typed host operation through the active session.
    pub(crate) fn submit_operation<T>(&self, operation: HostOperation<T>) -> RuntimeResult<T> {
        let request = operation.request().clone();
        let outcome = self.submit_request(request)?;

        operation.decode_outcome(outcome)
    }

    /// Poll host ingress and drain queued host events using the active session.
    pub fn poll(&self, timeout_nanos: Option<u64>) -> RuntimeResult<PollResult> {
        // advance host and session ingress before draining queued host events
        self.advance_ingress()?;

        // drain the queued host event stream directly
        let events = self.queue.poll_events(timeout_nanos)?;
        // report queue pressure independently from the drained event batch
        let dropped_event_count = self.queue.take_dropped_event_count();

        Ok(PollResult {
            events,
            dropped_event_count,
        })
    }

    /// Return one shared host wake handle.
    pub fn poll_wake_handle(&self) -> Arc<dyn PollerWakeHandle> {
        self.queue.poll_wake_handle()
    }

    /// Return the logical runtime id for this session.
    pub const fn runtime_id(&self) -> RuntimeId {
        self.runtime_id
    }

    /// Return the process-global host routing id used for callback registration.
    pub(crate) const fn host_session_id(&self) -> HostSessionId {
        self.host_session_id
    }

    /// Build one host request context for this session.
    fn session_context(&self) -> SessionContext {
        SessionContext {
            host_session_id: self.host_session_id,
            platform: self.platform(),
            os_options: self.os_options.clone(),
            app_identity: self.app_declaration.identity.clone(),
            is_process_main_context: self.is_process_main_context(),
        }
    }

    /// Build one request context for one explicit request id.
    fn request_context_with_id(&self, request_id: HostRequestId) -> RequestContext {
        let session_context = self.session_context();

        RequestContext {
            request_id,
            host_session_id: session_context.host_session_id,
            platform: session_context.platform,
            os_options: session_context.os_options,
            app_identity: session_context.app_identity,
            is_process_main_context: session_context.is_process_main_context,
        }
    }

    /// Submit one normalized runtime-owned host request with one explicit request id.
    pub(crate) fn submit_with_id(
        &self,
        request_id: HostRequestId,
        request: HostRequest,
    ) -> RuntimeResult<HostRequestOutcome> {
        // request declaration
        require_declared_request(self.platform(), &self.app_declaration, &request)?;

        // request context
        let request_context = self.request_context_with_id(request_id);

        self.adapter.submit_request(&request_context, request)
    }

    /// Allocate one fresh request id for this session.
    pub(crate) fn allocate_request_id(&self) -> HostRequestId {
        let request_id = self.next_host_request_id.fetch_add(1, Ordering::Relaxed);

        HostRequestId(request_id)
    }

    /// Return whether the current execution context is the process main context.
    pub fn is_process_main_context(&self) -> bool {
        self.adapter.is_process_main_context()
    }

    /// Advance host-owned and session-owned ingress for this session.
    pub(crate) fn advance_ingress(&self) -> RuntimeResult<()> {
        // native ingress
        if self.is_native_ingress_enabled {
            self.adapter.advance_native_ingress()?;
        }

        // session ingress
        let session_context = self.session_context();
        self.adapter.advance_session_ingress(&session_context)?;
        HostSessionRegistry::advance_session_ingress(self.host_session_id)
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
        PlatformOsOptions, RuntimeAppDeclaration, RuntimeAppIntentDeclaration,
        RuntimeAppNotificationDeclaration,
    };

    use super::{
        HostAdapter, HostCleanup, HostRequest, HostRequestOutcome, Platform, PlatformHostOptions,
        RequestContext, RuntimeId, RuntimeResult, Session,
    };
    use crate::host::{HostRequestResult, HostSessionId};
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
        fn session_capabilities(&self, _runtime_id: HostSessionId) -> PlatformCapabilitySet {
            PlatformCapabilitySet::new()
        }

        /// Submit one request and return one deterministic test payload.
        fn submit_request(
            &self,
            _context: &RequestContext,
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

    /// Build one test session with one explicit platform and app declaration.
    fn test_session(platform: Platform, app: RuntimeAppDeclaration) -> (Session, Arc<AtomicUsize>) {
        let submit_count = Arc::new(AtomicUsize::new(0));
        let adapter = Arc::new(TestHostAdapter {
            platform,
            submit_count: submit_count.clone(),
        });
        let cleanup: Option<HostCleanup> = None;
        let runtime_id = RuntimeId(91);
        let host_options = PlatformHostOptions::default();
        let os_options = PlatformOsOptions::default();
        let session = Session::new_with_options(
            adapter,
            cleanup,
            runtime_id,
            host_options,
            os_options,
            app,
            true,
        );

        (session, submit_count)
    }

    /// Reject undeclared permission requests before adapter submission.
    #[test]
    fn test_submit_request_rejects_undeclared_permission_before_driver() {
        let (session, submit_count) =
            test_session(Platform::Android, RuntimeAppDeclaration::default());

        // submit one undeclared permission request
        let error = session
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
    fn test_submit_request_rejects_undeclared_ios_query_scheme_before_driver() {
        let (session, submit_count) = test_session(Platform::IOS, RuntimeAppDeclaration::default());

        // submit one undeclared query-scheme request
        let error = session
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
        let (session, submit_count) = test_session(Platform::IOS, app);

        // submit one declared notification permission request
        let outcome = session
            .submit_request(HostRequest::OsNotificationRequestPermission)
            .expect("declared notification permission request should succeed");
        // ensure the adapter received the request
        assert_eq!(submit_count.load(Ordering::Relaxed), 1);
        assert_eq!(
            outcome,
            HostRequestOutcome::immediate(HostRequestResult::NotificationPermissionState(
                NotificationPermissionState::Granted,
            )),
        );
    }

    /// Allow declaration-free document picker requests to reach the adapter.
    #[test]
    fn test_submit_request_allows_declaration_free_document_pick_adapter_path() {
        let (session, submit_count) =
            test_session(Platform::MacOS, RuntimeAppDeclaration::default());

        // submit one declaration-free document picker request
        let outcome = session
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
        // ensure the adapter received the request
        assert_eq!(submit_count.load(Ordering::Relaxed), 1);
        assert_eq!(
            outcome,
            HostRequestOutcome::immediate(HostRequestResult::DocumentDescriptors(Vec::new())),
        );
    }

    /// Reject undeclared Android file sharing before adapter submission.
    #[test]
    fn test_submit_request_rejects_undeclared_android_file_share_before_driver() {
        let (session, submit_count) =
            test_session(Platform::Android, RuntimeAppDeclaration::default());

        // submit one undeclared file-share request
        let error = session
            .submit_request(HostRequest::OsIntentSharePaths {
                paths: Vec::new(),
                content_type: Some("text/plain".to_string()),
            })
            .expect_err("undeclared Android file share should fail");
        let message = error.to_string();

        // keep the adapter out of the path
        assert_eq!(submit_count.load(Ordering::Relaxed), 0);
        assert!(message.contains("intents.sharesFiles"));
    }

    /// Allow declared Android file sharing to reach the adapter.
    #[test]
    fn test_submit_request_allows_declared_android_file_share_adapter_path() {
        let app = RuntimeAppDeclaration {
            intents: RuntimeAppIntentDeclaration {
                shares_files: true,
                ..RuntimeAppIntentDeclaration::default()
            },
            ..RuntimeAppDeclaration::default()
        };
        let (session, submit_count) = test_session(Platform::Android, app);

        // submit one declared file-share request
        let outcome = session
            .submit_request(HostRequest::OsIntentSharePaths {
                paths: Vec::new(),
                content_type: Some("text/plain".to_string()),
            })
            .expect("declared Android file share should succeed");

        // ensure the adapter received the request
        assert_eq!(submit_count.load(Ordering::Relaxed), 1);
        assert_eq!(outcome.result, HostRequestResult::None);
    }
}
