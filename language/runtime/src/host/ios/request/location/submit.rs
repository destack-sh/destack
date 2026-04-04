use std::mem::MaybeUninit;

use crate::diagnostic::RuntimeResult;
use crate::host::abi::location::{LocationLastKnownResponse, LocationServicesResponse};
use crate::host::core::callback::decode_callback_host_status;
use crate::host::core::error::invalid_argument_value;
use crate::host::os::apple::abi::location::{
    destack_host_ios_location_last_known, destack_host_ios_location_services_enabled,
    destack_host_ios_location_watch_close, destack_host_ios_location_watch_open,
};
use crate::host::{HostRequest, HostRequestOutcome, HostRequestResult};
use crate::platform::abi::NativeStringRef;

/// Return one iOS location request outcome when supported.
pub(crate) fn submit_location_request(
    runtime_id: u64,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    match request {
        HostRequest::OsLocationServicesEnabled => {
            let mut response = MaybeUninit::<LocationServicesResponse>::uninit();
            let call_status = unsafe {
                destack_host_ios_location_services_enabled(runtime_id, response.as_mut_ptr())
            };
            decode_callback_host_status(call_status, request.operation_name())?;
            let response = unsafe { response.assume_init() };
            decode_callback_host_status(response.status, request.operation_name())?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::Bool(response.is_enabled),
            )))
        }
        HostRequest::OsLocationLastKnown => {
            let mut response = MaybeUninit::<LocationLastKnownResponse>::uninit();
            let call_status =
                unsafe { destack_host_ios_location_last_known(runtime_id, response.as_mut_ptr()) };
            decode_callback_host_status(call_status, request.operation_name())?;
            let response = unsafe { response.assume_init() };
            decode_callback_host_status(response.status, request.operation_name())?;
            let Some(sample) = response.sample else {
                return Err(invalid_argument_value(
                    "response.sample",
                    format!(
                        "{} returned success without one location sample",
                        request.operation_name()
                    ),
                ));
            };

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::LocationSample(sample),
            )))
        }
        HostRequest::OsLocationWatchOpen { watch_id, options } => {
            let status = unsafe {
                destack_host_ios_location_watch_open(
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
                destack_host_ios_location_watch_close(runtime_id, NativeStringRef::from(watch_id))
            };
            decode_callback_host_status(status, request.operation_name())?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }
        _ => Ok(None),
    }
}
