use crate::diagnostic::RuntimeResult;
use crate::platform::os::abi_generated::{
    BackgroundEventOpenOptionsValue, BackgroundEventValue, BackgroundStatusValue,
    BackgroundTaskDescriptorValue, BackgroundTaskOptionsValue, BackgroundTaskResultValue,
};
use crate::platform::os::{BackgroundEventVm, BackgroundTaskDescriptorVm, state};
use crate::platform::{VmAbiCodec, VmArray, resource};
use crate::runtime::BindingCallContext;
use destack_vm as vm;

/// Read background scheduler status through runtime-owned OS state.
pub(crate) fn status(binding: &BindingCallContext) -> RuntimeResult<BackgroundStatusValue> {
    state::background_status(binding)
}

/// List background task registrations through runtime-owned OS state.
pub(crate) fn list(
    binding: &BindingCallContext,
) -> RuntimeResult<Vec<BackgroundTaskDescriptorValue>> {
    state::background_list(binding)
}

/// Register one background task through runtime-owned OS state.
pub(crate) fn register(
    binding: &BindingCallContext,
    options: BackgroundTaskOptionsValue,
) -> RuntimeResult<()> {
    state::background_register(binding, options)
}

/// Unregister one background task through runtime-owned OS state.
pub(crate) fn unregister(binding: &BindingCallContext, identifier: &str) -> RuntimeResult<()> {
    state::background_unregister(binding, identifier.to_string())
}

/// Trigger one background task through runtime-owned OS state.
pub(crate) fn trigger_test(binding: &BindingCallContext, identifier: &str) -> RuntimeResult<bool> {
    state::background_trigger_test(binding, identifier.to_string())
}

/// Complete one background task execution through runtime-owned OS state.
pub(crate) fn complete(
    binding: &BindingCallContext,
    execution_id: &str,
    result: BackgroundTaskResultValue,
) -> RuntimeResult<()> {
    state::background_complete(binding, execution_id.to_string(), result)
}

/// Open one background event stream.
pub(crate) fn event_open(
    binding: &BindingCallContext,
    options: BackgroundEventOpenOptionsValue,
) -> RuntimeResult<resource::BackgroundEventHandle> {
    state::background_event_open(binding, options)
}

/// Close one background event stream.
pub(crate) fn event_close(
    binding: &BindingCallContext,
    handle: resource::BackgroundEventHandle,
) -> RuntimeResult<()> {
    state::background_event_close(binding, handle)
}

/// Wait for one background event.
pub(crate) fn event_read(
    binding: &BindingCallContext,
    handle: resource::BackgroundEventHandle,
    timeout_ns: u64,
) -> RuntimeResult<BackgroundEventValue> {
    state::background_event_read(binding, handle, timeout_ns)
}

/// Poll one background event without blocking.
pub(crate) fn event_try_read(
    binding: &BindingCallContext,
    handle: resource::BackgroundEventHandle,
) -> RuntimeResult<BackgroundEventValue> {
    state::background_event_try_read(binding, handle)
}

/// Encode one background task descriptor list into one VM array.
pub(crate) fn list_vm(
    context: &mut vm::BindingContext<'_>,
    descriptors: &[BackgroundTaskDescriptorValue],
) -> RuntimeResult<VmArray<BackgroundTaskDescriptorVm>> {
    let mut encoded_descriptors = Vec::with_capacity(descriptors.len());

    // encode one descriptor per registered background task
    for descriptor in descriptors {
        let encoded_descriptor =
            BackgroundTaskDescriptorVm::from_value(&mut context.write(), descriptor.clone())?;
        encoded_descriptors.push(encoded_descriptor);
    }

    VmArray::from_values(&mut context.write(), &encoded_descriptors)
}

/// Encode one background event into one VM value.
pub(crate) fn event_vm(
    context: &mut vm::BindingContext<'_>,
    event: BackgroundEventValue,
) -> RuntimeResult<BackgroundEventVm> {
    BackgroundEventVm::from_value(&mut context.write(), event)
}
