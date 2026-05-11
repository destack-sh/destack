#![cfg_attr(test, allow(dead_code))]

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::core::{io_not_found, io_would_block, store_os_path_from_vm};
use crate::platform::os::{
    BackgroundEventOpenOptionsVm, BackgroundEventVm, BackgroundStatus, BackgroundTaskDescriptorVm,
    BackgroundTaskOptionsVm, BackgroundTaskResult, CalendarDescriptorVm, CalendarEventDraftVm,
    CalendarEventQueryVm, CalendarEventVm, ContactDraftVm, ContactPageVm, ContactQueryVm,
    ContactVm, CredentialAuthenticationOptionsVm, CredentialAuthenticationResultVm,
    CredentialQueryVm, CredentialRecordVm, CredentialWriteOptionsVm, DocumentAccess,
    DocumentDescriptorVm, DocumentPickOptionsVm, HostIdentityVm, IntentEventVm,
    IntentOpenOptionsVm, LifecycleEventVm, LifecycleState, LoadAverageVm, LocationSampleVm,
    LocationWatchOptionsVm, MediaAssetDescriptorVm, MediaAssetKind, MediaPageVm, MediaQueryVm,
    MountEntryVm, NetworkEventVm, NetworkStateVm, NotificationCategoryVm,
    NotificationEventOpenOptionsVm, NotificationEventVm, NotificationPermissionState,
    NotificationRequestVm, NotificationScheduledDescriptorVm, Permission, PermissionEntryVm,
    PermissionState, PowerState, SystemSnapshotVm,
};
use crate::platform::{VmAbiCodec, VmArray, VmSlice, fs, resource};
use crate::runtime::BindingCallContext;
use destack_vm as vm;

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
    result: Option<Vec<DocumentDescriptorVm>>,
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
    result: Option<Vec<PermissionEntryVm>>,
}

/// Resolve and consume one document pick result.
fn take_document_pick_result(
    binding: &BindingCallContext,
    handle: resource::DocumentPickHandle,
    operation: &'static str,
) -> RuntimeResult<Vec<DocumentDescriptorVm>> {
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
) -> RuntimeResult<Vec<PermissionEntryVm>> {
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
pub(crate) fn destack_os_background_complete(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    executionid: vm::StringHandle,
    argument_result: BackgroundTaskResult,
) -> RuntimeResult<()> {
    let execution_id = <vm::StringHandle as VmAbiCodec>::into_value(executionid, &context.read())?;

    background::complete(binding, execution_id.as_str(), argument_result)
}

/// Close one background-task event stream.
pub(crate) fn destack_os_background_event_close(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::BackgroundEventHandle,
) -> RuntimeResult<()> {
    background::event_close(binding, handle)
}

/// Open one background-task event stream.
pub(crate) fn destack_os_background_event_open(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    options: BackgroundEventOpenOptionsVm,
) -> RuntimeResult<resource::BackgroundEventHandle> {
    background::event_open(binding, options)
}

/// Wait for one background-task event.
pub(crate) fn destack_os_background_event_read(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::BackgroundEventHandle,
    timeoutns: u64,
) -> RuntimeResult<BackgroundEventVm> {
    let event = background::event_read(binding, handle, timeoutns)?;

    background::event_vm(context, event)
}

/// Poll one background-task event without blocking.
pub(crate) fn destack_os_background_event_try_read(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::BackgroundEventHandle,
) -> RuntimeResult<BackgroundEventVm> {
    let event = background::event_try_read(binding, handle)?;

    background::event_vm(context, event)
}

/// List background-task registrations.
pub(crate) fn destack_os_background_list(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<VmArray<BackgroundTaskDescriptorVm>> {
    let descriptors = background::list(binding)?;

    background::list_vm(context, &descriptors)
}

/// Register one background task.
pub(crate) fn destack_os_background_register(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    options: BackgroundTaskOptionsVm,
) -> RuntimeResult<()> {
    let options = <BackgroundTaskOptionsVm as VmAbiCodec>::into_value(options, &context.read())?;

    background::register(binding, options)
}

/// Read background-task scheduler status.
pub(crate) fn destack_os_background_status(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<BackgroundStatus> {
    background::status(binding)
}

/// Trigger one background task for testing.
pub(crate) fn destack_os_background_trigger_test(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    identifier: vm::StringHandle,
) -> RuntimeResult<bool> {
    let identifier = <vm::StringHandle as VmAbiCodec>::into_value(identifier, &context.read())?;

    background::trigger_test(binding, identifier.as_str())
}

/// Unregister one background task.
pub(crate) fn destack_os_background_unregister(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    identifier: vm::StringHandle,
) -> RuntimeResult<()> {
    let identifier = <vm::StringHandle as VmAbiCodec>::into_value(identifier, &context.read())?;

    background::unregister(binding, identifier.as_str())
}

/// Create one calendar event.
pub(crate) fn destack_os_calendar_event_create(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    event: CalendarEventDraftVm,
) -> RuntimeResult<vm::StringHandle> {
    let event = event.into_value(&context.read())?;
    let id = calendar::event_create(binding, event)?;

    calendar::id_vm(context, &id)
}

/// Delete one calendar event.
pub(crate) fn destack_os_calendar_event_delete(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    id: vm::StringHandle,
) -> RuntimeResult<()> {
    let id = <vm::StringHandle as VmAbiCodec>::into_value(id, &context.read())?;

    calendar::event_delete(binding, id.as_str())
}

/// List host calendar events.
pub(crate) fn destack_os_calendar_event_list(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    query: CalendarEventQueryVm,
) -> RuntimeResult<VmArray<CalendarEventVm>> {
    let query = query.into_value(&context.read())?;
    let events = calendar::event_list(binding, query)?;

    calendar::event_list_vm(context, &events)
}

/// Read one host calendar event.
pub(crate) fn destack_os_calendar_event_read(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    id: vm::StringHandle,
) -> RuntimeResult<CalendarEventVm> {
    let id = <vm::StringHandle as VmAbiCodec>::into_value(id, &context.read())?;
    let event = calendar::event_read(binding, id.as_str())?;

    calendar::event_vm(context, event)
}

/// Update one calendar event.
pub(crate) fn destack_os_calendar_event_update(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    id: vm::StringHandle,
    event: CalendarEventDraftVm,
) -> RuntimeResult<()> {
    let id = <vm::StringHandle as VmAbiCodec>::into_value(id, &context.read())?;
    let event = event.into_value(&context.read())?;

    calendar::event_update(binding, id.as_str(), event)
}

/// List host calendars.
pub(crate) fn destack_os_calendar_list(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<VmArray<CalendarDescriptorVm>> {
    let descriptors = calendar::list(binding)?;

    calendar::list_vm(context, &descriptors)
}

/// Create one contact.
pub(crate) fn destack_os_contact_create(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    contact: ContactDraftVm,
) -> RuntimeResult<vm::StringHandle> {
    let contact = contact.into_value(&context.read())?;
    let id = contact::create(binding, contact)?;

    contact::id_vm(context, &id)
}

/// Delete one contact.
pub(crate) fn destack_os_contact_delete(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    id: vm::StringHandle,
) -> RuntimeResult<()> {
    let id = <vm::StringHandle as VmAbiCodec>::into_value(id, &context.read())?;

    contact::delete(binding, id.as_str())
}

/// List contacts.
pub(crate) fn destack_os_contact_list(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    query: ContactQueryVm,
) -> RuntimeResult<ContactPageVm> {
    let query = query.into_value(&context.read())?;
    let page = contact::list(binding, query)?;

    contact::page_vm(context, page)
}

/// Read one contact by identifier.
pub(crate) fn destack_os_contact_read(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    id: vm::StringHandle,
) -> RuntimeResult<ContactVm> {
    let id = <vm::StringHandle as VmAbiCodec>::into_value(id, &context.read())?;
    let contact = contact::read(binding, id.as_str())?;

    contact::contact_vm(context, contact)
}

/// Search contacts.
pub(crate) fn destack_os_contact_search(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    querytext: vm::StringHandle,
    query: ContactQueryVm,
) -> RuntimeResult<ContactPageVm> {
    let query_text = <vm::StringHandle as VmAbiCodec>::into_value(querytext, &context.read())?;
    let query = query.into_value(&context.read())?;
    let page = contact::search(binding, query_text.as_str(), query)?;

    contact::page_vm(context, page)
}

/// Update one contact.
pub(crate) fn destack_os_contact_update(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    id: vm::StringHandle,
    contact: ContactDraftVm,
) -> RuntimeResult<()> {
    let id = <vm::StringHandle as VmAbiCodec>::into_value(id, &context.read())?;
    let contact = contact.into_value(&context.read())?;

    contact::update(binding, id.as_str(), contact)
}

/// Request one host credential authentication challenge.
pub(crate) fn destack_os_credentials_authenticate(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    options: CredentialAuthenticationOptionsVm,
) -> RuntimeResult<CredentialAuthenticationResultVm> {
    credentials::destack_os_credentials_authenticate_vm(binding, context, options)
}

/// Query credential presence.
pub(crate) fn destack_os_credentials_contains(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    service: vm::StringHandle,
    account: vm::StringHandle,
    accessgroup: Option<vm::StringHandle>,
) -> RuntimeResult<bool> {
    credentials::destack_os_credentials_contains_vm(binding, context, service, account, accessgroup)
}

/// Delete one credential record.
pub(crate) fn destack_os_credentials_delete(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    service: vm::StringHandle,
    account: vm::StringHandle,
    accessgroup: Option<vm::StringHandle>,
) -> RuntimeResult<()> {
    credentials::destack_os_credentials_delete_vm(binding, context, service, account, accessgroup)
}

/// Read one credential record.
pub(crate) fn destack_os_credentials_read(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    query: CredentialQueryVm,
) -> RuntimeResult<CredentialRecordVm> {
    credentials::destack_os_credentials_read_vm(binding, context, query)
}

/// Write one credential record.
pub(crate) fn destack_os_credentials_write(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    options: CredentialWriteOptionsVm,
) -> RuntimeResult<()> {
    credentials::destack_os_credentials_write_vm(binding, context, options)
}

/// Close one opened document handle.
pub(crate) fn destack_os_document_close(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::DocumentHandle,
) -> RuntimeResult<()> {
    document::close(binding, handle)
}

/// Flush one opened document handle.
pub(crate) fn destack_os_document_flush(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::DocumentHandle,
) -> RuntimeResult<()> {
    document::flush(binding, handle)
}

/// Open one document URI.
pub(crate) fn destack_os_document_open(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    uri: vm::StringHandle,
    access: DocumentAccess,
) -> RuntimeResult<resource::DocumentHandle> {
    let uri = context
        .string_ref(uri)
        .map_err(|error| RuntimeError::from(error).boxed())?;

    document::open(binding, uri.as_str(), access)
}

/// Close one document-picker transaction.
pub(crate) fn destack_os_document_pick_close(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::DocumentPickHandle,
) -> RuntimeResult<()> {
    // remove the one-shot transaction resource
    let removed = binding.worker().resources.remove_and_finalize(
        binding.world(),
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
pub(crate) fn destack_os_document_pick_open(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    options: DocumentPickOptionsVm,
) -> RuntimeResult<resource::DocumentPickHandle> {
    // resolve the picker result eagerly through the existing sync implementation
    let options = options.into_value(&context.read())?;
    let result = document::pick_vm(binding, context, options)?;
    let result = result.read_values(&context.read())?;

    // store the result behind one transaction handle
    let entry = resource::ResourceEntry::new(resource::ResourceKind::DocumentPick)
        .with_label(DOCUMENT_PICK_RESOURCE_LABEL)
        .with_payload(DocumentPickRequestResource {
            result: Some(result),
        });
    let handle = binding
        .worker()
        .resources
        .insert(binding.world(), entry, Some(binding.engine()));

    Ok(resource::DocumentPickHandle(handle))
}

/// Wait for one document-picker result.
pub(crate) fn destack_os_document_pick_read(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::DocumentPickHandle,
    timeoutns: u64,
) -> RuntimeResult<VmArray<DocumentDescriptorVm>> {
    // keep the generated timeout argument in the signature even though this shim resolves eagerly
    let _ = timeoutns;

    let result = take_document_pick_result(binding, handle, "destack.os.document.pickRead")?;

    VmArray::from_values(&mut context.write(), &result)
}

/// Poll one document-picker result without blocking.
pub(crate) fn destack_os_document_pick_try_read(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::DocumentPickHandle,
) -> RuntimeResult<VmArray<DocumentDescriptorVm>> {
    let result = take_document_pick_result(binding, handle, "destack.os.document.pickTryRead")?;

    VmArray::from_values(&mut context.write(), &result)
}

/// Read one chunk of document bytes.
pub(crate) fn destack_os_document_read(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::DocumentHandle,
    maxbytes: u32,
    timeoutns: u64,
) -> RuntimeResult<VmSlice<u8>> {
    let value = document::read(binding, handle, maxbytes, timeoutns)?;

    VmSlice::from_bytes(&mut context.write(), &value)
}

/// Poll one chunk of document bytes without blocking.
pub(crate) fn destack_os_document_try_read(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::DocumentHandle,
    maxbytes: u32,
) -> RuntimeResult<VmSlice<u8>> {
    let value = document::try_read(binding, handle, maxbytes)?;

    VmSlice::from_bytes(&mut context.write(), &value)
}

/// Write one chunk of document bytes.
pub(crate) fn destack_os_document_write(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::DocumentHandle,
    argument_bytes: VmSlice<u8>,
    timeoutns: u64,
) -> RuntimeResult<u32> {
    let bytes = argument_bytes.read_bytes(&context.read())?;

    document::write(binding, handle, &bytes, timeoutns)
}

/// Read host identity.
pub(crate) fn destack_os_host_identity(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<HostIdentityVm> {
    host::destack_os_host_identity_vm(binding, context)
}

/// Read host boot time.
pub(crate) fn destack_os_boot_time_unix_ns(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<u64> {
    info::read_boot_time_unix_ns(binding)
}

/// Read host load averages.
pub(crate) fn destack_os_load_average(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<LoadAverageVm> {
    info::read_load_average(binding)
}

/// Read host system information.
pub(crate) fn destack_os_system_snapshot(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<SystemSnapshotVm> {
    info::read_system_snapshot(binding)
}

/// Read host uptime.
pub(crate) fn destack_os_uptime_ns(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<u64> {
    info::read_uptime_ns(binding)
}

/// Query whether host can route one URL target.
pub(crate) fn destack_os_intent_can_open_url(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    url: vm::StringHandle,
) -> RuntimeResult<bool> {
    let url = context
        .string_ref(url)
        .map_err(|error| RuntimeError::from(error).boxed())?;

    intent::can_open_url(binding, url.as_str())
}

/// Close one host intent stream.
pub(crate) fn destack_os_intent_close(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::IntentHandle,
) -> RuntimeResult<()> {
    intent::close(binding, handle)
}

/// Open one host intent stream.
pub(crate) fn destack_os_intent_open(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    options: IntentOpenOptionsVm,
) -> RuntimeResult<resource::IntentHandle> {
    intent::open(binding, options)
}

/// Request host to open one file path target.
pub(crate) fn destack_os_intent_open_path(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    path: fs::OsPathVm,
) -> RuntimeResult<()> {
    let path = store_os_path_from_vm(binding, context, path)?;

    intent::open_path(binding, path)
}

/// Request host to open one URL target.
pub(crate) fn destack_os_intent_open_url(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    url: vm::StringHandle,
) -> RuntimeResult<()> {
    let url = context
        .string_ref(url)
        .map_err(|error| RuntimeError::from(error).boxed())?;

    intent::open_url(binding, url.as_str())
}

/// Wait for one inbound intent event.
pub(crate) fn destack_os_intent_read(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::IntentHandle,
    timeoutns: u64,
) -> RuntimeResult<IntentEventVm> {
    let value = intent::read_value(binding, handle, timeoutns)?;

    <IntentEventVm as VmAbiCodec>::from_value(&mut context.write(), value)
}

/// Share file paths through host share routing.
pub(crate) fn destack_os_intent_share_paths(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    paths: VmArray<fs::OsPathVm>,
    mimetype: Option<vm::StringHandle>,
) -> RuntimeResult<()> {
    let paths = paths.read_values(&context.read())?;
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
pub(crate) fn destack_os_intent_share_text(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
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
pub(crate) fn destack_os_intent_try_read(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::IntentHandle,
) -> RuntimeResult<IntentEventVm> {
    let value = intent::try_read_value(binding, handle)?;

    <IntentEventVm as VmAbiCodec>::from_value(&mut context.write(), value)
}

/// Close lifecycle event stream.
pub(crate) fn destack_os_lifecycle_close(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::LifecycleEventHandle,
) -> RuntimeResult<()> {
    lifecycle::close(binding, handle)
}

/// Open lifecycle event stream.
pub(crate) fn destack_os_lifecycle_open(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<resource::LifecycleEventHandle> {
    lifecycle::open(binding)
}

/// Wait for one lifecycle event.
pub(crate) fn destack_os_lifecycle_read(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::LifecycleEventHandle,
    timeoutns: u64,
) -> RuntimeResult<LifecycleEventVm> {
    let event = lifecycle::read(binding, handle, timeoutns)?;

    LifecycleEventVm::from_value(&mut context.write(), event)
}

/// Read current lifecycle state.
pub(crate) fn destack_os_lifecycle_state(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<LifecycleState> {
    lifecycle::state(binding)
}

/// Poll one lifecycle event without blocking.
pub(crate) fn destack_os_lifecycle_try_read(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::LifecycleEventHandle,
) -> RuntimeResult<LifecycleEventVm> {
    let event = lifecycle::try_read(binding, handle)?;

    LifecycleEventVm::from_value(&mut context.write(), event)
}

/// Read last known location sample.
pub(crate) fn destack_os_location_last_known(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<LocationSampleVm> {
    let sample = location::last_known(binding)?;

    location::sample_vm(context, sample)
}

/// Read whether host location services are enabled.
pub(crate) fn destack_os_location_services_enabled(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<bool> {
    location::services_enabled(binding)
}

/// Close location watch stream.
pub(crate) fn destack_os_location_watch_close(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::LocationWatchHandle,
) -> RuntimeResult<()> {
    location::watch_close(binding, handle)
}

/// Open location watch stream.
pub(crate) fn destack_os_location_watch_open(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    options: LocationWatchOptionsVm,
) -> RuntimeResult<resource::LocationWatchHandle> {
    let options = options.into_value(&context.read())?;

    location::watch_open(binding, options)
}

/// Wait for one location sample.
pub(crate) fn destack_os_location_watch_read(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::LocationWatchHandle,
    timeoutns: u64,
) -> RuntimeResult<LocationSampleVm> {
    let sample = location::watch_read(binding, handle, timeoutns)?;

    location::sample_vm(context, sample)
}

/// Poll one location sample without blocking.
pub(crate) fn destack_os_location_watch_try_read(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::LocationWatchHandle,
) -> RuntimeResult<LocationSampleVm> {
    let sample = location::watch_try_read(binding, handle)?;

    location::sample_vm(context, sample)
}

/// Delete media assets by identifier.
pub(crate) fn destack_os_media_delete(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    ids: VmArray<vm::StringHandle>,
) -> RuntimeResult<u32> {
    let ids = ids.into_value(&context.read())?;

    media::delete(binding, ids)
}

/// Import one file path into host media library.
pub(crate) fn destack_os_media_import_path(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    path: fs::OsPathVm,
    kind: MediaAssetKind,
) -> RuntimeResult<vm::StringHandle> {
    let path = store_os_path_from_vm(binding, context, path)?;
    let id = media::import_path(binding, path, kind)?;

    media::id_vm(context, &id)
}

/// List media assets.
pub(crate) fn destack_os_media_list(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    query: MediaQueryVm,
) -> RuntimeResult<MediaPageVm> {
    let query = query.into_value(&context.read())?;
    let page = media::list(binding, query)?;

    media::page_vm(context, page)
}

/// Read one media asset descriptor.
pub(crate) fn destack_os_media_read(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    id: vm::StringHandle,
) -> RuntimeResult<MediaAssetDescriptorVm> {
    let id = <vm::StringHandle as VmAbiCodec>::into_value(id, &context.read())?;
    let descriptor = media::read(binding, id.as_str())?;

    media::descriptor_vm(context, descriptor)
}

/// Mount one filesystem target.
pub(crate) fn destack_os_mount_list(
    _binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<VmArray<MountEntryVm>> {
    mount::read_mount_entries_vm(context)
}
/// Read host network state.
pub(crate) fn destack_os_network_state(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<NetworkStateVm> {
    network::state_vm(binding, context)
}

/// Close host network watch stream.
pub(crate) fn destack_os_network_watch_close(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::NetworkWatchHandle,
) -> RuntimeResult<()> {
    network::watch_close_vm(binding, context, handle)
}

/// Open host network watch stream.
pub(crate) fn destack_os_network_watch_open(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<resource::NetworkWatchHandle> {
    network::watch_open_vm(binding, context)
}

/// Wait for one host network event.
pub(crate) fn destack_os_network_watch_read(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::NetworkWatchHandle,
    timeoutns: u64,
) -> RuntimeResult<NetworkEventVm> {
    network::watch_read_vm(binding, context, handle, timeoutns)
}

/// Poll one host network event without blocking.
pub(crate) fn destack_os_network_watch_try_read(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::NetworkWatchHandle,
) -> RuntimeResult<NetworkEventVm> {
    network::watch_try_read_vm(binding, context, handle)
}

/// Cancel host notification.
pub(crate) fn destack_os_notification_cancel(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    id: vm::StringHandle,
) -> RuntimeResult<()> {
    let id = context.string_ref(id)?;

    notification::cancel(binding, id.as_str())
}

/// Cancel all host notifications for this runtime context.
pub(crate) fn destack_os_notification_cancel_all(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<()> {
    notification::cancel_all(binding)
}

/// List notification categories.
pub(crate) fn destack_os_notification_category_list(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<VmArray<NotificationCategoryVm>> {
    let values = notification::category_list(binding)?;

    notification::category_list_vm(context, &values)
}

/// Register notification categories.
pub(crate) fn destack_os_notification_category_set(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    categories: VmArray<NotificationCategoryVm>,
) -> RuntimeResult<()> {
    let categories = categories.into_value(&context.read())?;

    notification::category_set(binding, categories)
}

/// Close one notification event stream.
pub(crate) fn destack_os_notification_event_close(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::NotificationEventHandle,
) -> RuntimeResult<()> {
    notification::event_close(binding, handle)
}

/// Open one notification event stream.
pub(crate) fn destack_os_notification_event_open(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    options: NotificationEventOpenOptionsVm,
) -> RuntimeResult<resource::NotificationEventHandle> {
    let options = options.into_value(&context.read())?;

    notification::event_open(binding, options)
}

/// Wait for one notification event.
pub(crate) fn destack_os_notification_event_read(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::NotificationEventHandle,
    timeoutns: u64,
) -> RuntimeResult<NotificationEventVm> {
    let event = notification::event_read(binding, handle, timeoutns)?;

    notification::event_vm(context, event)
}

/// Poll one notification event without blocking.
pub(crate) fn destack_os_notification_event_try_read(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::NotificationEventHandle,
) -> RuntimeResult<NotificationEventVm> {
    let event = notification::event_try_read(binding, handle)?;

    notification::event_vm(context, event)
}

/// Cancel pending scheduled notification.
pub(crate) fn destack_os_notification_pending_cancel(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    id: vm::StringHandle,
) -> RuntimeResult<()> {
    let id = context.string_ref(id)?;

    notification::pending_cancel(binding, id.as_str())
}

/// Cancel all pending scheduled notifications.
pub(crate) fn destack_os_notification_pending_cancel_all(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<()> {
    notification::pending_cancel_all(binding)
}

/// List pending scheduled notifications.
pub(crate) fn destack_os_notification_pending_list(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<VmArray<NotificationScheduledDescriptorVm>> {
    let values = notification::pending_list(binding)?;

    notification::pending_list_vm(context, &values)
}

/// Read host notification permission state.
pub(crate) fn destack_os_notification_permission_state(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<NotificationPermissionState> {
    notification::permission_state(binding)
}

/// Post host notification.
pub(crate) fn destack_os_notification_post(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    request: NotificationRequestVm,
) -> RuntimeResult<vm::StringHandle> {
    let request = request.into_value(&context.read())?;
    let id = notification::post(binding, request)?;

    notification::id_vm(context, &id)
}

/// Close one notification-permission request transaction.
pub(crate) fn destack_os_notification_request_permission_close(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::NotificationPermissionRequestHandle,
) -> RuntimeResult<()> {
    // remove the one-shot transaction resource
    let removed = binding.worker().resources.remove_and_finalize(
        binding.world(),
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
pub(crate) fn destack_os_notification_request_permission_open(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<resource::NotificationPermissionRequestHandle> {
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
        .insert(binding.world(), entry, Some(binding.engine()));

    Ok(resource::NotificationPermissionRequestHandle(handle))
}

/// Wait for one notification-permission result.
pub(crate) fn destack_os_notification_request_permission_read(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::NotificationPermissionRequestHandle,
    timeoutns: u64,
) -> RuntimeResult<NotificationPermissionState> {
    // keep the generated timeout argument in the signature even though this shim resolves eagerly
    let _ = timeoutns;

    take_notification_permission_result(
        binding,
        handle,
        "destack.os.notification.requestPermissionRead",
    )
}

/// Poll one notification-permission result without blocking.
pub(crate) fn destack_os_notification_request_permission_try_read(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::NotificationPermissionRequestHandle,
) -> RuntimeResult<NotificationPermissionState> {
    take_notification_permission_result(
        binding,
        handle,
        "destack.os.notification.requestPermissionTryRead",
    )
}

/// Schedule host notification.
pub(crate) fn destack_os_notification_schedule(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    request: NotificationRequestVm,
) -> RuntimeResult<vm::StringHandle> {
    let request = request.into_value(&context.read())?;
    let id = notification::schedule(binding, request)?;

    notification::id_vm(context, &id)
}

/// Open host settings for runtime permissions.
pub(crate) fn destack_os_permission_open_settings(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<()> {
    permission::open_settings(binding)
}

/// Close one permission-request transaction.
pub(crate) fn destack_os_permission_request_close(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    handle: resource::PermissionRequestHandle,
) -> RuntimeResult<()> {
    // remove the one-shot transaction resource
    let removed = binding.worker().resources.remove_and_finalize(
        binding.world(),
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
pub(crate) fn destack_os_permission_request_many_open(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    permissions: VmArray<Permission>,
) -> RuntimeResult<resource::PermissionRequestHandle> {
    // resolve the permission result eagerly through the existing sync implementation
    let permissions = permissions.read_values(&context.read())?;
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
        .insert(binding.world(), entry, Some(binding.engine()));

    Ok(resource::PermissionRequestHandle(handle))
}

/// Open one permission-request transaction.
pub(crate) fn destack_os_permission_request_open(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    permission: Permission,
) -> RuntimeResult<resource::PermissionRequestHandle> {
    // resolve the permission result eagerly through the existing sync implementation
    let state = permission::request(binding, permission)?;
    let result = vec![PermissionEntryVm { permission, state }];

    // store the result behind one transaction handle
    let entry = resource::ResourceEntry::new(resource::ResourceKind::PermissionRequest)
        .with_label(PERMISSION_REQUEST_RESOURCE_LABEL)
        .with_payload(PermissionRequestResource {
            result: Some(result),
        });
    let handle = binding
        .worker()
        .resources
        .insert(binding.world(), entry, Some(binding.engine()));

    Ok(resource::PermissionRequestHandle(handle))
}

/// Wait for one permission-request result.
pub(crate) fn destack_os_permission_request_read(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::PermissionRequestHandle,
    timeoutns: u64,
) -> RuntimeResult<VmArray<PermissionEntryVm>> {
    // keep the generated timeout argument in the signature even though this shim resolves eagerly
    let _ = timeoutns;

    let result =
        take_permission_request_result(binding, handle, "destack.os.permission.requestRead")?;

    VmArray::from_values(&mut context.write(), &result)
}

/// Poll one permission-request result without blocking.
pub(crate) fn destack_os_permission_request_try_read(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    handle: resource::PermissionRequestHandle,
) -> RuntimeResult<VmArray<PermissionEntryVm>> {
    let result =
        take_permission_request_result(binding, handle, "destack.os.permission.requestTryRead")?;

    VmArray::from_values(&mut context.write(), &result)
}

/// Read one permission state.
pub(crate) fn destack_os_permission_state(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
    permission: Permission,
) -> RuntimeResult<PermissionState> {
    permission::state(binding, permission)
}

/// Read permission states.
pub(crate) fn destack_os_permission_state_many(
    binding: &BindingCallContext,
    context: &mut vm::BindingContext<'_>,
    permissions: VmArray<Permission>,
) -> RuntimeResult<VmArray<PermissionEntryVm>> {
    let permissions = permissions.read_values(&context.read())?;
    let values = permission::state_many(binding, &permissions)?;

    VmArray::from_values(&mut context.write(), &values)
}

/// Read current host power state.
pub(crate) fn destack_os_power_state(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<PowerState> {
    power::read_power_state(binding)
}

/// Request host suspend.
pub(crate) fn destack_os_suspend(
    binding: &BindingCallContext,
    _context: &mut vm::BindingContext<'_>,
) -> RuntimeResult<()> {
    power::suspend(binding)
}
