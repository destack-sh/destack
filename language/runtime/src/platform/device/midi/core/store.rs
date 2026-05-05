use crate::diagnostic::RuntimeResult;
use crate::platform::abi::NativeSlice;
use crate::platform::device::{
    MidiBackendDescriptor, MidiBackendDescriptorVm, MidiEvent, MidiEventVm, MidiInputRecord,
    MidiInputRecordVm, MidiPortDescriptor, MidiPortDescriptorVm,
};
use crate::platform::{NativeArray, VmArray, VmSlice};
use crate::runtime::BindingCallContext;
use destack_vm as vm;

use super::value::{
    MidiBackendDescriptorValue, MidiEventValue, MidiInputRecordValue, MidiPortDescriptorValue,
};

/// Store native backend descriptors as one binding slice.
pub(crate) fn store_backend_descriptors_native(
    binding: &BindingCallContext,
    values: Vec<MidiBackendDescriptorValue>,
) -> NativeSlice<MidiBackendDescriptor> {
    let descriptors = values
        .into_iter()
        .map(|value| value.into_native(binding))
        .collect();

    binding.store_slice(descriptors)
}

/// Store VM backend descriptors as one binding slice.
pub(crate) fn store_backend_descriptors_vm(
    context: &mut vm::BindingContext<'_>,
    values: Vec<MidiBackendDescriptorValue>,
) -> RuntimeResult<VmSlice<MidiBackendDescriptorVm>> {
    let descriptors = values
        .into_iter()
        .map(|value| value.into_vm(context))
        .collect::<RuntimeResult<Vec<_>>>()?;

    VmSlice::from_values(&mut context.write(), &descriptors)
}

/// Store native port descriptors as one binding slice.
pub(crate) fn store_port_descriptors_native(
    binding: &BindingCallContext,
    values: Vec<MidiPortDescriptorValue>,
) -> NativeSlice<MidiPortDescriptor> {
    let descriptors = values
        .into_iter()
        .map(|value| value.into_native(binding))
        .collect();

    binding.store_slice(descriptors)
}

/// Store VM port descriptors as one binding slice.
pub(crate) fn store_port_descriptors_vm(
    context: &mut vm::BindingContext<'_>,
    values: Vec<MidiPortDescriptorValue>,
) -> RuntimeResult<VmSlice<MidiPortDescriptorVm>> {
    let descriptors = values
        .into_iter()
        .map(|value| value.into_vm(context))
        .collect::<RuntimeResult<Vec<_>>>()?;

    VmSlice::from_values(&mut context.write(), &descriptors)
}

/// Store native input records as one binding array.
pub(crate) fn store_input_records_native(
    binding: &BindingCallContext,
    values: Vec<MidiInputRecordValue>,
) -> NativeArray<MidiInputRecord> {
    let records = values
        .into_iter()
        .map(|value| value.into_native(binding))
        .collect();

    binding.store_array(records)
}

/// Store VM input records as one binding array.
pub(crate) fn store_input_records_vm(
    context: &mut vm::BindingContext<'_>,
    values: Vec<MidiInputRecordValue>,
) -> RuntimeResult<VmArray<MidiInputRecordVm>> {
    let records = values
        .into_iter()
        .map(|value| value.into_vm(context))
        .collect::<RuntimeResult<Vec<_>>>()?;

    VmArray::from_values(&mut context.write(), &records)
}

/// Store one native input record.
pub(crate) fn store_input_record_native(
    binding: &BindingCallContext,
    value: MidiInputRecordValue,
) -> MidiInputRecord {
    value.into_native(binding)
}

/// Store one VM input record.
pub(crate) fn store_input_record_vm(
    context: &mut vm::BindingContext<'_>,
    value: MidiInputRecordValue,
) -> RuntimeResult<MidiInputRecordVm> {
    value.into_vm(context)
}

/// Store native MIDI events as one binding slice.
pub(crate) fn store_events_native(
    binding: &BindingCallContext,
    values: Vec<MidiEventValue>,
) -> NativeSlice<MidiEvent> {
    let events = values
        .into_iter()
        .map(|value| value.into_native(binding))
        .collect();

    binding.store_slice(events)
}

/// Store VM MIDI events as one binding slice.
pub(crate) fn store_events_vm(
    context: &mut vm::BindingContext<'_>,
    values: Vec<MidiEventValue>,
) -> RuntimeResult<VmSlice<MidiEventVm>> {
    let events = values
        .into_iter()
        .map(|value| value.into_vm(context))
        .collect::<RuntimeResult<Vec<_>>>()?;

    VmSlice::from_values(&mut context.write(), &events)
}

/// Store one native MIDI event.
pub(crate) fn store_event_native(binding: &BindingCallContext, value: MidiEventValue) -> MidiEvent {
    value.into_native(binding)
}

/// Store one VM MIDI event.
pub(crate) fn store_event_vm(
    context: &mut vm::BindingContext<'_>,
    value: MidiEventValue,
) -> RuntimeResult<MidiEventVm> {
    value.into_vm(context)
}
