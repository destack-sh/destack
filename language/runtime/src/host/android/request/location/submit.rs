use super::{
    destack_host_android_location_last_known, destack_host_android_location_services_enabled,
    destack_host_android_location_watch_close, destack_host_android_location_watch_open,
};
use crate::diagnostic::RuntimeResult;
use crate::host::callback::decode_callback_host_status;
use crate::host::core::{HostRequest, HostRequestOutcome, HostRequestResult};
use crate::platform::os::{LocationSample, LocationWatchOptions};
use crate::runtime::NativeStringRef;

/// Return one Android location request outcome when supported.
pub(crate) fn submit_location_request(
    runtime_id: u64,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    match request {
        HostRequest::OsLocationServicesEnabled => {
            let mut is_enabled = false;
            let status = unsafe {
                destack_host_android_location_services_enabled(runtime_id, &mut is_enabled)
            };
            decode_callback_host_status(status, request.operation_name())?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::Bool(is_enabled),
            )))
        }
        HostRequest::OsLocationLastKnown => {
            let mut sample = LocationSample {
                latitude_degrees: 0.0,
                longitude_degrees: 0.0,
                altitude_meters: None,
                horizontal_accuracy_meters: None,
                vertical_accuracy_meters: None,
                speed_meters_per_second: None,
                heading_degrees: None,
                timestamp_unix_ns: 0,
            };
            let status =
                unsafe { destack_host_android_location_last_known(runtime_id, &mut sample) };
            decode_callback_host_status(status, request.operation_name())?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::LocationSample(sample),
            )))
        }
        HostRequest::OsLocationWatchOpen { watch_id, options } => {
            let status = unsafe {
                destack_host_android_location_watch_open(
                    runtime_id,
                    NativeStringRef::from(watch_id),
                    *options,
                )
            };
            decode_callback_host_status(status, request.operation_name())?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }
        HostRequest::OsLocationWatchClose { watch_id } => {
            let status = unsafe {
                destack_host_android_location_watch_close(
                    runtime_id,
                    NativeStringRef::from(watch_id),
                )
            };
            decode_callback_host_status(status, request.operation_name())?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }
        _ => Ok(None),
    }
}
