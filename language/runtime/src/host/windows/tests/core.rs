use std::sync::{Mutex, OnceLock};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::app::document::pick::{
    HOST_DOCUMENT_PICK_OPERATION, normalized_document_extensions, validate_document_pick_options,
};
use crate::host::core::{
    HostRequest, HostRequestContext, HostRequestOutcome, HostRequestResult, HostRuntimeId,
};
use crate::platform::PlatformError;
use crate::platform::core::not_supported;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::abi_generated::{
    CalendarDescriptorValue, CalendarEventDraftValue, CalendarEventQueryValue, CalendarEventValue,
    ContactDraftValue, ContactPageValue, ContactQueryValue, ContactValue, DocumentDescriptorValue,
    DocumentPickOptionsValue, LocationSampleValue, LocationWatchOptionsValue,
};
use crate::platform::os::{Permission, PermissionState};

/// Shared document-pick hook used by Windows tests.
pub(crate) type WindowsDocumentPickHook =
    fn(DocumentPickOptionsValue) -> RuntimeResult<Vec<DocumentDescriptorValue>>;

/// Shared calendar-list hook used by Windows tests.
pub(crate) type WindowsCalendarListHook = fn() -> RuntimeResult<Vec<CalendarDescriptorValue>>;

/// Shared calendar event-list hook used by Windows tests.
pub(crate) type WindowsCalendarEventListHook =
    fn(CalendarEventQueryValue) -> RuntimeResult<Vec<CalendarEventValue>>;

/// Shared calendar event-read hook used by Windows tests.
pub(crate) type WindowsCalendarEventReadHook = fn(String) -> RuntimeResult<CalendarEventValue>;

/// Shared calendar event-create hook used by Windows tests.
pub(crate) type WindowsCalendarEventCreateHook =
    fn(CalendarEventDraftValue) -> RuntimeResult<String>;

/// Shared calendar event-update hook used by Windows tests.
pub(crate) type WindowsCalendarEventUpdateHook =
    fn(String, CalendarEventDraftValue) -> RuntimeResult<()>;

/// Shared calendar event-delete hook used by Windows tests.
pub(crate) type WindowsCalendarEventDeleteHook = fn(String) -> RuntimeResult<()>;

/// Shared contact-list hook used by Windows tests.
pub(crate) type WindowsContactListHook = fn(ContactQueryValue) -> RuntimeResult<ContactPageValue>;

/// Shared contact-search hook used by Windows tests.
pub(crate) type WindowsContactSearchHook =
    fn(String, ContactQueryValue) -> RuntimeResult<ContactPageValue>;

/// Shared contact-read hook used by Windows tests.
pub(crate) type WindowsContactReadHook = fn(String) -> RuntimeResult<ContactValue>;

/// Shared contact-create hook used by Windows tests.
pub(crate) type WindowsContactCreateHook = fn(ContactDraftValue) -> RuntimeResult<String>;

/// Shared contact-update hook used by Windows tests.
pub(crate) type WindowsContactUpdateHook = fn(String, ContactDraftValue) -> RuntimeResult<()>;

/// Shared contact-delete hook used by Windows tests.
pub(crate) type WindowsContactDeleteHook = fn(String) -> RuntimeResult<()>;

/// Installed Windows calendar hooks for tests.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct WindowsCalendarHooks {
    /// Hook for `calendarList`.
    pub(crate) list: Option<WindowsCalendarListHook>,
    /// Hook for `calendarEventList`.
    pub(crate) event_list: Option<WindowsCalendarEventListHook>,
    /// Hook for `calendarEventRead`.
    pub(crate) event_read: Option<WindowsCalendarEventReadHook>,
    /// Hook for `calendarEventCreate`.
    pub(crate) event_create: Option<WindowsCalendarEventCreateHook>,
    /// Hook for `calendarEventUpdate`.
    pub(crate) event_update: Option<WindowsCalendarEventUpdateHook>,
    /// Hook for `calendarEventDelete`.
    pub(crate) event_delete: Option<WindowsCalendarEventDeleteHook>,
}

/// Installed Windows contact hooks for tests.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct WindowsContactHooks {
    /// Hook for `contactList`.
    pub(crate) list: Option<WindowsContactListHook>,
    /// Hook for `contactSearch`.
    pub(crate) search: Option<WindowsContactSearchHook>,
    /// Hook for `contactRead`.
    pub(crate) read: Option<WindowsContactReadHook>,
    /// Hook for `contactCreate`.
    pub(crate) create: Option<WindowsContactCreateHook>,
    /// Hook for `contactUpdate`.
    pub(crate) update: Option<WindowsContactUpdateHook>,
    /// Hook for `contactDelete`.
    pub(crate) delete: Option<WindowsContactDeleteHook>,
}

/// Shared location-services hook used by Windows tests.
pub(crate) type WindowsLocationServicesEnabledHook = fn() -> RuntimeResult<bool>;

/// Shared last-known location hook used by Windows tests.
pub(crate) type WindowsLocationLastKnownHook = fn() -> RuntimeResult<LocationSampleValue>;

/// Shared location watch-open hook used by Windows tests.
pub(crate) type WindowsLocationWatchOpenHook =
    fn(HostRuntimeId, String, LocationWatchOptionsValue) -> RuntimeResult<()>;

/// Shared location watch-close hook used by Windows tests.
pub(crate) type WindowsLocationWatchCloseHook = fn(HostRuntimeId, String) -> RuntimeResult<()>;

/// Shared location permission hook used by Windows tests.
pub(crate) type WindowsLocationPermissionHook =
    fn(HostRuntimeId, Permission) -> RuntimeResult<PermissionState>;

/// Installed Windows location hooks for tests.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct WindowsLocationHooks {
    /// Hook for `locationServicesEnabled`.
    pub(crate) services_enabled: Option<WindowsLocationServicesEnabledHook>,
    /// Hook for `locationLastKnown`.
    pub(crate) last_known: Option<WindowsLocationLastKnownHook>,
    /// Hook for `locationWatchOpen`.
    pub(crate) watch_open: Option<WindowsLocationWatchOpenHook>,
    /// Hook for `locationWatchClose`.
    pub(crate) watch_close: Option<WindowsLocationWatchCloseHook>,
    /// Hook for `permissionRequest(location)`.
    pub(crate) request_permission: Option<WindowsLocationPermissionHook>,
}

/// Return the shared Windows document-pick hook slot for tests.
fn windows_document_pick_hook_slot() -> &'static Mutex<Option<WindowsDocumentPickHook>> {
    static HOOK: OnceLock<Mutex<Option<WindowsDocumentPickHook>>> = OnceLock::new();

    HOOK.get_or_init(|| Mutex::new(None))
}

/// Return the shared Windows calendar hook slot for tests.
fn windows_calendar_hook_slot() -> &'static Mutex<WindowsCalendarHooks> {
    static HOOK: OnceLock<Mutex<WindowsCalendarHooks>> = OnceLock::new();

    HOOK.get_or_init(|| Mutex::new(WindowsCalendarHooks::default()))
}

/// Return the shared Windows contact hook slot for tests.
fn windows_contact_hook_slot() -> &'static Mutex<WindowsContactHooks> {
    static HOOK: OnceLock<Mutex<WindowsContactHooks>> = OnceLock::new();

    HOOK.get_or_init(|| Mutex::new(WindowsContactHooks::default()))
}

/// Return the shared Windows location hook slot for tests.
fn windows_location_hook_slot() -> &'static Mutex<WindowsLocationHooks> {
    static HOOK: OnceLock<Mutex<WindowsLocationHooks>> = OnceLock::new();

    HOOK.get_or_init(|| Mutex::new(WindowsLocationHooks::default()))
}

/// Install one Windows document-pick hook for tests.
pub(crate) fn set_windows_document_test_pick_hook(hook: Option<WindowsDocumentPickHook>) {
    let mut slot = windows_document_pick_hook_slot()
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    *slot = hook;
}

/// Install one Windows calendar hook set for tests.
pub(crate) fn set_windows_calendar_test_hooks(hooks: WindowsCalendarHooks) {
    let mut slot = windows_calendar_hook_slot()
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    *slot = hooks;
}

/// Install one Windows contact hook set for tests.
pub(crate) fn set_windows_contact_test_hooks(hooks: WindowsContactHooks) {
    let mut slot = windows_contact_hook_slot()
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    *slot = hooks;
}

/// Install one Windows location hook set for tests.
pub(crate) fn set_windows_location_test_hooks(hooks: WindowsLocationHooks) {
    let mut slot = windows_location_hook_slot()
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    *slot = hooks;
}

/// Resolve the active Windows document-pick hook for tests.
fn require_test_pick_hook() -> RuntimeResult<WindowsDocumentPickHook> {
    let hook = windows_document_pick_hook_slot()
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

/// Return the active Windows calendar hooks for tests.
fn calendar_hooks() -> WindowsCalendarHooks {
    let slot = windows_calendar_hook_slot()
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    *slot
}

/// Return the active Windows contact hooks for tests.
fn contact_hooks() -> WindowsContactHooks {
    let slot = windows_contact_hook_slot()
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    *slot
}

/// Return the active Windows location hooks for tests.
fn location_hooks() -> WindowsLocationHooks {
    let slot = windows_location_hook_slot()
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    *slot
}

/// Submit one Windows calendar request from the host request lane in tests.
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

/// Submit one Windows contact request from the host request lane in tests.
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

/// Normalize Windows picker options for tests.
fn normalized_options(options: &DocumentPickOptionsValue) -> RuntimeResult<()> {
    validate_document_pick_options(options)?;

    let extensions = normalized_document_extensions(options)?;

    if options.allow_directories && !extensions.is_empty() {
        return Err(not_supported(HOST_DOCUMENT_PICK_OPERATION));
    }

    Ok(())
}

/// Pick documents from the Windows host request lane in tests.
pub(crate) fn pick_documents(
    _context: &HostRequestContext,
    options: &DocumentPickOptionsValue,
) -> RuntimeResult<Vec<DocumentDescriptorValue>> {
    normalized_options(options)?;
    let hook = require_test_pick_hook()?;
    hook(options.clone())
}

/// Submit one Windows location request from the host request lane in tests.
pub(crate) fn submit_location_request(
    context: &HostRequestContext,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    let hooks = location_hooks();

    match request {
        HostRequest::OsLocationServicesEnabled => {
            let Some(hook) = hooks.services_enabled else {
                return Err(missing_location_test_hook("location services"));
            };
            let is_enabled = hook()?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::Bool(is_enabled),
            )))
        }
        HostRequest::OsLocationLastKnown => {
            let Some(hook) = hooks.last_known else {
                return Err(missing_location_test_hook("last known location"));
            };
            let sample = hook()?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::LocationSample(sample),
            )))
        }
        HostRequest::OsLocationWatchOpen { watch_id, options } => {
            let Some(hook) = hooks.watch_open else {
                return Err(missing_location_test_hook("location watch open"));
            };

            hook(context.host_runtime_id, watch_id.clone(), *options)?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }
        HostRequest::OsLocationWatchClose { watch_id } => {
            let Some(hook) = hooks.watch_close else {
                return Err(missing_location_test_hook("location watch close"));
            };

            hook(context.host_runtime_id, watch_id.clone())?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }
        _ => Ok(None),
    }
}

/// Request one supported Windows location permission selector in tests.
pub(crate) fn request_location_permission(
    context: &HostRequestContext,
    permission: Permission,
) -> RuntimeResult<Option<PermissionState>> {
    if !matches!(permission, Permission::Location) {
        return Ok(None);
    }

    let hooks = location_hooks();
    let Some(hook) = hooks.request_permission else {
        return Ok(None);
    };

    let state = hook(context.host_runtime_id, permission)?;

    Ok(Some(state))
}

/// Remove one Windows runtime from the active location test lane.
pub(crate) fn unregister_location_runtime(_host_runtime_id: HostRuntimeId) {}

/// Build one missing-hook error for Windows location tests.
fn missing_location_test_hook(kind: &str) -> Box<crate::diagnostic::RuntimeError> {
    RuntimeError::from(PlatformError::generic(
        Some(PlatformErrorCode::Generic),
        format!("Windows location tests must install one {kind} hook before calling the host lane"),
    ))
    .boxed()
}

/// Build one missing-hook error for Windows calendar tests.
fn missing_calendar_test_hook(kind: &str) -> Box<crate::diagnostic::RuntimeError> {
    RuntimeError::from(PlatformError::generic(
        Some(PlatformErrorCode::Generic),
        format!("Windows calendar tests must install one {kind} hook before calling the host lane"),
    ))
    .boxed()
}

/// Build one missing-hook error for Windows contact tests.
fn missing_contact_test_hook(kind: &str) -> Box<crate::diagnostic::RuntimeError> {
    RuntimeError::from(PlatformError::generic(
        Some(PlatformErrorCode::Generic),
        format!("Windows contact tests must install one {kind} hook before calling the host lane"),
    ))
    .boxed()
}
