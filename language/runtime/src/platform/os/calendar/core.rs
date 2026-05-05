use destack_vm as vm;

use crate::diagnostic::RuntimeResult;
use crate::host::operation::calendar as host_calendar;
use crate::platform::os::abi_generated::{
    CalendarDescriptorValue, CalendarEventDraftValue, CalendarEventQueryValue, CalendarEventValue,
};
use crate::platform::os::{CalendarDescriptorVm, CalendarEventVm};
use crate::platform::{VmAbiCodec, VmArray};
use crate::runtime::BindingCallContext;

/// List host calendars through the active host session.
pub(crate) fn list(binding: &BindingCallContext) -> RuntimeResult<Vec<CalendarDescriptorValue>> {
    binding.host().submit_operation(host_calendar::list())
}

/// List host calendar events through the active host session.
pub(crate) fn event_list(
    binding: &BindingCallContext,
    query: CalendarEventQueryValue,
) -> RuntimeResult<Vec<CalendarEventValue>> {
    binding
        .host()
        .submit_operation(host_calendar::event_list(query))
}

/// Read one host calendar event through the active host session.
pub(crate) fn event_read(
    binding: &BindingCallContext,
    id: &str,
) -> RuntimeResult<CalendarEventValue> {
    binding
        .host()
        .submit_operation(host_calendar::event_read(id.to_string()))
}

/// Create one host calendar event through the active host session.
pub(crate) fn event_create(
    binding: &BindingCallContext,
    event: CalendarEventDraftValue,
) -> RuntimeResult<String> {
    binding
        .host()
        .submit_operation(host_calendar::event_create(event))
}

/// Update one host calendar event through the active host session.
pub(crate) fn event_update(
    binding: &BindingCallContext,
    id: &str,
    event: CalendarEventDraftValue,
) -> RuntimeResult<()> {
    binding
        .host()
        .submit_operation(host_calendar::event_update(id.to_string(), event))
}

/// Delete one host calendar event through the active host session.
pub(crate) fn event_delete(binding: &BindingCallContext, id: &str) -> RuntimeResult<()> {
    binding
        .host()
        .submit_operation(host_calendar::event_delete(id.to_string()))
}

/// Encode one calendar descriptor list into one VM array.
pub(crate) fn list_vm(
    context: &mut vm::BindingContext<'_>,
    descriptors: &[CalendarDescriptorValue],
) -> RuntimeResult<VmArray<CalendarDescriptorVm>> {
    let mut encoded_descriptors = Vec::with_capacity(descriptors.len());

    // encode one descriptor per host calendar
    for descriptor in descriptors {
        let encoded_descriptor =
            CalendarDescriptorVm::from_value(&mut context.write(), descriptor.clone())?;
        encoded_descriptors.push(encoded_descriptor);
    }

    VmArray::from_values(&mut context.write(), &encoded_descriptors)
}

/// Encode one calendar event list into one VM array.
pub(crate) fn event_list_vm(
    context: &mut vm::BindingContext<'_>,
    events: &[CalendarEventValue],
) -> RuntimeResult<VmArray<CalendarEventVm>> {
    let mut encoded_events = Vec::with_capacity(events.len());

    // encode one event per host calendar event
    for event in events {
        let encoded_event = CalendarEventVm::from_value(&mut context.write(), event.clone())?;
        encoded_events.push(encoded_event);
    }

    VmArray::from_values(&mut context.write(), &encoded_events)
}

/// Encode one calendar event into one VM value.
pub(crate) fn event_vm(
    context: &mut vm::BindingContext<'_>,
    event: CalendarEventValue,
) -> RuntimeResult<CalendarEventVm> {
    CalendarEventVm::from_value(&mut context.write(), event)
}

/// Encode one identifier into one VM string handle.
pub(crate) fn id_vm(
    context: &mut vm::BindingContext<'_>,
    id: &str,
) -> RuntimeResult<vm::StringHandle> {
    <vm::StringHandle as VmAbiCodec>::from_value(&mut context.write(), id.to_string())
}
