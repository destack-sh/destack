use crate::host::HostSessionId;
use crate::host::os::apple::abi::registry::resolve_ios_bindings;
use crate::runtime::action::{HostAction, HostActionSet};

/// Return the static iOS host actions.
pub(crate) fn static_actions() -> HostActionSet {
    let mut host_actions = HostActionSet::new();

    host_actions.insert_action(HostAction::OsLifecycleRead);
    host_actions.insert_action(HostAction::OsIntentRead);
    host_actions.insert_action(HostAction::OsPower);
    host_actions.insert_action(HostAction::OsPermissionRead);
    host_actions.insert_action(HostAction::OsNotificationPermission);

    host_actions
}

/// Return the runtime-dependent iOS host actions.
pub(crate) fn session_actions(host_session_id: HostSessionId) -> HostActionSet {
    let Ok(bindings) = resolve_ios_bindings(host_session_id.0) else {
        return HostActionSet::new();
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
        || background_callbacks.register_task.is_some()
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
        || contact_callbacks.delete_contact.is_some();

    let media_callbacks = &bindings.media;
    let has_media_read_callbacks = media_callbacks.list.is_some() || media_callbacks.read.is_some();
    let has_media_write_callbacks =
        media_callbacks.import_path.is_some() || media_callbacks.delete.is_some();

    let notification_callbacks = &bindings.notification;
    let has_notification_post_callbacks = notification_callbacks.cancel.is_some()
        || notification_callbacks.cancel_all.is_some()
        || notification_callbacks.post.is_some();

    let mut actions = HostActionSet::new();

    if has_document_pick_callbacks {
        actions.insert_action(HostAction::OsDocumentPick);
    }

    if has_permission_request_callbacks {
        actions.insert_action(HostAction::OsPermissionRequest);
    }

    if has_intent_callbacks {
        actions.insert_action(HostAction::OsIntentWrite);
    }

    if has_location_read_callbacks {
        actions.insert_action(HostAction::OsLocationRead);
    }

    if has_location_watch_callbacks {
        actions.insert_action(HostAction::OsLocationWatch);
    }

    if has_background_callbacks {
        actions.insert_action(HostAction::OsBackgroundControl);
    }

    if has_calendar_read_callbacks {
        actions.insert_action(HostAction::OsCalendarRead);
    }

    if has_calendar_write_callbacks {
        actions.insert_action(HostAction::OsCalendarWrite);
    }

    if has_contact_read_callbacks {
        actions.insert_action(HostAction::OsContactRead);
    }

    if has_contact_write_callbacks {
        actions.insert_action(HostAction::OsContactWrite);
    }

    if has_media_read_callbacks {
        actions.insert_action(HostAction::OsMediaRead);
    }

    if has_media_write_callbacks {
        actions.insert_action(HostAction::OsMediaWrite);
    }

    if has_notification_post_callbacks {
        actions.insert_action(HostAction::OsNotificationPost);
    }

    actions
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex, OnceLock};

    use crate::host::abi::background::{
        HostBackgroundCompleteRequest, HostBackgroundListResponse, HostBackgroundStatusResponse,
        HostBackgroundTaskOptions, HostBackgroundTriggerTestRequest,
        HostBackgroundTriggerTestResponse, HostBackgroundUnregisterRequest,
    };
    use crate::host::abi::calendar::{
        HostCalendarEventCreateResponse, HostCalendarEventDraft, HostCalendarListResponse,
    };
    use crate::host::abi::contact::{
        HostContactCreateResponse, HostContactDraft, HostContactPageResponse, HostContactQuery,
    };
    use crate::host::abi::document::HostDocumentRequest;
    use crate::host::abi::media::{
        HostMediaImportPathRequest, HostMediaImportPathResponse, HostMediaListRequest,
        HostMediaListResponse,
    };
    use crate::host::abi::notification::HostNotificationRequest;
    use crate::host::abi::permission::HostPermissionRequest;
    use crate::host::core::registry::HostSessionRegistrationGuard;
    use crate::host::os::apple::abi::background::callbacks::IosHostBackgroundCallbacks;
    use crate::host::os::apple::abi::bindings::IosHostBindings;
    use crate::host::os::apple::abi::calendar::callbacks::IosHostCalendarCallbacks;
    use crate::host::os::apple::abi::contact::callbacks::IosHostContactCallbacks;
    use crate::host::os::apple::abi::document::callbacks::IosHostDocumentCallbacks;
    use crate::host::os::apple::abi::intent::callbacks::IosHostIntentCallbacks;
    use crate::host::os::apple::abi::location::callbacks::IosHostLocationCallbacks;
    use crate::host::os::apple::abi::media::callbacks::IosHostMediaCallbacks;
    use crate::host::os::apple::abi::notification::callbacks::IosHostNotificationCallbacks;
    use crate::host::os::apple::abi::permission::callbacks::IosHostPermissionCallbacks;
    use crate::host::os::apple::abi::registry::{
        register_ios_bindings as register_ios_bindings_payload, unregister_ios_bindings,
    };
    use crate::host::{HOST_STATUS_OK, HostQueue, HostSessionId, HostSessionRegistry, Platform};
    use crate::platform::abi::NativeStringRef;
    use crate::runtime::action::HostAction;

    use super::{session_actions, static_actions};

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
        register_ios_bindings_payload(runtime_id, bindings)
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
        _response: *mut HostBackgroundStatusResponse,
    ) -> u32 {
        HOST_STATUS_OK
    }

    unsafe extern "C" fn test_background_register_callback(
        _runtime_id: u64,
        _options: HostBackgroundTaskOptions,
    ) -> u32 {
        HOST_STATUS_OK
    }

    unsafe extern "C" fn test_background_list_callback(
        _runtime_id: u64,
        _response: *mut HostBackgroundListResponse,
    ) -> u32 {
        HOST_STATUS_OK
    }

    unsafe extern "C" fn test_background_unregister_callback(
        _runtime_id: u64,
        _request: HostBackgroundUnregisterRequest,
    ) -> u32 {
        HOST_STATUS_OK
    }

    unsafe extern "C" fn test_background_trigger_test_callback(
        _runtime_id: u64,
        _request: HostBackgroundTriggerTestRequest,
        _response: *mut HostBackgroundTriggerTestResponse,
    ) -> u32 {
        HOST_STATUS_OK
    }

    unsafe extern "C" fn test_background_complete_callback(
        _runtime_id: u64,
        _request: HostBackgroundCompleteRequest,
    ) -> u32 {
        HOST_STATUS_OK
    }

    unsafe extern "C" fn test_calendar_list_callback(
        _runtime_id: u64,
        _response: *mut HostCalendarListResponse,
    ) -> u32 {
        HOST_STATUS_OK
    }

    unsafe extern "C" fn test_calendar_event_create_callback(
        _runtime_id: u64,
        _event: HostCalendarEventDraft,
        _response: *mut HostCalendarEventCreateResponse,
    ) -> u32 {
        HOST_STATUS_OK
    }

    unsafe extern "C" fn test_contact_list_callback(
        _runtime_id: u64,
        _query: HostContactQuery,
        _response: *mut HostContactPageResponse,
    ) -> u32 {
        HOST_STATUS_OK
    }

    unsafe extern "C" fn test_contact_create_callback(
        _runtime_id: u64,
        _draft: HostContactDraft,
        _response: *mut HostContactCreateResponse,
    ) -> u32 {
        HOST_STATUS_OK
    }

    unsafe extern "C" fn test_media_list_callback(
        _runtime_id: u64,
        _request: HostMediaListRequest,
        _response: *mut HostMediaListResponse,
    ) -> u32 {
        HOST_STATUS_OK
    }

    unsafe extern "C" fn test_media_import_path_callback(
        _runtime_id: u64,
        _request: HostMediaImportPathRequest,
        _response: *mut HostMediaImportPathResponse,
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
        _identifier: NativeStringRef,
    ) -> u32 {
        HOST_STATUS_OK
    }

    unsafe extern "C" fn test_notification_cancel_all_callback(_runtime_id: u64) -> u32 {
        HOST_STATUS_OK
    }

    /// Report the static iOS action baseline.
    #[test]
    fn test_static_actions_report_ios_host_baseline() {
        let actions = static_actions();

        assert!(actions.contains_action(HostAction::OsLifecycleRead));
        assert!(actions.contains_action(HostAction::OsIntentRead));
        assert!(actions.contains_action(HostAction::OsPower));
        assert!(actions.contains_action(HostAction::OsPermissionRead));
        assert!(actions.contains_action(HostAction::OsNotificationPermission));
    }

    /// Report no session actions without one registered iOS bindings table.
    #[test]
    fn test_session_actions_return_empty_without_ios_bindings() {
        let _lock = callback_test_lock().lock().unwrap();
        let (_queue, _registration, runtime_id) = register_ios_runtime();

        let actions = session_actions(HostSessionId(runtime_id));

        assert!(!actions.contains_action(HostAction::OsDocumentPick));
        assert!(!actions.contains_action(HostAction::OsPermissionRequest));
        assert!(!actions.contains_action(HostAction::OsIntentWrite));
        assert!(!actions.contains_action(HostAction::OsLocationRead));
        assert!(!actions.contains_action(HostAction::OsLocationWatch));
        assert!(!actions.contains_action(HostAction::OsBackgroundControl));
        assert!(!actions.contains_action(HostAction::OsCalendarRead));
        assert!(!actions.contains_action(HostAction::OsCalendarWrite));
        assert!(!actions.contains_action(HostAction::OsContactRead));
        assert!(!actions.contains_action(HostAction::OsContactWrite));
        assert!(!actions.contains_action(HostAction::OsMediaRead));
        assert!(!actions.contains_action(HostAction::OsMediaWrite));
        assert!(!actions.contains_action(HostAction::OsNotificationPost));
    }

    /// Report session actions from the registered iOS bindings table.
    #[test]
    fn test_session_actions_follow_ios_bindings_table() {
        let _lock = callback_test_lock().lock().unwrap();
        let (_queue, _registration, runtime_id) = register_ios_runtime();

        // register one representative callback for every dynamic iOS action
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
                text: Default::default(),
                background: IosHostBackgroundCallbacks {
                    status: Some(test_background_status_callback),
                    list: Some(test_background_list_callback),
                    register_task: Some(test_background_register_callback),
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

        // derive one runtime-scoped action set from that table
        let actions = session_actions(HostSessionId(runtime_id));

        assert!(actions.contains_action(HostAction::OsDocumentPick));
        assert!(actions.contains_action(HostAction::OsPermissionRequest));
        assert!(actions.contains_action(HostAction::OsIntentWrite));
        assert!(actions.contains_action(HostAction::OsLocationRead));
        assert!(actions.contains_action(HostAction::OsLocationWatch));
        assert!(actions.contains_action(HostAction::OsBackgroundControl));
        assert!(actions.contains_action(HostAction::OsCalendarRead));
        assert!(actions.contains_action(HostAction::OsCalendarWrite));
        assert!(actions.contains_action(HostAction::OsContactRead));
        assert!(actions.contains_action(HostAction::OsContactWrite));
        assert!(actions.contains_action(HostAction::OsMediaRead));
        assert!(actions.contains_action(HostAction::OsMediaWrite));
        assert!(actions.contains_action(HostAction::OsNotificationPost));
    }
}
