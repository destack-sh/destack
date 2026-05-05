use std::mem::ManuallyDrop;

use destack_vm::{BindingContext, StringHandle};
use destack_workspace::RuntimeOptions;

use crate::diagnostic::RuntimeResult;
use crate::platform::abi::NativeStringRef;
use crate::platform::os::abi_generated::{
    NotificationCategoryValue, NotificationDeliveredEventValue, NotificationEventMetadataValue,
    NotificationEventOpenOptionsValue, NotificationEventValue, NotificationImmediateTriggerValue,
    NotificationRequestValue, NotificationScheduledDescriptorValue,
    NotificationTimeIntervalTriggerValue, NotificationTriggerValue,
};
use crate::platform::os::notification::with_notification_test_mode;
use crate::platform::os::tests::{HarnessContext, HarnessValue, with_configured_harness_context};
use crate::platform::os::{
    NotificationCategory, NotificationCategoryVm, NotificationEvent, NotificationEventVm,
    NotificationPriority, NotificationRequest, NotificationRequestVm,
    NotificationScheduledDescriptor, NotificationScheduledDescriptorVm,
};
use crate::platform::{NativeAbiCodec, NativeArray, VmAbiCodec, VmArray};

/// Enable one notification declaration for notification request tests.
pub(super) fn enable_notification_declaration(options: &mut RuntimeOptions) {
    options.app.notifications.enabled = true;
}

/// Run one notification harness pass with notification test mode enabled.
pub(super) fn with_notification_context<T>(
    mut callback: impl for<'call> FnMut(HarnessContext<'call>) -> RuntimeResult<T>,
) -> RuntimeResult<T> {
    let mut result = None;

    with_notification_test_mode(|| {
        with_configured_harness_context(enable_notification_declaration, |context| {
            result = Some(callback(context));
            Ok(())
        });
    });

    result.expect("notification harness should capture one result")
}

/// Build one notification event-open payload for the active harness.
pub(super) fn notification_event_open_options(
    context: &HarnessContext<'_>,
) -> HarnessValue<NotificationEventOpenOptionsValue, NotificationEventOpenOptionsValue> {
    notification_event_open_options_with_flags(context, true, false, false)
}

/// Build one notification event-open payload for the active harness.
pub(super) fn notification_event_open_options_with_flags(
    context: &HarnessContext<'_>,
    include_delivered: bool,
    include_interacted: bool,
    include_dismissed: bool,
) -> HarnessValue<NotificationEventOpenOptionsValue, NotificationEventOpenOptionsValue> {
    let options = NotificationEventOpenOptionsValue {
        include_delivered,
        include_interacted,
        include_dismissed,
    };

    if context.vm_context.is_some() {
        return context.harness_value_vm(options);
    }

    context.harness_value(options)
}

/// Build one notification request payload for the active harness.
pub(super) fn notification_request_harness_value(
    context: &mut HarnessContext<'_>,
    request: NotificationRequestValue,
) -> HarnessValue<NotificationRequest, NotificationRequestVm> {
    match context.vm_context {
        Some(vm_context) => {
            let vm_context = unsafe { &mut *(vm_context as *mut BindingContext<'_>) };
            let request = NotificationRequestVm::from_value(&mut vm_context.write(), request)
                .expect("request should encode");

            HarnessValue::Vm(request)
        }
        None => HarnessValue::Native(NotificationRequest::from_value(
            context.call_context,
            request,
        )),
    }
}

/// Build one notification category array payload for the active harness.
pub(super) fn categories_harness_value(
    context: &mut HarnessContext<'_>,
    categories: &[NotificationCategoryValue],
) -> RuntimeResult<HarnessValue<NativeArray<NotificationCategory>, VmArray<NotificationCategoryVm>>>
{
    match context.vm_context {
        Some(vm_context) => {
            let vm_context = unsafe { &mut *(vm_context as *mut BindingContext<'_>) };
            let categories = categories
                .iter()
                .cloned()
                .map(|category| {
                    NotificationCategoryVm::from_value(&mut vm_context.write(), category)
                })
                .collect::<RuntimeResult<Vec<_>>>()?;
            let categories = VmArray::from_values(&mut vm_context.write(), &categories)?;

            Ok(HarnessValue::Vm(categories))
        }
        None => {
            let categories = categories
                .iter()
                .cloned()
                .map(|category| NotificationCategory::from_value(context.call_context, category))
                .collect::<Vec<_>>();

            Ok(HarnessValue::Native(native_array(categories)))
        }
    }
}

/// Decode one notification identifier into one owned string.
pub(super) fn decode_notification_id(
    context: &mut HarnessContext<'_>,
    id: HarnessValue<NativeStringRef, StringHandle>,
) -> RuntimeResult<String> {
    match id {
        HarnessValue::Native(id) => Ok(unsafe { id.as_str() }?.to_string()),
        HarnessValue::Vm(id) => {
            let vm_context = unsafe {
                &mut *(context
                    .vm_context
                    .expect("vm notification id decode requires one vm context")
                    as *mut BindingContext<'_>)
            };

            Ok(vm_context
                .string_ref(id)
                .map_err(Box::from)?
                .as_str()
                .to_string())
        }
    }
}

/// Decode one notification category array into owned values.
pub(super) fn decode_notification_categories(
    context: &mut HarnessContext<'_>,
    categories: HarnessValue<NativeArray<NotificationCategory>, VmArray<NotificationCategoryVm>>,
) -> RuntimeResult<Vec<NotificationCategoryValue>> {
    match categories {
        HarnessValue::Native(categories) => {
            let categories = unsafe { categories.as_slice()? };
            let mut decoded = Vec::with_capacity(categories.len());

            for category in categories {
                decoded.push(unsafe { NotificationCategory::into_value(*category)? });
            }

            Ok(decoded)
        }
        HarnessValue::Vm(categories) => {
            let vm_context = unsafe {
                &mut *(context
                    .vm_context
                    .expect("vm notification category decode requires one vm context")
                    as *mut BindingContext<'_>)
            };

            let categories = categories.read_values(&vm_context.read())?;
            let mut decoded = Vec::with_capacity(categories.len());

            for category in categories {
                decoded.push(NotificationCategoryVm::into_value(
                    category,
                    &vm_context.read(),
                )?);
            }

            Ok(decoded)
        }
    }
}

/// Decode one pending notification descriptor array into owned values.
pub(super) fn decode_notification_pending_list(
    context: &mut HarnessContext<'_>,
    pending: HarnessValue<
        NativeArray<NotificationScheduledDescriptor>,
        VmArray<NotificationScheduledDescriptorVm>,
    >,
) -> RuntimeResult<Vec<NotificationScheduledDescriptorValue>> {
    match pending {
        HarnessValue::Native(pending) => {
            let pending = unsafe { pending.as_slice()? };
            let mut decoded = Vec::with_capacity(pending.len());

            for descriptor in pending {
                decoded.push(unsafe { NotificationScheduledDescriptor::into_value(*descriptor)? });
            }

            Ok(decoded)
        }
        HarnessValue::Vm(pending) => {
            let vm_context = unsafe {
                &mut *(context
                    .vm_context
                    .expect("vm notification pending decode requires one vm context")
                    as *mut BindingContext<'_>)
            };

            let pending = pending.read_values(&vm_context.read())?;
            let mut decoded = Vec::with_capacity(pending.len());

            for descriptor in pending {
                decoded.push(NotificationScheduledDescriptorVm::into_value(
                    descriptor,
                    &vm_context.read(),
                )?);
            }

            Ok(decoded)
        }
    }
}

/// Decode one notification event into one owned delivered-event payload.
pub(super) fn decode_notification_event(
    context: &mut HarnessContext<'_>,
    event: HarnessValue<NotificationEvent, NotificationEventVm>,
) -> RuntimeResult<NotificationDeliveredEventValue> {
    let event = decode_notification_event_value(context, event)?;

    match event {
        NotificationEventValue::NotificationDeliveredEvent(event) => Ok(event),
        NotificationEventValue::NotificationInteractedEvent(_) => {
            panic!("expected one delivered notification event")
        }
        NotificationEventValue::NotificationDismissedEvent(_) => {
            panic!("expected one delivered notification event")
        }
    }
}

/// Decode one notification event into one owned event payload.
pub(super) fn decode_notification_event_value(
    context: &mut HarnessContext<'_>,
    event: HarnessValue<NotificationEvent, NotificationEventVm>,
) -> RuntimeResult<NotificationEventValue> {
    Ok(match event {
        HarnessValue::Native(event) => unsafe { NotificationEvent::into_value(event)? },
        HarnessValue::Vm(event) => {
            let vm_context = unsafe {
                &mut *(context
                    .vm_context
                    .expect("vm notification event decode requires one vm context")
                    as *mut BindingContext<'_>)
            };

            NotificationEventVm::into_value(event, &vm_context.read())?
        }
    })
}

/// Build shared notification event metadata for one injected host event.
pub(super) fn notification_event_metadata(
    id: &str,
    request: NotificationRequestValue,
    sequence: u64,
) -> NotificationEventMetadataValue {
    NotificationEventMetadataValue {
        timestamp_ns: sequence,
        sequence,
        id: id.to_string(),
        request,
    }
}

/// Build one harness string payload for the active binding lane.
pub(super) fn string_harness_value(
    context: &mut HarnessContext<'_>,
    value: &str,
) -> HarnessValue<NativeStringRef, StringHandle> {
    match context.vm_context {
        Some(vm_context) => {
            let vm_context = unsafe { &mut *(vm_context as *mut BindingContext<'_>) };
            let value = StringHandle::new(
                vm_context
                    .intern_string(value)
                    .expect("vm notification test string should intern"),
            );

            HarnessValue::Vm(value)
        }
        None => HarnessValue::Native(context.call_context.store_string(value)),
    }
}

/// Build one leaked native array payload for one test input vector.
pub(super) fn native_array<T>(values: Vec<T>) -> NativeArray<T> {
    let mut values = ManuallyDrop::new(values);

    NativeArray {
        data: values.as_mut_ptr(),
        len: values.len() as u32,
        capacity: values.capacity() as u32,
    }
}

/// Build one immediate notification request for delivery tests.
pub(super) fn notification_post_request() -> NotificationRequestValue {
    NotificationRequestValue {
        title: "Welcome".to_string(),
        subtitle: None,
        body: "Hello".to_string(),
        tag: "welcome".to_string(),
        channel_id: None,
        priority: NotificationPriority::Normal,
        badge_count: None,
        sound: None,
        category_id: None,
        thread_id: None,
        trigger: NotificationTriggerValue::NotificationImmediateTrigger(
            NotificationImmediateTriggerValue {
                kind: "immediate".to_string(),
            },
        ),
        action_id: None,
    }
}

/// Build one scheduled notification request for pending-list tests.
pub(super) fn notification_schedule_request() -> NotificationRequestValue {
    NotificationRequestValue {
        title: "Sync".to_string(),
        subtitle: None,
        body: "Sync finished".to_string(),
        tag: "sync".to_string(),
        channel_id: None,
        priority: NotificationPriority::Normal,
        badge_count: None,
        sound: None,
        category_id: None,
        thread_id: None,
        trigger: NotificationTriggerValue::NotificationTimeIntervalTrigger(
            NotificationTimeIntervalTriggerValue {
                kind: "timeInterval".to_string(),
                interval_ns: 1_000_000_000,
            },
        ),
        action_id: None,
    }
}
