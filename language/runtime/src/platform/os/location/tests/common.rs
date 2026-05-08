use destack_vm as vm;
use destack_workspace::{AppPermission, RuntimeOptions};

use crate::diagnostic::RuntimeResult;
use crate::platform::os::abi_generated::LocationSampleValue;
use crate::platform::os::tests::{HarnessContext, HarnessValue};
use crate::platform::os::{
    LocationAccuracy, LocationSample, LocationSampleVm, LocationWatchOptions,
};
use crate::platform::{NativeAbiCodec, VmAbiCodec};

/// Enable one location declaration so backend tests reach the host boundary.
pub(super) fn enable_location_declaration(options: &mut RuntimeOptions) {
    options
        .app
        .permissions
        .insert(AppPermission::Location, Default::default());
}

/// Build one location watch-open payload for the active harness.
pub(super) fn location_watch_options(
    context: &HarnessContext<'_>,
) -> HarnessValue<LocationWatchOptions, LocationWatchOptions> {
    let options = LocationWatchOptions {
        accuracy: LocationAccuracy::Balanced,
        minimum_interval_ns: 1_000_000_000,
        minimum_distance_meters: 5.0,
        include_heading: false,
    };

    if context.vm_context.is_some() {
        return context.harness_value_vm(options);
    }

    context.harness_value(options)
}

/// Decode one location sample payload from the active harness.
pub(super) fn decode_location_sample(
    context: &mut HarnessContext<'_>,
    value: HarnessValue<LocationSample, LocationSampleVm>,
) -> RuntimeResult<LocationSampleValue> {
    match value {
        HarnessValue::Native(value) => unsafe {
            <LocationSample as NativeAbiCodec>::into_value(value)
        },
        HarnessValue::Vm(value) => {
            let vm_context = unsafe {
                &mut *(context
                    .vm_context
                    .expect("vm context should exist for vm harness")
                    as *mut vm::BindingContext<'_>)
            };

            <LocationSampleVm as VmAbiCodec>::into_value(value, &vm_context.read())
        }
    }
}

/// Build one representative location sample payload.
pub(super) fn test_location_sample() -> LocationSampleValue {
    LocationSampleValue {
        latitude_degrees: 47.3769,
        longitude_degrees: 8.5417,
        altitude_meters: 408.0,
        horizontal_accuracy_meters: 12.0,
        vertical_accuracy_meters: 18.0,
        speed_meters_per_second: 2.5,
        heading_degrees: 180.0,
        timestamp_unix_ns: 123_000_000_000,
    }
}
