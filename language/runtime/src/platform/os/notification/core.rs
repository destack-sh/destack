use crate::diagnostic::RuntimeResult;
use crate::platform::os::abi_generated::{
    NotificationCategoryValue, NotificationEventOpenOptionsValue, NotificationEventValue,
    NotificationRequestValue, NotificationScheduledDescriptorValue,
};
use crate::platform::os::{
    NotificationCategoryVm, NotificationEventVm, NotificationPermissionState,
    NotificationScheduledDescriptorVm, permission, state,
};
use crate::platform::{VmAbiCodec, VmArray, resource};
use crate::runtime::BindingCallContext;
use destack_vm as vm;

/// Cancel one posted notification.
pub(crate) fn cancel(binding: &BindingCallContext, id: &str) -> RuntimeResult<()> {
    state::notification_cancel(binding, id.to_string())
}

/// Cancel every posted notification for this runtime.
pub(crate) fn cancel_all(binding: &BindingCallContext) -> RuntimeResult<()> {
    state::notification_cancel_all(binding)
}

/// List registered notification categories.
pub(crate) fn category_list(
    binding: &BindingCallContext,
) -> RuntimeResult<Vec<NotificationCategoryValue>> {
    state::notification_category_list(binding)
}

/// Register notification categories.
pub(crate) fn category_set(
    binding: &BindingCallContext,
    categories: Vec<NotificationCategoryValue>,
) -> RuntimeResult<()> {
    state::notification_category_set(binding, categories)
}

/// Close one notification event stream.
pub(crate) fn event_close(
    binding: &BindingCallContext,
    handle: resource::NotificationEventHandle,
) -> RuntimeResult<()> {
    state::notification_event_close(binding, handle)
}

/// Open one notification event stream.
pub(crate) fn event_open(
    binding: &BindingCallContext,
    options: NotificationEventOpenOptionsValue,
) -> RuntimeResult<resource::NotificationEventHandle> {
    state::notification_event_open(binding, options)
}

/// Wait for one notification event.
pub(crate) fn event_read(
    binding: &BindingCallContext,
    handle: resource::NotificationEventHandle,
    timeout_ns: u64,
) -> RuntimeResult<NotificationEventValue> {
    state::notification_event_read(binding, handle, timeout_ns)
}

/// Poll one notification event without blocking.
pub(crate) fn event_try_read(
    binding: &BindingCallContext,
    handle: resource::NotificationEventHandle,
) -> RuntimeResult<NotificationEventValue> {
    state::notification_event_try_read(binding, handle)
}

/// Cancel one scheduled notification.
pub(crate) fn pending_cancel(binding: &BindingCallContext, id: &str) -> RuntimeResult<()> {
    state::notification_pending_cancel(binding, id.to_string())
}

/// Cancel every scheduled notification for this runtime.
pub(crate) fn pending_cancel_all(binding: &BindingCallContext) -> RuntimeResult<()> {
    state::notification_pending_cancel_all(binding)
}

/// List scheduled notifications.
pub(crate) fn pending_list(
    binding: &BindingCallContext,
) -> RuntimeResult<Vec<NotificationScheduledDescriptorValue>> {
    state::notification_pending_list(binding)
}

/// Read the runtime-owned notification permission state.
pub(crate) fn permission_state(
    binding: &BindingCallContext,
) -> RuntimeResult<NotificationPermissionState> {
    permission::notification_state(binding)
}

/// Post one notification immediately.
pub(crate) fn post(
    binding: &BindingCallContext,
    request: NotificationRequestValue,
) -> RuntimeResult<String> {
    state::notification_post(binding, request)
}

/// Request host notification permission.
pub(crate) fn request_permission(
    binding: &BindingCallContext,
) -> RuntimeResult<NotificationPermissionState> {
    permission::request_notification(binding)
}

/// Schedule one notification for later delivery.
pub(crate) fn schedule(
    binding: &BindingCallContext,
    request: NotificationRequestValue,
) -> RuntimeResult<String> {
    state::notification_schedule(binding, request)
}

/// Encode registered categories into one VM array.
pub(crate) fn category_list_vm(
    context: &mut vm::BindingContext<'_>,
    categories: &[NotificationCategoryValue],
) -> RuntimeResult<VmArray<NotificationCategoryVm>> {
    let mut encoded_categories = Vec::with_capacity(categories.len());

    // encode one category per registered notification category
    for category in categories {
        let encoded_category =
            NotificationCategoryVm::from_value(&mut context.write(), category.clone())?;
        encoded_categories.push(encoded_category);
    }

    VmArray::from_values(&mut context.write(), &encoded_categories)
}

/// Encode scheduled descriptors into one VM array.
pub(crate) fn pending_list_vm(
    context: &mut vm::BindingContext<'_>,
    descriptors: &[NotificationScheduledDescriptorValue],
) -> RuntimeResult<VmArray<NotificationScheduledDescriptorVm>> {
    let mut encoded_descriptors = Vec::with_capacity(descriptors.len());

    // encode one descriptor per scheduled notification
    for descriptor in descriptors {
        let encoded_descriptor = NotificationScheduledDescriptorVm::from_value(
            &mut context.write(),
            descriptor.clone(),
        )?;
        encoded_descriptors.push(encoded_descriptor);
    }

    VmArray::from_values(&mut context.write(), &encoded_descriptors)
}

/// Encode one notification event into one VM value.
pub(crate) fn event_vm(
    context: &mut vm::BindingContext<'_>,
    event: NotificationEventValue,
) -> RuntimeResult<NotificationEventVm> {
    NotificationEventVm::from_value(&mut context.write(), event)
}

/// Encode one notification identifier into one VM handle.
pub(crate) fn id_vm(
    context: &mut vm::BindingContext<'_>,
    id: &str,
) -> RuntimeResult<vm::StringHandle> {
    <vm::StringHandle as VmAbiCodec>::from_value(&mut context.write(), id.to_string())
}
