use crate::diagnostic::RuntimeResult;
use crate::host::apple::message as apple_message;
use crate::host::core::{HostRequest, HostRequestContext, HostRequestOutcome};
use crate::host::ios::bridge::registry::resolve_ios_bindings;
use crate::host::ios::request::submit_request;
use crate::host::{HostAdapter, Platform};
use crate::runtime::capability::{PlatformCapability, PlatformCapabilitySet};
use crate::runtime::world::RuntimeId;

/// iOS host implementation.
#[derive(Debug, Default)]
pub(crate) struct IosHost;

impl IosHost {
    /// Create one iOS host.
    pub(crate) fn new() -> Self {
        Self
    }
}

impl HostAdapter for IosHost {
    fn platform(&self) -> Platform {
        Platform::IOS
    }

    fn static_capabilities(&self) -> PlatformCapabilitySet {
        let mut host_capabilities = PlatformCapabilitySet::new();

        // ios exposes runtime host intent ingress callbacks
        host_capabilities.insert_capability(PlatformCapability::OsLifecycleRead);
        host_capabilities.insert_capability(PlatformCapability::OsIntentRead);
        host_capabilities.insert_capability(PlatformCapability::OsPower);
        host_capabilities.insert_capability(PlatformCapability::OsPermissionRead);

        host_capabilities
    }

    fn is_process_main_context(&self) -> bool {
        apple_message::is_process_main_context()
    }

    fn process_native_ingress(&self) -> RuntimeResult<bool> {
        // service one ready slice of the Apple run loop
        Ok(apple_message::process_ingress_ready(true))
    }

    fn session_capabilities(&self, runtime_id: RuntimeId) -> PlatformCapabilitySet {
        let Ok(bindings) = resolve_ios_bindings(runtime_id.0) else {
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
