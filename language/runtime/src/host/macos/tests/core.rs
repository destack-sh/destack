use std::sync::{Mutex, OnceLock};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::core::{
    HostRequest, HostRequestContext, HostRequestOutcome, HostRequestResult, HostSessionId,
};
use crate::platform::PlatformError;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::abi_generated::{
    CalendarDescriptorValue, CalendarEventDraftValue, CalendarEventQueryValue, CalendarEventValue,
    ContactDraftValue, ContactPageValue, ContactQueryValue, ContactValue, DocumentDescriptorValue,
    DocumentPickOptionsValue, LocationSampleValue, LocationWatchOptionsValue,
};
use crate::platform::os::document::{
    validate_document_pick_options, validated_document_content_types, validated_document_extensions,
};
use crate::platform::os::{Permission, PermissionState};

/// Shared document-pick hook used by macOS tests.
pub(crate) type MacosDocumentPickHook =
    fn(DocumentPickOptionsValue) -> RuntimeResult<Vec<DocumentDescriptorValue>>;

/// Shared calendar hook set used by macOS tests.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct MacosCalendarHooks {
    /// Hook for `calendarList`.
    pub(crate) list: Option<fn() -> RuntimeResult<Vec<CalendarDescriptorValue>>>,
    /// Hook for `calendarEventList`.
    pub(crate) event_list:
        Option<fn(CalendarEventQueryValue) -> RuntimeResult<Vec<CalendarEventValue>>>,
    /// Hook for `calendarEventRead`.
    pub(crate) event_read: Option<fn(String) -> RuntimeResult<CalendarEventValue>>,
    /// Hook for `calendarEventCreate`.
    pub(crate) event_create: Option<fn(CalendarEventDraftValue) -> RuntimeResult<String>>,
    /// Hook for `calendarEventUpdate`.
    pub(crate) event_update: Option<fn(String, CalendarEventDraftValue) -> RuntimeResult<()>>,
    /// Hook for `calendarEventDelete`.
    pub(crate) event_delete: Option<fn(String) -> RuntimeResult<()>>,
}

/// Shared contact hook set used by macOS tests.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct MacosContactHooks {
    /// Hook for `contactList`.
    pub(crate) list: Option<fn(ContactQueryValue) -> RuntimeResult<ContactPageValue>>,
    /// Hook for `contactSearch`.
    pub(crate) search: Option<fn(String, ContactQueryValue) -> RuntimeResult<ContactPageValue>>,
    /// Hook for `contactRead`.
    pub(crate) read: Option<fn(String) -> RuntimeResult<ContactValue>>,
    /// Hook for `contactCreate`.
    pub(crate) create: Option<fn(ContactDraftValue) -> RuntimeResult<String>>,
    /// Hook for `contactUpdate`.
    pub(crate) update: Option<fn(String, ContactDraftValue) -> RuntimeResult<()>>,
    /// Hook for `contactDelete`.
    pub(crate) delete: Option<fn(String) -> RuntimeResult<()>>,
}

/// Shared location hook set used by macOS tests.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct MacosLocationHooks {
    /// Hook for `locationServicesEnabled`.
    pub(crate) services_enabled: Option<fn() -> RuntimeResult<bool>>,
    /// Hook for `locationLastKnown`.
    pub(crate) last_known: Option<fn() -> RuntimeResult<LocationSampleValue>>,
    /// Hook for `locationWatchOpen`.
    pub(crate) watch_open:
        Option<fn(HostSessionId, String, LocationWatchOptionsValue) -> RuntimeResult<()>>,
    /// Hook for `locationWatchClose`.
    pub(crate) watch_close: Option<fn(HostSessionId, String) -> RuntimeResult<()>>,
    /// Hook for `permission.request(location*)`.
    pub(crate) request_permission:
        Option<fn(HostSessionId, Permission) -> RuntimeResult<PermissionState>>,
}

/// Return the shared macOS document-pick hook slot for tests.
fn macos_document_pick_hook_slot() -> &'static Mutex<Option<MacosDocumentPickHook>> {
    static HOOK: OnceLock<Mutex<Option<MacosDocumentPickHook>>> = OnceLock::new();

    HOOK.get_or_init(|| Mutex::new(None))
}

/// Return the shared macOS calendar hook slot for tests.
fn macos_calendar_hook_slot() -> &'static Mutex<MacosCalendarHooks> {
    static HOOK: OnceLock<Mutex<MacosCalendarHooks>> = OnceLock::new();

    HOOK.get_or_init(|| Mutex::new(MacosCalendarHooks::default()))
}

/// Return the shared macOS contact hook slot for tests.
fn macos_contact_hook_slot() -> &'static Mutex<MacosContactHooks> {
    static HOOK: OnceLock<Mutex<MacosContactHooks>> = OnceLock::new();

    HOOK.get_or_init(|| Mutex::new(MacosContactHooks::default()))
}

/// Return the shared macOS location hook slot for tests.
fn macos_location_hook_slot() -> &'static Mutex<MacosLocationHooks> {
    static HOOK: OnceLock<Mutex<MacosLocationHooks>> = OnceLock::new();

    HOOK.get_or_init(|| Mutex::new(MacosLocationHooks::default()))
}

/// Install one macOS document-pick hook for tests.
pub(crate) fn set_macos_document_test_pick_hook(hook: Option<MacosDocumentPickHook>) {
    let mut slot = macos_document_pick_hook_slot()
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    *slot = hook;
}

/// Install one macOS calendar hook set for tests.
pub(crate) fn set_macos_calendar_test_hooks(hooks: MacosCalendarHooks) {
    let mut slot = macos_calendar_hook_slot()
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    *slot = hooks;
}

/// Install one macOS contact hook set for tests.
pub(crate) fn set_macos_contact_test_hooks(hooks: MacosContactHooks) {
    let mut slot = macos_contact_hook_slot()
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    *slot = hooks;
}

/// Install one macOS location hook set for tests.
pub(crate) fn set_macos_location_test_hooks(hooks: MacosLocationHooks) {
    let mut slot = macos_location_hook_slot()
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    *slot = hooks;
}

/// Resolve the active macOS calendar hook set for tests.
fn calendar_hooks() -> MacosCalendarHooks {
    *macos_calendar_hook_slot()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
}

/// Resolve the active macOS contact hook set for tests.
fn contact_hooks() -> MacosContactHooks {
    *macos_contact_hook_slot()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
}

/// Resolve the active macOS document-pick hook for tests.
fn require_test_pick_hook() -> RuntimeResult<MacosDocumentPickHook> {
    let hook = macos_document_pick_hook_slot()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .to_owned();

    let Some(hook) = hook else {
        return Err(RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::Generic),
            "document pick tests must install a picker hook before calling documentPick",
        ))
        .boxed());
    };

    Ok(hook)
}

/// Resolve the active macOS location hook set for tests.
fn location_hooks() -> MacosLocationHooks {
    *macos_location_hook_slot()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
}

/// Submit one macOS calendar request from the host request lane in tests.
pub(crate) fn submit_calendar_request(
    _context: &HostRequestContext,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    let hooks = calendar_hooks();

    match request {
        HostRequest::OsCalendarList => {
            let Some(hook) = hooks.list else {
                return Err(missing_calendar_test_hook("calendar list"));
            };
            let calendars = hook()?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::CalendarDescriptors(calendars),
            )))
        }
        HostRequest::OsCalendarEventList { query } => {
            let Some(hook) = hooks.event_list else {
                return Err(missing_calendar_test_hook("calendar event list"));
            };
            let events = hook(query.clone())?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::CalendarEvents(events),
            )))
        }
        HostRequest::OsCalendarEventRead { id } => {
            let Some(hook) = hooks.event_read else {
                return Err(missing_calendar_test_hook("calendar event read"));
            };
            let event = hook(id.clone())?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::CalendarEvent(event),
            )))
        }
        HostRequest::OsCalendarEventCreate { event } => {
            let Some(hook) = hooks.event_create else {
                return Err(missing_calendar_test_hook("calendar event create"));
            };
            let id = hook(event.clone())?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::String(id),
            )))
        }
        HostRequest::OsCalendarEventUpdate { id, event } => {
            let Some(hook) = hooks.event_update else {
                return Err(missing_calendar_test_hook("calendar event update"));
            };
            hook(id.clone(), event.clone())?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }
        HostRequest::OsCalendarEventDelete { id } => {
            let Some(hook) = hooks.event_delete else {
                return Err(missing_calendar_test_hook("calendar event delete"));
            };
            hook(id.clone())?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }
        _ => Ok(None),
    }
}

/// Submit one macOS contact request from the host request lane in tests.
pub(crate) fn submit_contact_request(
    _context: &HostRequestContext,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    let hooks = contact_hooks();

    match request {
        HostRequest::OsContactList { query } => {
            let Some(hook) = hooks.list else {
                return Err(missing_contact_test_hook("contact list"));
            };
            let page = hook(query.clone())?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::ContactPage(page),
            )))
        }
        HostRequest::OsContactSearch { query_text, query } => {
            let Some(hook) = hooks.search else {
                return Err(missing_contact_test_hook("contact search"));
            };
            let page = hook(query_text.clone(), query.clone())?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::ContactPage(page),
            )))
        }
        HostRequest::OsContactRead { id } => {
            let Some(hook) = hooks.read else {
                return Err(missing_contact_test_hook("contact read"));
            };
            let contact = hook(id.clone())?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::Contact(contact),
            )))
        }
        HostRequest::OsContactCreate { contact } => {
            let Some(hook) = hooks.create else {
                return Err(missing_contact_test_hook("contact create"));
            };
            let id = hook(contact.clone())?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::String(id),
            )))
        }
        HostRequest::OsContactUpdate { id, contact } => {
            let Some(hook) = hooks.update else {
                return Err(missing_contact_test_hook("contact update"));
            };
            hook(id.clone(), contact.clone())?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }
        HostRequest::OsContactDelete { id } => {
            let Some(hook) = hooks.delete else {
                return Err(missing_contact_test_hook("contact delete"));
            };
            hook(id.clone())?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }
        _ => Ok(None),
    }
}

/// Pick documents from the macOS host request lane in tests.
pub(crate) fn pick_documents(
    _context: &HostRequestContext,
    options: &DocumentPickOptionsValue,
) -> RuntimeResult<Vec<DocumentDescriptorValue>> {
    validate_document_pick_options(options)?;
    validated_document_content_types(options)?;
    validated_document_extensions(options)?;
    let hook = require_test_pick_hook()?;
    hook(options.clone())
}

/// Submit one macOS location request from the host request lane in tests.
pub(crate) fn submit_location_request(
    context: &HostRequestContext,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    let hooks = location_hooks();

    match request {
        // one services-enabled read
        HostRequest::OsLocationServicesEnabled => {
            let Some(hook) = hooks.services_enabled else {
                return Err(missing_location_test_hook("location services"));
            };
            let is_enabled = hook()?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::Bool(is_enabled),
            )))
        }

        // one last-known read
        HostRequest::OsLocationLastKnown => {
            let Some(hook) = hooks.last_known else {
                return Err(missing_location_test_hook("last known location"));
            };
            let sample = hook()?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::LocationSample(sample),
            )))
        }

        // one watch-open request
        HostRequest::OsLocationWatchOpen { watch_id, options } => {
            let Some(hook) = hooks.watch_open else {
                return Err(missing_location_test_hook("location watch open"));
            };

            hook(context.host_session_id, watch_id.clone(), *options)?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }

        // one watch-close request
        HostRequest::OsLocationWatchClose { watch_id } => {
            let Some(hook) = hooks.watch_close else {
                return Err(missing_location_test_hook("location watch close"));
            };

            hook(context.host_session_id, watch_id.clone())?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }
        _ => Ok(None),
    }
}

/// Request one supported macOS location permission selector in tests.
pub(crate) fn request_location_permission(
    context: &HostRequestContext,
    permission: Permission,
) -> RuntimeResult<Option<PermissionState>> {
    let hooks = location_hooks();
    let Some(hook) = hooks.request_permission else {
        return Ok(None);
    };

    if !matches!(
        permission,
        Permission::Location | Permission::LocationBackground
    ) {
        return Ok(None);
    }

    let state = hook(context.host_session_id, permission)?;

    Ok(Some(state))
}

/// Remove one macOS runtime from the active location test lane.
pub(crate) fn unregister_location_runtime(_host_runtime_id: HostSessionId) {}

/// Build one missing-hook error for macOS location tests.
fn missing_location_test_hook(kind: &str) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::generic(
        Some(PlatformErrorCode::Generic),
        format!("macOS location tests must install one {kind} hook before calling the host lane"),
    ))
    .boxed()
}

/// Build one missing-hook error for macOS calendar tests.
fn missing_calendar_test_hook(kind: &str) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::generic(
        Some(PlatformErrorCode::Generic),
        format!("macOS calendar tests must install one {kind} hook before calling the host lane"),
    ))
    .boxed()
}

/// Build one missing-hook error for macOS contact tests.
fn missing_contact_test_hook(kind: &str) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::generic(
        Some(PlatformErrorCode::Generic),
        format!("macOS contact tests must install one {kind} hook before calling the host lane"),
    ))
    .boxed()
}
