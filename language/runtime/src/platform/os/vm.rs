#![allow(dead_code)]
#![allow(unused_imports)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::os::*;
use crate::platform::{PlatformError, VmArray, VmSlice, fs, resource};
use crate::runtime::{BindingCallContext, NativeStringRef};
use destack_vm as vm;

use super::credentials::{
    CredentialAuthenticationOptionsOwned, CredentialQueryOwned, CredentialWriteOptionsOwned,
    authenticate_credentials, contains_credentials, delete_credentials, normalize_optional_string,
    read_credentials, write_credentials,
};
use super::{host_impl as host_os_host, info as host_os_info, power as host_os_power};

/// Invoke one host call that writes through an out pointer.
fn call_out<T>(call: impl FnOnce(*mut T) -> RuntimeResult<()>) -> RuntimeResult<T> {
    // allocate one uninitialized output slot for the host call
    let mut out = std::mem::MaybeUninit::<T>::uninit();

    // execute call and assume initialization on success
    call(out.as_mut_ptr())?;
    Ok(unsafe { out.assume_init() })
}

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
pub(crate) fn destack_os_clipboard_clear(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.clipboard.clear is not available in the VM yet",
    ))
    .boxed())
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
pub(crate) fn destack_os_clipboard_has_text(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<bool> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.clipboard.hasText is not available in the VM yet",
    ))
    .boxed())
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
pub(crate) fn destack_os_intent_can_open_url(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    url: vm::StringHandle,
) -> RuntimeResult<bool> {
    let _ = url;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.intent.canOpenUrl is not available in the VM yet",
    ))
    .boxed())
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
pub(crate) fn destack_os_location_services_enabled(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<bool> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.location.servicesEnabled is not available in the VM yet",
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
pub(crate) fn destack_os_calendar_event_read(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    id: vm::StringHandle,
) -> RuntimeResult<CalendarEventVm> {
    let _ = id;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.calendar.eventRead is not available in the VM yet",
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
pub(crate) fn destack_os_notification_category_list(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<VmArray<NotificationCategoryVm>> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.notification.categoryList is not available in the VM yet",
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
pub(crate) fn destack_os_notification_category_set(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    categories: VmArray<NotificationCategoryVm>,
) -> RuntimeResult<()> {
    let _ = categories;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.notification.categorySet is not available in the VM yet",
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
pub(crate) fn destack_os_notification_pending_cancel(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    id: vm::StringHandle,
) -> RuntimeResult<()> {
    let _ = id;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.notification.pendingCancel is not available in the VM yet",
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
pub(crate) fn destack_os_notification_pending_cancel_all(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.notification.pendingCancelAll is not available in the VM yet",
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
pub(crate) fn destack_os_notification_pending_list(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<VmArray<NotificationScheduledDescriptorVm>> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.notification.pendingList is not available in the VM yet",
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
pub(crate) fn destack_os_notification_schedule(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    request: NotificationRequestVm,
) -> RuntimeResult<vm::StringHandle> {
    let _ = request;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.notification.schedule is not available in the VM yet",
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
pub(crate) fn destack_os_permission_request_many(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    permissions: VmArray<Permission>,
) -> RuntimeResult<VmArray<PermissionEntryVm>> {
    let _ = permissions;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.permission.requestMany is not available in the VM yet",
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
pub(crate) fn destack_os_permission_state_many(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    permissions: VmArray<Permission>,
) -> RuntimeResult<VmArray<PermissionEntryVm>> {
    let _ = permissions;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.permission.stateMany is not available in the VM yet",
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
pub(crate) fn destack_os_background_complete(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    executionid: vm::StringHandle,
    argument_result: BackgroundTaskResult,
) -> RuntimeResult<()> {
    let _ = (executionid, argument_result);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.background.complete is not available in the VM yet",
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
pub(crate) fn destack_os_background_event_close(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::BackgroundEventHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.background.event.close is not available in the VM yet",
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
pub(crate) fn destack_os_background_event_open(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    options: BackgroundEventOpenOptionsVm,
) -> RuntimeResult<resource::BackgroundEventHandle> {
    let _ = options;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.background.event.open is not available in the VM yet",
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
pub(crate) fn destack_os_background_event_read(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::BackgroundEventHandle,
    timeoutns: u64,
) -> RuntimeResult<BackgroundEventVm> {
    let _ = (handle, timeoutns);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.background.event.read is not available in the VM yet",
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
pub(crate) fn destack_os_background_event_try_read(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::BackgroundEventHandle,
) -> RuntimeResult<BackgroundEventVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.background.event.tryRead is not available in the VM yet",
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
pub(crate) fn destack_os_background_list(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<VmArray<BackgroundTaskDescriptorVm>> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.background.list is not available in the VM yet",
    ))
    .boxed())
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
pub(crate) fn destack_os_background_register(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    options: BackgroundTaskOptionsVm,
) -> RuntimeResult<()> {
    let _ = options;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.background.register is not available in the VM yet",
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
pub(crate) fn destack_os_background_status(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<BackgroundStatus> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.background.status is not available in the VM yet",
    ))
    .boxed())
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
pub(crate) fn destack_os_background_trigger_test(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    identifier: vm::StringHandle,
) -> RuntimeResult<bool> {
    let _ = identifier;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.background.triggerTest is not available in the VM yet",
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
pub(crate) fn destack_os_background_unregister(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    identifier: vm::StringHandle,
) -> RuntimeResult<()> {
    let _ = identifier;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.background.unregister is not available in the VM yet",
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
pub(crate) fn destack_os_calendar_event_create(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    event: CalendarEventDraftVm,
) -> RuntimeResult<vm::StringHandle> {
    let _ = event;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.calendar.eventCreate is not available in the VM yet",
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
pub(crate) fn destack_os_calendar_event_delete(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    id: vm::StringHandle,
) -> RuntimeResult<()> {
    let _ = id;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.calendar.eventDelete is not available in the VM yet",
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
pub(crate) fn destack_os_calendar_event_list(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    query: CalendarEventQueryVm,
) -> RuntimeResult<VmArray<CalendarEventVm>> {
    let _ = query;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.calendar.eventList is not available in the VM yet",
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
pub(crate) fn destack_os_calendar_event_update(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    id: vm::StringHandle,
    event: CalendarEventDraftVm,
) -> RuntimeResult<()> {
    let _ = (id, event);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.calendar.eventUpdate is not available in the VM yet",
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
pub(crate) fn destack_os_calendar_list(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<VmArray<CalendarDescriptorVm>> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.calendar.list is not available in the VM yet",
    ))
    .boxed())
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
pub(crate) fn destack_os_clipboard_read_bytes(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    format: ClipboardBinaryFormat,
) -> RuntimeResult<VmSlice<u8>> {
    let _ = format;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.clipboard.readBytes is not available in the VM yet",
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
pub(crate) fn destack_os_clipboard_read_text(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<vm::StringHandle> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.clipboard.readText is not available in the VM yet",
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
pub(crate) fn destack_os_clipboard_sequence(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<u64> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.clipboard.sequence is not available in the VM yet",
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
pub(crate) fn destack_os_clipboard_write_bytes(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    format: ClipboardBinaryFormat,
    argument_bytes: VmSlice<u8>,
) -> RuntimeResult<()> {
    let _ = (format, argument_bytes);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.clipboard.writeBytes is not available in the VM yet",
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
pub(crate) fn destack_os_clipboard_write_text(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    text: vm::StringHandle,
) -> RuntimeResult<()> {
    let _ = text;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.clipboard.writeText is not available in the VM yet",
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
pub(crate) fn destack_os_contact_create(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    contact: ContactDraftVm,
) -> RuntimeResult<vm::StringHandle> {
    let _ = contact;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.contact.create is not available in the VM yet",
    ))
    .boxed())
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
pub(crate) fn destack_os_contact_delete(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    id: vm::StringHandle,
) -> RuntimeResult<()> {
    let _ = id;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.contact.delete is not available in the VM yet",
    ))
    .boxed())
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
pub(crate) fn destack_os_contact_list(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    query: ContactQueryVm,
) -> RuntimeResult<ContactPageVm> {
    let _ = query;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.contact.list is not available in the VM yet",
    ))
    .boxed())
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
pub(crate) fn destack_os_contact_read(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    id: vm::StringHandle,
) -> RuntimeResult<ContactVm> {
    let _ = id;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.contact.read is not available in the VM yet",
    ))
    .boxed())
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
pub(crate) fn destack_os_contact_search(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    querytext: vm::StringHandle,
    query: ContactQueryVm,
) -> RuntimeResult<ContactPageVm> {
    let _ = (querytext, query);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.contact.search is not available in the VM yet",
    ))
    .boxed())
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
pub(crate) fn destack_os_contact_update(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    id: vm::StringHandle,
    contact: ContactDraftVm,
) -> RuntimeResult<()> {
    let _ = (id, contact);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.contact.update is not available in the VM yet",
    ))
    .boxed())
}

/// Request one host credential authentication challenge.
///
/// Request one host authentication challenge and return challenge outcome.
///
/// # Platform
/// Unix and Windows.
/// Uses keychain authentication prompts on Apple platforms, host callback bridge lanes on Android, and Windows CredUI prompt lanes for `BiometricOrDeviceCredential`.
/// Windows `Biometric` requests use Windows Biometric Framework lanes where available.
/// Windows `DeviceCredential` requests use CredUI prompt plus host logon verification lanes.
/// Linux and other unsupported Unix hosts return `notSupported`.
///
/// # Errors
/// Returns invalidArgumentValue, ioPermissionDenied, ioWouldBlock, ioInterrupted, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.credentials.auth`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_os_credentials_authenticate(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    options: CredentialAuthenticationOptionsVm,
) -> RuntimeResult<CredentialAuthenticationResultVm> {
    // decode vm authentication options into owned values
    let options = CredentialAuthenticationOptionsOwned {
        title: vm_string_to_owned(context, options.title, "options.title")?,
        subtitle: vm_string_to_owned(context, options.subtitle, "options.subtitle")?,
        message: vm_string_to_owned(context, options.message, "options.message")?,
        requirement: options.requirement,
    };

    // execute one authentication challenge
    authenticate_credentials(binding, &options)
}

/// Query credential presence.
///
/// Return whether one credential record exists for one service and account pair.
///
/// # Platform
/// Unix and Windows.
/// Uses host credential-query APIs.
/// Optional access-group routing is honored on Apple keychain backends and Android host callback backends.
/// Returns `notSupported` on backends without access-group lanes.
///
/// # Errors
/// Returns invalidArgumentValue, ioPermissionDenied, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.credentials.read`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_os_credentials_contains(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    service: vm::StringHandle,
    account: vm::StringHandle,
    access_group: vm::StringHandle,
) -> RuntimeResult<bool> {
    // decode vm service, account, and optional access-group values
    let service = vm_string_to_owned(context, service, "service")?;
    let account = vm_string_to_owned(context, account, "account")?;
    let access_group =
        normalize_optional_string(vm_string_to_owned(context, access_group, "accessGroup")?);

    // execute one contains query
    contains_credentials(binding, &service, &account, access_group.as_deref())
}

/// Delete one credential record.
///
/// Delete one secure credential payload from host credential store.
///
/// # Platform
/// Unix and Windows.
/// Uses host credential-delete APIs.
/// Optional access-group routing is honored on Apple keychain backends and Android host callback backends.
/// Returns `notSupported` on backends without access-group lanes.
///
/// # Errors
/// Returns invalidArgumentValue, ioNotFound, ioPermissionDenied, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `os.credentials.write`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_os_credentials_delete(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    service: vm::StringHandle,
    account: vm::StringHandle,
    access_group: vm::StringHandle,
) -> RuntimeResult<()> {
    // decode vm service, account, and optional access-group values
    let service = vm_string_to_owned(context, service, "service")?;
    let account = vm_string_to_owned(context, account, "account")?;
    let access_group =
        normalize_optional_string(vm_string_to_owned(context, access_group, "accessGroup")?);

    // execute one delete operation
    delete_credentials(binding, &service, &account, access_group.as_deref())
}

/// Read one credential record.
///
/// Read one secure credential payload from host credential store.
///
/// # Platform
/// Unix and Windows.
/// Uses Keychain on Apple platforms, host callback bridge lanes on Android, Windows Credential Manager, and Linux keyutils plus Secret Service credential stores where available.
///
/// # Errors
/// Returns invalidArgumentValue, ioNotFound, ioPermissionDenied, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.credentials.read`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_os_credentials_read(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    query: CredentialQueryVm,
) -> RuntimeResult<CredentialRecordVm> {
    // decode vm query payload into owned values
    let query = CredentialQueryOwned {
        service: vm_string_to_owned(context, query.service, "query.service")?,
        account: vm_string_to_owned(context, query.account, "query.account")?,
        access_group: normalize_optional_string(vm_string_to_owned(
            context,
            query.access_group,
            "query.accessGroup",
        )?),
        require_authentication: query.require_authentication,
    };

    // execute one read operation
    let record = read_credentials(binding, &query)?;

    // encode one vm record payload
    let service = vm::StringHandle::new(context.intern_string(&record.service));
    let account = vm::StringHandle::new(context.intern_string(&record.account));
    let bytes = VmSlice::from_bytes(context, &record.bytes);

    Ok(CredentialRecordVm {
        service,
        account,
        bytes,
        created_unix_ns: record.created_unix_ns,
        modified_unix_ns: record.modified_unix_ns,
    })
}

/// Write one credential record.
///
/// Create or replace one secure credential payload in host credential store.
///
/// # Platform
/// Unix and Windows.
/// Uses host credential-write APIs.
/// `replaceExisting=false` is strict within one runtime process and best effort across concurrent external writers.
///
/// # Errors
/// Returns invalidArgumentValue, ioAlreadyExists, ioPermissionDenied, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.credentials.write`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_os_credentials_write(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    options: CredentialWriteOptionsVm,
) -> RuntimeResult<()> {
    // decode vm write payload into owned values
    let options = CredentialWriteOptionsOwned {
        service: vm_string_to_owned(context, options.service, "options.service")?,
        account: vm_string_to_owned(context, options.account, "options.account")?,
        access_group: normalize_optional_string(vm_string_to_owned(
            context,
            options.access_group,
            "options.accessGroup",
        )?),
        bytes: vm_bytes_to_owned(context, options.bytes, "options.bytes")?,
        accessibility: options.accessibility,
        authentication: options.authentication,
        replace_existing: options.replace_existing,
    };

    // execute one write operation
    write_credentials(binding, &options)
}

/// Decode one vm string handle into owned text.
fn vm_string_to_owned(
    context: &mut vm::ExternalCallContext<'_>,
    value: vm::StringHandle,
    field: &str,
) -> RuntimeResult<String> {
    // resolve one vm string ref from the call context
    let value = context.string_ref(value).map_err(|error| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            field,
            format!("invalid vm string handle: {error}"),
        ))
        .boxed()
    })?;

    Ok(value.as_str().to_string())
}

/// Decode one vm byte slice into owned bytes.
fn vm_bytes_to_owned(
    context: &mut vm::ExternalCallContext<'_>,
    value: VmSlice<u8>,
    field: &str,
) -> RuntimeResult<Vec<u8>> {
    // copy one vm byte slice into owned memory
    let bytes = value.read_bytes(context).map_err(|error| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            field,
            format!("invalid vm byte slice: {error}"),
        ))
        .boxed()
    })?;

    Ok(bytes.to_vec())
}

/// Decode one native string reference into one vm string handle.
fn native_string_to_vm(
    context: &mut vm::ExternalCallContext<'_>,
    value: NativeStringRef,
    field: &str,
) -> RuntimeResult<vm::StringHandle> {
    // decode one native string and validate utf-8 payload
    let value = unsafe { value.as_str() }.map_err(|_| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            field,
            "native host string payload was not valid utf-8",
        ))
        .boxed()
    })?;

    // intern one vm string handle for the decoded payload
    Ok(vm::StringHandle::new(context.intern_string(value)))
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
pub(crate) fn destack_os_document_close(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::DocumentHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.document.close is not available in the VM yet",
    ))
    .boxed())
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
pub(crate) fn destack_os_document_flush(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::DocumentHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.document.flush is not available in the VM yet",
    ))
    .boxed())
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
pub(crate) fn destack_os_document_open(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    uri: vm::StringHandle,
    access: DocumentAccess,
) -> RuntimeResult<resource::DocumentHandle> {
    let _ = (uri, access);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.document.open is not available in the VM yet",
    ))
    .boxed())
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
pub(crate) fn destack_os_document_pick(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    options: DocumentPickOptionsVm,
) -> RuntimeResult<VmArray<DocumentDescriptorVm>> {
    let _ = options;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.document.pick is not available in the VM yet",
    ))
    .boxed())
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
pub(crate) fn destack_os_document_read(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::DocumentHandle,
    maxbytes: u32,
    timeoutns: u64,
) -> RuntimeResult<VmSlice<u8>> {
    let _ = (handle, maxbytes, timeoutns);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.document.read is not available in the VM yet",
    ))
    .boxed())
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
pub(crate) fn destack_os_document_try_read(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::DocumentHandle,
    maxbytes: u32,
) -> RuntimeResult<VmSlice<u8>> {
    let _ = (handle, maxbytes);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.document.tryRead is not available in the VM yet",
    ))
    .boxed())
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
pub(crate) fn destack_os_document_write(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::DocumentHandle,
    argument_bytes: VmSlice<u8>,
    timeoutns: u64,
) -> RuntimeResult<u32> {
    let _ = (handle, argument_bytes, timeoutns);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.document.write is not available in the VM yet",
    ))
    .boxed())
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
pub(crate) fn destack_os_host_identity(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<HostIdentityVm> {
    // forward to host-native implementation
    let identity = call_out(|out| unsafe { host_os_host::destack_os_host_identity(binding, out) })?;

    // decode native host identity strings into vm handles
    let hostname = native_string_to_vm(context, identity.hostname, "identity.hostname")?;
    let kernel = native_string_to_vm(context, identity.kernel, "identity.kernel")?;
    let release = native_string_to_vm(context, identity.release, "identity.release")?;
    let architecture =
        native_string_to_vm(context, identity.architecture, "identity.architecture")?;

    // return one vm host-identity payload
    Ok(HostIdentityVm {
        hostname,
        kernel,
        release,
        architecture,
    })
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
pub(crate) fn destack_os_boot_time_unix_ns(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<u64> {
    call_out(|out| unsafe { host_os_info::destack_os_boot_time_unix_ns(binding, out) })
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
pub(crate) fn destack_os_load_average(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<LoadAverageVm> {
    call_out(|out| unsafe { host_os_info::destack_os_load_average(binding, out) })
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
pub(crate) fn destack_os_system_snapshot(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<SystemSnapshotVm> {
    call_out(|out| unsafe { host_os_info::destack_os_system_snapshot(binding, out) })
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
pub(crate) fn destack_os_uptime_ns(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<u64> {
    call_out(|out| unsafe { host_os_info::destack_os_uptime_ns(binding, out) })
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
pub(crate) fn destack_os_intent_close(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::IntentHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.intent.close is not available in the VM yet",
    ))
    .boxed())
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
pub(crate) fn destack_os_intent_open(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    options: IntentOpenOptionsVm,
) -> RuntimeResult<resource::IntentHandle> {
    let _ = options;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.intent.open is not available in the VM yet",
    ))
    .boxed())
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
pub(crate) fn destack_os_intent_open_path(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    path: fs::OsPathVm,
) -> RuntimeResult<()> {
    let _ = path;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.intent.openPath is not available in the VM yet",
    ))
    .boxed())
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
pub(crate) fn destack_os_intent_open_url(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    url: vm::StringHandle,
) -> RuntimeResult<()> {
    let _ = url;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.intent.openUrl is not available in the VM yet",
    ))
    .boxed())
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
pub(crate) fn destack_os_intent_read(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::IntentHandle,
    timeoutns: u64,
) -> RuntimeResult<IntentEventVm> {
    let _ = (handle, timeoutns);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.intent.read is not available in the VM yet",
    ))
    .boxed())
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
pub(crate) fn destack_os_intent_share_paths(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    paths: VmArray<fs::OsPathVm>,
    mimetype: vm::StringHandle,
) -> RuntimeResult<()> {
    let _ = (paths, mimetype);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.intent.sharePaths is not available in the VM yet",
    ))
    .boxed())
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
pub(crate) fn destack_os_intent_share_text(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    text: vm::StringHandle,
    mimetype: vm::StringHandle,
) -> RuntimeResult<()> {
    let _ = (text, mimetype);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.intent.shareText is not available in the VM yet",
    ))
    .boxed())
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
pub(crate) fn destack_os_intent_try_read(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::IntentHandle,
) -> RuntimeResult<IntentEventVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.intent.tryRead is not available in the VM yet",
    ))
    .boxed())
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
pub(crate) fn destack_os_lifecycle_close(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::LifecycleEventHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.lifecycle.close is not available in the VM yet",
    ))
    .boxed())
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
pub(crate) fn destack_os_lifecycle_open(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<resource::LifecycleEventHandle> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.lifecycle.open is not available in the VM yet",
    ))
    .boxed())
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
pub(crate) fn destack_os_lifecycle_read(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::LifecycleEventHandle,
    timeoutns: u64,
) -> RuntimeResult<LifecycleEventVm> {
    let _ = (handle, timeoutns);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.lifecycle.read is not available in the VM yet",
    ))
    .boxed())
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
pub(crate) fn destack_os_lifecycle_state(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<LifecycleState> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.lifecycle.state is not available in the VM yet",
    ))
    .boxed())
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
pub(crate) fn destack_os_lifecycle_try_read(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::LifecycleEventHandle,
) -> RuntimeResult<LifecycleEventVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.lifecycle.tryRead is not available in the VM yet",
    ))
    .boxed())
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
pub(crate) fn destack_os_location_last_known(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<LocationSampleVm> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.location.lastKnown is not available in the VM yet",
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
pub(crate) fn destack_os_location_watch_close(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::LocationWatchHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.location.watchClose is not available in the VM yet",
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
pub(crate) fn destack_os_location_watch_open(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    options: LocationWatchOptionsVm,
) -> RuntimeResult<resource::LocationWatchHandle> {
    let _ = options;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.location.watchOpen is not available in the VM yet",
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
pub(crate) fn destack_os_location_watch_read(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::LocationWatchHandle,
    timeoutns: u64,
) -> RuntimeResult<LocationSampleVm> {
    let _ = (handle, timeoutns);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.location.watchRead is not available in the VM yet",
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
pub(crate) fn destack_os_location_watch_try_read(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::LocationWatchHandle,
) -> RuntimeResult<LocationSampleVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.location.watchTryRead is not available in the VM yet",
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
pub(crate) fn destack_os_media_delete(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    ids: VmArray<vm::StringHandle>,
) -> RuntimeResult<u32> {
    let _ = ids;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.media.delete is not available in the VM yet",
    ))
    .boxed())
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
pub(crate) fn destack_os_media_import_path(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    path: fs::OsPathVm,
    kind: MediaAssetKind,
) -> RuntimeResult<vm::StringHandle> {
    let _ = (path, kind);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.media.importPath is not available in the VM yet",
    ))
    .boxed())
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
pub(crate) fn destack_os_media_list(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    query: MediaQueryVm,
) -> RuntimeResult<MediaPageVm> {
    let _ = query;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.media.list is not available in the VM yet",
    ))
    .boxed())
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
pub(crate) fn destack_os_media_read(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    id: vm::StringHandle,
) -> RuntimeResult<MediaAssetDescriptorVm> {
    let _ = id;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.media.read is not available in the VM yet",
    ))
    .boxed())
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
pub(crate) fn destack_os_mount_add(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    source: vm::StringHandle,
    target: fs::OsPathVm,
    filesystem: vm::StringHandle,
    flags: u64,
    data: vm::StringHandle,
) -> RuntimeResult<()> {
    let _ = (source, target, filesystem, flags, data);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.mount.add is not available in the VM yet",
    ))
    .boxed())
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
pub(crate) fn destack_os_mount_list(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<VmArray<MountEntryVm>> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.mount.list is not available in the VM yet",
    ))
    .boxed())
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
pub(crate) fn destack_os_mount_remove(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    target: fs::OsPathVm,
    flags: u64,
) -> RuntimeResult<()> {
    let _ = (target, flags);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.mount.remove is not available in the VM yet",
    ))
    .boxed())
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
pub(crate) fn destack_os_network_state(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<NetworkStateVm> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.network.state is not available in the VM yet",
    ))
    .boxed())
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
pub(crate) fn destack_os_network_watch_close(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::NetworkWatchHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.network.watchClose is not available in the VM yet",
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
pub(crate) fn destack_os_network_watch_open(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<resource::NetworkWatchHandle> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.network.watchOpen is not available in the VM yet",
    ))
    .boxed())
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
pub(crate) fn destack_os_network_watch_read(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::NetworkWatchHandle,
    timeoutns: u64,
) -> RuntimeResult<NetworkEventVm> {
    let _ = (handle, timeoutns);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.network.watchRead is not available in the VM yet",
    ))
    .boxed())
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
pub(crate) fn destack_os_network_watch_try_read(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::NetworkWatchHandle,
) -> RuntimeResult<NetworkEventVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.network.watchTryRead is not available in the VM yet",
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
pub(crate) fn destack_os_notification_cancel(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    id: vm::StringHandle,
) -> RuntimeResult<()> {
    let _ = id;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.notification.cancel is not available in the VM yet",
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
pub(crate) fn destack_os_notification_cancel_all(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.notification.cancelAll is not available in the VM yet",
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
pub(crate) fn destack_os_notification_event_close(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::NotificationEventHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.notification.event.close is not available in the VM yet",
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
pub(crate) fn destack_os_notification_event_open(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    options: NotificationEventOpenOptionsVm,
) -> RuntimeResult<resource::NotificationEventHandle> {
    let _ = options;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.notification.event.open is not available in the VM yet",
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
pub(crate) fn destack_os_notification_event_read(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::NotificationEventHandle,
    timeoutns: u64,
) -> RuntimeResult<NotificationEventVm> {
    let _ = (handle, timeoutns);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.notification.event.read is not available in the VM yet",
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
pub(crate) fn destack_os_notification_event_try_read(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::NotificationEventHandle,
) -> RuntimeResult<NotificationEventVm> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.notification.event.tryRead is not available in the VM yet",
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
pub(crate) fn destack_os_notification_permission_state(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<NotificationPermissionState> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.notification.permissionState is not available in the VM yet",
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
pub(crate) fn destack_os_notification_post(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    request: NotificationRequestVm,
) -> RuntimeResult<vm::StringHandle> {
    let _ = request;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.notification.post is not available in the VM yet",
    ))
    .boxed())
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
pub(crate) fn destack_os_notification_request_permission(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<NotificationPermissionState> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.notification.requestPermission is not available in the VM yet",
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
pub(crate) fn destack_os_permission_open_settings(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.permission.openSettings is not available in the VM yet",
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
pub(crate) fn destack_os_permission_request(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    permission: Permission,
) -> RuntimeResult<PermissionState> {
    let _ = permission;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.permission.request is not available in the VM yet",
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
pub(crate) fn destack_os_permission_state(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    permission: Permission,
) -> RuntimeResult<PermissionState> {
    let _ = permission;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.permission.state is not available in the VM yet",
    ))
    .boxed())
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
pub(crate) fn destack_os_power_state(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<PowerState> {
    call_out(|out| unsafe { host_os_power::destack_os_power_state(binding, out) })
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
pub(crate) fn destack_os_suspend(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<()> {
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.os.power.suspend is not available in the VM yet",
    ))
    .boxed())
}
