use crate::host::core::HostSessionId;
use crate::host::ios::abi::registry::resolve_ios_bindings;
use crate::runtime::capability::{PlatformCapability, PlatformCapabilitySet};

/// Return the static iOS host capabilities.
pub(crate) fn static_capabilities() -> PlatformCapabilitySet {
    let mut host_capabilities = PlatformCapabilitySet::new();

    host_capabilities.insert_capability(PlatformCapability::OsLifecycleRead);
    host_capabilities.insert_capability(PlatformCapability::OsIntentRead);
    host_capabilities.insert_capability(PlatformCapability::OsPower);
    host_capabilities.insert_capability(PlatformCapability::OsPermissionRead);
    host_capabilities.insert_capability(PlatformCapability::OsNotificationPermission);

    host_capabilities
}

/// Return the runtime-dependent iOS host capabilities.
pub(crate) fn session_capabilities(host_session_id: HostSessionId) -> PlatformCapabilitySet {
    let Ok(bindings) = resolve_ios_bindings(host_session_id.0) else {
        return PlatformCapabilitySet::new();
    };

    let document_callbacks = &bindings.document;
    let has_document_pick_callbacks = document_callbacks.pick.is_some();

    let permission_callbacks = &bindings.permission;
    let has_permission_request_callbacks = permission_callbacks.request.is_some();

    let callbacks = &bindings.intent;
    let has_intent_callbacks = callbacks.can_open_url.is_some()
        || callbacks.open_url.is_some()
        || callbacks.open_path.is_some()
        || callbacks.share_text.is_some()
        || callbacks.share_paths.is_some();

    let location_callbacks = &bindings.location;
    let has_location_read_callbacks =
        location_callbacks.services_enabled.is_some() || location_callbacks.last_known.is_some();
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
        || notification_callbacks.post.is_some();

    let mut capabilities = PlatformCapabilitySet::new();

    if has_document_pick_callbacks {
        capabilities.insert_capability(PlatformCapability::OsDocumentPick);
    }

    if has_permission_request_callbacks {
        capabilities.insert_capability(PlatformCapability::OsPermissionRequest);
    }

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

    if has_notification_post_callbacks {
        capabilities.insert_capability(PlatformCapability::OsNotificationPost);
    }

    capabilities
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex, OnceLock};

    use crate::host::abi::calendar::{
        HostCalendarDescriptorArray, HostCalendarEventDraft, HostCalendarEventId,
    };
    use crate::host::abi::document::HostDocumentRequest;
    use crate::host::abi::media::{HostMediaPage, HostMediaQuery};
    use crate::host::abi::notification::HostNotificationRequest;
    use crate::host::abi::permission::HostPermissionRequest;
    use crate::host::core::registry::HostSessionRegistrationGuard;
    use crate::host::core::{HostQueue, HostSessionId, HostSessionRegistry};
    use crate::host::ios::abi::background::callbacks::IosHostBackgroundCallbacks;
    use crate::host::ios::abi::bindings::{IosHostBindings, destack_host_ios_register_bindings};
    use crate::host::ios::abi::calendar::callbacks::IosHostCalendarCallbacks;
    use crate::host::ios::abi::contact::callbacks::IosHostContactCallbacks;
    use crate::host::ios::abi::document::callbacks::IosHostDocumentCallbacks;
    use crate::host::ios::abi::intent::callbacks::IosHostIntentCallbacks;
    use crate::host::ios::abi::location::callbacks::IosHostLocationCallbacks;
    use crate::host::ios::abi::media::callbacks::IosHostMediaCallbacks;
    use crate::host::ios::abi::notification::callbacks::IosHostNotificationCallbacks;
    use crate::host::ios::abi::permission::callbacks::IosHostPermissionCallbacks;
    use crate::host::ios::abi::registry::unregister_ios_bindings;
    use crate::host::{HOST_STATUS_OK, Platform};
    use crate::platform::NativeArray;
    use crate::platform::os::abi_generated::{
        BackgroundStatus, BackgroundTaskDescriptor, BackgroundTaskOptions, BackgroundTaskResult,
        ContactDraft, ContactPage,
    };
    use crate::runtime::capability::PlatformCapability;
    use crate::runtime::{NativeSlice, NativeStringRef};

    use super::{session_capabilities, static_capabilities};

    /// Return the shared test lock for iOS bindings registration.
    fn callback_test_lock() -> &'static Mutex<()> {
        static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

        TEST_LOCK.get_or_init(|| Mutex::new(()))
    }

    /// Register one temporary iOS host queue and keep registration state alive.
    fn register_ios_runtime() -> (Arc<HostQueue>, HostSessionRegistrationGuard, u64) {
        let runtime_id = HostSessionRegistry::allocate_session_id();
        let queue = Arc::new(HostQueue::new(runtime_id));
        let registration = HostSessionRegistry::register_queue(
            Platform::IOS,
            runtime_id,
            Arc::clone(&queue),
            Some(unregister_ios_bindings),
        );
        let runtime_id = registration.host_session_id().0;

        (queue, registration, runtime_id)
    }

    /// Register one runtime-scoped iOS host bindings payload.
    fn register_ios_bindings(runtime_id: u64, bindings: IosHostBindings) -> u32 {
        unsafe { destack_host_ios_register_bindings(runtime_id, bindings) }
    }

    unsafe extern "C" fn test_document_pick_callback(
        _runtime_id: u64,
        _request: HostDocumentRequest,
    ) -> u32 {
        HOST_STATUS_OK
    }

    unsafe extern "C" fn test_permission_open_settings_callback(_runtime_id: u64) -> u32 {
        HOST_STATUS_OK
    }

    unsafe extern "C" fn test_permission_request_callback(
        _runtime_id: u64,
        _request: HostPermissionRequest,
    ) -> u32 {
        HOST_STATUS_OK
    }

    unsafe extern "C" fn test_intent_open_url_callback(
        _runtime_id: u64,
        _url: NativeStringRef,
    ) -> u32 {
        HOST_STATUS_OK
    }

    unsafe extern "C" fn test_location_services_enabled_callback(
        _runtime_id: u64,
        _is_enabled: *mut bool,
    ) -> u32 {
        HOST_STATUS_OK
    }

    unsafe extern "C" fn test_location_watch_open_callback(
        _runtime_id: u64,
        _watch_id: NativeStringRef,
        _options: crate::platform::os::LocationWatchOptions,
    ) -> u32 {
        HOST_STATUS_OK
    }

    unsafe extern "C" fn test_background_status_callback(
        _runtime_id: u64,
        _status: *mut BackgroundStatus,
    ) -> u32 {
        HOST_STATUS_OK
    }

    unsafe extern "C" fn test_background_register_callback(
        _runtime_id: u64,
        _options: BackgroundTaskOptions,
    ) -> u32 {
        HOST_STATUS_OK
    }

    unsafe extern "C" fn test_background_list_callback(
        _runtime_id: u64,
        _output_descriptors: *mut NativeArray<BackgroundTaskDescriptor>,
    ) -> u32 {
        HOST_STATUS_OK
    }

    unsafe extern "C" fn test_background_unregister_callback(
        _runtime_id: u64,
        _identifier: NativeStringRef,
    ) -> u32 {
        HOST_STATUS_OK
    }

    unsafe extern "C" fn test_background_trigger_test_callback(
        _runtime_id: u64,
        _identifier: NativeStringRef,
        _is_triggered: *mut bool,
    ) -> u32 {
        HOST_STATUS_OK
    }

    unsafe extern "C" fn test_background_complete_callback(
        _runtime_id: u64,
        _execution_id: NativeStringRef,
        _result: BackgroundTaskResult,
    ) -> u32 {
        HOST_STATUS_OK
    }

    unsafe extern "C" fn test_calendar_list_callback(
        _runtime_id: u64,
        _output_calendars: *mut HostCalendarDescriptorArray,
    ) -> u32 {
        HOST_STATUS_OK
    }

    unsafe extern "C" fn test_calendar_event_create_callback(
        _runtime_id: u64,
        _event: HostCalendarEventDraft,
        _output_id: *mut HostCalendarEventId,
    ) -> u32 {
        HOST_STATUS_OK
    }

    unsafe extern "C" fn test_contact_list_callback(
        _runtime_id: u64,
        _query: crate::host::abi::contact::HostContactQuery,
        _output_page: *mut ContactPage,
    ) -> u32 {
        HOST_STATUS_OK
    }

    unsafe extern "C" fn test_contact_create_callback(
        _runtime_id: u64,
        _draft: ContactDraft,
        _output_id: *mut NativeStringRef,
    ) -> u32 {
        HOST_STATUS_OK
    }

    unsafe extern "C" fn test_media_list_callback(
        _runtime_id: u64,
        _query: HostMediaQuery,
        _output_page: *mut HostMediaPage,
    ) -> u32 {
        HOST_STATUS_OK
    }

    unsafe extern "C" fn test_media_import_path_callback(
        _runtime_id: u64,
        _path: NativeStringRef,
        _kind: i32,
        _output_id: *mut NativeStringRef,
    ) -> u32 {
        HOST_STATUS_OK
    }

    unsafe extern "C" fn test_notification_post_callback(
        _runtime_id: u64,
        _request: HostNotificationRequest,
    ) -> u32 {
        HOST_STATUS_OK
    }

    unsafe extern "C" fn test_notification_cancel_callback(
        _runtime_id: u64,
        _id: NativeSlice<u8>,
    ) -> u32 {
        HOST_STATUS_OK
    }

    unsafe extern "C" fn test_notification_cancel_all_callback(_runtime_id: u64) -> u32 {
        HOST_STATUS_OK
    }

    /// Report the static iOS capability baseline.
    #[test]
    fn test_static_capabilities_report_ios_host_baseline() {
        let capabilities = static_capabilities();

        assert!(capabilities.contains_capability(PlatformCapability::OsLifecycleRead));
        assert!(capabilities.contains_capability(PlatformCapability::OsIntentRead));
        assert!(capabilities.contains_capability(PlatformCapability::OsPower));
        assert!(capabilities.contains_capability(PlatformCapability::OsPermissionRead));
        assert!(capabilities.contains_capability(PlatformCapability::OsNotificationPermission));
    }

    /// Report no session capabilities without one registered iOS bindings table.
    #[test]
    fn test_session_capabilities_return_empty_without_ios_bindings() {
        let _lock = callback_test_lock().lock().unwrap();
        let (_queue, _registration, runtime_id) = register_ios_runtime();

        let capabilities = session_capabilities(HostSessionId(runtime_id));

        assert!(!capabilities.contains_capability(PlatformCapability::OsDocumentPick));
        assert!(!capabilities.contains_capability(PlatformCapability::OsPermissionRequest));
        assert!(!capabilities.contains_capability(PlatformCapability::OsIntentWrite));
        assert!(!capabilities.contains_capability(PlatformCapability::OsLocationRead));
        assert!(!capabilities.contains_capability(PlatformCapability::OsLocationWatch));
        assert!(!capabilities.contains_capability(PlatformCapability::OsBackgroundControl));
        assert!(!capabilities.contains_capability(PlatformCapability::OsCalendarRead));
        assert!(!capabilities.contains_capability(PlatformCapability::OsCalendarWrite));
        assert!(!capabilities.contains_capability(PlatformCapability::OsContactRead));
        assert!(!capabilities.contains_capability(PlatformCapability::OsContactWrite));
        assert!(!capabilities.contains_capability(PlatformCapability::OsMediaRead));
        assert!(!capabilities.contains_capability(PlatformCapability::OsMediaWrite));
        assert!(!capabilities.contains_capability(PlatformCapability::OsNotificationPost));
    }

    /// Report session capabilities from the registered iOS bindings table.
    #[test]
    fn test_session_capabilities_follow_ios_bindings_table() {
        let _lock = callback_test_lock().lock().unwrap();
        let (_queue, _registration, runtime_id) = register_ios_runtime();

        // register one representative callback for every dynamic iOS lane
        let status = register_ios_bindings(
            runtime_id,
            IosHostBindings {
                document: IosHostDocumentCallbacks {
                    pick: Some(test_document_pick_callback),
                },
                permission: IosHostPermissionCallbacks {
                    open_settings: Some(test_permission_open_settings_callback),
                    request: Some(test_permission_request_callback),
                },
                background: IosHostBackgroundCallbacks {
                    status: Some(test_background_status_callback),
                    list: Some(test_background_list_callback),
                    register: Some(test_background_register_callback),
                    unregister: Some(test_background_unregister_callback),
                    trigger_test: Some(test_background_trigger_test_callback),
                    complete: Some(test_background_complete_callback),
                },
                calendar: IosHostCalendarCallbacks {
                    list: Some(test_calendar_list_callback),
                    event_create: Some(test_calendar_event_create_callback),
                    ..IosHostCalendarCallbacks::default()
                },
                contact: IosHostContactCallbacks {
                    list: Some(test_contact_list_callback),
                    create: Some(test_contact_create_callback),
                    ..IosHostContactCallbacks::default()
                },
                intent: IosHostIntentCallbacks {
                    open_url: Some(test_intent_open_url_callback),
                    ..IosHostIntentCallbacks::default()
                },
                location: IosHostLocationCallbacks {
                    services_enabled: Some(test_location_services_enabled_callback),
                    watch_open: Some(test_location_watch_open_callback),
                    ..IosHostLocationCallbacks::default()
                },
                media: IosHostMediaCallbacks {
                    list: Some(test_media_list_callback),
                    import_path: Some(test_media_import_path_callback),
                    ..IosHostMediaCallbacks::default()
                },
                notification: IosHostNotificationCallbacks {
                    cancel: Some(test_notification_cancel_callback),
                    cancel_all: Some(test_notification_cancel_all_callback),
                    post: Some(test_notification_post_callback),
                },
            },
        );
        assert_eq!(status, HOST_STATUS_OK);

        // derive one runtime-scoped capability set from that table
        let capabilities = session_capabilities(HostSessionId(runtime_id));

        assert!(capabilities.contains_capability(PlatformCapability::OsDocumentPick));
        assert!(capabilities.contains_capability(PlatformCapability::OsPermissionRequest));
        assert!(capabilities.contains_capability(PlatformCapability::OsIntentWrite));
        assert!(capabilities.contains_capability(PlatformCapability::OsLocationRead));
        assert!(capabilities.contains_capability(PlatformCapability::OsLocationWatch));
        assert!(capabilities.contains_capability(PlatformCapability::OsBackgroundControl));
        assert!(capabilities.contains_capability(PlatformCapability::OsCalendarRead));
        assert!(capabilities.contains_capability(PlatformCapability::OsCalendarWrite));
        assert!(capabilities.contains_capability(PlatformCapability::OsContactRead));
        assert!(capabilities.contains_capability(PlatformCapability::OsContactWrite));
        assert!(capabilities.contains_capability(PlatformCapability::OsMediaRead));
        assert!(capabilities.contains_capability(PlatformCapability::OsMediaWrite));
        assert!(capabilities.contains_capability(PlatformCapability::OsNotificationPost));
    }
}
