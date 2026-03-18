use crate::diagnostic::RuntimeResult;
use crate::host::apple::message as apple_message;
use crate::host::core::error::not_supported;
use crate::host::core::{HostRequest, HostRequestContext, HostRequestOutcome, HostRuntimeId};
use crate::host::ios::bridge::registry::resolve_ios_bindings;
use crate::host::ios::request::submit_request;
use crate::host::{HostAdapter, Platform};
use crate::runtime::capability::{PlatformCapability, PlatformCapabilitySet};

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
        host_capabilities.insert_capability(PlatformCapability::OsBackgroundRead);
        host_capabilities.insert_capability(PlatformCapability::OsPower);
        host_capabilities.insert_capability(PlatformCapability::OsPermissionRead);
        host_capabilities.insert_capability(PlatformCapability::OsNotificationPermission);

        host_capabilities
    }

    fn is_process_main_context(&self) -> bool {
        apple_message::is_process_main_context()
    }

    fn process_native_ingress(&self) -> RuntimeResult<()> {
        // service one ready slice of the Apple run loop
        apple_message::process_ingress_ready(true);

        Ok(())
    }

    fn session_capabilities(&self, host_runtime_id: HostRuntimeId) -> PlatformCapabilitySet {
        let Ok(bindings) = resolve_ios_bindings(host_runtime_id.0) else {
            return PlatformCapabilitySet::new();
        };
        let callbacks = &bindings.intent;
        let has_intent_callbacks = callbacks.can_open_url.is_some()
            || callbacks.open_url.is_some()
            || callbacks.open_path.is_some()
            || callbacks.share_text.is_some()
            || callbacks.share_paths.is_some();
        let location_callbacks = &bindings.location;
        let has_location_read_callbacks = location_callbacks.services_enabled.is_some()
            || location_callbacks.last_known.is_some();
        let has_location_watch_callbacks =
            location_callbacks.watch_open.is_some() || location_callbacks.watch_close.is_some();
        let background_callbacks = &bindings.background;
        let has_background_callbacks = background_callbacks.status.is_some()
            || background_callbacks.list.is_some()
            || background_callbacks.register.is_some()
            || background_callbacks.unregister.is_some()
            || background_callbacks.trigger_test.is_some()
            || background_callbacks.complete.is_some();
        let calendar_callbacks = &bindings.calendar;
        let has_calendar_read_callbacks = calendar_callbacks.list.is_some()
            || calendar_callbacks.event_list.is_some()
            || calendar_callbacks.event_read.is_some();
        let has_calendar_write_callbacks = calendar_callbacks.event_create.is_some()
            || calendar_callbacks.event_update.is_some()
            || calendar_callbacks.event_delete.is_some();
        let contact_callbacks = &bindings.contact;
        let has_contact_read_callbacks = contact_callbacks.list.is_some()
            || contact_callbacks.search.is_some()
            || contact_callbacks.read.is_some();
        let has_contact_write_callbacks = contact_callbacks.create.is_some()
            || contact_callbacks.update.is_some()
            || contact_callbacks.delete.is_some();
        let media_callbacks = &bindings.media;
        let has_media_read_callbacks =
            media_callbacks.list.is_some() || media_callbacks.describe.is_some();
        let has_media_write_callbacks =
            media_callbacks.import_path.is_some() || media_callbacks.delete.is_some();
        let notification_callbacks = &bindings.notification;
        let has_notification_post_callbacks = notification_callbacks.cancel.is_some()
            || notification_callbacks.cancel_all.is_some()
            || notification_callbacks.category_list.is_some()
            || notification_callbacks.category_set.is_some()
            || notification_callbacks.pending_list.is_some()
            || notification_callbacks.pending_cancel.is_some()
            || notification_callbacks.pending_cancel_all.is_some()
            || notification_callbacks.post.is_some()
            || notification_callbacks.schedule.is_some();
        let mut capabilities = PlatformCapabilitySet::new();

        if has_intent_callbacks {
            capabilities.insert_capability(PlatformCapability::OsIntentWrite);
        }

        if has_location_read_callbacks {
            capabilities.insert_capability(PlatformCapability::OsLocationRead);
        }

        if has_location_watch_callbacks {
            capabilities.insert_capability(PlatformCapability::OsLocationWatch);
        }

        if has_background_callbacks {
            capabilities.insert_capability(PlatformCapability::OsBackgroundControl);
        }

        if has_calendar_read_callbacks {
            capabilities.insert_capability(PlatformCapability::OsCalendarRead);
        }

        if has_calendar_write_callbacks {
            capabilities.insert_capability(PlatformCapability::OsCalendarWrite);
        }

        if has_contact_read_callbacks {
            capabilities.insert_capability(PlatformCapability::OsContactRead);
        }

        if has_contact_write_callbacks {
            capabilities.insert_capability(PlatformCapability::OsContactWrite);
        }

        if has_media_read_callbacks {
            capabilities.insert_capability(PlatformCapability::OsMediaRead);
        }

        if has_media_write_callbacks {
            capabilities.insert_capability(PlatformCapability::OsMediaWrite);
        }

        if notification_callbacks.request_permission.is_some() {
            capabilities.insert_capability(PlatformCapability::OsNotificationPermission);
        }

        if has_notification_post_callbacks {
            capabilities.insert_capability(PlatformCapability::OsNotificationPost);
        }

        capabilities
    }

    fn submit_request(
        &self,
        context: &HostRequestContext,
        request: HostRequest,
    ) -> RuntimeResult<HostRequestOutcome> {
        if let Some(outcome) = submit_request(context.host_runtime_id.0, &request)? {
            return Ok(outcome);
        }

        Err(not_supported(request.operation_name()))
    }
}
