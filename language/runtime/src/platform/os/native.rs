#![allow(clippy::missing_safety_doc)]
#![cfg_attr(test, allow(dead_code))]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::{NativeSlice, NativeStringRef};
use crate::platform::core::{io_not_found, io_would_block};
use crate::platform::{NativeAbiCodec, NativeArray, PlatformError};

use crate::runtime::BindingCallContext;

use crate::platform::os::{
    BackgroundEvent, BackgroundEventOpenOptions, BackgroundStatus, BackgroundTaskDescriptor,
    BackgroundTaskOptions, BackgroundTaskResult, CalendarDescriptor, CalendarEvent,
    CalendarEventDraft, CalendarEventQuery, Contact, ContactDraft, ContactPage, ContactQuery,
    CredentialAuthenticationOptions, CredentialAuthenticationResult, CredentialQuery,
    CredentialRecord, CredentialWriteOptions, DocumentAccess, DocumentDescriptor,
    DocumentPickOptions, HostIdentity, IntentEvent, IntentOpenOptions, LifecycleEvent,
    LifecycleState, LoadAverage, LocationSample, LocationWatchOptions, MediaAssetDescriptor,
    MediaAssetKind, MediaPage, MediaQuery, MountEntry, NetworkEvent, NetworkState,
    NotificationCategory, NotificationEvent, NotificationEventOpenOptions,
    NotificationPermissionState, NotificationRequest, NotificationScheduledDescriptor, Permission,
    PermissionEntry, PermissionState, PowerState, SystemSnapshot,
};
use crate::platform::{fs, resource};

use crate::platform::os::{
    background, calendar, contact, credentials, document, host, info, intent, lifecycle, location,
    media, mount, network, notification, permission, power,
};

/// The label for one document pick transaction resource.
const DOCUMENT_PICK_RESOURCE_LABEL: &str = "os.document.pick";

/// The label for one notification permission transaction resource.
const NOTIFICATION_PERMISSION_REQUEST_RESOURCE_LABEL: &str = "os.notification.permission.request";

/// The label for one permission transaction resource.
const PERMISSION_REQUEST_RESOURCE_LABEL: &str = "os.permission.request";

/// The payload for one document pick transaction.
#[derive(Debug)]
struct DocumentPickRequestResource {
    /// The pending picker result.
    result: Option<Vec<DocumentDescriptor>>,
}

/// The payload for one notification permission transaction.
#[derive(Debug)]
struct NotificationPermissionRequestResource {
    /// The pending permission result.
    result: Option<NotificationPermissionState>,
}

/// The payload for one permission transaction.
#[derive(Debug)]
struct PermissionRequestResource {
    /// The pending permission entries.
    result: Option<Vec<PermissionEntry>>,
}

/// Resolve and consume one document pick result.
fn take_document_pick_result(
    binding: &BindingCallContext,
    handle: resource::DocumentPickHandle,
    operation: &'static str,
) -> RuntimeResult<Vec<DocumentDescriptor>> {
    let result = binding
        .worker()
        .resources
        .with_entry_mut(handle.0, |entry| {
            if entry.kind != resource::ResourceKind::DocumentPick {
                return None;
            }

            let request = entry.payload_mut::<DocumentPickRequestResource>()?;

            Some(request.result.take())
        });

    // validate the handle kind and payload first
    let Some(result) = result.flatten() else {
        return Err(io_not_found(operation, "unknown document pick handle"));
    };

    // then consume the one-shot transaction result
    let Some(result) = result else {
        return Err(io_would_block(
            operation,
            "document pick result has already been consumed",
        ));
    };

    Ok(result)
}

/// Resolve and consume one notification permission result.
fn take_notification_permission_result(
    binding: &BindingCallContext,
    handle: resource::NotificationPermissionRequestHandle,
    operation: &'static str,
) -> RuntimeResult<NotificationPermissionState> {
    let result = binding
        .worker()
        .resources
        .with_entry_mut(handle.0, |entry| {
            if entry.kind != resource::ResourceKind::NotificationPermissionRequest {
                return None;
            }

            let request = entry.payload_mut::<NotificationPermissionRequestResource>()?;

            Some(request.result.take())
        });

    // validate the handle kind and payload first
    let Some(result) = result.flatten() else {
        return Err(io_not_found(
            operation,
            "unknown notification permission request handle",
        ));
    };

    // then consume the one-shot transaction result
    let Some(result) = result else {
        return Err(io_would_block(
            operation,
            "notification permission result has already been consumed",
        ));
    };

    Ok(result)
}

/// Resolve and consume one permission request result.
fn take_permission_request_result(
    binding: &BindingCallContext,
    handle: resource::PermissionRequestHandle,
    operation: &'static str,
) -> RuntimeResult<Vec<PermissionEntry>> {
    let result = binding
        .worker()
        .resources
        .with_entry_mut(handle.0, |entry| {
            if entry.kind != resource::ResourceKind::PermissionRequest {
                return None;
            }

            let request = entry.payload_mut::<PermissionRequestResource>()?;

            Some(request.result.take())
        });

    // validate the handle kind and payload first
    let Some(result) = result.flatten() else {
        return Err(io_not_found(operation, "unknown permission request handle"));
    };

    // then consume the one-shot transaction result
    let Some(result) = result else {
        return Err(io_would_block(
            operation,
            "permission request result has already been consumed",
        ));
    };

    Ok(result)
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
    binding: &BindingCallContext,
    executionid: NativeStringRef,
    argument_result: BackgroundTaskResult,
) -> RuntimeResult<()> {
    let execution_id = unsafe { <NativeStringRef as NativeAbiCodec>::into_value(executionid)? };

    background::complete(binding, execution_id.as_str(), argument_result)
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
    binding: &BindingCallContext,
    handle: resource::BackgroundEventHandle,
) -> RuntimeResult<()> {
    background::event_close(binding, handle)
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
    binding: &BindingCallContext,
    out: *mut resource::BackgroundEventHandle,
    options: BackgroundEventOpenOptions,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let handle = background::event_open(binding, options)?;

    unsafe {
        out.write(handle);
    }

    Ok(())
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
    binding: &BindingCallContext,
    out: *mut BackgroundEvent,
    handle: resource::BackgroundEventHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let event = background::event_read(binding, handle, timeoutns)?;
    let event = <BackgroundEvent as NativeAbiCodec>::from_value(binding, event);

    unsafe {
        out.write(event);
    }

    Ok(())
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
    binding: &BindingCallContext,
    out: *mut BackgroundEvent,
    handle: resource::BackgroundEventHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let event = background::event_try_read(binding, handle)?;
    let event = <BackgroundEvent as NativeAbiCodec>::from_value(binding, event);

    unsafe {
        out.write(event);
    }

    Ok(())
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
    binding: &BindingCallContext,
    out: *mut NativeArray<BackgroundTaskDescriptor>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let descriptors = background::list(binding)?;
    let descriptors =
        <NativeArray<BackgroundTaskDescriptor> as NativeAbiCodec>::from_value(binding, descriptors);

    unsafe {
        out.write(descriptors);
    }

    Ok(())
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
    binding: &BindingCallContext,
    options: BackgroundTaskOptions,
) -> RuntimeResult<()> {
    let options = unsafe { <BackgroundTaskOptions as NativeAbiCodec>::into_value(options)? };

    background::register(binding, options)
}

/// Read background-task scheduler status.
///
/// Read the current background-task scheduler status for this host runtime.
///
/// # Platform
/// Unix and Windows.
/// Uses host scheduler bridges on supported desktop platforms.
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
    binding: &BindingCallContext,
    out: *mut BackgroundStatus,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let status = background::status(binding)?;
    let status = <BackgroundStatus as NativeAbiCodec>::from_value(binding, status);

    unsafe {
        out.write(status);
    }

    Ok(())
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
    binding: &BindingCallContext,
    out: *mut bool,
    identifier: NativeStringRef,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let identifier = unsafe { <NativeStringRef as NativeAbiCodec>::into_value(identifier)? };
    let triggered = background::trigger_test(binding, identifier.as_str())?;

    unsafe {
        out.write(triggered);
    }

    Ok(())
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
    binding: &BindingCallContext,
    identifier: NativeStringRef,
) -> RuntimeResult<()> {
    let identifier = unsafe { <NativeStringRef as NativeAbiCodec>::into_value(identifier)? };

    background::unregister(binding, identifier.as_str())
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
    binding: &BindingCallContext,
    out: *mut NativeStringRef,
    event: CalendarEventDraft,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let event = unsafe { <CalendarEventDraft as NativeAbiCodec>::into_value(event)? };
    let id = calendar::event_create(binding, event)?;
    let id = <NativeStringRef as NativeAbiCodec>::from_value(binding, id);

    unsafe {
        out.write(id);
    }

    Ok(())
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
    binding: &BindingCallContext,
    id: NativeStringRef,
) -> RuntimeResult<()> {
    let id = unsafe { <NativeStringRef as NativeAbiCodec>::into_value(id)? };

    calendar::event_delete(binding, id.as_str())
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
    binding: &BindingCallContext,
    out: *mut NativeArray<CalendarEvent>,
    query: CalendarEventQuery,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let query = unsafe { <CalendarEventQuery as NativeAbiCodec>::into_value(query)? };
    let events = calendar::event_list(binding, query)?;
    let events = <NativeArray<CalendarEvent> as NativeAbiCodec>::from_value(binding, events);

    unsafe {
        out.write(events);
    }

    Ok(())
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
    binding: &BindingCallContext,
    out: *mut CalendarEvent,
    id: NativeStringRef,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let id = unsafe { <NativeStringRef as NativeAbiCodec>::into_value(id)? };
    let event = calendar::event_read(binding, id.as_str())?;
    let event = <CalendarEvent as NativeAbiCodec>::from_value(binding, event);

    unsafe {
        out.write(event);
    }

    Ok(())
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
    binding: &BindingCallContext,
    id: NativeStringRef,
    event: CalendarEventDraft,
) -> RuntimeResult<()> {
    let id = unsafe { <NativeStringRef as NativeAbiCodec>::into_value(id)? };
    let event = unsafe { <CalendarEventDraft as NativeAbiCodec>::into_value(event)? };

    calendar::event_update(binding, id.as_str(), event)
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
    binding: &BindingCallContext,
    out: *mut NativeArray<CalendarDescriptor>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let descriptors = calendar::list(binding)?;
    let descriptors =
        <NativeArray<CalendarDescriptor> as NativeAbiCodec>::from_value(binding, descriptors);

    unsafe {
        out.write(descriptors);
    }

    Ok(())
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
    binding: &BindingCallContext,
    out: *mut NativeStringRef,
    contact: ContactDraft,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let contact = unsafe { <ContactDraft as NativeAbiCodec>::into_value(contact)? };
    let id = contact::create(binding, contact)?;
    let id = <NativeStringRef as NativeAbiCodec>::from_value(binding, id);

    unsafe {
        out.write(id);
    }

    Ok(())
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
    binding: &BindingCallContext,
    id: NativeStringRef,
) -> RuntimeResult<()> {
    let id = unsafe { <NativeStringRef as NativeAbiCodec>::into_value(id)? };

    contact::delete(binding, id.as_str())
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
    binding: &BindingCallContext,
    out: *mut ContactPage,
    query: ContactQuery,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let query = unsafe { <ContactQuery as NativeAbiCodec>::into_value(query)? };
    let page = contact::list(binding, query)?;
    let page = <ContactPage as NativeAbiCodec>::from_value(binding, page);

    unsafe {
        out.write(page);
    }

    Ok(())
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
    binding: &BindingCallContext,
    out: *mut Contact,
    id: NativeStringRef,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let id = unsafe { <NativeStringRef as NativeAbiCodec>::into_value(id)? };
    let contact = contact::read(binding, id.as_str())?;
    let contact = <Contact as NativeAbiCodec>::from_value(binding, contact);

    unsafe {
        out.write(contact);
    }

    Ok(())
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
    binding: &BindingCallContext,
    out: *mut ContactPage,
    querytext: NativeStringRef,
    query: ContactQuery,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let query_text = unsafe { <NativeStringRef as NativeAbiCodec>::into_value(querytext)? };
    let query = unsafe { <ContactQuery as NativeAbiCodec>::into_value(query)? };
    let page = contact::search(binding, query_text.as_str(), query)?;
    let page = <ContactPage as NativeAbiCodec>::from_value(binding, page);

    unsafe {
        out.write(page);
    }

    Ok(())
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
    binding: &BindingCallContext,
    id: NativeStringRef,
    contact: ContactDraft,
) -> RuntimeResult<()> {
    let id = unsafe { <NativeStringRef as NativeAbiCodec>::into_value(id)? };
    let contact = unsafe { <ContactDraft as NativeAbiCodec>::into_value(contact)? };

    contact::update(binding, id.as_str(), contact)
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
pub(crate) unsafe fn destack_os_credentials_authenticate(
    binding: &BindingCallContext,
    out: *mut CredentialAuthenticationResult,
    options: CredentialAuthenticationOptions,
) -> RuntimeResult<()> {
    unsafe { credentials::destack_os_credentials_authenticate_native(binding, out, options) }
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
pub(crate) unsafe fn destack_os_credentials_contains(
    binding: &BindingCallContext,
    out: *mut bool,
    service: NativeStringRef,
    account: NativeStringRef,
    accessgroup: Option<NativeStringRef>,
) -> RuntimeResult<()> {
    unsafe {
        credentials::destack_os_credentials_contains_native(
            binding,
            out,
            service,
            account,
            accessgroup,
        )
    }
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
pub(crate) unsafe fn destack_os_credentials_delete(
    binding: &BindingCallContext,
    service: NativeStringRef,
    account: NativeStringRef,
    accessgroup: Option<NativeStringRef>,
) -> RuntimeResult<()> {
    unsafe {
        credentials::destack_os_credentials_delete_native(binding, service, account, accessgroup)
    }
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
pub(crate) unsafe fn destack_os_credentials_read(
    binding: &BindingCallContext,
    out: *mut CredentialRecord,
    query: CredentialQuery,
) -> RuntimeResult<()> {
    unsafe { credentials::destack_os_credentials_read_native(binding, out, query) }
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
pub(crate) unsafe fn destack_os_credentials_write(
    binding: &BindingCallContext,
    options: CredentialWriteOptions,
) -> RuntimeResult<()> {
    unsafe { credentials::destack_os_credentials_write_native(binding, options) }
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
    binding: &BindingCallContext,
    handle: resource::DocumentHandle,
) -> RuntimeResult<()> {
    document::close(binding, handle)
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
    binding: &BindingCallContext,
    handle: resource::DocumentHandle,
) -> RuntimeResult<()> {
    document::flush(binding, handle)
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
    binding: &BindingCallContext,
    out: *mut resource::DocumentHandle,
    uri: NativeStringRef,
    access: DocumentAccess,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let uri = unsafe { uri.as_str()? };
    let value = document::open(binding, uri, access)?;

    unsafe {
        out.write(value);
    }

    Ok(())
}

/// Close one document-picker transaction.
///
/// Close one document-picker transaction handle and release host routing state.
///
/// # Platform
/// Unix and Windows.
/// Uses host picker callback unregistration and runtime resource cleanup.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `os.document.pick`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_document_pick_close(
    binding: &BindingCallContext,
    handle: resource::DocumentPickHandle,
) -> RuntimeResult<()> {
    // remove the one-shot transaction resource
    let removed = binding.worker().resources.remove_and_finalize(
        &binding.world(),
        handle.0,
        Some(binding.engine()),
    );

    if !removed {
        return Err(io_not_found(
            "destack.os.document.pickClose",
            "unknown document pick handle",
        ));
    }

    Ok(())
}

/// Open one document-picker transaction.
///
/// Start one host document-picker interaction and return one transaction handle.
///
/// # Platform
/// Unix and Windows.
/// Uses SAF or system picker APIs on Android, UIDocumentPicker on Apple platforms, common file dialogs on Windows, and desktop file-picker bridges or portals on Unix desktop hosts.
///
/// # Errors
/// Returns invalidArgument, ioPermissionDenied, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.document.pick`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_document_pick_open(
    binding: &BindingCallContext,
    out: *mut resource::DocumentPickHandle,
    options: DocumentPickOptions,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the picker result eagerly through the existing sync implementation
    let options = unsafe { DocumentPickOptions::into_value(options)? };
    let result = document::pick_native(binding, options)?;

    // store the result behind one transaction handle
    let entry = resource::ResourceEntry::new(resource::ResourceKind::DocumentPick)
        .with_label(DOCUMENT_PICK_RESOURCE_LABEL)
        .with_payload(DocumentPickRequestResource {
            result: Some(result),
        });
    let handle = binding
        .worker()
        .resources
        .insert(&binding.world(), entry, Some(binding.engine()));

    unsafe {
        out.write(resource::DocumentPickHandle(handle));
    }

    Ok(())
}

/// Wait for one document-picker result.
///
/// Wait for the completion of one earlier document-picker transaction.
///
/// # Platform
/// Unix and Windows.
/// Uses host picker completion queues and runtime transaction state.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInterrupted, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.document.pick`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_document_pick_read(
    binding: &BindingCallContext,
    out: *mut NativeArray<DocumentDescriptor>,
    handle: resource::DocumentPickHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // keep the generated timeout argument in the signature even though this shim resolves eagerly
    let _ = timeoutns;

    let result = take_document_pick_result(binding, handle, "destack.os.document.pickRead")?;
    let result = binding.store_array(result);

    unsafe {
        out.write(result);
    }

    Ok(())
}

/// Poll one document-picker result without blocking.
///
/// Poll the completion of one earlier document-picker transaction without waiting.
///
/// # Platform
/// Unix and Windows.
/// Uses nonblocking host picker completion queue reads.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.document.pick`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_document_pick_try_read(
    binding: &BindingCallContext,
    out: *mut NativeArray<DocumentDescriptor>,
    handle: resource::DocumentPickHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let result = take_document_pick_result(binding, handle, "destack.os.document.pickTryRead")?;
    let result = binding.store_array(result);

    unsafe {
        out.write(result);
    }

    Ok(())
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
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::DocumentHandle,
    maxbytes: u32,
    timeoutns: u64,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let value = document::read(binding, handle, maxbytes, timeoutns)?;
    let value = binding.store_slice(value);

    unsafe {
        out.write(value);
    }

    Ok(())
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
    binding: &BindingCallContext,
    out: *mut NativeSlice<u8>,
    handle: resource::DocumentHandle,
    maxbytes: u32,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let value = document::try_read(binding, handle, maxbytes)?;
    let value = binding.store_slice(value);

    unsafe {
        out.write(value);
    }

    Ok(())
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
    binding: &BindingCallContext,
    out: *mut u32,
    handle: resource::DocumentHandle,
    argument_bytes: NativeSlice<u8>,
    timeoutns: u64,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let bytes = unsafe { argument_bytes.as_slice()? };
    let written = document::write(binding, handle, bytes, timeoutns)?;

    unsafe {
        out.write(written);
    }

    Ok(())
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
    binding: &BindingCallContext,
    out: *mut HostIdentity,
) -> RuntimeResult<()> {
    unsafe { host::destack_os_host_identity(binding, out) }
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
    binding: &BindingCallContext,
    out: *mut u64,
) -> RuntimeResult<()> {
    unsafe { info::destack_os_boot_time_unix_ns(binding, out) }
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
    binding: &BindingCallContext,
    out: *mut LoadAverage,
) -> RuntimeResult<()> {
    unsafe { info::destack_os_load_average(binding, out) }
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
    binding: &BindingCallContext,
    out: *mut SystemSnapshot,
) -> RuntimeResult<()> {
    unsafe { info::destack_os_system_snapshot(binding, out) }
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
    binding: &BindingCallContext,
    out: *mut u64,
) -> RuntimeResult<()> {
    unsafe { info::destack_os_uptime_ns(binding, out) }
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
    binding: &BindingCallContext,
    out: *mut bool,
    url: NativeStringRef,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let url = unsafe { url.as_str()? };
    let value = intent::can_open_url(binding, url)?;

    unsafe {
        out.write(value);
    }

    Ok(())
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
    binding: &BindingCallContext,
    handle: resource::IntentHandle,
) -> RuntimeResult<()> {
    intent::close(binding, handle)
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
    binding: &BindingCallContext,
    out: *mut resource::IntentHandle,
    options: IntentOpenOptions,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let value = intent::open(binding, options)?;
    unsafe {
        *out = value;
    }

    Ok(())
}

/// Request host to open one file path target.
///
/// Ask the host platform or app framework to open one file path with default routing.
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
    binding: &BindingCallContext,
    path: fs::OsPath,
) -> RuntimeResult<()> {
    intent::open_path(binding, path)
}

/// Request host to open one URL target.
///
/// Ask the host platform or app framework to open one URL with default routing.
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
    binding: &BindingCallContext,
    url: NativeStringRef,
) -> RuntimeResult<()> {
    let url = unsafe { url.as_str()? };

    intent::open_url(binding, url)
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
    binding: &BindingCallContext,
    out: *mut IntentEvent,
    handle: resource::IntentHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let value = intent::read_value(binding, handle, timeoutns)?;
    unsafe {
        *out = <IntentEvent as NativeAbiCodec>::from_value(binding, value);
    }

    Ok(())
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
    binding: &BindingCallContext,
    paths: NativeArray<fs::OsPath>,
    mimetype: Option<NativeStringRef>,
) -> RuntimeResult<()> {
    let paths = unsafe { paths.as_slice() }?.to_vec();
    let mimetype = mimetype
        .map(|mimetype| unsafe { mimetype.as_str() })
        .transpose()?;

    intent::share_paths(binding, paths, mimetype)
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
    binding: &BindingCallContext,
    text: NativeStringRef,
    mimetype: Option<NativeStringRef>,
) -> RuntimeResult<()> {
    let text = unsafe { text.as_str()? };
    let mimetype = mimetype
        .map(|mimetype| unsafe { mimetype.as_str() })
        .transpose()?;

    intent::share_text(binding, text, mimetype)
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
    binding: &BindingCallContext,
    out: *mut IntentEvent,
    handle: resource::IntentHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let value = intent::try_read_value(binding, handle)?;
    unsafe {
        *out = <IntentEvent as NativeAbiCodec>::from_value(binding, value);
    }

    Ok(())
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
    binding: &BindingCallContext,
    handle: resource::LifecycleEventHandle,
) -> RuntimeResult<()> {
    lifecycle::close(binding, handle)
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
    binding: &BindingCallContext,
    out: *mut resource::LifecycleEventHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let handle = lifecycle::open(binding)?;

    unsafe {
        out.write(handle);
    }

    Ok(())
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
    binding: &BindingCallContext,
    out: *mut LifecycleEvent,
    handle: resource::LifecycleEventHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let event = lifecycle::read(binding, handle, timeoutns)?;
    let event = LifecycleEvent::from_value(binding, event);

    unsafe {
        out.write(event);
    }

    Ok(())
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
    binding: &BindingCallContext,
    out: *mut LifecycleState,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let state = lifecycle::state(binding)?;

    unsafe {
        out.write(state);
    }

    Ok(())
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
    binding: &BindingCallContext,
    out: *mut LifecycleEvent,
    handle: resource::LifecycleEventHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let event = lifecycle::try_read(binding, handle)?;
    let event = LifecycleEvent::from_value(binding, event);

    unsafe {
        out.write(event);
    }

    Ok(())
}

/// Read last known location sample.
///
/// Read the most recent location sample from the host location service.
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
    binding: &BindingCallContext,
    out: *mut LocationSample,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let sample = location::last_known(binding)?;

    unsafe {
        out.write(sample);
    }

    Ok(())
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
    binding: &BindingCallContext,
    out: *mut bool,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let is_enabled = location::services_enabled(binding)?;

    unsafe {
        out.write(is_enabled);
    }

    Ok(())
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
    binding: &BindingCallContext,
    handle: resource::LocationWatchHandle,
) -> RuntimeResult<()> {
    location::watch_close(binding, handle)
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
    binding: &BindingCallContext,
    out: *mut resource::LocationWatchHandle,
    options: LocationWatchOptions,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let handle = location::watch_open(binding, options)?;

    unsafe {
        out.write(handle);
    }

    Ok(())
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
    binding: &BindingCallContext,
    out: *mut LocationSample,
    handle: resource::LocationWatchHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let sample = location::watch_read(binding, handle, timeoutns)?;

    unsafe {
        out.write(sample);
    }

    Ok(())
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
    binding: &BindingCallContext,
    out: *mut LocationSample,
    handle: resource::LocationWatchHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let sample = location::watch_try_read(binding, handle)?;

    unsafe {
        out.write(sample);
    }

    Ok(())
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
    binding: &BindingCallContext,
    out: *mut u32,
    ids: NativeArray<NativeStringRef>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let ids = unsafe { ids.into_value()? };
    let deleted_count = media::delete(binding, ids)?;

    unsafe {
        out.write(deleted_count);
    }

    Ok(())
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
    binding: &BindingCallContext,
    out: *mut NativeStringRef,
    path: fs::OsPath,
    kind: MediaAssetKind,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let id = media::import_path(binding, path, kind)?;
    let id = <NativeStringRef as NativeAbiCodec>::from_value(binding, id);

    unsafe {
        out.write(id);
    }

    Ok(())
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
    binding: &BindingCallContext,
    out: *mut MediaPage,
    query: MediaQuery,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let query = unsafe { <MediaQuery as NativeAbiCodec>::into_value(query)? };
    let page = media::list(binding, query)?;
    let page = <MediaPage as NativeAbiCodec>::from_value(binding, page);

    unsafe {
        out.write(page);
    }

    Ok(())
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
    binding: &BindingCallContext,
    out: *mut MediaAssetDescriptor,
    id: NativeStringRef,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let id = unsafe { <NativeStringRef as NativeAbiCodec>::into_value(id)? };
    let descriptor = media::read(binding, id.as_str())?;
    let descriptor = <MediaAssetDescriptor as NativeAbiCodec>::from_value(binding, descriptor);

    unsafe {
        out.write(descriptor);
    }

    Ok(())
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
    binding: &BindingCallContext,
    out: *mut NativeArray<MountEntry>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let entries = mount::read_mount_entries(binding)?;

    unsafe { out.write(binding.store_array(entries)) };

    Ok(())
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
    binding: &BindingCallContext,
    out: *mut NetworkState,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    unsafe {
        *out = network::state(binding)?;
    }

    Ok(())
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
    binding: &BindingCallContext,
    handle: resource::NetworkWatchHandle,
) -> RuntimeResult<()> {
    network::watch_close(binding, handle)
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
    binding: &BindingCallContext,
    out: *mut resource::NetworkWatchHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    unsafe {
        *out = network::watch_open(binding)?;
    }

    Ok(())
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
    binding: &BindingCallContext,
    out: *mut NetworkEvent,
    handle: resource::NetworkWatchHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    unsafe {
        *out = network::watch_read(binding, handle, timeoutns)?;
    }

    Ok(())
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
    binding: &BindingCallContext,
    out: *mut NetworkEvent,
    handle: resource::NetworkWatchHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    unsafe {
        *out = network::watch_try_read(binding, handle)?;
    }

    Ok(())
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
    binding: &BindingCallContext,
    id: NativeStringRef,
) -> RuntimeResult<()> {
    let id = unsafe { id.as_str()? };

    notification::cancel(binding, id)
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
    binding: &BindingCallContext,
) -> RuntimeResult<()> {
    notification::cancel_all(binding)
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
    binding: &BindingCallContext,
    out: *mut NativeArray<NotificationCategory>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let values = notification::category_list(binding)?;
    let values = values
        .into_iter()
        .map(|value| NotificationCategory::from_value(binding, value))
        .collect::<Vec<_>>();
    let values = binding.store_array(values);

    unsafe {
        out.write(values);
    }

    Ok(())
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
    binding: &BindingCallContext,
    categories: NativeArray<NotificationCategory>,
) -> RuntimeResult<()> {
    let categories = unsafe { categories.into_value()? };

    notification::category_set(binding, categories)
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
    binding: &BindingCallContext,
    handle: resource::NotificationEventHandle,
) -> RuntimeResult<()> {
    notification::event_close(binding, handle)
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
    binding: &BindingCallContext,
    out: *mut resource::NotificationEventHandle,
    options: NotificationEventOpenOptions,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let options = unsafe { NotificationEventOpenOptions::into_value(options)? };
    let handle = notification::event_open(binding, options)?;

    unsafe {
        out.write(handle);
    }

    Ok(())
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
    binding: &BindingCallContext,
    out: *mut NotificationEvent,
    handle: resource::NotificationEventHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let event = notification::event_read(binding, handle, timeoutns)?;
    let event = NotificationEvent::from_value(binding, event);

    unsafe {
        out.write(event);
    }

    Ok(())
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
    binding: &BindingCallContext,
    out: *mut NotificationEvent,
    handle: resource::NotificationEventHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let event = notification::event_try_read(binding, handle)?;
    let event = NotificationEvent::from_value(binding, event);

    unsafe {
        out.write(event);
    }

    Ok(())
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
    binding: &BindingCallContext,
    id: NativeStringRef,
) -> RuntimeResult<()> {
    let id = unsafe { id.as_str()? };

    notification::pending_cancel(binding, id)
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
    binding: &BindingCallContext,
) -> RuntimeResult<()> {
    notification::pending_cancel_all(binding)
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
    binding: &BindingCallContext,
    out: *mut NativeArray<NotificationScheduledDescriptor>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let values = notification::pending_list(binding)?;
    let values = values
        .into_iter()
        .map(|value| NotificationScheduledDescriptor::from_value(binding, value))
        .collect::<Vec<_>>();
    let values = binding.store_array(values);

    unsafe {
        out.write(values);
    }

    Ok(())
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
    binding: &BindingCallContext,
    out: *mut NotificationPermissionState,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let value = notification::permission_state(binding)?;

    unsafe {
        out.write(value);
    }

    Ok(())
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
    binding: &BindingCallContext,
    out: *mut NativeStringRef,
    request: NotificationRequest,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let request = unsafe { NotificationRequest::into_value(request)? };
    let id = notification::post(binding, request)?;
    let id = binding.store_string(&id);

    unsafe {
        out.write(id);
    }

    Ok(())
}

/// Close one notification-permission request transaction.
///
/// Close one notification-permission request handle and release host routing state.
///
/// # Platform
/// Unix and Windows.
/// Uses host notification callback unregistration and runtime resource cleanup.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `os.notification.permission`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_notification_request_permission_close(
    binding: &BindingCallContext,
    handle: resource::NotificationPermissionRequestHandle,
) -> RuntimeResult<()> {
    // remove the one-shot transaction resource
    let removed = binding.worker().resources.remove_and_finalize(
        &binding.world(),
        handle.0,
        Some(binding.engine()),
    );

    if !removed {
        return Err(io_not_found(
            "destack.os.notification.requestPermissionClose",
            "unknown notification permission request handle",
        ));
    }

    Ok(())
}

/// Open one notification-permission request transaction.
///
/// Start one host notification authorization request and return one transaction handle.
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
pub(crate) unsafe fn destack_os_notification_request_permission_open(
    binding: &BindingCallContext,
    out: *mut resource::NotificationPermissionRequestHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the permission result eagerly through the existing sync implementation
    let result = notification::request_permission(binding)?;

    // store the result behind one transaction handle
    let entry = resource::ResourceEntry::new(resource::ResourceKind::NotificationPermissionRequest)
        .with_label(NOTIFICATION_PERMISSION_REQUEST_RESOURCE_LABEL)
        .with_payload(NotificationPermissionRequestResource {
            result: Some(result),
        });
    let handle = binding
        .worker()
        .resources
        .insert(&binding.world(), entry, Some(binding.engine()));

    unsafe {
        out.write(resource::NotificationPermissionRequestHandle(handle));
    }

    Ok(())
}

/// Wait for one notification-permission result.
///
/// Wait for the completion of one earlier notification-permission transaction.
///
/// # Platform
/// Unix and Windows.
/// Uses host notification permission completion queues and runtime transaction state.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInterrupted, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.notification.permission`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_notification_request_permission_read(
    binding: &BindingCallContext,
    out: *mut NotificationPermissionState,
    handle: resource::NotificationPermissionRequestHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // keep the generated timeout argument in the signature even though this shim resolves eagerly
    let _ = timeoutns;

    let result = take_notification_permission_result(
        binding,
        handle,
        "destack.os.notification.requestPermissionRead",
    )?;

    unsafe {
        out.write(result);
    }

    Ok(())
}

/// Poll one notification-permission result without blocking.
///
/// Poll the completion of one earlier notification-permission transaction without waiting.
///
/// # Platform
/// Unix and Windows.
/// Uses nonblocking host notification permission completion queue reads.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.notification.permission`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_notification_request_permission_try_read(
    binding: &BindingCallContext,
    out: *mut NotificationPermissionState,
    handle: resource::NotificationPermissionRequestHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let result = take_notification_permission_result(
        binding,
        handle,
        "destack.os.notification.requestPermissionTryRead",
    )?;

    unsafe {
        out.write(result);
    }

    Ok(())
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
    binding: &BindingCallContext,
    out: *mut NativeStringRef,
    request: NotificationRequest,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let request = unsafe { NotificationRequest::into_value(request)? };
    let id = notification::schedule(binding, request)?;
    let id = binding.store_string(&id);

    unsafe {
        out.write(id);
    }

    Ok(())
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
    binding: &BindingCallContext,
) -> RuntimeResult<()> {
    permission::open_settings(binding)
}

/// Close one permission-request transaction.
///
/// Close one permission-request transaction handle and release host routing state.
///
/// # Platform
/// Unix and Windows.
/// Uses host permission callback unregistration and runtime resource cleanup.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `os.permission.request`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_permission_request_close(
    binding: &BindingCallContext,
    handle: resource::PermissionRequestHandle,
) -> RuntimeResult<()> {
    // remove the one-shot transaction resource
    let removed = binding.worker().resources.remove_and_finalize(
        &binding.world(),
        handle.0,
        Some(binding.engine()),
    );

    if !removed {
        return Err(io_not_found(
            "destack.os.permission.requestClose",
            "unknown permission request handle",
        ));
    }

    Ok(())
}

/// Open one multi-permission request transaction.
///
/// Start one host authorization request for one permission selector list and return one transaction handle.
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
pub(crate) unsafe fn destack_os_permission_request_many_open(
    binding: &BindingCallContext,
    out: *mut resource::PermissionRequestHandle,
    permissions: NativeArray<Permission>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the permission result eagerly through the existing sync implementation
    let permissions = unsafe { permissions.as_slice()? };
    let permissions = permissions.to_vec();
    let result = permission::request_many(binding, permissions)?;

    // store the result behind one transaction handle
    let entry = resource::ResourceEntry::new(resource::ResourceKind::PermissionRequest)
        .with_label(PERMISSION_REQUEST_RESOURCE_LABEL)
        .with_payload(PermissionRequestResource {
            result: Some(result),
        });
    let handle = binding
        .worker()
        .resources
        .insert(&binding.world(), entry, Some(binding.engine()));

    unsafe {
        out.write(resource::PermissionRequestHandle(handle));
    }

    Ok(())
}

/// Open one permission-request transaction.
///
/// Start one host authorization request for one permission selector and return one transaction handle.
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
pub(crate) unsafe fn destack_os_permission_request_open(
    binding: &BindingCallContext,
    out: *mut resource::PermissionRequestHandle,
    permission: Permission,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve the permission result eagerly through the existing sync implementation
    let state = permission::request(binding, permission)?;
    let result = vec![PermissionEntry { permission, state }];

    // store the result behind one transaction handle
    let entry = resource::ResourceEntry::new(resource::ResourceKind::PermissionRequest)
        .with_label(PERMISSION_REQUEST_RESOURCE_LABEL)
        .with_payload(PermissionRequestResource {
            result: Some(result),
        });
    let handle = binding
        .worker()
        .resources
        .insert(&binding.world(), entry, Some(binding.engine()));

    unsafe {
        out.write(resource::PermissionRequestHandle(handle));
    }

    Ok(())
}

/// Wait for one permission-request result.
///
/// Wait for the completion of one earlier permission-request transaction.
///
/// # Platform
/// Unix and Windows.
/// Uses host permission completion queues and runtime transaction state.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInterrupted, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.permission.request`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_permission_request_read(
    binding: &BindingCallContext,
    out: *mut NativeArray<PermissionEntry>,
    handle: resource::PermissionRequestHandle,
    timeoutns: u64,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // keep the generated timeout argument in the signature even though this shim resolves eagerly
    let _ = timeoutns;

    let result =
        take_permission_request_result(binding, handle, "destack.os.permission.requestRead")?;
    let result = binding.store_array(result);

    unsafe {
        out.write(result);
    }

    Ok(())
}

/// Poll one permission-request result without blocking.
///
/// Poll the completion of one earlier permission-request transaction without waiting.
///
/// # Platform
/// Unix and Windows.
/// Uses nonblocking host permission completion queue reads.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.permission.request`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) unsafe fn destack_os_permission_request_try_read(
    binding: &BindingCallContext,
    out: *mut NativeArray<PermissionEntry>,
    handle: resource::PermissionRequestHandle,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    let result =
        take_permission_request_result(binding, handle, "destack.os.permission.requestTryRead")?;
    let result = binding.store_array(result);

    unsafe {
        out.write(result);
    }

    Ok(())
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
    binding: &BindingCallContext,
    out: *mut PermissionState,
    permission: Permission,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let value = permission::state(binding, permission)?;

    unsafe { out.write(value) };

    Ok(())
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
    binding: &BindingCallContext,
    out: *mut NativeArray<PermissionEntry>,
    permissions: NativeArray<Permission>,
) -> RuntimeResult<()> {
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let permissions = unsafe { permissions.as_slice()? };
    let values = permission::state_many(binding, permissions)?;
    let values = binding.store_array(values);

    unsafe { out.write(values) };

    Ok(())
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
    binding: &BindingCallContext,
    out: *mut PowerState,
) -> RuntimeResult<()> {
    unsafe { power::destack_os_power_state(binding, out) }
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
pub(crate) unsafe fn destack_os_suspend(binding: &BindingCallContext) -> RuntimeResult<()> {
    unsafe { power::destack_os_suspend(binding) }
}
