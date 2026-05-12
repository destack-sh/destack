use crate::host::HostSessionId;
use crate::runtime::action::{Action, ActionSet};

/// Return the static iOS host actions.
pub(crate) fn static_actions() -> ActionSet {
    let mut host_actions = ActionSet::new();

    host_actions.insert_action(Action::OsPowerRead);

    host_actions
}

/// Return the runtime-dependent iOS host actions.
pub(crate) fn session_actions(host_session_id: HostSessionId) -> ActionSet {
    let _ = host_session_id;

    ActionSet::new()
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
    use crate::host::os::apple::abi::binding::IosHostBindings;
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
    use crate::runtime::action::Action;

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

        assert!(actions.contains_action(Action::OsPowerRead));
    }

    /// Report no session actions without one registered iOS bindings table.
    #[test]
    fn test_session_actions_return_empty_without_ios_bindings() {
        let _lock = callback_test_lock().lock().unwrap();
        let (_queue, _registration, runtime_id) = register_ios_runtime();

        assert!(session_actions(HostSessionId(runtime_id)).is_empty());
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

        assert!(session_actions(HostSessionId(runtime_id)).is_empty());
    }
}
