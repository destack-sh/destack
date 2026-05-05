use crate::diagnostic::RuntimeResult;
use crate::platform::os::{LocationSample, LocationSampleVm, LocationWatchOptions, state};
use crate::platform::{VmAbiCodec, resource};
use crate::runtime::BindingCallContext;
use destack_vm as vm;

/// Read the most recent location sample through runtime-owned OS state.
pub(crate) fn last_known(binding: &BindingCallContext) -> RuntimeResult<LocationSample> {
    state::location_last_known(binding)
}

/// Read whether host location services are enabled.
pub(crate) fn services_enabled(binding: &BindingCallContext) -> RuntimeResult<bool> {
    state::location_services_enabled(binding)
}

/// Open one location watch stream.
pub(crate) fn watch_open(
    binding: &BindingCallContext,
    options: LocationWatchOptions,
) -> RuntimeResult<resource::LocationWatchHandle> {
    state::location_watch_open(binding, options)
}

/// Close one location watch stream.
pub(crate) fn watch_close(
    binding: &BindingCallContext,
    handle: resource::LocationWatchHandle,
) -> RuntimeResult<()> {
    state::location_watch_close(binding, handle)
}

/// Wait for one location sample.
pub(crate) fn watch_read(
    binding: &BindingCallContext,
    handle: resource::LocationWatchHandle,
    timeout_ns: u64,
) -> RuntimeResult<LocationSample> {
    state::location_watch_read(binding, handle, timeout_ns)
}

/// Poll one location sample without blocking.
pub(crate) fn watch_try_read(
    binding: &BindingCallContext,
    handle: resource::LocationWatchHandle,
) -> RuntimeResult<LocationSample> {
    state::location_watch_try_read(binding, handle)
}

/// Encode one location sample into one VM value.
pub(crate) fn sample_vm(
    context: &mut vm::BindingContext<'_>,
    sample: LocationSample,
) -> RuntimeResult<LocationSampleVm> {
    LocationSampleVm::from_value(&mut context.write(), sample)
}
