#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::os::bindings_generated as bindings;
use crate::platform::{NativeArray, NativeSlice, NativeStringRef, PlatformError};

use crate::runtime::BindingCallContext;
use bindings::*;

use crate::platform::os::{
    BackgroundEvent, BackgroundEventKind, BackgroundEventOpenOptions, BackgroundStatus,
    BackgroundTaskDescriptor, BackgroundTaskOptions, BackgroundTaskResult, BackgroundTriggerKind,
    CalendarAccess, CalendarAvailability, CalendarDescriptor, CalendarEvent, CalendarEventDraft,
    CalendarEventQuery, ClipboardBinaryFormat, Contact, ContactAddress, ContactDraft, ContactEmail,
    ContactName, ContactOrganization, ContactPage, ContactPhone, ContactQuery,
    CredentialAccessibility, CredentialAuthenticationMechanism, CredentialAuthenticationOptions,
    CredentialAuthenticationPolicy, CredentialAuthenticationResult, CredentialQuery,
    CredentialRecord, CredentialWriteOptions, DocumentAccess, DocumentDescriptor,
    DocumentPickOptions, HostIdentity, IntentEvent, IntentKind, IntentOpenOptions, IntentPayload,
    LifecycleEvent, LifecycleEventKind, LifecycleEventPayload, LifecycleLowMemoryPayload,
    LifecycleLowPowerPayload, LifecycleState, LoadAverage, LocationAccuracy, LocationSample,
    LocationWatchOptions, MediaAssetDescriptor, MediaAssetKind, MediaPage, MediaQuery, MountEntry,
    NetworkCellularGeneration, NetworkConnectionType, NetworkEvent, NetworkState,
    NotificationCategory, NotificationEvent, NotificationEventKind, NotificationEventOpenOptions,
    NotificationPermissionState, NotificationPriority, NotificationRequest,
    NotificationScheduledDescriptor, Permission, PermissionEntry, PermissionState, PowerState,
    SystemSnapshot,
};
use crate::platform::{fs, resource};

/// Clear clipboard payload.
///
/// Clear current host clipboard ownership or payload contents.
///
/// # Platform
/// Unix and Windows.
/// Uses host clipboard clear and owner-reset APIs.
///
/// # Errors
/// Returns ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `os.clipboard.write`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_clipboard_clear(context: &BindingCallContext) -> RuntimeResult<()> {
    let _ = context;

    Err(RuntimeError::from(PlatformError::not_supported("destack.os.clipboard.clear")).boxed())
}

/// Query whether text clipboard payload exists.
///
/// Return whether one text payload is currently available on the host clipboard.
///
/// # Platform
/// Unix and Windows.
/// Uses host clipboard query APIs and selection ownership checks.
///
/// # Errors
/// Returns ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.clipboard.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_os_clipboard_has_text(
    context: &BindingCallContext,
    out: *mut bool,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out);

    Err(RuntimeError::from(PlatformError::not_supported("destack.os.clipboard.hasText")).boxed())
}

/// Query whether host can route one URL target.
///
/// Ask host routing policy whether one URL target can be opened.
///
/// # Platform
/// Unix and Windows.
/// Uses `canOpenURL` or `resolveActivity` style APIs where available.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `os.intent.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_os_intent_can_open_url(
    context: &BindingCallContext,
    out: *mut bool,
    url: NativeStringRef,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, url);

    Err(RuntimeError::from(PlatformError::not_supported("destack.os.intent.canOpenUrl")).boxed())
}

/// Read whether host location services are enabled.
///
/// Read global host location-service availability before per-runtime authorization checks.
///
/// # Platform
/// Unix and Windows.
/// Uses host location service-status APIs.
///
/// # Errors
/// Returns ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.location.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_os_location_services_enabled(
    context: &BindingCallContext,
    out: *mut bool,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.location.servicesEnabled",
    ))
    .boxed())
}

/// Read one host calendar event.
///
/// Read one host calendar event payload by stable identifier.
///
/// # Platform
/// Unix and Windows.
/// Uses host calendar read APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.calendar.read`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_calendar_event_read(
    context: &BindingCallContext,
    out: *mut CalendarEvent,
    id: NativeStringRef,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, id);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.calendar.eventRead",
    ))
    .boxed())
}

/// List notification categories.
///
/// Enumerate registered host notification categories for this runtime context.
///
/// # Platform
/// Unix and Windows.
/// Uses host notification category query APIs where available.
///
/// # Errors
/// Returns ioPermissionDenied, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.notification.post`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_os_notification_category_list(
    context: &BindingCallContext,
    out: *mut NativeArray<NotificationCategory>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.notification.categoryList",
    ))
    .boxed())
}

/// Register notification categories.
///
/// Register host notification categories and actions for this runtime context.
///
/// # Platform
/// Unix and Windows.
/// Uses host notification category registration APIs.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `os.notification.post`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_notification_category_set(
    context: &BindingCallContext,
    categories: NativeArray<NotificationCategory>,
) -> RuntimeResult<()> {
    let _ = (context, categories);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.notification.categorySet",
    ))
    .boxed())
}

/// Cancel pending scheduled notification.
///
/// Cancel one pending scheduled host notification by identifier.
///
/// # Platform
/// Unix and Windows.
/// Uses host scheduled-notification cancellation APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `os.notification.post`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_notification_pending_cancel(
    context: &BindingCallContext,
    id: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = (context, id);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.notification.pendingCancel",
    ))
    .boxed())
}

/// Cancel all pending scheduled notifications.
///
/// Cancel all pending scheduled notifications owned by this runtime context.
///
/// # Platform
/// Unix and Windows.
/// Uses host scheduled-notification cancellation APIs.
///
/// # Errors
/// Returns ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `os.notification.post`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_notification_pending_cancel_all(
    context: &BindingCallContext,
) -> RuntimeResult<()> {
    let _ = context;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.notification.pendingCancelAll",
    ))
    .boxed())
}

/// List pending scheduled notifications.
///
/// Enumerate pending notification requests owned by this runtime context.
///
/// # Platform
/// Unix and Windows.
/// Uses host pending-notification query APIs.
///
/// # Errors
/// Returns ioPermissionDenied, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.notification.post`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_os_notification_pending_list(
    context: &BindingCallContext,
    out: *mut NativeArray<NotificationScheduledDescriptor>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.notification.pendingList",
    ))
    .boxed())
}

/// Schedule host notification.
///
/// Schedule one host notification request for deferred delivery according to trigger policy.
///
/// # Platform
/// Unix and Windows.
/// Uses host notification scheduling APIs on each platform.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `os.notification.post`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_notification_schedule(
    context: &BindingCallContext,
    out: *mut NativeStringRef,
    request: NotificationRequest,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, request);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.notification.schedule",
    ))
    .boxed())
}

/// Request multiple permissions.
///
/// Request host authorization for one selector list and return resulting states.
///
/// # Platform
/// Unix and Windows.
/// Uses batched host permission request APIs where available.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `os.permission.request`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_permission_request_many(
    context: &BindingCallContext,
    out: *mut NativeArray<PermissionEntry>,
    permissions: NativeArray<Permission>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, permissions);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.permission.requestMany",
    ))
    .boxed())
}

/// Read permission states.
///
/// Read current host permission states for one selector list.
///
/// # Platform
/// Unix and Windows.
/// Uses host permission-state APIs and policy bridges.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.permission.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_os_permission_state_many(
    context: &BindingCallContext,
    out: *mut NativeArray<PermissionEntry>,
    permissions: NativeArray<Permission>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, permissions);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.permission.stateMany",
    ))
    .boxed())
}

/// Report completion for one scheduled background-task execution.
///
/// Submit final execution status for one scheduled task execution token.
///
/// # Platform
/// Unix and Windows.
/// Uses host background scheduler completion APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `os.background.control`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_background_complete(
    context: &BindingCallContext,
    executionid: NativeStringRef,
    argument_result: BackgroundTaskResult,
) -> RuntimeResult<()> {
    let _ = (context, executionid, argument_result);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.background.complete",
    ))
    .boxed())
}

/// Close one background-task event stream.
///
/// Close one opened event stream and release host callback routing resources.
///
/// # Platform
/// Unix and Windows.
/// Uses host callback unregistration APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `os.background.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_os_background_event_close(
    context: &BindingCallContext,
    handle: resource::BackgroundEventHandle,
) -> RuntimeResult<()> {
    let _ = (context, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.background.event.close",
    ))
    .boxed())
}

/// Open one background-task event stream.
///
/// Open one event stream for task-ready and expiration callbacks.
///
/// # Platform
/// Unix and Windows.
/// Uses host background scheduler callback bridges.
///
/// # Errors
/// Returns ioWouldBlock, notSupported.
///
/// # Security
/// Requires `os.background.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_os_background_event_open(
    context: &BindingCallContext,
    out: *mut resource::BackgroundEventHandle,
    options: BackgroundEventOpenOptions,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, options);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.background.event.open",
    ))
    .boxed())
}

/// Wait for one background-task event.
///
/// Wait for one queued background-task event from one opened event stream.
///
/// # Platform
/// Unix and Windows.
/// Uses host background event queues.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInterrupted, notSupported.
///
/// # Security
/// Requires `os.background.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_os_background_event_read(
    context: &BindingCallContext,
    out: *mut BackgroundEvent,
    handle: resource::BackgroundEventHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, handle, timeoutns);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.background.event.read",
    ))
    .boxed())
}

/// Poll one background-task event without blocking.
///
/// Poll one queued background-task event from one opened event stream without waiting.
///
/// # Platform
/// Unix and Windows.
/// Uses host nonblocking background event queue reads.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `os.background.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_os_background_event_try_read(
    context: &BindingCallContext,
    out: *mut BackgroundEvent,
    handle: resource::BackgroundEventHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.background.event.tryRead",
    ))
    .boxed())
}

/// List background-task registrations.
///
/// Enumerate background-task registrations for this runtime identity.
///
/// # Platform
/// Unix and Windows.
/// Uses host background scheduler registration queries.
///
/// # Errors
/// Returns ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.background.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_os_background_list(
    context: &BindingCallContext,
    out: *mut NativeArray<BackgroundTaskDescriptor>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out);

    Err(RuntimeError::from(PlatformError::not_supported("destack.os.background.list")).boxed())
}

/// Register one background task.
///
/// Create or replace one background-task registration for this runtime identity.
///
/// # Platform
/// Unix and Windows.
/// Uses host background scheduler registration APIs.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.background.control`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_background_register(
    context: &BindingCallContext,
    options: BackgroundTaskOptions,
) -> RuntimeResult<()> {
    let _ = (context, options);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.background.register",
    ))
    .boxed())
}

/// Read background-task scheduler status.
///
/// Read the current background-task scheduler status for this host runtime.
///
/// # Platform
/// Unix and Windows.
/// Uses BGTaskScheduler on Apple platforms, WorkManager or JobScheduler on Android, and host scheduler bridges on desktop platforms.
///
/// # Errors
/// Returns ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.background.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_os_background_status(
    context: &BindingCallContext,
    out: *mut BackgroundStatus,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out);

    Err(RuntimeError::from(PlatformError::not_supported("destack.os.background.status")).boxed())
}

/// Trigger one background task for testing.
///
/// Ask the host scheduler to trigger one registered task immediately for development validation where supported.
///
/// # Platform
/// Unix and Windows.
/// Uses host developer test hooks where available.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `os.background.control`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_background_trigger_test(
    context: &BindingCallContext,
    out: *mut bool,
    identifier: NativeStringRef,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, identifier);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.background.triggerTest",
    ))
    .boxed())
}

/// Unregister one background task.
///
/// Remove one background-task registration by identifier.
///
/// # Platform
/// Unix and Windows.
/// Uses host background scheduler unregistration APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `os.background.control`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_background_unregister(
    context: &BindingCallContext,
    identifier: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = (context, identifier);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.background.unregister",
    ))
    .boxed())
}

/// Create one calendar event.
///
/// Create one host calendar event and return its stable identifier.
///
/// # Platform
/// Unix and Windows.
/// Uses host calendar-write APIs where available.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.calendar.write`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_calendar_event_create(
    context: &BindingCallContext,
    out: *mut NativeStringRef,
    event: CalendarEventDraft,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, event);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.calendar.eventCreate",
    ))
    .boxed())
}

/// Delete one calendar event.
///
/// Delete one existing host calendar event by identifier.
///
/// # Platform
/// Unix and Windows.
/// Uses host calendar-delete APIs where available.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `os.calendar.write`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_calendar_event_delete(
    context: &BindingCallContext,
    id: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = (context, id);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.calendar.eventDelete",
    ))
    .boxed())
}

/// List host calendar events.
///
/// Enumerate host calendar events over one selected UTC time range.
///
/// # Platform
/// Unix and Windows.
/// Uses host calendar query APIs.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.calendar.read`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_calendar_event_list(
    context: &BindingCallContext,
    out: *mut NativeArray<CalendarEvent>,
    query: CalendarEventQuery,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, query);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.calendar.eventList",
    ))
    .boxed())
}

/// Update one calendar event.
///
/// Update one existing host calendar event by identifier.
///
/// # Platform
/// Unix and Windows.
/// Uses host calendar-write APIs where available.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.calendar.write`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_calendar_event_update(
    context: &BindingCallContext,
    id: NativeStringRef,
    event: CalendarEventDraft,
) -> RuntimeResult<()> {
    let _ = (context, id, event);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.calendar.eventUpdate",
    ))
    .boxed())
}

/// List host calendars.
///
/// Enumerate readable host calendars and return descriptor metadata.
///
/// # Platform
/// Unix and Windows.
/// Uses EventKit on Apple platforms, CalendarContract on Android, and calendar provider APIs on desktop hosts.
///
/// # Errors
/// Returns ioPermissionDenied, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.calendar.read`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_calendar_list(
    context: &BindingCallContext,
    out: *mut NativeArray<CalendarDescriptor>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out);

    Err(RuntimeError::from(PlatformError::not_supported("destack.os.calendar.list")).boxed())
}

/// Read binary clipboard payload.
///
/// Read one binary payload in one selected clipboard format.
///
/// # Platform
/// Unix and Windows.
/// Uses host clipboard APIs for binary format extraction.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.clipboard.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_os_clipboard_read_bytes(
    context: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    format: ClipboardBinaryFormat,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, format);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.clipboard.readBytes",
    ))
    .boxed())
}

/// Read text clipboard payload.
///
/// Read one text payload from the host clipboard.
///
/// # Platform
/// Unix and Windows.
/// Uses host clipboard APIs such as X11 or Wayland selection protocols and Win32 clipboard APIs.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.clipboard.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_os_clipboard_read_text(
    context: &BindingCallContext,
    out: *mut NativeStringRef,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.clipboard.readText",
    ))
    .boxed())
}

/// Read clipboard sequence number.
///
/// Read one monotonic sequence marker for host clipboard contents.
/// Sequence semantics are host defined but monotonic within one host clipboard service lifetime.
///
/// # Platform
/// Unix and Windows.
/// Uses host clipboard sequence APIs where available.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.clipboard.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_os_clipboard_sequence(
    context: &BindingCallContext,
    out: *mut u64,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.clipboard.sequence",
    ))
    .boxed())
}

/// Write binary clipboard payload.
///
/// Write one binary payload in one selected clipboard format.
///
/// # Platform
/// Unix and Windows.
/// Uses host clipboard APIs for binary format insertion.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `os.clipboard.write`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_clipboard_write_bytes(
    context: &BindingCallContext,
    format: ClipboardBinaryFormat,
    argument_bytes: NativeSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (context, format, argument_bytes);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.clipboard.writeBytes",
    ))
    .boxed())
}

/// Write text clipboard payload.
///
/// Write one text payload into the host clipboard.
///
/// # Platform
/// Unix and Windows.
/// Uses host clipboard write APIs and ownership semantics.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `os.clipboard.write`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_clipboard_write_text(
    context: &BindingCallContext,
    text: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = (context, text);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.clipboard.writeText",
    ))
    .boxed())
}

/// Create one contact.
///
/// Create one host contact record and return its stable identifier.
///
/// # Platform
/// Unix and Windows.
/// Uses host contact-write APIs where available.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.contact.write`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_contact_create(
    context: &BindingCallContext,
    out: *mut NativeStringRef,
    contact: ContactDraft,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, contact);

    Err(RuntimeError::from(PlatformError::not_supported("destack.os.contact.create")).boxed())
}

/// Delete one contact.
///
/// Delete one existing host contact by identifier.
///
/// # Platform
/// Unix and Windows.
/// Uses host contact-delete APIs where available.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `os.contact.write`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_contact_delete(
    context: &BindingCallContext,
    id: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = (context, id);

    Err(RuntimeError::from(PlatformError::not_supported("destack.os.contact.delete")).boxed())
}

/// List contacts.
///
/// Return one page of host contacts with one selected field set.
///
/// # Platform
/// Unix and Windows.
/// Uses Contacts framework on Apple platforms, ContactsContract on Android, and address-book provider APIs on desktop hosts.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.contact.read`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_contact_list(
    context: &BindingCallContext,
    out: *mut ContactPage,
    query: ContactQuery,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, query);

    Err(RuntimeError::from(PlatformError::not_supported("destack.os.contact.list")).boxed())
}

/// Read one contact by identifier.
///
/// Read one host contact payload by stable identifier.
///
/// # Platform
/// Unix and Windows.
/// Uses host contact read APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.contact.read`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_contact_read(
    context: &BindingCallContext,
    out: *mut Contact,
    id: NativeStringRef,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, id);

    Err(RuntimeError::from(PlatformError::not_supported("destack.os.contact.read")).boxed())
}

/// Search contacts.
///
/// Return one page of host contacts matching one backend query string.
///
/// # Platform
/// Unix and Windows.
/// Uses host contact search APIs where available.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.contact.read`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_contact_search(
    context: &BindingCallContext,
    out: *mut ContactPage,
    querytext: NativeStringRef,
    query: ContactQuery,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, querytext, query);

    Err(RuntimeError::from(PlatformError::not_supported("destack.os.contact.search")).boxed())
}

/// Update one contact.
///
/// Update one existing host contact by identifier.
///
/// # Platform
/// Unix and Windows.
/// Uses host contact-write APIs where available.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.contact.write`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_contact_update(
    context: &BindingCallContext,
    id: NativeStringRef,
    contact: ContactDraft,
) -> RuntimeResult<()> {
    let _ = (context, id, contact);

    Err(RuntimeError::from(PlatformError::not_supported("destack.os.contact.update")).boxed())
}

/// Request one host credential authentication challenge.
///
/// Request one host authentication challenge and return challenge outcome.
///
/// # Platform
/// Unix and Windows.
/// Uses LocalAuthentication and biometric manager APIs when available.
///
/// # Errors
/// Returns ioPermissionDenied, ioWouldBlock, ioInterrupted, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.credentials.auth`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_credentials_authenticate(
    context: &BindingCallContext,
    out: *mut CredentialAuthenticationResult,
    options: CredentialAuthenticationOptions,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, options);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.credentials.authenticate",
    ))
    .boxed())
}

/// Query credential presence.
///
/// Return whether one credential record exists for one service and account pair.
///
/// # Platform
/// Unix and Windows.
/// Uses host credential-query APIs.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.credentials.read`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_credentials_contains(
    context: &BindingCallContext,
    out: *mut bool,
    service: NativeStringRef,
    account: NativeStringRef,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, service, account);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.credentials.contains",
    ))
    .boxed())
}

/// Delete one credential record.
///
/// Delete one secure credential payload from host credential store.
///
/// # Platform
/// Unix and Windows.
/// Uses host credential-delete APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `os.credentials.write`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_credentials_delete(
    context: &BindingCallContext,
    service: NativeStringRef,
    account: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = (context, service, account);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.credentials.delete",
    ))
    .boxed())
}

/// Read one credential record.
///
/// Read one secure credential payload from host credential store.
///
/// # Platform
/// Unix and Windows.
/// Uses Keychain on Apple platforms, Keystore-backed secure storage on Android, and credential manager APIs on desktop hosts.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.credentials.read`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_credentials_read(
    context: &BindingCallContext,
    out: *mut CredentialRecord,
    query: CredentialQuery,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, query);

    Err(RuntimeError::from(PlatformError::not_supported("destack.os.credentials.read")).boxed())
}

/// Write one credential record.
///
/// Create or replace one secure credential payload in host credential store.
///
/// # Platform
/// Unix and Windows.
/// Uses host credential-write APIs.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.credentials.write`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_credentials_write(
    context: &BindingCallContext,
    options: CredentialWriteOptions,
) -> RuntimeResult<()> {
    let _ = (context, options);

    Err(RuntimeError::from(PlatformError::not_supported("destack.os.credentials.write")).boxed())
}

/// Close one opened document handle.
///
/// Close one opened document-provider handle and release host resources.
///
/// # Platform
/// Unix and Windows.
/// Uses host descriptor close APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `os.document.control`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_document_close(
    context: &BindingCallContext,
    handle: resource::DocumentHandle,
) -> RuntimeResult<()> {
    let _ = (context, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.os.document.close")).boxed())
}

/// Flush one opened document handle.
///
/// Flush buffered outbound document bytes for one opened handle.
///
/// # Platform
/// Unix and Windows.
/// Uses host document-provider flush APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `os.document.write`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_document_flush(
    context: &BindingCallContext,
    handle: resource::DocumentHandle,
) -> RuntimeResult<()> {
    let _ = (context, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.os.document.flush")).boxed())
}

/// Open one document URI.
///
/// Open one host document-provider URI with one selected access mode.
///
/// # Platform
/// Unix and Windows.
/// Uses host document-provider open APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `os.document.control`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_document_open(
    context: &BindingCallContext,
    out: *mut resource::DocumentHandle,
    uri: NativeStringRef,
    access: DocumentAccess,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, uri, access);

    Err(RuntimeError::from(PlatformError::not_supported("destack.os.document.open")).boxed())
}

/// Pick documents from host picker UI.
///
/// Open host picker UI and return selected document descriptors.
///
/// # Platform
/// Unix and Windows.
/// Uses SAF or system picker APIs on Android, UIDocumentPicker on Apple platforms, and host file-picker bridges on desktop hosts.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.document.pick`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_document_pick(
    context: &BindingCallContext,
    out: *mut NativeArray<DocumentDescriptor>,
    options: DocumentPickOptions,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, options);

    Err(RuntimeError::from(PlatformError::not_supported("destack.os.document.pick")).boxed())
}

/// Read one chunk of document bytes.
///
/// Read up to `maxBytes` from one opened document handle.
///
/// # Platform
/// Unix and Windows.
/// Uses host document-provider read APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInterrupted, notSupported.
///
/// # Security
/// Requires `os.document.read`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_document_read(
    context: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::DocumentHandle,
    maxbytes: u32,
    timeoutns: u64,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, handle, maxbytes, timeoutns);

    Err(RuntimeError::from(PlatformError::not_supported("destack.os.document.read")).boxed())
}

/// Poll one chunk of document bytes without blocking.
///
/// Read up to `maxBytes` from one opened document handle without waiting.
///
/// # Platform
/// Unix and Windows.
/// Uses host nonblocking document-provider read APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `os.document.read`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_document_try_read(
    context: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::DocumentHandle,
    maxbytes: u32,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, handle, maxbytes);

    Err(RuntimeError::from(PlatformError::not_supported("destack.os.document.tryRead")).boxed())
}

/// Write one chunk of document bytes.
///
/// Write one byte chunk into one opened document handle and return written byte count.
///
/// # Platform
/// Unix and Windows.
/// Uses host document-provider write APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `os.document.write`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_document_write(
    context: &BindingCallContext,
    out: *mut u32,
    handle: resource::DocumentHandle,
    argument_bytes: NativeSlice<u8>,
    timeoutns: u64,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, handle, argument_bytes, timeoutns);

    Err(RuntimeError::from(PlatformError::not_supported("destack.os.document.write")).boxed())
}

/// Read host identity.
///
/// Return one normalized host identity payload.
/// Identity fields are sourced from host kernel and runtime normalization rules.
///
/// # Platform
/// Unix and Windows.
/// Uses uname and hostname APIs on Unix and version and hostname APIs on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.hostname`, `os.sysinfo`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_os_host_identity(
    context: &BindingCallContext,
    out: *mut HostIdentity,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out);

    Err(RuntimeError::from(PlatformError::not_supported("destack.os.host.identity")).boxed())
}

/// Read host boot time.
///
/// Return the Unix timestamp for host boot time in nanoseconds.
/// Timestamp origin and precision follow host timekeeping interfaces.
///
/// # Platform
/// Unix and Windows.
/// Uses boot-time sysctl or procfs style sources on Unix and boot-time system info on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.sysinfo`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_os_boot_time_unix_ns(
    context: &BindingCallContext,
    out: *mut u64,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.info.bootTimeUnixNs",
    ))
    .boxed())
}

/// Read host load averages.
///
/// Return host load averages over one, five, and fifteen minute windows.
/// Values reflect host scheduler accounting and may be unavailable on some kernels.
///
/// # Platform
/// Unix only.
/// Uses getloadavg style interfaces or kernel load-average exports.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.sysinfo`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_os_load_average(
    context: &BindingCallContext,
    out: *mut LoadAverage,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out);

    Err(RuntimeError::from(PlatformError::not_supported("destack.os.info.loadAverage")).boxed())
}

/// Read host system information.
///
/// Return one normalized system-information payload.
/// Topology and capacity fields are sampled from host APIs at call time.
///
/// # Platform
/// Unix and Windows.
/// Uses sysconf/sysinfo-style APIs on Unix and GlobalMemoryStatusEx plus processor APIs on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.sysinfo`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_os_system_snapshot(
    context: &BindingCallContext,
    out: *mut SystemSnapshot,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.info.systemSnapshot",
    ))
    .boxed())
}

/// Read host uptime.
///
/// Return host uptime in nanoseconds from system boot.
/// Uptime source follows host monotonic uptime facilities.
///
/// # Platform
/// Unix and Windows.
/// Uses clock_gettime style uptime on Unix and GetTickCount64 style uptime on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.sysinfo`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_os_uptime_ns(
    context: &BindingCallContext,
    out: *mut u64,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out);

    Err(RuntimeError::from(PlatformError::not_supported("destack.os.info.uptimeNs")).boxed())
}

/// Close one host intent stream.
///
/// Close one opened host intent stream and release host callback routing resources.
///
/// # Platform
/// Unix and Windows.
/// Uses backend-specific intent callback unregistration.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `os.intent.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_os_intent_close(
    context: &BindingCallContext,
    handle: resource::IntentHandle,
) -> RuntimeResult<()> {
    let _ = (context, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.os.intent.close")).boxed())
}

/// Open one host intent stream.
///
/// Open one inbound host intent stream for activation and share payload events.
///
/// # Platform
/// Unix and Windows.
/// Uses shell activation hooks on desktop platforms and runtime host intent bridges on mobile-like hosts.
///
/// # Errors
/// Returns ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `os.intent.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_os_intent_open(
    context: &BindingCallContext,
    out: *mut resource::IntentHandle,
    options: IntentOpenOptions,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, options);

    Err(RuntimeError::from(PlatformError::not_supported("destack.os.intent.open")).boxed())
}

/// Request host to open one file path target.
///
/// Ask host shell or app framework to open one file path with default routing.
///
/// # Platform
/// Unix and Windows.
/// Uses shell open-file APIs and host launcher integration.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `os.intent.write`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_intent_open_path(
    context: &BindingCallContext,
    path: fs::OsPath,
) -> RuntimeResult<()> {
    let _ = (context, path);

    Err(RuntimeError::from(PlatformError::not_supported("destack.os.intent.openPath")).boxed())
}

/// Request host to open one URL target.
///
/// Ask host shell or app framework to open one URL with default routing.
///
/// # Platform
/// Unix and Windows.
/// Uses shell open-url APIs on desktop platforms and intent launch APIs on mobile-like hosts.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `os.intent.write`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_intent_open_url(
    context: &BindingCallContext,
    url: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = (context, url);

    Err(RuntimeError::from(PlatformError::not_supported("destack.os.intent.openUrl")).boxed())
}

/// Wait for one inbound intent event.
///
/// Wait for one queued inbound intent event from one opened intent stream.
///
/// # Platform
/// Unix and Windows.
/// Uses backend intent event queues or callback bridges.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInterrupted, notSupported.
///
/// # Security
/// Requires `os.intent.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_os_intent_read(
    context: &BindingCallContext,
    out: *mut IntentEvent,
    handle: resource::IntentHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, handle, timeoutns);

    Err(RuntimeError::from(PlatformError::not_supported("destack.os.intent.read")).boxed())
}

/// Share file paths through host share routing.
///
/// Ask host share infrastructure to present one file list to target applications.
///
/// # Platform
/// Unix and Windows.
/// Uses platform share APIs and shell handoff integration.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `os.intent.write`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_intent_share_paths(
    context: &BindingCallContext,
    paths: NativeArray<fs::OsPath>,
    mimetype: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = (context, paths, mimetype);

    Err(RuntimeError::from(PlatformError::not_supported("destack.os.intent.sharePaths")).boxed())
}

/// Share one text payload through host share routing.
///
/// Ask host share infrastructure to present one text payload to target applications.
///
/// # Platform
/// Unix and Windows.
/// Uses platform share APIs where available.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `os.intent.write`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_intent_share_text(
    context: &BindingCallContext,
    text: NativeStringRef,
    mimetype: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = (context, text, mimetype);

    Err(RuntimeError::from(PlatformError::not_supported("destack.os.intent.shareText")).boxed())
}

/// Poll one inbound intent event without blocking.
///
/// Poll one queued inbound intent event from one opened intent stream.
///
/// # Platform
/// Unix and Windows.
/// Uses backend nonblocking intent queue reads.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `os.intent.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_os_intent_try_read(
    context: &BindingCallContext,
    out: *mut IntentEvent,
    handle: resource::IntentHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.os.intent.tryRead")).boxed())
}

/// Close lifecycle event stream.
///
/// Close one lifecycle event stream and release host callback routing resources.
///
/// # Platform
/// Unix and Windows.
/// Uses host callback unregistration APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `os.lifecycle.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_os_lifecycle_close(
    context: &BindingCallContext,
    handle: resource::LifecycleEventHandle,
) -> RuntimeResult<()> {
    let _ = (context, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.os.lifecycle.close")).boxed())
}

/// Open lifecycle event stream.
///
/// Open one lifecycle event stream for runtime lifecycle transitions.
///
/// # Platform
/// Unix and Windows.
/// Uses host application lifecycle callback bridges.
///
/// # Errors
/// Returns ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `os.lifecycle.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_os_lifecycle_open(
    context: &BindingCallContext,
    out: *mut resource::LifecycleEventHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out);

    Err(RuntimeError::from(PlatformError::not_supported("destack.os.lifecycle.open")).boxed())
}

/// Wait for one lifecycle event.
///
/// Wait for one lifecycle event from one opened stream.
///
/// # Platform
/// Unix and Windows.
/// Uses host lifecycle event queues.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInterrupted, notSupported.
///
/// # Security
/// Requires `os.lifecycle.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_os_lifecycle_read(
    context: &BindingCallContext,
    out: *mut LifecycleEvent,
    handle: resource::LifecycleEventHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, handle, timeoutns);

    Err(RuntimeError::from(PlatformError::not_supported("destack.os.lifecycle.read")).boxed())
}

/// Read current lifecycle state.
///
/// Return the current runtime lifecycle state from the host integration layer.
///
/// # Platform
/// Unix and Windows.
/// Uses host application lifecycle bridges on desktop and mobile platforms.
///
/// # Errors
/// Returns ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.lifecycle.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_os_lifecycle_state(
    context: &BindingCallContext,
    out: *mut LifecycleState,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out);

    Err(RuntimeError::from(PlatformError::not_supported("destack.os.lifecycle.state")).boxed())
}

/// Poll one lifecycle event without blocking.
///
/// Poll one lifecycle event from one opened stream without waiting.
///
/// # Platform
/// Unix and Windows.
/// Uses nonblocking host lifecycle event queue reads.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `os.lifecycle.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_os_lifecycle_try_read(
    context: &BindingCallContext,
    out: *mut LifecycleEvent,
    handle: resource::LifecycleEventHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, handle);

    Err(RuntimeError::from(PlatformError::not_supported("destack.os.lifecycle.tryRead")).boxed())
}

/// Read last known location sample.
///
/// Read one cached location sample from the host location service.
///
/// # Platform
/// Unix and Windows.
/// Uses CoreLocation on Apple platforms, FusedLocationProvider or LocationManager on Android, and Geolocator on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.location.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_os_location_last_known(
    context: &BindingCallContext,
    out: *mut LocationSample,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.location.lastKnown",
    ))
    .boxed())
}

/// Close location watch stream.
///
/// Close one location watch stream and release host subscription resources.
///
/// # Platform
/// Unix and Windows.
/// Uses host location unsubscription APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `os.location.watch`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_os_location_watch_close(
    context: &BindingCallContext,
    handle: resource::LocationWatchHandle,
) -> RuntimeResult<()> {
    let _ = (context, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.location.watchClose",
    ))
    .boxed())
}

/// Open location watch stream.
///
/// Open one location watch stream with one selected update policy.
///
/// # Platform
/// Unix and Windows.
/// Uses host location subscription APIs.
///
/// # Errors
/// Returns ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `os.location.watch`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_os_location_watch_open(
    context: &BindingCallContext,
    out: *mut resource::LocationWatchHandle,
    options: LocationWatchOptions,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, options);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.location.watchOpen",
    ))
    .boxed())
}

/// Wait for one location sample.
///
/// Wait for one location sample from one opened location watch stream.
///
/// # Platform
/// Unix and Windows.
/// Uses host location update queues.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInterrupted, notSupported.
///
/// # Security
/// Requires `os.location.watch`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_os_location_watch_read(
    context: &BindingCallContext,
    out: *mut LocationSample,
    handle: resource::LocationWatchHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, handle, timeoutns);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.location.watchRead",
    ))
    .boxed())
}

/// Poll one location sample without blocking.
///
/// Poll one location sample from one opened location watch stream without waiting.
///
/// # Platform
/// Unix and Windows.
/// Uses nonblocking host location update queue reads.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `os.location.watch`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_os_location_watch_try_read(
    context: &BindingCallContext,
    out: *mut LocationSample,
    handle: resource::LocationWatchHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.location.watchTryRead",
    ))
    .boxed())
}

/// Delete media assets by identifier.
///
/// Delete host media assets and return deleted asset count.
///
/// # Platform
/// Unix and Windows.
/// Uses host media-library delete APIs.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.media.write`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_media_delete(
    context: &BindingCallContext,
    out: *mut u32,
    ids: NativeArray<NativeStringRef>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, ids);

    Err(RuntimeError::from(PlatformError::not_supported("destack.os.media.delete")).boxed())
}

/// Import one file path into host media library.
///
/// Import one file from one runtime-visible path and return one created media identifier.
///
/// # Platform
/// Unix and Windows.
/// Uses host media-library write APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.media.write`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_media_import_path(
    context: &BindingCallContext,
    out: *mut NativeStringRef,
    path: fs::OsPath,
    kind: MediaAssetKind,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, path, kind);

    Err(RuntimeError::from(PlatformError::not_supported("destack.os.media.importPath")).boxed())
}

/// List media assets.
///
/// Return one page of host media assets for one query.
///
/// # Platform
/// Unix and Windows.
/// Uses Photos framework on Apple platforms, MediaStore on Android, and host media-library bridges on desktop hosts.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.media.read`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_media_list(
    context: &BindingCallContext,
    out: *mut MediaPage,
    query: MediaQuery,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, query);

    Err(RuntimeError::from(PlatformError::not_supported("destack.os.media.list")).boxed())
}

/// Read one media asset descriptor.
///
/// Read one host media asset descriptor by stable identifier.
///
/// # Platform
/// Unix and Windows.
/// Uses host media-library read APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.media.read`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_media_read(
    context: &BindingCallContext,
    out: *mut MediaAssetDescriptor,
    id: NativeStringRef,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, id);

    Err(RuntimeError::from(PlatformError::not_supported("destack.os.media.read")).boxed())
}

/// Mount one filesystem target.
///
/// Mount one source on one target path with explicit flags and data.
/// Mount privilege checks and propagation policy are host-defined.
///
/// # Platform
/// Unix and Windows.
/// Uses mount(2)-family APIs on Unix and volume-mount APIs on Windows.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.mount`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_os_mount_add(
    context: &BindingCallContext,
    source: NativeStringRef,
    target: fs::OsPath,
    filesystem: NativeStringRef,
    flags: u64,
    data: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = (context, source, target, filesystem, flags, data);

    Err(RuntimeError::from(PlatformError::not_supported("destack.os.mount.add")).boxed())
}

/// Enumerate mount table entries.
///
/// Return one snapshot of the current host mount table.
/// Entry shape is normalized but field availability is host-dependent.
///
/// # Platform
/// Unix and Windows.
/// Uses mount table APIs on Unix and volume enumeration APIs on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.mount`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_os_mount_list(
    context: &BindingCallContext,
    out: *mut NativeArray<MountEntry>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out);

    Err(RuntimeError::from(PlatformError::not_supported("destack.os.mount.list")).boxed())
}

/// Unmount one filesystem target.
///
/// Unmount one target path with explicit unmount flags.
/// Forced unmount behavior follows host kernel semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses umount or unmount APIs on Unix and volume unmount APIs on Windows.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.mount`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_os_mount_remove(
    context: &BindingCallContext,
    target: fs::OsPath,
    flags: u64,
) -> RuntimeResult<()> {
    let _ = (context, target, flags);

    Err(RuntimeError::from(PlatformError::not_supported("destack.os.mount.remove")).boxed())
}

/// Read host network state.
///
/// Read one point-in-time host network state snapshot.
///
/// # Platform
/// Unix and Windows.
/// Uses Network.framework path monitoring on Apple platforms, ConnectivityManager on Android, and NetworkInformation APIs on Windows.
///
/// # Errors
/// Returns ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.network.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_os_network_state(
    context: &BindingCallContext,
    out: *mut NetworkState,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out);

    Err(RuntimeError::from(PlatformError::not_supported("destack.os.network.state")).boxed())
}

/// Close host network watch stream.
///
/// Close one host network watch stream and release host subscription resources.
///
/// # Platform
/// Unix and Windows.
/// Uses host network callback unsubscription APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `os.network.watch`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_os_network_watch_close(
    context: &BindingCallContext,
    handle: resource::NetworkWatchHandle,
) -> RuntimeResult<()> {
    let _ = (context, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.network.watchClose",
    ))
    .boxed())
}

/// Open host network watch stream.
///
/// Open one host network watch stream for connectivity state transitions.
///
/// # Platform
/// Unix and Windows.
/// Uses host network callback subscription APIs.
///
/// # Errors
/// Returns ioWouldBlock, notSupported.
///
/// # Security
/// Requires `os.network.watch`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_os_network_watch_open(
    context: &BindingCallContext,
    out: *mut resource::NetworkWatchHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out);

    Err(RuntimeError::from(PlatformError::not_supported("destack.os.network.watchOpen")).boxed())
}

/// Wait for one host network event.
///
/// Wait for one queued host network event from one opened watch stream.
///
/// # Platform
/// Unix and Windows.
/// Uses host network event queues.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInterrupted, notSupported.
///
/// # Security
/// Requires `os.network.watch`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_os_network_watch_read(
    context: &BindingCallContext,
    out: *mut NetworkEvent,
    handle: resource::NetworkWatchHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, handle, timeoutns);

    Err(RuntimeError::from(PlatformError::not_supported("destack.os.network.watchRead")).boxed())
}

/// Poll one host network event without blocking.
///
/// Poll one queued host network event from one opened watch stream without waiting.
///
/// # Platform
/// Unix and Windows.
/// Uses host nonblocking network event queue reads.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `os.network.watch`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_os_network_watch_try_read(
    context: &BindingCallContext,
    out: *mut NetworkEvent,
    handle: resource::NetworkWatchHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.network.watchTryRead",
    ))
    .boxed())
}

/// Cancel host notification.
///
/// Cancel one previously posted host notification by identifier.
///
/// # Platform
/// Unix and Windows.
/// Uses host notification cancellation APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `os.notification.post`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_notification_cancel(
    context: &BindingCallContext,
    id: NativeStringRef,
) -> RuntimeResult<()> {
    let _ = (context, id);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.notification.cancel",
    ))
    .boxed())
}

/// Cancel all host notifications for this runtime context.
///
/// Cancel all currently posted host notifications owned by this runtime context.
///
/// # Platform
/// Unix and Windows.
/// Uses host notification cancellation APIs.
///
/// # Errors
/// Returns ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `os.notification.post`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_notification_cancel_all(
    context: &BindingCallContext,
) -> RuntimeResult<()> {
    let _ = context;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.notification.cancelAll",
    ))
    .boxed())
}

/// Close one notification event stream.
///
/// Close one opened event stream and release host callback routing resources.
///
/// # Platform
/// Unix and Windows.
/// Uses host callback unregistration APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `os.notification.permission`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_notification_event_close(
    context: &BindingCallContext,
    handle: resource::NotificationEventHandle,
) -> RuntimeResult<()> {
    let _ = (context, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.notification.event.close",
    ))
    .boxed())
}

/// Open one notification event stream.
///
/// Open one event stream for delivered, interacted, and dismissed notification events.
///
/// # Platform
/// Unix and Windows.
/// Uses host notification callback bridges.
///
/// # Errors
/// Returns ioWouldBlock, notSupported.
///
/// # Security
/// Requires `os.notification.permission`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_notification_event_open(
    context: &BindingCallContext,
    out: *mut resource::NotificationEventHandle,
    options: NotificationEventOpenOptions,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, options);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.notification.event.open",
    ))
    .boxed())
}

/// Wait for one notification event.
///
/// Wait for one queued notification event from one opened event stream.
///
/// # Platform
/// Unix and Windows.
/// Uses host notification event queues.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInterrupted, notSupported.
///
/// # Security
/// Requires `os.notification.permission`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_notification_event_read(
    context: &BindingCallContext,
    out: *mut NotificationEvent,
    handle: resource::NotificationEventHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, handle, timeoutns);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.notification.event.read",
    ))
    .boxed())
}

/// Poll one notification event without blocking.
///
/// Poll one queued notification event from one opened event stream without waiting.
///
/// # Platform
/// Unix and Windows.
/// Uses host nonblocking notification event queue reads.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `os.notification.permission`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_notification_event_try_read(
    context: &BindingCallContext,
    out: *mut NotificationEvent,
    handle: resource::NotificationEventHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, handle);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.notification.event.tryRead",
    ))
    .boxed())
}

/// Read host notification permission state.
///
/// Return current host notification permission state for this runtime context.
///
/// # Platform
/// Unix and Windows.
/// Uses host notification authorization APIs.
///
/// # Errors
/// Returns ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.notification.permission`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_os_notification_permission_state(
    context: &BindingCallContext,
    out: *mut NotificationPermissionState,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.notification.permissionState",
    ))
    .boxed())
}

/// Post host notification.
///
/// Submit one host notification request and return one host notification identifier.
///
/// # Platform
/// Unix and Windows.
/// Uses host notification center APIs on each platform.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `os.notification.post`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_notification_post(
    context: &BindingCallContext,
    out: *mut NativeStringRef,
    request: NotificationRequest,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, request);

    Err(RuntimeError::from(PlatformError::not_supported("destack.os.notification.post")).boxed())
}

/// Request host notification permission.
///
/// Request notification permission through host authorization flow and return resulting permission state.
///
/// # Platform
/// Unix and Windows.
/// Uses host notification permission request APIs where supported.
///
/// # Errors
/// Returns ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `os.notification.permission`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_notification_request_permission(
    context: &BindingCallContext,
    out: *mut NotificationPermissionState,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.notification.requestPermission",
    ))
    .boxed())
}

/// Open host settings for runtime permissions.
///
/// Request host navigation to the runtime permission settings page.
///
/// # Platform
/// Unix and Windows.
/// Uses host settings-intent APIs when available.
///
/// # Errors
/// Returns ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `os.permission.request`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_permission_open_settings(
    context: &BindingCallContext,
) -> RuntimeResult<()> {
    let _ = context;

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.permission.openSettings",
    ))
    .boxed())
}

/// Request one permission.
///
/// Request host authorization for one permission selector.
///
/// # Platform
/// Unix and Windows.
/// Uses host permission-request dialogs and policy APIs.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `os.permission.request`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_permission_request(
    context: &BindingCallContext,
    out: *mut PermissionState,
    permission: Permission,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, permission);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.permission.request",
    ))
    .boxed())
}

/// Read one permission state.
///
/// Read the current host permission state for one permission selector.
///
/// # Platform
/// Unix and Windows.
/// Uses host permission-state APIs on Android and Apple platforms, and host policy bridges on desktop platforms.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.permission.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_os_permission_state(
    context: &BindingCallContext,
    out: *mut PermissionState,
    permission: Permission,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out, permission);

    Err(RuntimeError::from(PlatformError::not_supported("destack.os.permission.state")).boxed())
}

/// Read current host power state.
///
/// Return one normalized host power-state classification.
/// State mapping follows runtime normalization over host power APIs.
///
/// # Platform
/// Unix and Windows.
/// Uses host power management APIs such as sysfs and IOKit on Unix-like systems and GetSystemPowerStatus on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.power`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_os_power_state(
    context: &BindingCallContext,
    out: *mut PowerState,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (context, out);

    Err(RuntimeError::from(PlatformError::not_supported("destack.os.power.state")).boxed())
}

/// Request host suspend.
///
/// Request one host suspend transition through platform power APIs.
/// Request acceptance and timing are host-policy and privilege dependent.
///
/// # Platform
/// Unix and Windows.
/// Uses host power-management APIs where supported.
///
/// # Errors
/// Returns ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `os.power`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_suspend(context: &BindingCallContext) -> RuntimeResult<()> {
    let _ = context;

    Err(RuntimeError::from(PlatformError::not_supported("destack.os.power.suspend")).boxed())
}
