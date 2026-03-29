use std::mem::MaybeUninit;

use crate::diagnostic::RuntimeResult;
use crate::host::abi::media::{
    HostMediaDeleteResponse, HostMediaImportPathResponse, HostMediaListResponse,
    HostMediaReadResponse, decode_media_delete_response, decode_media_import_path_response,
    decode_media_list_response, decode_media_read_response, encode_media_delete_request,
    encode_media_import_path_request, encode_media_list_request,
};
use crate::host::android::abi::media::{
    destack_host_android_media_delete, destack_host_android_media_import_path,
    destack_host_android_media_list, destack_host_android_media_read,
};
use crate::host::core::callback::decode_callback_host_status;
use crate::host::core::{HostRequest, HostRequestOutcome, HostRequestResult};
use crate::platform::abi::NativeStringRef;
use crate::platform::fs::{OsPath, core as core_fs};
use crate::platform::os::MediaAssetKind;
use crate::platform::os::abi_generated::{
    MediaAssetDescriptorValue, MediaPageValue, MediaQueryValue,
};
use crate::runtime::BindingCallContext;

/// Return one Android media request outcome when supported.
pub(crate) fn submit_media_request(
    runtime_id: u64,
    request: &HostRequest,
) -> RuntimeResult<Option<HostRequestOutcome>> {
    match request {
        HostRequest::OsMediaList { query } => {
            let page = submit_media_list(runtime_id, request.operation_name(), query)?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::MediaPage(page),
            )))
        }
        HostRequest::OsMediaRead { id } => {
            let descriptor = submit_media_read(runtime_id, request.operation_name(), id)?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::MediaAssetDescriptor(descriptor),
            )))
        }
        HostRequest::OsMediaImportPath { path, kind } => {
            let id = submit_media_import_path(runtime_id, request.operation_name(), *path, *kind)?;

            Ok(Some(HostRequestOutcome::immediate(
                HostRequestResult::String(id),
            )))
        }
        HostRequest::OsMediaDelete { ids } => {
            let deleted_count = submit_media_delete(runtime_id, request.operation_name(), ids)?;

            Ok(Some(HostRequestOutcome::immediate(HostRequestResult::U32(
                deleted_count,
            ))))
        }
        _ => Ok(None),
    }
}

/// Submit one Android media-list request.
fn submit_media_list(
    runtime_id: u64,
    operation: &'static str,
    query: &MediaQueryValue,
) -> RuntimeResult<MediaPageValue> {
    let binding = BindingCallContext::from_current_agent_for_native()?;
    let request = encode_media_list_request(&binding, query);
    let mut response = MaybeUninit::<HostMediaListResponse>::uninit();
    let status =
        unsafe { destack_host_android_media_list(runtime_id, request, response.as_mut_ptr()) };
    decode_callback_host_status(status, operation)?;

    let response = unsafe { response.assume_init() };

    unsafe { decode_media_list_response(response, operation) }
}

/// Submit one Android media-read request.
fn submit_media_read(
    runtime_id: u64,
    operation: &'static str,
    id: &str,
) -> RuntimeResult<MediaAssetDescriptorValue> {
    let mut response = MaybeUninit::<HostMediaReadResponse>::uninit();
    let status = unsafe {
        destack_host_android_media_read(
            runtime_id,
            NativeStringRef::from(id),
            response.as_mut_ptr(),
        )
    };
    decode_callback_host_status(status, operation)?;

    let response = unsafe { response.assume_init() };

    unsafe { decode_media_read_response(response, operation) }
}

/// Submit one Android media-import request.
fn submit_media_import_path(
    runtime_id: u64,
    operation: &'static str,
    path: OsPath,
    kind: MediaAssetKind,
) -> RuntimeResult<String> {
    let binding = BindingCallContext::from_current_agent_for_native()?;
    let path = core_fs::os_path_to_utf8_string(path, "path")?;
    let request = encode_media_import_path_request(&binding, &path, kind);
    let mut response = MaybeUninit::<HostMediaImportPathResponse>::uninit();
    let status = unsafe {
        destack_host_android_media_import_path(runtime_id, request, response.as_mut_ptr())
    };
    decode_callback_host_status(status, operation)?;

    let response = unsafe { response.assume_init() };

    unsafe { decode_media_import_path_response(response, operation) }
}

/// Submit one Android media-delete request.
fn submit_media_delete(
    runtime_id: u64,
    operation: &'static str,
    ids: &[String],
) -> RuntimeResult<u32> {
    let binding = BindingCallContext::from_current_agent_for_native()?;
    let request = encode_media_delete_request(&binding, ids);
    let mut response = MaybeUninit::<HostMediaDeleteResponse>::uninit();
    let status =
        unsafe { destack_host_android_media_delete(runtime_id, request, response.as_mut_ptr()) };
    decode_callback_host_status(status, operation)?;

    let response = unsafe { response.assume_init() };

    Ok(decode_media_delete_response(response))
}
