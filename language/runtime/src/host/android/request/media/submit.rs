use super::{
    destack_host_android_media_delete, destack_host_android_media_describe,
    destack_host_android_media_import_path, destack_host_android_media_list,
};
use crate::diagnostic::RuntimeResult;
use crate::host::callback::{
    decode_callback_host_status, encode_callback_host_json, read_buffered_callback_host_json,
    read_buffered_callback_host_string,
};
use crate::host::core::{HostRequest, HostRequestOutcome, HostRequestResult};
use crate::platform::core::invalid_argument_value;
use crate::platform::fs::core as core_fs;
use crate::platform::os::MediaAssetKind;
use crate::platform::os::abi_generated::{
    MediaAssetDescriptorValue, MediaPageValue, MediaQueryValue,
};
use crate::runtime::{NativeSlice, NativeStringRef};

/// Convert one callback payload length into ABI width.
fn callback_payload_length(length: usize, operation: &'static str) -> RuntimeResult<u32> {
    u32::try_from(length).map_err(|_| {
        invalid_argument_value(
            "payload",
            format!("{operation} payload exceeded u32 callback ABI width"),
        )
    })
}

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
            let descriptor = submit_media_describe(runtime_id, request.operation_name(), id)?;

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
    let payload = encode_callback_host_json(query, operation)?;
    let payload_length = callback_payload_length(payload.len(), operation)?;
    let payload = NativeSlice {
        data: payload.as_ptr() as *mut u8,
        len: payload_length,
    };

    read_buffered_callback_host_json(operation, "media page", |output, output_written| unsafe {
        destack_host_android_media_list(runtime_id, payload, output, output_written)
    })
}

/// Submit one Android media-describe request.
fn submit_media_describe(
    runtime_id: u64,
    operation: &'static str,
    id: &str,
) -> RuntimeResult<MediaAssetDescriptorValue> {
    read_buffered_callback_host_json(
        operation,
        "media asset descriptor",
        |output, output_written| unsafe {
            destack_host_android_media_describe(
                runtime_id,
                NativeStringRef::from(id),
                output,
                output_written,
            )
        },
    )
}

/// Submit one Android media-import request.
fn submit_media_import_path(
    runtime_id: u64,
    operation: &'static str,
    path: crate::platform::fs::OsPath,
    kind: MediaAssetKind,
) -> RuntimeResult<String> {
    let path = core_fs::os_path_to_utf8_string(path, "path")?;

    read_buffered_callback_host_string(operation, |output_id, output_written| unsafe {
        destack_host_android_media_import_path(
            runtime_id,
            NativeStringRef::from(&path),
            kind as i32,
            output_id,
            output_written,
        )
    })
}

/// Submit one Android media-delete request.
fn submit_media_delete(
    runtime_id: u64,
    operation: &'static str,
    ids: &[String],
) -> RuntimeResult<u32> {
    let payload = encode_callback_host_json(ids, operation)?;
    let payload_length = callback_payload_length(payload.len(), operation)?;
    let payload = NativeSlice {
        data: payload.as_ptr() as *mut u8,
        len: payload_length,
    };
    let mut deleted_count = 0_u32;
    let status =
        unsafe { destack_host_android_media_delete(runtime_id, payload, &mut deleted_count) };
    decode_callback_host_status(status, operation)?;

    Ok(deleted_count)
}
