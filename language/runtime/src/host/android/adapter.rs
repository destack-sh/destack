use crate::diagnostic::RuntimeResult;
use crate::host::android::bridge::registry::resolve_android_bindings;
use crate::host::android::ingress::message as android_message;
use crate::host::android::request::submit_request;
use crate::host::core::{HostRequest, HostRequestContext, HostRequestOutcome};
use crate::host::{HostAdapter, Platform};
use crate::runtime::capability::{PlatformCapability, PlatformCapabilitySet};
use crate::runtime::world::RuntimeId;

/// Android host implementation.
#[derive(Debug, Default)]
pub(crate) struct AndroidHost;

impl AndroidHost {
    /// Create one Android host.
    pub(crate) fn new() -> Self {
        Self
    }
}

impl HostAdapter for AndroidHost {
    fn platform(&self) -> Platform {
        Platform::Android
    }

    fn static_capabilities(&self) -> PlatformCapabilitySet {
        let mut host_capabilities = PlatformCapabilitySet::new();

        // android exposes runtime host intent ingress callbacks
        host_capabilities.insert_capability(PlatformCapability::OsLifecycleRead);
        host_capabilities.insert_capability(PlatformCapability::OsIntentRead);
        host_capabilities.insert_capability(PlatformCapability::OsPower);
        host_capabilities.insert_capability(PlatformCapability::OsPermissionRead);

        host_capabilities
    }

    fn process_native_ingress(&self) -> RuntimeResult<bool> {
        // service one ready slice of the Android looper
        Ok(android_message::process_ingress_ready(true))
    }

    fn runtime_session_capabilities(&self, runtime_id: RuntimeId) -> PlatformCapabilitySet {
        let Ok(bindings) = resolve_android_bindings(runtime_id.0) else {
            return PlatformCapabilitySet::new();
        };
        let callbacks = &bindings.intent;
        let has_intent_callbacks = callbacks.can_open_url.is_some()
            || callbacks.open_url.is_some()
            || callbacks.open_path.is_some()
            || callbacks.share_text.is_some()
            || callbacks.share_paths.is_some();
        let mut capabilities = PlatformCapabilitySet::new();

        if has_intent_callbacks {
            capabilities.insert_capability(PlatformCapability::OsIntentWrite);
        }

        capabilities
    }

    fn submit_request(
        &self,
        context: &HostRequestContext,
        request: HostRequest,
    ) -> RuntimeResult<HostRequestOutcome> {
        if let Some(outcome) = submit_request(context.runtime_id.0, &request)? {
            return Ok(outcome);
        }

        Err(crate::host::core::error::not_supported(
            request.operation_name(),
        ))
    }
}
