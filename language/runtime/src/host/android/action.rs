use crate::host::HostSessionId;
use crate::runtime::action::{Action, ActionSet};

/// Return the static Android host actions.
pub(crate) fn static_actions() -> ActionSet {
    let mut host_actions = ActionSet::new();

    host_actions.insert_action(Action::OsPowerRead);

    host_actions
}

/// Return the runtime-dependent Android host actions.
pub(crate) fn session_actions(host_session_id: HostSessionId) -> ActionSet {
    let _ = host_session_id;

    ActionSet::new()
}

#[cfg(test)]
mod tests {
    use crate::host::os::android::abi::background::callbacks::AndroidHostBackgroundCallbacks;
    use crate::host::os::android::abi::background::types::{
        HostBackgroundCompleteRequest, HostBackgroundListResponse, HostBackgroundStatusResponse,
        HostBackgroundTaskOptions, HostBackgroundTriggerTestRequest,
        HostBackgroundTriggerTestResponse, HostBackgroundUnregisterRequest,
    };
    use crate::host::os::android::abi::binding::AndroidHostBindings;
    use crate::host::os::android::abi::calendar::callbacks::AndroidHostCalendarCallbacks;
    use crate::host::os::android::abi::calendar::types::{
        HostCalendarEventCreateResponse, HostCalendarEventDraft, HostCalendarListResponse,
    };
    use crate::host::os::android::abi::contact::callbacks::AndroidHostContactCallbacks;
    use crate::host::os::android::abi::contact::types::{
        HostContactCreateResponse, HostContactDraft, HostContactPageResponse, HostContactQuery,
    };
    use crate::host::os::android::abi::document::callbacks::AndroidHostDocumentCallbacks;
    use crate::host::os::android::abi::document::types::HostDocumentRequest;
    use crate::host::os::android::abi::intent::callbacks::AndroidHostIntentCallbacks;
    use crate::host::os::android::abi::location::callbacks::AndroidHostLocationCallbacks;
    use crate::host::os::android::abi::location::types::{
        LocationServicesResponse, LocationWatchOptions,
    };
    use crate::host::os::android::abi::media::callbacks::AndroidHostMediaCallbacks;
    use crate::host::os::android::abi::media::types::{
        HostMediaImportPathRequest, HostMediaImportPathResponse, HostMediaListRequest,
        HostMediaListResponse,
    };
    use crate::host::os::android::abi::notification::callbacks::AndroidHostNotificationCallbacks;
    use crate::host::os::android::abi::notification::types::HostNotificationRequest;
    use crate::host::os::android::abi::permission::callbacks::AndroidHostPermissionCallbacks;
    use crate::host::os::android::abi::permission::types::HostPermissionRequest;
    use crate::host::os::android::tests::{
        callback_test_lock, register_android_bindings, register_android_runtime,
    };
    use crate::host::{HOST_STATUS_OK, HostSessionId};
    use crate::platform::abi::NativeStringRef;
    use crate::runtime::action::Action;

    use super::{session_actions, static_actions};

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
        _response: *mut LocationServicesResponse,
    ) -> u32 {
        HOST_STATUS_OK
    }

    unsafe extern "C" fn test_location_watch_open_callback(
        _runtime_id: u64,
        _watch_id: NativeStringRef,
        _options: LocationWatchOptions,
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

    /// Report the static Android action baseline.
    #[test]
    fn test_static_actions_report_android_host_baseline() {
        let actions = static_actions();

        assert!(actions.contains_action(Action::OsPowerRead));
    }

    /// Report no session actions without one registered Android bindings table.
    #[test]
    fn test_session_actions_return_empty_without_android_bindings() {
        let _lock = callback_test_lock().lock().unwrap();
        let (_queue, _registration, runtime_id) = register_android_runtime();

        assert!(session_actions(HostSessionId(runtime_id)).is_empty());
    }

    /// Report session actions from the registered Android bindings table.
    #[test]
    fn test_session_actions_follow_android_bindings_table() {
        let _lock = callback_test_lock().lock().unwrap();
        let (_queue, _registration, runtime_id) = register_android_runtime();

        // register one representative callback for every dynamic Android action
        let status = register_android_bindings(
            runtime_id,
            AndroidHostBindings {
                document: AndroidHostDocumentCallbacks {
                    pick: Some(test_document_pick_callback),
                },
                permission: AndroidHostPermissionCallbacks {
                    open_settings: Some(test_permission_open_settings_callback),
                    request: Some(test_permission_request_callback),
                },
                background: AndroidHostBackgroundCallbacks {
                    status: Some(test_background_status_callback),
                    list: Some(test_background_list_callback),
                    register_task: Some(test_background_register_callback),
                    unregister: Some(test_background_unregister_callback),
                    trigger_test: Some(test_background_trigger_test_callback),
                    complete: Some(test_background_complete_callback),
                },
                calendar: AndroidHostCalendarCallbacks {
                    list: Some(test_calendar_list_callback),
                    event_create: Some(test_calendar_event_create_callback),
                    ..AndroidHostCalendarCallbacks::default()
                },
                contact: AndroidHostContactCallbacks {
                    list: Some(test_contact_list_callback),
                    create: Some(test_contact_create_callback),
                    ..AndroidHostContactCallbacks::default()
                },
                intent: AndroidHostIntentCallbacks {
                    open_url: Some(test_intent_open_url_callback),
                    ..AndroidHostIntentCallbacks::default()
                },
                location: AndroidHostLocationCallbacks {
                    services_enabled: Some(test_location_services_enabled_callback),
                    watch_open: Some(test_location_watch_open_callback),
                    ..AndroidHostLocationCallbacks::default()
                },
                media: AndroidHostMediaCallbacks {
                    list: Some(test_media_list_callback),
                    import_path: Some(test_media_import_path_callback),
                    ..AndroidHostMediaCallbacks::default()
                },
                notification: AndroidHostNotificationCallbacks {
                    cancel: Some(test_notification_cancel_callback),
                    cancel_all: Some(test_notification_cancel_all_callback),
                    post: Some(test_notification_post_callback),
                },
                ..AndroidHostBindings::default()
            },
        );
        assert_eq!(status, HOST_STATUS_OK);

        assert!(session_actions(HostSessionId(runtime_id)).is_empty());
    }
}
