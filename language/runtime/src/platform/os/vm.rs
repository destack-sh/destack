use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::core::store_os_path_from_vm;
use crate::platform::os::{
    BackgroundEventOpenOptionsVm, BackgroundEventVm, BackgroundStatus, BackgroundTaskDescriptorVm,
    BackgroundTaskOptionsVm, BackgroundTaskResult, CalendarDescriptorVm, CalendarEventDraftVm,
    CalendarEventQueryVm, CalendarEventVm, ClipboardBinaryFormat, ContactDraftVm, ContactPageVm,
    ContactQueryVm, ContactVm, CredentialAuthenticationOptionsVm, CredentialAuthenticationResultVm,
    CredentialQueryVm, CredentialRecordVm, CredentialWriteOptionsVm, DocumentAccess,
    DocumentAccessGrantVm, DocumentDescriptorVm, DocumentPickOptionsVm, HostIdentityVm,
    IntentEventVm, IntentOpenOptionsVm, LifecycleEventVm, LifecycleState, LoadAverageVm,
    LocationSampleVm, LocationWatchOptionsVm, MediaAssetDescriptorVm, MediaAssetKind, MediaEventVm,
    MediaPageVm, MediaQueryVm, MediaWatchOptionsVm, MountEntryVm, NetworkEventVm, NetworkStateVm,
    NotificationCategoryVm, NotificationEventOpenOptionsVm, NotificationEventVm,
    NotificationPermissionState, NotificationRequestVm, NotificationScheduledDescriptorVm,
    Permission, PermissionEntryVm, PermissionState, PowerState, SystemSnapshotVm,
};
use crate::platform::{VmAbiCodec, VmArray, VmSlice, fs, resource};
use crate::runtime::BindingCallContext;
use destack_vm as vm;

use crate::platform::os::{
    background, calendar, clipboard, contact, credentials, document, host, info, intent, lifecycle,
    location, media, mount, network, notification, permission, power,
};

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
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    executionid: vm::StringHandle,
    argument_result: BackgroundTaskResult,
) -> RuntimeResult<()> {
    let execution_id = <vm::StringHandle as VmAbiCodec>::into_value(executionid, context)?;

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
pub(crate) fn destack_os_background_event_close(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
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
pub(crate) fn destack_os_background_event_open(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    options: BackgroundEventOpenOptionsVm,
) -> RuntimeResult<resource::BackgroundEventHandle> {
    background::event_open(binding, options)
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
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::BackgroundEventHandle,
    timeoutns: u64,
) -> RuntimeResult<BackgroundEventVm> {
    let event = background::event_read(binding, handle, timeoutns)?;

    background::event_vm(context, event)
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
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::BackgroundEventHandle,
) -> RuntimeResult<BackgroundEventVm> {
    let event = background::event_try_read(binding, handle)?;

    background::event_vm(context, event)
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
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<VmArray<BackgroundTaskDescriptorVm>> {
    let descriptors = background::list(binding)?;

    background::list_vm(context, &descriptors)
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
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    options: BackgroundTaskOptionsVm,
) -> RuntimeResult<()> {
    let options = <BackgroundTaskOptionsVm as VmAbiCodec>::into_value(options, context)?;

    background::register(binding, options)
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
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<BackgroundStatus> {
    background::status(binding)
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
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    identifier: vm::StringHandle,
) -> RuntimeResult<bool> {
    let identifier = <vm::StringHandle as VmAbiCodec>::into_value(identifier, context)?;

    background::trigger_test(binding, identifier.as_str())
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
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    identifier: vm::StringHandle,
) -> RuntimeResult<()> {
    let identifier = <vm::StringHandle as VmAbiCodec>::into_value(identifier, context)?;

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
pub(crate) fn destack_os_calendar_event_create(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    event: CalendarEventDraftVm,
) -> RuntimeResult<vm::StringHandle> {
    let event = event.into_value(context)?;
    let id = calendar::event_create(binding, event)?;

    calendar::id_vm(context, &id)
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
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    id: vm::StringHandle,
) -> RuntimeResult<()> {
    let id = <vm::StringHandle as VmAbiCodec>::into_value(id, context)?;

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
pub(crate) fn destack_os_calendar_event_list(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    query: CalendarEventQueryVm,
) -> RuntimeResult<VmArray<CalendarEventVm>> {
    let query = query.into_value(context)?;
    let events = calendar::event_list(binding, query)?;

    calendar::event_list_vm(context, &events)
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
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    id: vm::StringHandle,
) -> RuntimeResult<CalendarEventVm> {
    let id = <vm::StringHandle as VmAbiCodec>::into_value(id, context)?;
    let event = calendar::event_read(binding, id.as_str())?;

    calendar::event_vm(context, event)
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
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    id: vm::StringHandle,
    event: CalendarEventDraftVm,
) -> RuntimeResult<()> {
    let id = <vm::StringHandle as VmAbiCodec>::into_value(id, context)?;
    let event = event.into_value(context)?;

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
pub(crate) fn destack_os_calendar_list(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<VmArray<CalendarDescriptorVm>> {
    let descriptors = calendar::list(binding)?;

    calendar::list_vm(context, &descriptors)
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
    clipboard::clear()
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
    clipboard::has_text()
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
    context: &mut vm::ExternalCallContext<'_>,
    format: ClipboardBinaryFormat,
) -> RuntimeResult<VmSlice<u8>> {
    let value = clipboard::read_bytes(format)?;

    VmSlice::from_bytes(context, &value)
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
    context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<vm::StringHandle> {
    let value = clipboard::read_text()?;

    Ok(vm::StringHandle::new(context.intern_string(&value)?))
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
    clipboard::sequence()
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
    context: &mut vm::ExternalCallContext<'_>,
    format: ClipboardBinaryFormat,
    argument_bytes: VmSlice<u8>,
) -> RuntimeResult<()> {
    let bytes = argument_bytes.read_bytes(context)?;

    clipboard::write_bytes(format, &bytes)
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
    context: &mut vm::ExternalCallContext<'_>,
    text: vm::StringHandle,
) -> RuntimeResult<()> {
    let text = context
        .string_ref(text)
        .map_err(|error| RuntimeError::from(error).boxed())?;

    clipboard::write_text(text.as_str())
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
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    contact: ContactDraftVm,
) -> RuntimeResult<vm::StringHandle> {
    let contact = contact.into_value(context)?;
    let id = contact::create(binding, contact)?;

    contact::id_vm(context, &id)
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
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    id: vm::StringHandle,
) -> RuntimeResult<()> {
    let id = <vm::StringHandle as VmAbiCodec>::into_value(id, context)?;

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
pub(crate) fn destack_os_contact_list(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    query: ContactQueryVm,
) -> RuntimeResult<ContactPageVm> {
    let query = query.into_value(context)?;
    let page = contact::list(binding, query)?;

    contact::page_vm(context, page)
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
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    id: vm::StringHandle,
) -> RuntimeResult<ContactVm> {
    let id = <vm::StringHandle as VmAbiCodec>::into_value(id, context)?;
    let contact = contact::read(binding, id.as_str())?;

    contact::contact_vm(context, contact)
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
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    querytext: vm::StringHandle,
    query: ContactQueryVm,
) -> RuntimeResult<ContactPageVm> {
    let query_text = <vm::StringHandle as VmAbiCodec>::into_value(querytext, context)?;
    let query = query.into_value(context)?;
    let page = contact::search(binding, query_text.as_str(), query)?;

    contact::page_vm(context, page)
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
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    id: vm::StringHandle,
    contact: ContactDraftVm,
) -> RuntimeResult<()> {
    let id = <vm::StringHandle as VmAbiCodec>::into_value(id, context)?;
    let contact = contact.into_value(context)?;

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
pub(crate) fn destack_os_credentials_authenticate(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    options: CredentialAuthenticationOptionsVm,
) -> RuntimeResult<CredentialAuthenticationResultVm> {
    credentials::destack_os_credentials_authenticate_vm(binding, context, options)
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
    accessgroup: Option<vm::StringHandle>,
) -> RuntimeResult<bool> {
    credentials::destack_os_credentials_contains_vm(binding, context, service, account, accessgroup)
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
    accessgroup: Option<vm::StringHandle>,
) -> RuntimeResult<()> {
    credentials::destack_os_credentials_delete_vm(binding, context, service, account, accessgroup)
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
    credentials::destack_os_credentials_read_vm(binding, context, query)
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
    credentials::destack_os_credentials_write_vm(binding, context, options)
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
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::DocumentHandle,
) -> RuntimeResult<()> {
    document::close(binding, handle)
}

/// List persisted document-access grants.
pub(crate) fn destack_os_document_access_list(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<VmArray<DocumentAccessGrantVm>> {
    document::access_list_vm(binding, context)
}

/// Open one persisted document-access grant.
pub(crate) fn destack_os_document_access_open(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    id: vm::StringHandle,
    access: DocumentAccess,
) -> RuntimeResult<resource::DocumentHandle> {
    let id = context
        .string_ref(id)
        .map_err(|error| RuntimeError::from(error).boxed())?;

    document::access_open(binding, id.as_str(), access)
}

/// Persist external document access for later reopen.
pub(crate) fn destack_os_document_access_persist(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    documents: VmArray<DocumentDescriptorVm>,
    access: DocumentAccess,
) -> RuntimeResult<VmArray<DocumentAccessGrantVm>> {
    let documents = documents.into_value(context)?;

    document::access_persist_vm(binding, context, documents, access)
}

/// Revoke persisted document-access grants.
pub(crate) fn destack_os_document_access_revoke(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    ids: VmArray<vm::StringHandle>,
) -> RuntimeResult<u32> {
    let ids = ids.into_value(context)?;

    document::access_revoke(binding, ids)
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
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::DocumentHandle,
) -> RuntimeResult<()> {
    document::flush(binding, handle)
}

/// Import selected documents into app-owned storage.
pub(crate) fn destack_os_document_import(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    documents: VmArray<DocumentDescriptorVm>,
) -> RuntimeResult<VmArray<DocumentDescriptorVm>> {
    let documents = documents.into_value(context)?;

    document::import_vm(binding, context, documents)
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
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    uri: vm::StringHandle,
    access: DocumentAccess,
) -> RuntimeResult<resource::DocumentHandle> {
    let uri = context
        .string_ref(uri)
        .map_err(|error| RuntimeError::from(error).boxed())?;

    document::open(binding, uri.as_str(), access)
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
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    options: DocumentPickOptionsVm,
) -> RuntimeResult<VmArray<DocumentDescriptorVm>> {
    let options = options.into_value(context)?;

    document::pick_vm(binding, context, options)
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
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::DocumentHandle,
    maxbytes: u32,
    timeoutns: u64,
) -> RuntimeResult<VmSlice<u8>> {
    let value = document::read(binding, handle, maxbytes, timeoutns)?;

    VmSlice::from_bytes(context, &value)
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
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::DocumentHandle,
    maxbytes: u32,
) -> RuntimeResult<VmSlice<u8>> {
    let value = document::try_read(binding, handle, maxbytes)?;

    VmSlice::from_bytes(context, &value)
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
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::DocumentHandle,
    argument_bytes: VmSlice<u8>,
    timeoutns: u64,
) -> RuntimeResult<u32> {
    let bytes = argument_bytes.read_bytes(context)?;

    document::write(binding, handle, &bytes, timeoutns)
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
    host::destack_os_host_identity_vm(binding, context)
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
    info::read_boot_time_unix_ns(binding)
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
    info::read_load_average(binding)
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
    info::read_system_snapshot(binding)
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
    info::read_uptime_ns(binding)
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
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    url: vm::StringHandle,
) -> RuntimeResult<bool> {
    let url = context
        .string_ref(url)
        .map_err(|error| RuntimeError::from(error).boxed())?;

    intent::can_open_url(binding, url.as_str())
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
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
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
pub(crate) fn destack_os_intent_open(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    options: IntentOpenOptionsVm,
) -> RuntimeResult<resource::IntentHandle> {
    intent::open(binding, options)
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
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    path: fs::OsPathVm,
) -> RuntimeResult<()> {
    let path = store_os_path_from_vm(binding, context, path)?;

    intent::open_path(binding, path)
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
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    url: vm::StringHandle,
) -> RuntimeResult<()> {
    let url = context
        .string_ref(url)
        .map_err(|error| RuntimeError::from(error).boxed())?;

    intent::open_url(binding, url.as_str())
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
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::IntentHandle,
    timeoutns: u64,
) -> RuntimeResult<IntentEventVm> {
    let value = intent::read_value(binding, handle, timeoutns)?;

    <IntentEventVm as VmAbiCodec>::from_value(context, value)
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
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    paths: VmArray<fs::OsPathVm>,
    mimetype: Option<vm::StringHandle>,
) -> RuntimeResult<()> {
    let paths = paths.read_values(context)?;
    let mut native_paths = Vec::with_capacity(paths.len());

    for path in paths {
        native_paths.push(store_os_path_from_vm(binding, context, path)?);
    }

    let mimetype = match mimetype {
        Some(mimetype) => Some(
            context
                .string_ref(mimetype)
                .map_err(|error| RuntimeError::from(error).boxed())?
                .as_str()
                .to_string(),
        ),
        None => None,
    };

    intent::share_paths(binding, native_paths, mimetype.as_deref())
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
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    text: vm::StringHandle,
    mimetype: Option<vm::StringHandle>,
) -> RuntimeResult<()> {
    let text = context
        .string_ref(text)
        .map_err(|error| RuntimeError::from(error).boxed())?;
    let mimetype = match mimetype {
        Some(mimetype) => Some(
            context
                .string_ref(mimetype)
                .map_err(|error| RuntimeError::from(error).boxed())?
                .as_str()
                .to_string(),
        ),
        None => None,
    };

    intent::share_text(binding, text.as_str(), mimetype.as_deref())
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
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::IntentHandle,
) -> RuntimeResult<IntentEventVm> {
    let value = intent::try_read_value(binding, handle)?;

    <IntentEventVm as VmAbiCodec>::from_value(context, value)
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
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
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
pub(crate) fn destack_os_lifecycle_open(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<resource::LifecycleEventHandle> {
    lifecycle::open(binding)
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
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::LifecycleEventHandle,
    timeoutns: u64,
) -> RuntimeResult<LifecycleEventVm> {
    let event = lifecycle::read(binding, handle, timeoutns)?;

    LifecycleEventVm::from_value(context, event)
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
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<LifecycleState> {
    lifecycle::state(binding)
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
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::LifecycleEventHandle,
) -> RuntimeResult<LifecycleEventVm> {
    let event = lifecycle::try_read(binding, handle)?;

    LifecycleEventVm::from_value(context, event)
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
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<LocationSampleVm> {
    let sample = location::last_known(binding)?;

    location::sample_vm(context, sample)
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
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<bool> {
    location::services_enabled(binding)
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
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
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
pub(crate) fn destack_os_location_watch_open(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    options: LocationWatchOptionsVm,
) -> RuntimeResult<resource::LocationWatchHandle> {
    let options = options.into_value(context)?;

    location::watch_open(binding, options)
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
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::LocationWatchHandle,
    timeoutns: u64,
) -> RuntimeResult<LocationSampleVm> {
    let sample = location::watch_read(binding, handle, timeoutns)?;

    location::sample_vm(context, sample)
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
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::LocationWatchHandle,
) -> RuntimeResult<LocationSampleVm> {
    let sample = location::watch_try_read(binding, handle)?;

    location::sample_vm(context, sample)
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
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    ids: VmArray<vm::StringHandle>,
) -> RuntimeResult<u32> {
    let ids = ids.into_value(context)?;

    media::delete(binding, ids)
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
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    path: fs::OsPathVm,
    kind: MediaAssetKind,
) -> RuntimeResult<vm::StringHandle> {
    let path = store_os_path_from_vm(binding, context, path)?;
    let id = media::import_path(binding, path, kind)?;

    media::id_vm(context, &id)
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
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    query: MediaQueryVm,
) -> RuntimeResult<MediaPageVm> {
    let query = query.into_value(context)?;
    let page = media::list(binding, query)?;

    media::page_vm(context, page)
}

/// Describe one media asset descriptor.
///
/// Read one richer host media asset descriptor by stable identifier.
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
pub(crate) fn destack_os_media_describe(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    id: vm::StringHandle,
) -> RuntimeResult<MediaAssetDescriptorVm> {
    let id = <vm::StringHandle as VmAbiCodec>::into_value(id, context)?;
    let descriptor = media::describe(binding, id.as_str())?;

    media::descriptor_vm(context, descriptor)
}

/// Close one media watch stream.
///
/// Close one opened media watch stream and release runtime watch resources.
///
/// # Platform
/// Unix and Windows.
/// Uses runtime media watch state.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `os.media.read`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_os_media_watch_close(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::MediaWatchHandle,
) -> RuntimeResult<()> {
    media::watch_close(binding, handle)
}

/// Open media watch stream.
///
/// Open one runtime-owned media watch stream and track add or update or remove events for one filtered asset set.
///
/// # Platform
/// Unix and Windows.
/// Uses host media-list queries and runtime watch state.
///
/// # Errors
/// Returns ioPermissionDenied, ioWouldBlock, ioInvalidData, notSupported.
///
/// # Security
/// Requires `os.media.read`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_os_media_watch_open(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    options: MediaWatchOptionsVm,
) -> RuntimeResult<resource::MediaWatchHandle> {
    let options = options.into_value(context)?;

    media::watch_open(binding, options)
}

/// Wait for one media watch event.
///
/// Wait for one queued media watch event from one opened watch stream.
///
/// # Platform
/// Unix and Windows.
/// Uses runtime media watch state.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, ioInterrupted, notSupported.
///
/// # Security
/// Requires `os.media.read`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_os_media_watch_read(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::MediaWatchHandle,
    timeoutns: u64,
) -> RuntimeResult<MediaEventVm> {
    let event = media::watch_read(binding, handle, timeoutns)?;

    media::event_vm(context, event)
}

/// Poll one media watch event without blocking.
///
/// Poll one queued media watch event from one opened watch stream without waiting.
///
/// # Platform
/// Unix and Windows.
/// Uses runtime media watch state.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `os.media.read`.
///
/// # Replay
/// External, recordable.
pub(crate) fn destack_os_media_watch_try_read(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::MediaWatchHandle,
) -> RuntimeResult<MediaEventVm> {
    let event = media::watch_try_read(binding, handle)?;

    media::event_vm(context, event)
}

/// Mount one filesystem target.
///
/// Mount one source on one target path with explicit flags and data.
/// Mount privilege checks and propagation policy are host-defined.
///
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
    context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<VmArray<MountEntryVm>> {
    mount::read_mount_entries_vm(context)
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
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<NetworkStateVm> {
    network::state_vm(binding, context)
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
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::NetworkWatchHandle,
) -> RuntimeResult<()> {
    network::watch_close_vm(binding, context, handle)
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
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<resource::NetworkWatchHandle> {
    network::watch_open_vm(binding, context)
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
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::NetworkWatchHandle,
    timeoutns: u64,
) -> RuntimeResult<NetworkEventVm> {
    network::watch_read_vm(binding, context, handle, timeoutns)
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
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::NetworkWatchHandle,
) -> RuntimeResult<NetworkEventVm> {
    network::watch_try_read_vm(binding, context, handle)
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
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    id: vm::StringHandle,
) -> RuntimeResult<()> {
    let id = context.string_ref(id)?;

    notification::cancel(binding, id.as_str())
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
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
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
pub(crate) fn destack_os_notification_category_list(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<VmArray<NotificationCategoryVm>> {
    let values = notification::category_list(binding)?;

    notification::category_list_vm(context, &values)
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
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    categories: VmArray<NotificationCategoryVm>,
) -> RuntimeResult<()> {
    let categories = categories.into_value(context)?;

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
pub(crate) fn destack_os_notification_event_close(
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
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
pub(crate) fn destack_os_notification_event_open(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    options: NotificationEventOpenOptionsVm,
) -> RuntimeResult<resource::NotificationEventHandle> {
    let options = options.into_value(context)?;

    notification::event_open(binding, options)
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
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::NotificationEventHandle,
    timeoutns: u64,
) -> RuntimeResult<NotificationEventVm> {
    let event = notification::event_read(binding, handle, timeoutns)?;

    notification::event_vm(context, event)
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
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    handle: resource::NotificationEventHandle,
) -> RuntimeResult<NotificationEventVm> {
    let event = notification::event_try_read(binding, handle)?;

    notification::event_vm(context, event)
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
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    id: vm::StringHandle,
) -> RuntimeResult<()> {
    let id = context.string_ref(id)?;

    notification::pending_cancel(binding, id.as_str())
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
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
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
pub(crate) fn destack_os_notification_pending_list(
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<VmArray<NotificationScheduledDescriptorVm>> {
    let values = notification::pending_list(binding)?;

    notification::pending_list_vm(context, &values)
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
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<NotificationPermissionState> {
    notification::permission_state(binding)
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
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    request: NotificationRequestVm,
) -> RuntimeResult<vm::StringHandle> {
    let request = request.into_value(context)?;
    let id = notification::post(binding, request)?;

    notification::id_vm(context, &id)
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
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<NotificationPermissionState> {
    notification::request_permission(binding)
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
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    request: NotificationRequestVm,
) -> RuntimeResult<vm::StringHandle> {
    let request = request.into_value(context)?;
    let id = notification::schedule(binding, request)?;

    notification::id_vm(context, &id)
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
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<()> {
    permission::open_settings(binding)
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
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    permission: Permission,
) -> RuntimeResult<PermissionState> {
    permission::request(binding, permission)
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
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    permissions: VmArray<Permission>,
) -> RuntimeResult<VmArray<PermissionEntryVm>> {
    let permissions = permissions.read_values(context)?;
    let values = permission::request_many(binding, permissions)?;

    VmArray::from_values(context, &values)
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
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    permission: Permission,
) -> RuntimeResult<PermissionState> {
    permission::state(binding, permission)
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
    binding: &BindingCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    permissions: VmArray<Permission>,
) -> RuntimeResult<VmArray<PermissionEntryVm>> {
    let permissions = permissions.read_values(context)?;
    let values = permission::state_many(binding, &permissions)?;

    VmArray::from_values(context, &values)
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
    power::read_power_state(binding)
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
    binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
) -> RuntimeResult<()> {
    power::suspend(binding)
}
