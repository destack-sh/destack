use super::{
    destack_host_android_intent_can_open_url, destack_host_android_intent_open_path,
    destack_host_android_intent_open_url, destack_host_android_intent_share_paths,
    destack_host_android_intent_share_text,
};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::core::error::not_supported;
use crate::host::core::{
    HOST_STATUS_BUFFER_TOO_SMALL, HOST_STATUS_FAILED, HOST_STATUS_INVALID_ARGUMENT,
    HOST_STATUS_NOT_FOUND, HOST_STATUS_NOT_SUPPORTED, HOST_STATUS_OK,
    HOST_STATUS_PERMISSION_DENIED, HostRequest, HostRequestOutcome, HostRequestResult,
};
use crate::platform::diagnostic::{PlatformError, PlatformErrorCode};
use crate::platform::fs::core as core_fs;
use crate::runtime::{NativeStringRef, NativeStringSlice};

/// Return one Android intent request outcome when supported.
pub(crate) fn submit_intent_request(
    runtime_id: u64,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    match request {
        HostRequest::OsIntentCanOpenUrl { url } => {
            let mut is_supported = false;
            let status = unsafe {
                destack_host_android_intent_can_open_url(
                    runtime_id,
                    NativeStringRef::from(url),
                    &mut is_supported,
                )
            };
            decode_host_status(status, request.operation_name())?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::Bool(is_supported),
            )))
        }
        HostRequest::OsIntentOpenUrl { url } => {
            let status = unsafe {
                destack_host_android_intent_open_url(runtime_id, NativeStringRef::from(url))
            };
            decode_host_status(status, request.operation_name())?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }
        HostRequest::OsIntentOpenPath { path } => {
            let path = core_fs::os_path_to_utf8_string(*path, "path")?;
            let status = unsafe {
                destack_host_android_intent_open_path(runtime_id, NativeStringRef::from(&path))
            };
            decode_host_status(status, request.operation_name())?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }
        HostRequest::OsIntentShareText { text, content_type } => {
            let status = unsafe {
                destack_host_android_intent_share_text(
                    runtime_id,
                    NativeStringRef::from(text),
                    content_type.is_some(),
                    content_type
                        .as_ref()
                        .map_or(NativeStringRef::from(""), NativeStringRef::from),
                )
            };
            decode_host_status(status, request.operation_name())?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }
        HostRequest::OsIntentSharePaths {
            paths,
            content_type,
        } => {
            let path_strings = paths
                .iter()
                .map(|path| core_fs::os_path_to_utf8_string(*path, "path"))
                .collect::<RuntimeResult<Vec<_>>>()?;
            let path_refs = path_strings
                .iter()
                .map(NativeStringRef::from)
                .collect::<Vec<_>>();
            let status = unsafe {
                destack_host_android_intent_share_paths(
                    runtime_id,
                    NativeStringSlice::from_slice(&path_refs),
                    content_type.is_some(),
                    content_type
                        .as_ref()
                        .map_or(NativeStringRef::from(""), NativeStringRef::from),
                )
            };
            decode_host_status(status, request.operation_name())?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::None)))
        }
        _ => Ok(None),
    }
}

/// Decode one Android host status code into one runtime result.
fn decode_host_status(status: u32, operation: &'static str) -> RuntimeResult<()> {
    match status {
        HOST_STATUS_OK => Ok(()),
        HOST_STATUS_NOT_SUPPORTED => Err(not_supported(operation)),
        HOST_STATUS_INVALID_ARGUMENT => {
            Err(RuntimeError::from(PlatformError::invalid_argument_value(
                "request",
                format!("{operation} rejected one invalid argument"),
            ))
            .boxed())
        }
        HOST_STATUS_NOT_FOUND => Err(RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::Generic),
            format!("{operation} could not resolve one runtime-scoped Android request binding"),
        ))
        .boxed()),
        HOST_STATUS_PERMISSION_DENIED => Err(RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::IoPermissionDenied),
            format!("{operation} was denied by the Android host"),
        ))
        .boxed()),
        HOST_STATUS_BUFFER_TOO_SMALL => Err(RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::Generic),
            format!("{operation} reported one unexpected buffer negotiation"),
        ))
        .boxed()),
        HOST_STATUS_FAILED => Err(RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::Generic),
            format!("{operation} failed in the Android host"),
        ))
        .boxed()),
        _ => Err(RuntimeError::from(PlatformError::generic(
            Some(PlatformErrorCode::Generic),
            format!("{operation} returned one unknown Android host status"),
        ))
        .boxed()),
    }
}
